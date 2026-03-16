<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import SvgIcon from './SvgIcon.vue';

interface Snippet {
  id?: number;
  name: string;
  command: string;
  group?: string;
}

const snippets = ref<Snippet[]>([]);
const emit = defineEmits(['run-snippet', 'close']);

async function loadSnippets() {
  try {
    snippets.value = await invoke('get_snippets');
  } catch (err) {
    console.error('Failed to load snippets:', err);
  }
}

async function addSnippet() {
  const name = prompt('命令名称 (Snippet Name):');
  const command = prompt('具体命令 (Command):');
  if (name && command) {
     try {
       await invoke('add_snippet', { name, command, category: null });
       loadSnippets();
     } catch (err) {
       console.error('Failed to add snippet:', err);
     }
  }
}

async function deleteSnippet(id: number) {
  if (!confirm('确定删除该命令片段吗？')) return;
  try {
    await invoke('delete_snippet', { id });
    loadSnippets();
  } catch (err) {
    console.error('Failed to delete snippet:', err);
  }
}

function runSnippet(snippet: Snippet) {
  emit('run-snippet', snippet.command);
}

onMounted(loadSnippets);
</script>

<template>
  <div class="snippet-sidebar">
    <div class="sidebar-header">
      <span class="sidebar-title">命令片段</span>
      <div class="header-actions">
        <button class="add-btn" @click="addSnippet" title="添加片段">
          <SvgIcon name="add" size="14" />
        </button>
        <button class="close-sidebar-btn" @click="emit('close')" title="关闭侧边栏">
          <SvgIcon name="close" size="14" />
        </button>
      </div>
    </div>
    
    <div class="snippet-list">
       <div v-for="s in snippets" :key="s.id" class="snippet-item" @click="runSnippet(s)">
          <SvgIcon name="snippet" size="18" class="snippet-icon" />
          <div class="snippet-info">
             <div class="snippet-name">{{ s.name }}</div>
             <div class="snippet-command" :title="s.command">{{ s.command }}</div>
          </div>
          <div class="snippet-actions">
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
  animation: slide-in 0.3s ease-out;
}

@keyframes slide-in {
  from { transform: translateX(100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}

.sidebar-header {
  padding: 24px 16px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.sidebar-title {
  font-size: 0.9rem;
  font-weight: 700;
  color: var(--text-main);
  opacity: 0.9;
}

.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.add-btn {
  background: var(--accent-color);
  color: white;
  border: none;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1rem;
  transition: all 0.2s;
  box-shadow: 0 2px 4px rgba(57, 108, 216, 0.2);
}

.add-btn:hover {
  background: var(--accent-hover);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(57, 108, 216, 0.3);
}

.close-sidebar-btn {
  background: var(--glass-bg);
  border: 1px solid var(--border-color);
  color: var(--text-dim);
  width: 28px;
  height: 28px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.2rem;
  transition: all 0.2s;
}

.close-sidebar-btn:hover {
  background: var(--sidebar-hover);
  color: var(--danger);
  border-color: var(--danger);
}

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
</style>
