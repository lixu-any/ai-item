<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const emit = defineEmits(['close', 'toast']);

const apiKey = ref('');
const baseUrl = ref('');
const model = ref('');
const showApiKey = ref(false);
const isSaving = ref(false);

async function loadSettings() {
  try {
    const aiKey = await invoke<string | null>('get_setting', { key: 'ai_api_key' });
    const aiUrl = await invoke<string | null>('get_setting', { key: 'ai_base_url' });
    const aiModel = await invoke<string | null>('get_setting', { key: 'ai_model' });
    apiKey.value = aiKey || '';
    baseUrl.value = aiUrl || 'https://api.deepseek.com/v1';
    model.value = aiModel || 'deepseek-chat';
  } catch (err) {
    console.error('Failed to load settings:', err);
  }
}

async function saveSettings() {
  isSaving.value = true;
  try {
    await invoke('save_setting', { key: 'ai_api_key', value: apiKey.value.trim() });
    await invoke('save_setting', { key: 'ai_base_url', value: baseUrl.value.trim() });
    await invoke('save_setting', { key: 'ai_model', value: model.value.trim() });
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
    <!-- Header -->
    <header class="settings-header">
      <div class="header-icon-wrap">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z"/></svg>
      </div>
      <div class="header-text">
        <h1 class="header-title">全局设置</h1>
        <p class="header-sub">AI 助手配置中心</p>
      </div>
    </header>

    <div class="divider"></div>

    <!-- Body -->
    <div class="settings-body">
      <!-- 快速预设 -->
      <div class="section">
        <div class="section-label">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>
          快速选择服务商
        </div>
        <div class="presets">
          <button class="preset-btn" :class="{ active: baseUrl.startsWith('https://api.deepseek.com') }" @click="setPreset('https://api.deepseek.com/v1', 'deepseek-chat')">
            <span class="dot" style="background:#3b82f6;"></span> DeepSeek
          </button>
          <button class="preset-btn" :class="{ active: baseUrl.startsWith('https://api.openai.com') }" @click="setPreset('https://api.openai.com/v1', 'gpt-4o')">
            <span class="dot" style="background:#10a37f;"></span> OpenAI
          </button>
          <button class="preset-btn" :class="{ active: baseUrl.startsWith('https://api.siliconflow.cn') }" @click="setPreset('https://api.siliconflow.cn/v1', 'deepseek-ai/DeepSeek-V3')">
            <span class="dot" style="background:#a855f7;"></span> 硅基流动
          </button>
        </div>
      </div>

      <!-- 表单 -->
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

    <!-- Footer -->
    <div class="divider"></div>
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

.divider { height: 1px; background: #e2e8f0; flex-shrink: 0; }

/* Header */
.settings-header {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 20px 24px 16px;
  background: #ffffff;
  flex-shrink: 0;
}
.header-icon-wrap {
  width: 42px; height: 42px;
  border-radius: 12px;
  background: linear-gradient(135deg, #dbeafe, #ede9fe);
  border: 1px solid #bfdbfe;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0;
  color: #3b82f6;
}
.header-icon-wrap svg { width: 20px; height: 20px; }
.header-title {
  font-size: 15px;
  font-weight: 600;
  color: #0f172a;
}
.header-sub {
  font-size: 11.5px;
  color: #94a3b8;
  margin-top: 2px;
}

/* Body */
.settings-body {
  flex: 1;
  overflow-y: auto;
  padding: 18px 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.section { display: flex; flex-direction: column; gap: 10px; }

.section-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.7px;
  color: #3b82f6;
}
.section-label svg { width: 13px; height: 13px; }

/* Presets */
.presets { display: flex; gap: 8px; }
.preset-btn {
  display: flex;
  align-items: center;
  gap: 6px;
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

/* Form */
.form-field { display: flex; flex-direction: column; gap: 6px; }
label { font-size: 13px; font-weight: 500; color: #374151; }
.label-hint { font-weight: 400; color: #9ca3af; font-size: 12px; }

.input-wrap {
  position: relative;
  display: flex;
  align-items: center;
}
.ico {
  position: absolute;
  left: 11px;
  width: 14px; height: 14px;
  color: #9ca3af;
  pointer-events: none;
}
.input-wrap input {
  width: 100%;
  padding: 9px 12px 9px 34px;
  background: #fff;
  border: 1.5px solid #e2e8f0;
  border-radius: 8px;
  color: #1e293b;
  font-size: 13px;
  font-family: inherit;
  transition: border-color 0.15s, box-shadow 0.15s;
  outline: none;
}
.input-wrap input::placeholder { color: #cbd5e1; }
.input-wrap input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px #3b82f620;
}
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

.hint { font-size: 11.5px; color: #94a3b8; line-height: 1.5; }

/* Footer */
.settings-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 14px 24px;
  background: #ffffff;
  flex-shrink: 0;
}
.btn-cancel {
  padding: 8px 20px;
  border-radius: 8px;
  border: 1.5px solid #e2e8f0;
  background: #fff;
  color: #64748b;
  font-size: 13px;
  cursor: pointer;
  font-family: inherit;
  transition: all 0.15s;
}
.btn-cancel:hover { border-color: #cbd5e1; color: #374151; }

.btn-save {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 22px;
  border-radius: 8px;
  border: none;
  background: #3b82f6;
  color: white;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  font-family: inherit;
  transition: all 0.15s;
  box-shadow: 0 2px 8px #3b82f640;
}
.btn-save:hover:not(:disabled) {
  background: #2563eb;
  box-shadow: 0 4px 12px #3b82f650;
  transform: translateY(-1px);
}
.btn-save:disabled { opacity: 0.5; cursor: not-allowed; transform: none; }
.btn-save svg { width: 14px; height: 14px; }

.spin { animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.settings-body::-webkit-scrollbar { width: 5px; }
.settings-body::-webkit-scrollbar-track { background: transparent; }
.settings-body::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 3px; }
</style>
