<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const props = defineProps<{
  session: any;
  isActive: boolean;
}>();

const MAX_POINTS = 20;

// Data structure for the sparklines
const cpuHistory = ref<number[]>(new Array(MAX_POINTS).fill(0));
const memHistory = ref<number[]>(new Array(MAX_POINTS).fill(0));

const currentCpu = ref(0);
const currentMem = ref(0);
const isPolling = ref(false);
let pollTimer: number | null = null;

function parseKV(raw: string): Record<string, string> {
  const kv: Record<string, string> = {};
  for (const line of raw.split('\n')) {
    const idx = line.indexOf('=');
    if (idx > 0) kv[line.slice(0, idx).trim()] = line.slice(idx + 1).trim();
  }
  return kv;
}

async function fetchStats() {
  if (!props.session?.host || !props.isActive || isPolling.value) return;
  isPolling.value = true;
  try {
    const host = props.session.host;
    const raw = await invoke<string>('get_host_stats', {
      host: host.host, port: host.port,
      username: host.username,
      password: host.password || null,
      privateKey: host.private_key || null,
    });
    
    // Process stats
    const kv = parseKV(raw);
    const cpu = parseFloat(kv['CPU_USED'] || '0') || 0;
    const memTotal = parseFloat(kv['MEM_TOTAL'] || '0') || 0;
    const memUsed = parseFloat(kv['MEM_USED'] || '0') || 0;
    const memPct = memTotal > 0 ? (memUsed / memTotal) * 100 : 0;

    currentCpu.value = cpu;
    currentMem.value = memPct;

    // Update history arrays (push and shift)
    cpuHistory.value.shift();
    cpuHistory.value.push(cpu);

    memHistory.value.shift();
    memHistory.value.push(memPct);

  } catch (err) {
    // Silently ignore polling errors so it doesn't bother the user
    console.warn('[NanoHUD] Polling failed', err);
  } finally {
    isPolling.value = false;
  }
}

function startPolling() {
  if (pollTimer) return;
  fetchStats(); // initial fetch
  pollTimer = window.setInterval(fetchStats, 5000); // 5 seconds interval
}

function stopPolling() {
  if (pollTimer) {
    window.clearInterval(pollTimer);
    pollTimer = null;
  }
}

watch(() => props.isActive, (active) => {
  if (active) startPolling();
  else stopPolling();
});

onMounted(() => {
  if (props.isActive) startPolling();
});

onUnmounted(() => {
  stopPolling();
});

// Helper for generating SVG path strings from history (0-100 range -> 0-30px height)
function generatePath(history: number[]) {
  if (history.length === 0) return '';
  const widthStr = 60; // total SVG width
  const heightStr = 24; // total SVG height
  const xStep = widthStr / (MAX_POINTS - 1);
  
  const points = history.map((val, i) => {
    const x = i * xStep;
    // val is 0..100 -> invert Y since SVG 0 is at top
    const y = heightStr - (val / 100) * heightStr;
    return `${x},${y}`;
  });
  
  return `M ${points.join(' L ')}`;
}

// Generates a path that can be filled (to create gradient shading below the line)
function generateFillPath(history: number[]) {
  const linePath = generatePath(history);
  if (!linePath) return '';
  return `${linePath} L 60,24 L 0,24 Z`;
}
</script>

<template>
  <div class="nano-hud" :class="{ 'inactive': !isActive }">
    <div class="hud-layer">
      <!-- CPU -->
      <div class="hud-item cpu">
        <div class="hud-label">
          <span>CPU</span>
          <span class="hud-val">{{ currentCpu.toFixed(0) }}%</span>
        </div>
        <div class="hud-graph">
          <svg viewBox="0 0 60 24" preserveAspectRatio="none">
            <!-- Glow/Fill setup -->
            <defs>
              <linearGradient id="cpu-fill" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="rgba(192, 132, 252, 0.4)" />
                <stop offset="100%" stop-color="rgba(192, 132, 252, 0.0)" />
              </linearGradient>
            </defs>
            <path :d="generateFillPath(cpuHistory)" fill="url(#cpu-fill)" />
            <path :d="generatePath(cpuHistory)" fill="none" stroke="#c084fc" stroke-width="1.5" stroke-linejoin="round" />
          </svg>
        </div>
      </div>

      <!-- MEM -->
      <div class="hud-item mem">
        <div class="hud-label">
          <span>MEM</span>
          <span class="hud-val">{{ currentMem.toFixed(0) }}%</span>
        </div>
        <div class="hud-graph">
          <svg viewBox="0 0 60 24" preserveAspectRatio="none">
            <defs>
              <linearGradient id="mem-fill" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="rgba(45, 212, 191, 0.4)" />
                <stop offset="100%" stop-color="rgba(45, 212, 191, 0.0)" />
              </linearGradient>
            </defs>
            <path :d="generateFillPath(memHistory)" fill="url(#mem-fill)" />
            <path :d="generatePath(memHistory)" fill="none" stroke="#2dd4bf" stroke-width="1.5" stroke-linejoin="round" />
          </svg>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.nano-hud {
  position: absolute;
  top: 14px;
  right: 24px;
  background: rgba(15, 23, 42, 0.4);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 8px 10px;
  pointer-events: none; /* Let clicks pass through to terminal */
  z-index: 10; /* 低于 AI 侧边栏(z-index:50)，侧边栏打开时自然覆盖 HUD */
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  transition: opacity 0.3s ease;
}

.nano-hud.inactive {
  opacity: 0;
}

.hud-layer {
  display: flex;
  gap: 16px;
  align-items: center;
}

.hud-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 60px; /* Exact match to SVG viewbox width */
}

.hud-label {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  font-family: 'JetBrains Mono', 'SFMono-Regular', Consolas, monospace;
}

.hud-label span:first-child {
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.5px;
}

.hud-label .hud-val {
  font-size: 11px;
  font-weight: 800;
}

.cpu .hud-label span:first-child { color: rgba(192, 132, 252, 0.7); }
.cpu .hud-label .hud-val { color: #c084fc; text-shadow: 0 0 6px rgba(192, 132, 252, 0.4); }

.mem .hud-label span:first-child { color: rgba(45, 212, 191, 0.7); }
.mem .hud-label .hud-val { color: #2dd4bf; text-shadow: 0 0 6px rgba(45, 212, 191, 0.4); }

.hud-graph {
  height: 24px;
  width: 100%;
  border-bottom: 1px dashed rgba(255, 255, 255, 0.1);
}
.hud-graph svg {
  width: 100%;
  height: 100%;
  overflow: visible;
}
</style>
