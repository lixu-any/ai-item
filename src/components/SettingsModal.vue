<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { emit as tauriEmit } from '@tauri-apps/api/event';

const emit = defineEmits(['close', 'toast']);

// ---- 当前选中 Tab ----
const activeTab = ref<'ai' | 'terminal' | 'ssh'>('ai');

// ---- AI 设置 ----
const apiKey = ref('');
const baseUrl = ref('');
const model = ref('');
const showApiKey = ref(false);

// ---- 终端设置 ----
const termFontSize = ref(14);
const termLineHeight = ref(1.2);
const termFontFamily = ref('"Cascadia Code", Menlo, Monaco, monospace');
const termCursorStyle = ref<'block' | 'underline' | 'bar'>('block');
const tabTitleMode = ref<'ip' | 'name'>('ip');

const fontPresets = [
  { label: 'Cascadia Code', value: '"Cascadia Code", Menlo, Monaco, monospace' },
  { label: 'JetBrains Mono', value: '"JetBrains Mono", Menlo, monospace' },
  { label: 'Fira Code', value: '"Fira Code", Menlo, Monaco, monospace' },
  { label: 'SF Mono', value: '"SF Mono", Menlo, monospace' },
  { label: 'Menlo', value: 'Menlo, Monaco, monospace' },
  { label: 'Courier New', value: '"Courier New", monospace' },
];

// ---- SSH 设置 ----
const sshTimeout = ref(15);
const sshKeepalive = ref(60);

const isSaving = ref(false);

async function loadSettings() {
  try {
    apiKey.value = (await invoke<string | null>('get_setting', { key: 'ai_api_key' })) || '';
    baseUrl.value = (await invoke<string | null>('get_setting', { key: 'ai_base_url' })) || 'https://api.deepseek.com/v1';
    model.value = (await invoke<string | null>('get_setting', { key: 'ai_model' })) || 'deepseek-chat';

    const fontSize = await invoke<string | null>('get_setting', { key: 'term_font_size' });
    termFontSize.value = fontSize ? parseInt(fontSize) : 14;

    const lineHeight = await invoke<string | null>('get_setting', { key: 'term_line_height' });
    termLineHeight.value = lineHeight ? parseFloat(lineHeight) : 1.2;

    const fontFamily = await invoke<string | null>('get_setting', { key: 'term_font_family' });
    termFontFamily.value = fontFamily || '"Cascadia Code", Menlo, Monaco, monospace';

    const cursorStyle = await invoke<string | null>('get_setting', { key: 'term_cursor_style' });
    termCursorStyle.value = (cursorStyle as any) || 'block';

    const titleMode = await invoke<string | null>('get_setting', { key: 'tab_title_mode' });
    tabTitleMode.value = (titleMode as any) || 'ip';

    const timeout = await invoke<string | null>('get_setting', { key: 'ssh_connect_timeout' });
    sshTimeout.value = timeout ? parseInt(timeout) : 15;

    const keepalive = await invoke<string | null>('get_setting', { key: 'ssh_keepalive' });
    sshKeepalive.value = keepalive ? parseInt(keepalive) : 60;
  } catch (err) {
    console.error('加载设置失败:', err);
  }
}

async function saveSettings() {
  isSaving.value = true;
  try {
    await invoke('save_setting', { key: 'ai_api_key', value: apiKey.value.trim() });
    await invoke('save_setting', { key: 'ai_base_url', value: baseUrl.value.trim() });
    await invoke('save_setting', { key: 'ai_model', value: model.value.trim() });
    await invoke('save_setting', { key: 'term_font_size', value: String(termFontSize.value) });
    await invoke('save_setting', { key: 'term_line_height', value: String(termLineHeight.value) });
    await invoke('save_setting', { key: 'term_font_family', value: termFontFamily.value });
    await invoke('save_setting', { key: 'term_cursor_style', value: termCursorStyle.value });
    await invoke('save_setting', { key: 'tab_title_mode', value: tabTitleMode.value });
    await invoke('save_setting', { key: 'ssh_connect_timeout', value: String(sshTimeout.value) });
    await invoke('save_setting', { key: 'ssh_keepalive', value: String(sshKeepalive.value) });

    // 广播终端设置变更，让已打开的终端实时生效
    await tauriEmit('terminal-settings-changed', {
      fontSize: termFontSize.value,
      lineHeight: termLineHeight.value,
      fontFamily: termFontFamily.value,
      cursorStyle: termCursorStyle.value,
    });

    emit('toast', { message: '设置已保存', type: 'success' });
    emit('close');
  } catch (err) {
    emit('toast', { message: '保存失败: ' + err, type: 'error' });
  } finally {
    isSaving.value = false;
  }
}

function setPreset(url: string, mdl: string) {
  baseUrl.value = url;
  model.value = mdl;
}

onMounted(loadSettings);
</script>

<template>
  <div class="settings-root">
    <!-- 顶部渐变装饰条 -->
    <div class="accent-bar"></div>

    <div class="settings-layout">
      <!-- 左侧 Tab 导航 -->
      <nav class="settings-nav">
        <div class="nav-title">设置</div>

        <button class="nav-item" :class="{ active: activeTab === 'ai' }" @click="activeTab = 'ai'">
          <span class="nav-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="8" r="4"/><path d="M6 20v-2a4 4 0 014-4h4a4 4 0 014 4v2"/></svg>
          </span>
          <span>AI 助手</span>
        </button>

        <button class="nav-item" :class="{ active: activeTab === 'terminal' }" @click="activeTab = 'terminal'">
          <span class="nav-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>
          </span>
          <span>终端</span>
        </button>

        <button class="nav-item" :class="{ active: activeTab === 'ssh' }" @click="activeTab = 'ssh'">
          <span class="nav-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M5 12h14M12 5l7 7-7 7"/></svg>
          </span>
          <span>SSH 连接</span>
        </button>
      </nav>

      <!-- 右侧内容区 -->
      <div class="settings-content">
        <!-- ===== AI 助手 ===== -->
        <div v-if="activeTab === 'ai'" class="tab-pane">
          <div class="pane-header">
            <h2>AI 助手</h2>
            <p>配置 AI 服务的接入信息，支持 OpenAI 兼容接口</p>
          </div>

          <div class="section">
            <div class="section-label">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>
              快速服务商预设
            </div>
            <div class="presets">
              <button class="preset-btn" :class="{ active: baseUrl.startsWith('https://api.deepseek.com') }"
                @click="setPreset('https://api.deepseek.com/v1', 'deepseek-chat')">
                <span class="dot" style="background:#3b82f6;"></span> DeepSeek
              </button>
              <button class="preset-btn" :class="{ active: baseUrl.startsWith('https://api.openai.com') }"
                @click="setPreset('https://api.openai.com/v1', 'gpt-4o')">
                <span class="dot" style="background:#10a37f;"></span> OpenAI
              </button>
              <button class="preset-btn" :class="{ active: baseUrl.startsWith('https://api.siliconflow.cn') }"
                @click="setPreset('https://api.siliconflow.cn/v1', 'deepseek-ai/DeepSeek-V3')">
                <span class="dot" style="background:#a855f7;"></span> 硅基流动
              </button>
            </div>
          </div>

          <div class="section">
            <div class="section-label">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71"/></svg>
              接口配置
            </div>

            <div class="form-field">
              <label>API 地址 <span class="label-hint">(Base URL)</span></label>
              <div class="input-wrap">
                <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71"/></svg>
                <input v-model="baseUrl" type="text" placeholder="https://api.deepseek.com/v1" />
              </div>
            </div>

            <div class="form-field">
              <label>密钥 <span class="label-hint">(API Key)</span></label>
              <div class="input-wrap">
                <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg>
                <input v-model="apiKey" :type="showApiKey ? 'text' : 'password'" placeholder="sk-..." />
                <button class="eye-btn" @click="showApiKey = !showApiKey">
                  <svg v-if="!showApiKey" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                  <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
                </button>
              </div>
            </div>

            <div class="form-field">
              <label>模型 <span class="label-hint">(Model Name)</span></label>
              <div class="input-wrap">
                <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>
                <input v-model="model" type="text" placeholder="deepseek-chat" />
              </div>
              <p class="hint">例：deepseek-chat / gpt-4o / claude-3-opus-20240229</p>
            </div>
          </div>
        </div>

        <!-- ===== 终端 ===== -->
        <div v-if="activeTab === 'terminal'" class="tab-pane">
          <div class="pane-header">
            <h2>终端</h2>
            <p>调整终端的外观和交互行为</p>
          </div>

          <div class="section">
            <div class="section-label">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>
              字体
            </div>

            <!-- 字体族预设 -->
            <div class="form-field">
              <label>终端字体</label>
              <div class="font-presets">
                <button
                  v-for="f in fontPresets" :key="f.value"
                  class="font-preset-btn"
                  :class="{ active: termFontFamily === f.value }"
                  :style="{ fontFamily: f.value }"
                  @click="termFontFamily = f.value"
                >{{ f.label }}</button>
              </div>
            </div>

            <!-- 字体大小 + 行高 并排 -->
            <div class="form-row">
              <div class="form-field">
                <label>字体大小</label>
                <div class="input-wrap-num">
                  <input v-model.number="termFontSize" type="number" min="10" max="28" step="1" />
                  <span class="unit">px</span>
                </div>
              </div>
              <div class="form-field">
                <label>行高</label>
                <div class="input-wrap-num">
                  <input v-model.number="termLineHeight" type="number" min="1.0" max="2.0" step="0.1" />
                  <span class="unit">×</span>
                </div>
              </div>
            </div>

            <!-- 实时预览 -->
            <div class="term-preview" :style="{ fontSize: termFontSize + 'px', fontFamily: termFontFamily, lineHeight: termLineHeight }">
              <span class="term-prompt">user@nixu:~$</span> ls -la /home<br/>
              <span style="color:#22d3ee">drwxr-xr-x</span>  2 lixu  staff   64 Mar 16 14:00 <span style="color:#a78bfa">projects</span><br/>
              <span style="color:#22d3ee">-rw-r--r--</span>  1 lixu  staff  128 Mar 16 12:00 .zshrc
            </div>
          </div>

          <!-- 光标样式 -->
          <div class="section">
            <div class="section-label">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/></svg>
              光标
            </div>
            <div class="form-field">
              <label>光标样式</label>
              <div class="cursor-btns">
                <button class="cursor-opt" :class="{ active: termCursorStyle === 'block' }" @click="termCursorStyle = 'block'">
                  <span class="cursor-demo" style="background:#fff; width:10px; height:16px; display:inline-block;"></span>
                  块状
                </button>
                <button class="cursor-opt" :class="{ active: termCursorStyle === 'underline' }" @click="termCursorStyle = 'underline'">
                  <span class="cursor-demo" style="border-bottom:2px solid #fff; width:10px; display:inline-block;"></span>
                  下划线
                </button>
                <button class="cursor-opt" :class="{ active: termCursorStyle === 'bar' }" @click="termCursorStyle = 'bar'">
                  <span class="cursor-demo" style="border-left:2px solid #fff; width:2px; height:16px; display:inline-block;"></span>
                  竖线
                </button>
              </div>
            </div>
          </div>

          <!-- 标签页标题 -->
          <div class="section">
            <div class="section-label">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><rect x="2" y="3" width="20" height="5" rx="1"/><line x1="2" y1="12" x2="22" y2="12"/><line x1="2" y1="17" x2="22" y2="17"/></svg>
              标签页标题
            </div>
            <div class="form-field">
              <label>连接时标签显示</label>
              <div class="toggle-opts">
                <button class="toggle-opt" :class="{ active: tabTitleMode === 'name' }" @click="tabTitleMode = 'name'">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M20 7H4a2 2 0 00-2 2v6a2 2 0 002 2h16a2 2 0 002-2V9a2 2 0 00-2-2z"/><line x1="12" y1="12" x2="12" y2="12"/></svg>
                  服务器名称
                  <span class="opt-example">pre腾讯</span>
                </button>
                <button class="toggle-opt" :class="{ active: tabTitleMode === 'ip' }" @click="tabTitleMode = 'ip'">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 010 20M12 2a15.3 15.3 0 000 20"/></svg>
                  IP 地址
                  <span class="opt-example">root@152.x.x.x</span>
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- ===== SSH 连接 ===== -->
        <div v-if="activeTab === 'ssh'" class="tab-pane">
          <div class="pane-header">
            <h2>SSH 连接</h2>
            <p>调整 SSH 连接的超时和保活策略</p>
          </div>

          <div class="section">
            <div class="section-label">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
              超时设置
            </div>

            <div class="form-row">
              <div class="form-field">
                <label>连接超时</label>
                <div class="input-wrap-num">
                  <input v-model.number="sshTimeout" type="number" min="5" max="120" step="1" />
                  <span class="unit">秒</span>
                </div>
                <p class="hint">TCP 握手超时时间，默认 15s</p>
              </div>

              <div class="form-field">
                <label>Keepalive 间隔</label>
                <div class="input-wrap-num">
                  <input v-model.number="sshKeepalive" type="number" min="0" max="600" step="10" />
                  <span class="unit">秒</span>
                </div>
                <p class="hint">0 = 禁用，推荐 60s 防断连</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 底部 Footer -->
    <div class="footer-divider"></div>
    <footer class="settings-footer">
      <button class="btn-cancel" @click="emit('close')">取消</button>
      <button class="btn-save" :disabled="isSaving" @click="saveSettings">
        <svg v-if="!isSaving" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M19 21H5a2 2 0 01-2-2V5a2 2 0 012-2h11l5 5v11a2 2 0 01-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/></svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" class="spin"><path d="M21 12a9 9 0 11-6.219-8.56"/></svg>
        {{ isSaving ? '保存中...' : '保存配置' }}
      </button>
    </footer>
  </div>
</template>

<style scoped>
* { box-sizing: border-box; margin: 0; padding: 0; }

.settings-root {
  width: 100%;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f5f7fa;
  color: #1e293b;
  font-family: -apple-system, 'PingFang SC', 'Inter', sans-serif;
  overflow: hidden;
}

.accent-bar {
  height: 3px;
  background: linear-gradient(90deg, #3b82f6, #8b5cf6, #ec4899);
  flex-shrink: 0;
}

/* ===== 主布局 ===== */
.settings-layout {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* ===== 左侧导航 ===== */
.settings-nav {
  width: 160px;
  flex-shrink: 0;
  background: #fff;
  border-right: 1px solid #e2e8f0;
  display: flex;
  flex-direction: column;
  padding: 16px 10px;
  gap: 2px;
}

.nav-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.8px;
  color: #94a3b8;
  padding: 0 8px 10px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 9px 10px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: #475569;
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  text-align: left;
  transition: all 0.15s;
  width: 100%;
  -webkit-appearance: none;
  appearance: none;
}
.nav-item:hover { background: #f1f5f9; color: #1e293b; }
.nav-item.active {
  background: #eff6ff;
  color: #2563eb;
  font-weight: 600;
}
.nav-icon {
  width: 22px; height: 22px;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0;
}
.nav-icon svg { width: 15px; height: 15px; }

/* ===== 右侧内容 ===== */
.settings-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 28px;
}
.settings-content::-webkit-scrollbar { width: 5px; }
.settings-content::-webkit-scrollbar-track { background: transparent; }
.settings-content::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 3px; }

.tab-pane { display: flex; flex-direction: column; gap: 22px; }

.pane-header { padding-bottom: 16px; border-bottom: 1px solid #e2e8f0; }
.pane-header h2 { font-size: 16px; font-weight: 700; color: #0f172a; }
.pane-header p { font-size: 12px; color: #94a3b8; margin-top: 4px; }

/* ===== Section ===== */
.section { display: flex; flex-direction: column; gap: 12px; }
.section-label {
  display: flex; align-items: center; gap: 6px;
  font-size: 11px; font-weight: 600;
  text-transform: uppercase; letter-spacing: 0.7px;
  color: #3b82f6;
}
.section-label svg { width: 13px; height: 13px; }

/* ===== Presets ===== */
.presets { display: flex; gap: 8px; flex-wrap: wrap; }
.preset-btn {
  display: flex; align-items: center; gap: 6px;
  padding: 6px 14px;
  border-radius: 20px;
  border: 1.5px solid #e2e8f0;
  background: #fff;
  color: #475569;
  font-size: 12.5px;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
}
.preset-btn:hover { border-color: #93c5fd; color: #1e40af; }
.preset-btn.active { border-color: #3b82f6; background: #eff6ff; color: #2563eb; font-weight: 500; }
.dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }

/* ===== 字体预设 ===== */
.font-presets {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
}
.font-preset-btn {
  padding: 7px 10px;
  border-radius: 8px;
  border: 1.5px solid #e2e8f0;
  background: #fff;
  color: #475569;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
  text-align: left;
  appearance: none; -webkit-appearance: none;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.font-preset-btn:hover { border-color: #93c5fd; color: #1e40af; background: #f8faff; }
.font-preset-btn.active { border-color: #3b82f6; background: #eff6ff; color: #2563eb; font-weight: 600; }

/* ===== 终端预览 ===== */
.term-preview {
  padding: 12px 16px;
  background: #0d1117;
  border-radius: 8px;
  color: #d4d4d4;
  font-family: monospace;
  border: 1px solid #1e293b;
  user-select: none;
  line-height: 1.6;
}
.term-prompt { color: #22d3ee; font-weight: 600; }

/* ===== 光标样式选择 ===== */
.cursor-btns { display: flex; gap: 8px; }
.cursor-opt {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 16px;
  border-radius: 8px;
  border: 1.5px solid #e2e8f0;
  background: #1e293b;
  color: #94a3b8;
  font-size: 12.5px;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
  appearance: none; -webkit-appearance: none;
}
.cursor-opt:hover { border-color: #3b82f6; color: #e2e8f0; }
.cursor-opt.active { border-color: #3b82f6; background: #1e3a5f; color: #fff; font-weight: 600; }

/* ===== 标签标题模式切换 ===== */
.toggle-opts { display: flex; gap: 10px; }
.toggle-opt {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 12px 10px;
  border-radius: 10px;
  border: 1.5px solid #e2e8f0;
  background: #fff;
  color: #475569;
  font-size: 12.5px;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
  appearance: none; -webkit-appearance: none;
}
.toggle-opt svg { width: 18px; height: 18px; }
.toggle-opt:hover { border-color: #93c5fd; color: #1e40af; background: #f8faff; }
.toggle-opt.active { border-color: #3b82f6; background: #eff6ff; color: #2563eb; font-weight: 600; }
.opt-example {
  font-size: 11px;
  color: #94a3b8;
  font-family: monospace;
  background: #f1f5f9;
  padding: 2px 6px;
  border-radius: 4px;
  margin-top: 2px;
}
.toggle-opt.active .opt-example { background: #dbeafe; color: #2563eb; }


/* ===== Form ===== */
.form-field { display: flex; flex-direction: column; gap: 6px; }
.form-row { display: flex; gap: 16px; }
.form-row .form-field { flex: 1; }

label { font-size: 13px; font-weight: 500; color: #374151; }
.label-hint { font-weight: 400; color: #9ca3af; font-size: 12px; }
.hint { font-size: 11.5px; color: #94a3b8; line-height: 1.5; }

.input-wrap {
  position: relative; display: flex; align-items: center;
}
.ico {
  position: absolute; left: 11px;
  width: 14px; height: 14px;
  color: #9ca3af; pointer-events: none;
}
.input-wrap input {
  width: 100%;
  padding: 9px 12px 9px 34px;
  background: #fff;
  border: 1.5px solid #e2e8f0;
  border-radius: 8px;
  color: #1e293b;
  font-size: 13px; font-family: inherit;
  outline: none;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.input-wrap input::placeholder { color: #cbd5e1; }
.input-wrap input:focus { border-color: #3b82f6; box-shadow: 0 0 0 3px #3b82f620; }
.eye-btn {
  position: absolute; right: 10px;
  width: 24px; height: 24px;
  border: none; background: transparent;
  color: #9ca3af; cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  border-radius: 4px; transition: color 0.15s;
}
.eye-btn:hover { color: #475569; }
.eye-btn svg { width: 14px; height: 14px; }

/* ===== 数值输入 ===== */
.input-wrap-num {
  display: flex; align-items: center;
  border: 1.5px solid #e2e8f0;
  border-radius: 8px; overflow: hidden; background: #fff;
  transition: border-color 0.15s;
  width: fit-content;
}
.input-wrap-num:focus-within { border-color: #3b82f6; box-shadow: 0 0 0 3px #3b82f620; }
.input-wrap-num input {
  width: 72px; padding: 9px 10px;
  border: none; outline: none; background: transparent;
  color: #1e293b; font-size: 13px; font-family: inherit;
}
.unit {
  padding: 0 10px;
  font-size: 12px; color: #9ca3af;
  background: #f8fafc;
  border-left: 1px solid #e2e8f0;
  align-self: stretch;
  display: flex; align-items: center; white-space: nowrap;
}

/* ===== 字体预览 ===== */
.num-row { display: flex; align-items: center; gap: 16px; }
.size-preview {
  padding: 6px 14px;
  background: #1e293b;
  color: #a8e6cf;
  border-radius: 6px;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  white-space: nowrap;
  transition: font-size 0.2s;
}

/* ===== Footer ===== */
.footer-divider { height: 1px; background: #e2e8f0; flex-shrink: 0; }
.settings-footer {
  display: flex; justify-content: flex-end; gap: 10px;
  padding: 14px 24px; background: #fff; flex-shrink: 0;
}
.btn-cancel {
  padding: 8px 20px; border-radius: 8px;
  border: 1.5px solid #e2e8f0; background: #fff;
  color: #64748b; font-size: 13px;
  cursor: pointer; font-family: inherit;
  transition: all 0.15s; -webkit-appearance: none; appearance: none;
}
.btn-cancel:hover { border-color: #cbd5e1; color: #374151; }
.btn-save {
  display: flex; align-items: center; gap: 6px;
  padding: 8px 22px; border-radius: 8px; border: none;
  background: #3b82f6; color: white;
  font-size: 13px; font-weight: 500;
  cursor: pointer; font-family: inherit;
  transition: all 0.15s;
  box-shadow: 0 2px 8px #3b82f640;
  -webkit-appearance: none;
  appearance: none;
}
.btn-save:hover:not(:disabled) { background: #2563eb; transform: translateY(-1px); }
.btn-save:disabled { opacity: 0.5; cursor: not-allowed; transform: none; }
.btn-save svg { width: 14px; height: 14px; }
.spin { animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
</style>
