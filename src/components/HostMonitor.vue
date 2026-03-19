<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

const props = defineProps<{
  session: any;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();


interface TopProc {
  pid: string;
  cpu: string;
  name: string;
}

interface MonitorStats {
  os: string; hostname: string; uptimeRaw: string; load1: number;
  cpuUsed: number;
  memTotal: number; memUsed: number; memPct: number;
  diskTotal: number; diskUsed: number; diskPct: number;
  netRxBps?: number; netTxBps?: number;
  top5Procs?: TopProc[];
}

const monitorLoading = ref(true);
const monitorStats = ref<MonitorStats | null>(null);
let monitorTimer: number | null = null;
const isVisible = ref(true);

function formatUptime(rawStr: string | undefined): string {
  if (!rawStr) return '-';
  const match = rawStr.match(/up\s+(.*?)(,\s+\d+\s+user|,?\s+load average)/);
  if (!match) return rawStr;
  
  let upStr = match[1].trim(); 
  upStr = upStr.replace(/days?/g, '天');
  upStr = upStr.replace(/min(utes?)?/g, '分钟');
  upStr = upStr.replace(/hours?/g, '小时');
  
  upStr = upStr.replace(/(\d+):(\d+)/, (_, h, min) => {
    return `${parseInt(h, 10)} 小时 ${parseInt(min, 10)} 分钟`;
  });
  
  return upStr.replace(/,\s*/g, ' ');
}

function formatNetworkSpeed(bytes: number): string {
  if (bytes === 0 || isNaN(bytes)) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function parseKV(raw: string): Record<string, string> {
  const kv: Record<string, string> = {};
  for (const line of raw.split('\n')) {
    const idx = line.indexOf('=');
    if (idx > 0) kv[line.slice(0, idx).trim()] = line.slice(idx + 1).trim();
  }
  return kv;
}

async function doFetchStats(showSpinner: boolean) {
  if (!props.session?.host || !isVisible.value) return;
  if (showSpinner) monitorLoading.value = true;
  try {
    const host = props.session.host;
    const raw = await invoke<string>('get_host_stats', {
      host: host.host, port: host.port,
      username: host.username,
      password: host.password || null,
      privateKey: host.private_key || null,
    });
    if (!isVisible.value) return;
    
    const kv = parseKV(raw);
    const num = (k: string) => parseFloat(kv[k] || '0') || 0;
    const memTotal = num('MEM_TOTAL'), memUsed = num('MEM_USED');
    const diskTotal = num('DISK_TOTAL'), diskUsed = num('DISK_USED');
    const netRxBps = num('NET_RX_BPS');
    const netTxBps = num('NET_TX_BPS');
    
    const top5Raw = kv['TOP5_PROCS'] || '';
    const top5Procs: TopProc[] = top5Raw.split(';').filter(Boolean).map(item => {
      const parts = item.split('|');
      return { pid: parts[0] || '?', cpu: parts[1] || '0', name: parts[2] || '?' };
    });

    monitorStats.value = {
      os: kv['OS'] || 'N/A',
      hostname: kv['HOSTNAME'] || host.host,
      uptimeRaw: kv['UPTIME_RAW'] || '',
      load1: num('LOAD1'),
      cpuUsed: num('CPU_USED'),
      memTotal, memUsed,
      memPct: memTotal > 0 ? Math.round(memUsed / memTotal * 100) : 0,
      diskTotal, diskUsed,
      diskPct: num('DISK_PCT'),
      netRxBps,
      netTxBps,
      top5Procs
    };
  } catch (e: any) {
    if (isVisible.value) {
      monitorStats.value = { 
        os: `获取失败: ${e}`, hostname: '', uptimeRaw: '', load1: 0, 
        cpuUsed: 0, memTotal: 0, memUsed: 0, memPct: 0, 
        diskTotal: 0, diskUsed: 0, diskPct: 0 
      };
    }
  } finally {
    if (showSpinner) monitorLoading.value = false;
  }
}

function handleRefresh() {
  doFetchStats(true);
}

function handleClose() {
  isVisible.value = false;
  if (monitorTimer !== null) {
    window.clearInterval(monitorTimer);
    monitorTimer = null;
  }
  emit('close');
}

onMounted(async () => {
  await doFetchStats(true);
  monitorTimer = window.setInterval(() => {
    if (!isVisible.value) {
      if (monitorTimer) window.clearInterval(monitorTimer);
      monitorTimer = null;
      return;
    }
    doFetchStats(false);
  }, 3000);
});

onUnmounted(() => {
  isVisible.value = false;
  if (monitorTimer) {
    window.clearInterval(monitorTimer);
    monitorTimer = null;
  }
});
</script>

<template>
  <div class="monitor-overlay" @click.self="handleClose">
    <div class="monitor-modal">
      <!-- Header -->
      <div class="monitor-header">
        <div class="monitor-title-area">
          <div class="monitor-icon-wrap">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" width="20" height="20">
              <rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8m-4-4v4"/><polyline points="6 9 10 13 14 8 18 12"/>
            </svg>
          </div>
          <div>
            <div class="monitor-modal-title">主机监控</div>
            <div class="monitor-modal-sub" v-if="monitorStats">
              {{ monitorStats.hostname }} · {{ monitorStats.os }}
            </div>
            <div class="monitor-modal-sub" v-else-if="props.session?.host">
              {{ props.session.host.host }}
            </div>
          </div>
        </div>
        <button class="monitor-close" @click="handleClose">×</button>
      </div>

      <!-- Loading -->
      <div v-if="monitorLoading" class="monitor-loading">
        <div class="monitor-spinner"></div>
        <span>正在连接并采集数据...</span>
      </div>

      <!-- Dashboard -->
      <div v-else-if="monitorStats" class="monitor-dash">
        <!-- Row 1: CPU + Uptime -->
        <div class="dash-row">
          <!-- CPU Gauge -->
          <div class="dash-card dash-cpu">
            <div class="dash-card-label">CPU 使用率</div>
            <div class="cpu-gauge-wrap">
              <svg class="cpu-gauge" viewBox="0 0 120 80" width="120" height="80">
                <!-- Background arc -->
                <path d="M 15 75 A 50 50 0 0 1 105 75" fill="none" stroke="#e8edff" stroke-width="10" stroke-linecap="round"/>
                <!-- Value arc -->
                <path
                  d="M 15 75 A 50 50 0 0 1 105 75"
                  fill="none"
                  :stroke="monitorStats.cpuUsed > 80 ? '#ef4444' : monitorStats.cpuUsed > 60 ? '#f59e0b' : '#6366f1'"
                  stroke-width="10"
                  stroke-linecap="round"
                  :stroke-dasharray="`${monitorStats.cpuUsed * 1.571} 157.1`"
                />
                <text x="60" y="68" text-anchor="middle" font-size="20" font-weight="800" :fill="monitorStats.cpuUsed > 80 ? '#ef4444' : '#1a1a2e'">{{ monitorStats.cpuUsed.toFixed(1) }}%</text>
              </svg>
            </div>
            <div class="cpu-gauge-hint">1分钟负载: <strong>{{ monitorStats.load1 }}</strong></div>
          </div>

          <!-- Uptime / Info -->
          <div class="dash-card dash-info">
            <div class="dash-card-label">运行时长</div>
            <div class="uptime-text">{{ formatUptime(monitorStats.uptimeRaw) }}</div>
          </div>
        </div>

        <!-- Row 2: Memory + Disk -->
        <div class="dash-row">
          <!-- Memory -->
          <div class="dash-card">
            <div class="dash-card-label">
              内存
              <span class="dash-pct-badge" :class="{ warn: monitorStats.memPct > 80 }">{{ monitorStats.memPct }}%</span>
            </div>
            <div class="bar-wrap">
              <div class="bar-bg">
                <div
                  class="bar-fill"
                  :class="{ 'bar-warn': monitorStats.memPct > 80 }"
                  :style="{ width: monitorStats.memPct + '%' }"
                ></div>
              </div>
            </div>
            <div class="bar-meta">
              <span>已用 {{ (monitorStats.memUsed / 1024).toFixed(1) }} GB</span>
              <span>共 {{ (monitorStats.memTotal / 1024).toFixed(1) }} GB</span>
            </div>
          </div>

          <!-- Disk -->
          <div class="dash-card">
            <div class="dash-card-label">
              磁盘 (/)
              <span class="dash-pct-badge" :class="{ warn: monitorStats.diskPct > 80 }">{{ monitorStats.diskPct }}%</span>
            </div>
            <div class="bar-wrap">
              <div class="bar-bg">
                <div
                  class="bar-fill bar-disk"
                  :class="{ 'bar-warn': monitorStats.diskPct > 80 }"
                  :style="{ width: monitorStats.diskPct + '%' }"
                ></div>
              </div>
            </div>
            <div class="bar-meta">
              <span>已用 {{ (monitorStats.diskUsed / 1024).toFixed(1) }} GB</span>
              <span>共 {{ (monitorStats.diskTotal / 1024).toFixed(1) }} GB</span>
            </div>
          </div>
        </div>

        <!-- Row 3: Network + Top Processes -->
        <div class="dash-row">
          <!-- Network -->
          <div class="dash-card">
            <div class="dash-card-label">实时网速</div>
            <div class="net-stats">
              <div class="net-item">
                <span class="net-icon tx">↑</span>
                <span class="net-value">{{ formatNetworkSpeed(monitorStats.netTxBps || 0) }}/s</span>
              </div>
              <div class="net-item">
                <span class="net-icon rx">↓</span>
                <span class="net-value">{{ formatNetworkSpeed(monitorStats.netRxBps || 0) }}/s</span>
              </div>
            </div>
          </div>
          
          <!-- Top CPU Processes -->
          <div class="dash-card dash-procs">
            <div class="dash-card-label">最高能耗进程 (TOP 5)</div>
            <div class="proc-list" v-if="monitorStats.top5Procs && monitorStats.top5Procs.length">
              <div class="proc-header">
                <span style="width: 60px">PID</span>
                <span style="flex: 1; padding-left: 8px">COMMAND</span>
                <span style="width: 55px; text-align: right">%CPU</span>
              </div>
              <div class="proc-item" v-for="p in monitorStats.top5Procs" :key="p.pid">
                <span class="proc-pid">{{ p.pid }}</span>
                <span class="proc-name" :title="p.name">{{ p.name }}</span>
                <span class="proc-cpu">{{ p.cpu }}%</span>
              </div>
            </div>
            <div v-else class="proc-empty">暂无数据 / 权限不足</div>
          </div>
        </div>
      </div>

      <div class="monitor-footer">
        <button class="monitor-refresh-btn" @click="handleRefresh" :disabled="monitorLoading">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13">
            <path d="M1 4v6h6"/><path d="M23 20v-6h-6"/><path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4-4.64 4.36A9 9 0 0 1 3.51 15"/>
          </svg>
          刷新数据
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* ===== 监控快照 Modal CSS ===== */
.monitor-overlay {
  position: fixed; inset: 0; z-index: 300;
  background: rgba(0,0,0,0.4); backdrop-filter: blur(6px);
  display: flex; align-items: center; justify-content: center;
}
.monitor-modal {
  background: #fff; border-radius: 18px;
  width: 560px; max-width: 94vw; max-height: 80vh;
  box-shadow: 0 28px 80px rgba(0,0,0,0.22);
  display: flex; flex-direction: column; overflow: hidden;
}
.monitor-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 18px 22px; border-bottom: 1px solid #eff0f8;
  background: linear-gradient(120deg, #f8f9ff 0%, #eef2ff 100%);
}
.monitor-title-area { display: flex; align-items: center; gap: 12px; }
.monitor-icon-wrap {
  width: 40px; height: 40px; border-radius: 12px;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  display: flex; align-items: center; justify-content: center;
  color: white; flex-shrink: 0;
}
.monitor-modal-title { font-size: 16px; font-weight: 800; color: #1a1a2e; }
.monitor-modal-sub { font-size: 11px; color: #6b7280; margin-top: 1px; }
.monitor-close {
  background: none; border: none; font-size: 22px; cursor: pointer;
  color: #9ca3af; line-height: 1; padding: 2px 4px; border-radius: 6px;
  transition: all 0.15s;
}
.monitor-close:hover { background: #f3f4f6; color: #374151; }

.monitor-loading {
  display: flex; flex-direction: column; align-items: center; gap: 14px;
  padding: 50px; color: #6b7280; font-size: 13px;
}
.monitor-spinner {
  width: 34px; height: 34px; border: 3px solid #e5e7eb;
  border-top-color: #6366f1; border-radius: 50%; animation: spin 0.8s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

.monitor-dash { padding: 16px; display: flex; flex-direction: column; gap: 12px; overflow-y: auto; }
.dash-row { display: flex; gap: 12px; }
.dash-card {
  flex: 1 1 0%; min-width: 0; background: #f8faff; border: 1.5px solid #e8edff;
  border-radius: 14px; padding: 16px; display: flex; flex-direction: column; gap: 8px;
  transition: box-shadow 0.15s;
}
.dash-card:hover { box-shadow: 0 6px 20px rgba(99,102,241,0.1); }
.dash-card-label {
  font-size: 10px; font-weight: 800; text-transform: uppercase;
  letter-spacing: 0.07em; color: #9ca3af;
  display: flex; align-items: center; justify-content: space-between;
}
.dash-pct-badge {
  font-size: 12px; font-weight: 700; color: #6366f1;
  background: #eef2ff; border-radius: 6px; padding: 1px 8px;
}
.dash-pct-badge.warn { color: #ef4444; background: #fef2f2; }

/* CPU gauge */
.dash-cpu { align-items: center; }
.cpu-gauge-wrap { display: flex; justify-content: center; }
.cpu-gauge-hint { font-size: 11px; color: #6b7280; text-align: center; }
.cpu-gauge-hint strong { color: #374151; }

/* Memory / Disk bars */
.bar-wrap { padding: 4px 0; }
.bar-bg {
  height: 10px; background: #e8edff; border-radius: 99px; overflow: hidden;
}
.bar-fill {
  height: 100%; border-radius: 99px;
  background: linear-gradient(90deg, #6366f1, #8b5cf6);
  transition: width 0.6s cubic-bezier(.2,.8,.4,1);
}
.bar-fill.bar-disk { background: linear-gradient(90deg, #06b6d4, #3b82f6); }
.bar-fill.bar-warn { background: linear-gradient(90deg, #f59e0b, #ef4444); }
.bar-meta {
  display: flex; justify-content: space-between;
  font-size: 11px; color: #6b7280;
}

.net-stats { display: flex; flex-direction: column; gap: 16px; margin-top: 14px; }
.net-item { display: flex; align-items: center; gap: 12px; background: #f8f9fa; padding: 10px 14px; border-radius: 8px; }
.net-icon { display: flex; align-items: center; justify-content: center; width: 24px; height: 24px; border-radius: 4px; font-weight: bold; font-size: 14px; }
.net-icon.tx { background: #fef3c7; color: #f59e0b; }
.net-icon.rx { background: #d1fae5; color: #10b981; }
.net-value { font-size: 15px; font-weight: 600; color: #333; letter-spacing: 0.5px; }

.proc-list { display: flex; flex-direction: column; gap: 6px; margin-top: 12px; }
.proc-header { display: flex; justify-content: space-between; align-items: center; font-size: 11px; color: #999; padding: 0 8px 4px 8px; border-bottom: 1px solid #eee; margin-bottom: 2px; }
.proc-item { display: flex; justify-content: space-between; align-items: center; gap: 8px; font-size: 13px; padding: 6px 8px; background: #f8f9fa; border-radius: 6px; transition: background 0.2s; }
.proc-item:hover { background: #eef2f6; }
.proc-pid { width: 60px; color: #777; font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace; }
.proc-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #4f46e5; font-weight: 500; }
.proc-cpu { width: 55px; text-align: right; font-weight: 700; color: #ef4444; font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace; }
.proc-empty { font-size: 13px; color: #777; margin-top: 16px; text-align: center; }

.uptime-text { font-size: 12px; color: #374151; font-family: 'JetBrains Mono', monospace; white-space: pre-wrap; line-height: 1.6; word-break: break-all; }

.monitor-footer { padding: 12px 20px; border-top: 1px solid #eff0f8; display: flex; justify-content: flex-end; }
.monitor-refresh-btn { display: flex; align-items: center; gap: 6px; padding: 7px 20px; background: #6366f1; color: white; border: none; border-radius: 9px; font-size: 13px; font-weight: 700; cursor: pointer; transition: background 0.15s; }
.monitor-refresh-btn:hover:not(:disabled) { background: #4f46e5; }
.monitor-refresh-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
