<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { confirm } from '@tauri-apps/plugin-dialog';
import SvgIcon from './SvgIcon.vue';

interface Snippet {
  id?: number;
  name: string;
  command: string;
  group?: string;
}

const snippets = ref<Snippet[]>([]);
const showAddForm = ref(false);
const isEditing = ref(false);
const editingId = ref<number | null>(null);
const newSnippetName = ref('');
const newSnippetCommand = ref('');
const emit = defineEmits(['run-snippet', 'close']);

// ---- 占位符 modal ----
const showParamModal = ref(false);
const paramModalCmd = ref('');
const paramKeys = ref<string[]>([]);
const paramValues = ref<Record<string, string>>({});


async function loadSnippets() {
  try {
    snippets.value = await invoke('get_snippets');
  } catch (err) {
    console.error('Failed to load snippets:', err);
  }
}

async function saveSnippet() {
  if (!newSnippetName.value.trim() || !newSnippetCommand.value.trim()) return;
  try {
     if (isEditing.value && editingId.value) {
       await invoke('update_snippet', { 
         snippet: { id: editingId.value, name: newSnippetName.value, command: newSnippetCommand.value, group: null }
       });
     } else {
       await invoke('add_snippet', { name: newSnippetName.value, command: newSnippetCommand.value, category: null });
     }
     
     showAddForm.value = false;
     isEditing.value = false;
     editingId.value = null;
     newSnippetName.value = '';
     newSnippetCommand.value = '';
     loadSnippets();
  } catch (err) {
     console.error('Failed to add snippet:', err);
  }
}

async function deleteSnippet(id: number) {
  const confirmed = await confirm('确定删除该命令片段吗？', { title: 'Nixu', kind: 'warning' });
  if (!confirmed) return;
  try {
    await invoke('delete_snippet', { id });
    loadSnippets();
  } catch (err) {
    console.error('Failed to delete snippet:', err);
  }
}

function editSnippet(snippet: Snippet) {
  newSnippetName.value = snippet.name;
  newSnippetCommand.value = snippet.command;
  editingId.value = snippet.id!;
  isEditing.value = true;
  showAddForm.value = true;
}

function runSnippet(snippet: Snippet) {
  // 检测 {xxx} 占位符
  const matches = [...snippet.command.matchAll(/\{([^}]+)\}/g)];
  const keys = [...new Set(matches.map(m => m[1]))];
  if (keys.length === 0) {
    emit('run-snippet', snippet.command);
    return;
  }
  paramModalCmd.value = snippet.command;
  paramKeys.value = keys;
  paramValues.value = Object.fromEntries(keys.map(k => [k, '']));
  showParamModal.value = true;
}

function confirmParams() {
  let cmd = paramModalCmd.value;
  for (const [k, v] of Object.entries(paramValues.value)) {
    cmd = cmd.split(`{${k}}`).join(v);
  }
  emit('run-snippet', cmd);
  showParamModal.value = false;
}

onMounted(loadSnippets);
</script>

<template>
  <div class="snippet-sidebar">
    <!-- Header -->
    <div class="sidebar-header">
      <div class="header-title">
        <SvgIcon name="snippet" size="16" />
        <span>命令片段</span>
      </div>
      <div class="header-actions">
        <button class="icon-btn add" @click="showAddForm = true" title="添加片段">
          <SvgIcon name="add" size="14" />
        </button>
        <button class="icon-btn" @click="emit('close')" title="关闭">
          <SvgIcon name="close" size="14" />
        </button>
      </div>
    </div>
    
    <div class="snippet-list">
       <div v-if="showAddForm" class="snippet-add-form">
         <div class="form-title">{{ isEditing ? '编辑片段' : '添加片段' }}</div>
         <input v-model="newSnippetName" placeholder="片段名称 (e.g. Update System)" class="add-input" />
         <textarea v-model="newSnippetCommand" placeholder="具体命令" class="add-input code-input" rows="3"></textarea>
         <div class="add-actions">
           <button class="action-btn cancel" @click="showAddForm = false; isEditing = false;">取消</button>
           <button class="action-btn save" @click="saveSnippet">保存</button>
         </div>
       </div>

       <div v-for="s in snippets" :key="s.id" class="snippet-item" @click="runSnippet(s)">
          <SvgIcon name="snippet" size="18" class="snippet-icon" />
          <div class="snippet-info">
             <div class="snippet-name">{{ s.name }}</div>
             <div class="snippet-command" :title="s.command">{{ s.command }}</div>
          </div>
          <div class="snippet-actions">
            <button class="action-btn edit" @click.stop="editSnippet(s)" title="编辑">
              <SvgIcon name="edit" size="14" />
            </button>
            <button class="action-btn delete" @click.stop="deleteSnippet(s.id!)" title="删除">
              <SvgIcon name="delete" size="14" />
            </button>
          </div>
       </div>
       
       <div v-if="snippets.length === 0" class="empty-snippets">
         <SvgIcon name="group" size="48" class="empty-icon" />
         <p>暂无常用命令</p>
         <p style="font-size: 0.7rem; margin-top: 4px;">点击右上角 + 开始添加</p>
       </div>
    </div>

    <!-- 占位符填参数 Modal -->
    <div v-if="showParamModal" class="param-modal-overlay" @click.self="showParamModal = false">
      <div class="param-modal">
        <div class="param-modal-title">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" width="16" height="16"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
          填写参数
        </div>
        <div class="param-modal-preview">{{ paramModalCmd }}</div>
        <div class="param-fields">
          <div v-for="key in paramKeys" :key="key" class="param-field">
            <label>{{ key }}</label>
            <input
              v-model="paramValues[key]"
              :placeholder="`输入 ${key} 的值`"
              class="param-input"
              @keydown.enter="confirmParams"
            />
          </div>
        </div>
        <div class="param-modal-actions">
          <button class="pm-cancel" @click="showParamModal = false">取消</button>
          <button class="pm-confirm" @click="confirmParams">执行</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.snippet-sidebar {
  width: 280px;
  background-color: var(--sidebar-bg);
  border-left: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  height: 100%;
}


.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color, #e2e8f0);
  flex-shrink: 0;
}

.header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-main, #1e293b);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.icon-btn {
  width: 26px; height: 26px;
  border-radius: 6px;
  border: none;
  background: transparent;
  -webkit-appearance: none;
  appearance: none;
  color: var(--text-dim, #94a3b8);
  cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  transition: background 0.15s, color 0.15s;
}
.icon-btn:hover { background: var(--bg-hover, #f1f5f9); color: var(--text-main, #1e293b); }
.icon-btn.add {
  background: var(--accent-color, #3b82f6);
  color: white;
}
.icon-btn.add:hover { background: #2563eb; color: white; }

.snippet-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.snippet-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 12px;
  background: white;
  margin-bottom: 10px;
  cursor: pointer;
  border: 1px solid var(--border-color);
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 1px 3px rgba(0,0,0,0.02);
}

.snippet-item:hover {
  background: var(--bg-dark);
  border-color: var(--accent-color);
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0,0,0,0.05);
}

.snippet-icon {
  font-size: 1.1rem;
  opacity: 0.9;
}

.snippet-info {
  flex: 1;
  overflow: hidden;
}

.snippet-name {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-main);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.snippet-command {
  font-size: 0.65rem;
  color: var(--text-dim);
  font-family: 'JetBrains Mono', monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 4px;
  opacity: 0.8;
}

.snippet-actions {
  opacity: 0;
  transition: opacity 0.2s;
}

.snippet-item:hover .snippet-actions {
  opacity: 1;
}

.action-btn {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 0.9rem;
  color: var(--text-dim);
  padding: 4px;
  border-radius: 4px;
}

.action-btn:hover {
  background: rgba(0,0,0,0.05);
}

.action-btn.delete:hover {
  color: var(--danger);
  background: rgba(255, 77, 79, 0.1);
}

.empty-snippets {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 20px;
  text-align: center;
  color: var(--text-dim);
}

.empty-icon {
  font-size: 3rem;
  margin-bottom: 16px;
  opacity: 0.3;
}

.empty-snippets p {
  font-size: 0.85rem;
  margin: 0;
  opacity: 0.6;
}

.snippet-add-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: white;
  border-radius: 12px;
  border: 1px solid var(--border-color);
  margin-bottom: 12px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.05);
  animation: slide-down 0.2s ease-out;
}

@keyframes slide-down {
  from { opacity: 0; transform: translateY(-10px); }
  to { opacity: 1; transform: translateY(0); }
}

.form-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-main);
  margin-bottom: 4px;
}

.add-input {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-dark);
  color: var(--text-main);
  font-size: 0.8rem;
  outline: none;
  transition: all 0.2s;
  box-sizing: border-box;
}

.add-input:focus {
  border-color: var(--accent-color);
  background: white;
  box-shadow: 0 0 0 2px rgba(57, 108, 216, 0.1);
}

.code-input {
  font-family: 'JetBrains Mono', monospace;
  resize: vertical;
}

.add-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

.action-btn.cancel {
  background: transparent;
  color: var(--text-dim);
  padding: 4px 12px;
  border-radius: 4px;
}

.action-btn.save {
  background: var(--accent-color);
  color: white;
  padding: 4px 12px;
  border-radius: 4px;
}

.action-btn.save:hover {
  background: var(--accent-hover);
}

/* ===== Placeholder Modal ===== */
.param-modal-overlay {
  position: fixed; inset: 0; z-index: 200;
  background: rgba(0,0,0,0.35); backdrop-filter: blur(4px);
  display: flex; align-items: center; justify-content: center;
}
.param-modal {
  background: white; border-radius: 14px;
  padding: 24px; width: 340px; max-width: 90vw;
  box-shadow: 0 20px 60px rgba(0,0,0,0.2);
  display: flex; flex-direction: column; gap: 16px;
}
.param-modal-title {
  display: flex; align-items: center; gap: 8px;
  font-size: 15px; font-weight: 700; color: #1a1a2e;
}
.param-modal-preview {
  background: #f0f4ff; border-radius: 8px; padding: 10px 14px;
  font-family: 'JetBrains Mono', monospace; font-size: 12px;
  color: #3b4cca; word-break: break-all;
}
.param-fields { display: flex; flex-direction: column; gap: 10px; }
.param-field { display: flex; flex-direction: column; gap: 4px; }
.param-field label { font-size: 12px; font-weight: 600; color: #6366f1; }
.param-input {
  padding: 8px 12px; border: 1.5px solid #e2e8f0; border-radius: 8px;
  font-size: 13px; outline: none; transition: border-color 0.15s;
}
.param-input:focus { border-color: #6366f1; }
.param-modal-actions { display: flex; gap: 10px; justify-content: flex-end; }
.pm-cancel {
  padding: 7px 18px; border-radius: 8px; border: 1.5px solid #e2e8f0;
  background: white; font-size: 13px; cursor: pointer; font-weight: 500;
}
.pm-confirm {
  padding: 7px 20px; border-radius: 8px; border: none;
  background: #6366f1; color: white; font-size: 13px;
  cursor: pointer; font-weight: 600; transition: background 0.15s;
}
.pm-confirm:hover { background: #4f46e5; }
</style>
