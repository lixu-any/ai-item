<template>
  <div class="sftp-browser" @dragover.prevent @drop.prevent>
    <!-- Overlay for OS file drop -->
    <div class="drag-overlay" v-if="isDraggingFile">
      <div class="drag-overlay-content">
        <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="drag-icon"><path d="M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242"/><path d="M12 12v9"/><path d="m8 16 4-4 4 4"/></svg>
        <h2 style="font-size:16px;color:#333;margin:0">松开以传送到本目录</h2>
      </div>
    </div>
    <div class="sftp-header">
      <div class="path-bar">
        <button class="icon-btn" @click="goUp" :disabled="currentPath === '/' || !currentPath">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12 7-7 7 7"/><path d="M12 19V5"/></svg>
        </button>
        <div class="path-display" @click="startEditPath" v-if="!isEditingPath">
          {{ currentPath || 'Connecting...' }}
        </div>
        <input 
          v-else 
          ref="pathInputRef"
          v-model="editPathValue" 
          @blur="finishEditPath" 
          @keydown.enter="finishEditPath"
          class="path-input" 
        />
      </div>
      <div class="sftp-actions">
        <button class="icon-btn" @click="refresh" title="刷新">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>
        </button>
        <button class="icon-btn" @click="mkdir" title="新建文件夹">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 10v6"/><path d="M9 13h6"/><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg>
        </button>
        <button class="icon-btn" @click="uploadFile" title="上传文件">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/></svg>
        </button>
      </div>
    </div>
    <div class="sftp-content" @contextmenu.prevent="onEmptyContextMenu">
      <div v-if="loading" class="loading-overlay">正在加载...</div>
      <table class="file-table" v-else>
        <thead>
          <tr>
            <th class="col-name">名称</th>
            <th class="col-size">大小</th>
            <th class="col-type">类型</th>
            <th class="col-perms">权限</th>
            <th class="col-time">修改时间</th>
            <th class="col-actions">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="currentPath !== '/'" class="file-action-row" @click="goUp"
              @dragenter.prevent="dragOverDir = '..'"
              @dragover.prevent="dragOverDir = '..'"
              @dragleave.prevent="handleDragLeave($event)"
              @drop.prevent.stop="onInternalDrop('..')"
              :class="{ 'drag-over': dragOverDir === '..' }"
              style="cursor: pointer;">
            <td colspan="6">
              <span class="back-link" style="display:flex;align-items:center;gap:6px;color:#6366f1;font-weight:600;padding-left:4px">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
                返回上级目录 (..)
              </span>
            </td>
          </tr>
          <tr v-for="file in sortedFiles" :key="file.name" 
              class="file-item"
              :draggable="true"
              @dragstart="onInternalDragStart($event, file)"
              @dragend="onInternalDragEnd"
              @dragenter.prevent="handleInternalDragEnter($event, file)"
              @dragover.prevent="handleInternalDragEnter($event, file)"
              @dragleave.prevent="handleDragLeave($event)"
              @drop.prevent.stop="handleInternalDrop($event, file)"
              :class="{ 'drag-over': dragOverDir === file.name }"
              @dblclick="handleDoubleClick(file)" 
              @contextmenu.prevent.stop="onFileContextMenu($event, file)">
            <td class="file-name-cell">
              <svg v-if="file.is_dir" class="file-icon folder" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg>
              <svg v-else class="file-icon file" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/></svg>
              <span class="file-name">{{ file.name }}</span>
            </td>
            <td>{{ file.is_dir ? '-' : formatSize(file.size) }}</td>
            <td>{{ file.is_dir ? '文件夹' : '文件' }}</td>
            <td>{{ file.permissions ? (file.permissions & 0o777).toString(8).padStart(3, '0') : '-' }}</td>
            <td>{{ file.modified_time ? new Date(file.modified_time * 1000).toLocaleString() : '-' }}</td>
            <td class="action-cell">
              <button class="text-btn" @click.stop="downloadFile(file)" v-if="!file.is_dir">下载</button>
              <button class="text-btn" @click.stop="renameFile(file)">重命名</button>
              <button class="text-btn danger" @click.stop="deleteFile(file)">删除</button>
            </td>
          </tr>
          <tr v-if="files.length === 0 && !loading">
            <td colspan="6" class="empty-state">目录为空</td>
          </tr>
        </tbody>
      </table>
    </div>
      <ContextMenu
        v-model:visible="showMenu"
        :x="menuX"
        :y="menuY"
        :items="menuItems"
        @action="handleMenuAction"
      />

    <!-- 自定义输入确认框，取代被拦截的 window.prompt -->
    <div v-if="promptVisible" class="sftp-prompt-overlay" @click.self="cancelPrompt">
      <div class="sftp-prompt-box">
        <div class="prompt-title">{{ promptTitle }}</div>
        <input 
          id="sftp-prompt-input"
          v-model="promptValue" 
          @keydown.enter="confirmPrompt"
          @keydown.esc="cancelPrompt"
          class="prompt-input"
        />
        <div class="prompt-actions">
          <button class="btn-cancel" @click="cancelPrompt">取消</button>
          <button class="btn-confirm" @click="confirmPrompt">确定</button>
        </div>
      </div>
    </div>

    <!-- 自定义确认框，取代被拦截的 window.confirm -->
    <div v-if="confirmVisible" class="sftp-prompt-overlay" @click.self="cancelConfirm">
      <div class="sftp-prompt-box">
        <div class="prompt-title">请确认</div>
        <div class="confirm-message">{{ confirmMessage }}</div>
        <div class="prompt-actions">
          <button class="btn-cancel" @click="cancelConfirm">取消</button>
          <button class="btn-confirm danger" @click="applyConfirm">删除</button>
        </div>
      </div>
    </div>

    <!-- 传输进度条 -->
    <div class="transfer-panel" v-if="Object.keys(activeTransfers).length > 0">
      <div v-for="(task, file) in activeTransfers" :key="file" class="transfer-item">
        <div class="transfer-info">
          <span class="transfer-name" :title="String(file)">{{ file }} ({{ task.transfer_type === 'download' ? '下载' : '上传' }})</span>
          <div class="transfer-meta">
            <span class="transfer-size">{{ formatSize(task.transferred) }} / {{ formatSize(task.total) }}</span>
            <span class="transfer-percent">{{ task.total > 0 ? Math.round((task.transferred / task.total) * 100) : 0 }}%</span>
          </div>
        </div>
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: (task.total > 0 ? (task.transferred / task.total) * 100 : 0) + '%' }"></div>
        </div>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open, save } from '@tauri-apps/plugin-dialog';
import ContextMenu, { MenuItem } from './ContextMenu.vue';

interface Host {
  name: string;
  host: string;
  port: number;
  username: string;
  password?: string | null;
  private_key?: string | null;
}

interface FileMetadata {
  name: string;
  is_dir: boolean;
  size: number;
  modified_time: number | null;
  permissions: number | null;
}

const props = defineProps<{
  sessionId: string;
  host: Host;
  isActive: boolean;
}>();

const emit = defineEmits(['toast']);

function showToast(message: string, type: 'success' | 'error' = 'success') {
  emit('toast', { message, type });
}

const currentPath = ref('');
const files = ref<FileMetadata[]>([]);
const loading = ref(false);
const hasConnected = ref(false);

const isDraggingFile = ref(false);
const draggedInternalFile = ref<FileMetadata | null>(null);
const dragOverDir = ref<string | null>(null);

let unlistenDropHover: any = null;
let unlistenDrop: any = null;
let unlistenDropCancelled: any = null;

interface TransferProgress {
  session_id: string;
  file: string;
  transfer_type: 'download' | 'upload';
  transferred: number;
  total: number;
}
const activeTransfers = ref<Record<string, TransferProgress>>({});

const isEditingPath = ref(false);
const editPathValue = ref('');
const pathInputRef = ref<HTMLInputElement | null>(null);

const showMenu = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const menuItems = ref<MenuItem[]>([]);
const contextFile = ref<FileMetadata | null>(null);

function onEmptyContextMenu(e: MouseEvent) {
  contextFile.value = null;
  menuItems.value = [
    { label: '刷新', action: 'refresh' },
    { label: '新建文件夹', action: 'mkdir' },
    { label: '上传文件', action: 'upload' }
  ];
  menuX.value = e.clientX;
  menuY.value = e.clientY;
  showMenu.value = true;
}

function onFileContextMenu(e: MouseEvent, file: FileMetadata) {
  contextFile.value = file;
  const mItems: MenuItem[] = [
    { label: '下载', action: 'download', disabled: file.is_dir }
  ];
  
  if (!file.is_dir) {
    const lowerName = file.name.toLowerCase();
    if (lowerName.endsWith('.zip') || lowerName.endsWith('.tar.gz') || lowerName.endsWith('.tgz') || lowerName.endsWith('.tar')) {
      mItems.push({ label: '解压到本目录', action: 'extract' });
    }
  }
  
  mItems.push(
    { label: '压缩为 .tar.gz', action: 'compress' },
    { label: '重命名', action: 'rename' },
    { divider: true },
    { label: '删除', action: 'delete', danger: true }
  );

  menuItems.value = mItems;
  menuX.value = e.clientX;
  menuY.value = e.clientY;
  showMenu.value = true;
}

function handleMenuAction(action: string) {
  showMenu.value = false;
  if (!contextFile.value) {
    if (action === 'refresh') refresh();
    if (action === 'mkdir') mkdir();
    if (action === 'upload') uploadFile();
  } else {
    const file = contextFile.value;
    if (action === 'download' && !file.is_dir) downloadFile(file);
    if (action === 'extract') extractFile(file);
    if (action === 'compress') compressFile(file);
    if (action === 'rename') renameFile(file);
    if (action === 'delete') deleteFile(file);
  }
}

const promptVisible = ref(false);
const promptTitle = ref('');
const promptValue = ref('');
const promptAction = ref<((val: string) => void) | null>(null);

function customPrompt(title: string, defVal: string = ''): Promise<string | null> {
  return new Promise(resolve => {
    promptTitle.value = title;
    promptValue.value = defVal;
    promptAction.value = (val: string) => {
      promptVisible.value = false;
      resolve(val || null);
    };
    promptVisible.value = true;
    nextTick(() => {
      const input = document.getElementById('sftp-prompt-input');
      if (input) input.focus();
    });
  });
}

function cancelPrompt() {
  if (promptAction.value) promptAction.value('');
}
function confirmPrompt() {
  if (promptAction.value) promptAction.value(promptValue.value);
}

const confirmVisible = ref(false);
const confirmMessage = ref('');
const confirmAction = ref<((val: boolean) => void) | null>(null);

function customConfirm(message: string): Promise<boolean> {
  return new Promise(resolve => {
    confirmMessage.value = message;
    confirmAction.value = (val: boolean) => {
      confirmVisible.value = false;
      resolve(val);
    };
    confirmVisible.value = true;
  });
}

function cancelConfirm() {
  if (confirmAction.value) confirmAction.value(false);
}
function applyConfirm() {
  if (confirmAction.value) confirmAction.value(true);
}

// 初始化并按需连接
let unlistenProgress: (() => void) | null = null;
onMounted(async () => {
  unlistenProgress = await listen<TransferProgress>('sftp-progress', (event) => {
    if (event.payload.session_id === props.sessionId) {
      if (event.payload.transferred >= event.payload.total && event.payload.total > 0) {
        setTimeout(() => {
          delete activeTransfers.value[event.payload.file];
        }, 1500);
      }
      activeTransfers.value[event.payload.file] = event.payload;
    }
  });

  unlistenDropHover = await listen('tauri://drag-enter', () => {
    if (props.isActive && !draggedInternalFile.value) {
      isDraggingFile.value = true;
    }
  });

  unlistenDropCancelled = await listen('tauri://drag-leave', () => {
    isDraggingFile.value = false;
  });

  unlistenDrop = await listen('tauri://drop', (event: any) => {
    if (!props.isActive) return;
    isDraggingFile.value = false;
    if (draggedInternalFile.value) return;

    const paths = event.payload?.paths || event.payload;
    if (Array.isArray(paths) && paths.length > 0) {
      handleOsFileDrop(paths);
    }
  });

  if (props.isActive) {
    hasConnected.value = true;
    await connect();
  }
});

watch(() => props.isActive, async (active) => {
  if (active && !hasConnected.value) {
    hasConnected.value = true;
    await connect();
  }
});

onUnmounted(async () => {
  if (unlistenProgress) unlistenProgress();
  if (unlistenDropHover) unlistenDropHover();
  if (unlistenDropCancelled) unlistenDropCancelled();
  if (unlistenDrop) unlistenDrop();

  if (props.sessionId) {
    try {
      await invoke('sftp_close', { sessionId: props.sessionId });
    } catch(e) {}
  }
});

function onInternalDragStart(e: DragEvent, file: FileMetadata) {
  draggedInternalFile.value = file;
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', file.name);
  }
}

function onInternalDragEnd() {
  draggedInternalFile.value = null;
  dragOverDir.value = null;
}

function handleDragEnter(e: DragEvent, name: string) {
  dragOverDir.value = name;
}

function handleInternalDragEnter(e: DragEvent, file: FileMetadata) {
  if (file.is_dir) dragOverDir.value = file.name;
}

function handleDragLeave(e: DragEvent) {
  const current = e.currentTarget as HTMLElement;
  const related = e.relatedTarget as Node | null;
  // If moving within the same TR row, ignore the leave event
  if (!current.contains(related)) {
    dragOverDir.value = null;
  }
}

async function handleInternalDrop(e: DragEvent, file: FileMetadata) {
  if (!file.is_dir) {
    showToast(`Drop failed: ${file.name} is not a directory`, 'error');
    return;
  }
  await onInternalDrop(file.name);
}

async function onInternalDrop(targetDirName: string) {
  dragOverDir.value = null;
  const file = draggedInternalFile.value;
  if (!file) {
    showToast(`Drop failed: draggedInternalFile is null`, 'error');
    return;
  }
  draggedInternalFile.value = null;
  if (file.name === targetDirName) {
    showToast(`Drop ignored: same file name`, 'error');
    return; 
  }
  
  const basePath = currentPath.value.replace(/\/$/, '');
  const oldPath = basePath === '' ? `/${file.name}` : `${basePath}/${file.name}`;
  
  let newPath = '';
  if (targetDirName === '..') {
    const parts = basePath.split('/');
    parts.pop();
    const upPath = parts.join('/') || '/';
    newPath = (upPath === '/' ? '' : upPath) + '/' + file.name;
  } else {
    newPath = basePath === '' ? `/${targetDirName}/${file.name}` : `${basePath}/${targetDirName}/${file.name}`;
  }
  
  try {
    showToast(`正在移动 ${file.name}...`);
    await invoke('sftp_rename', {
        sessionId: props.sessionId,
        oldPath,
        newPath
    });
    showToast('移动成功!');
    await refresh();
  } catch(e: any) {
    showToast(`移动失败: ${e}`, 'error');
  }
}

async function handleOsFileDrop(paths: string[]) {
  for (const p of paths) {
      if (!p) continue;
      const filename = p.split(/[/\\]/).pop();
      if (!filename) continue;
      
      const bp = currentPath.value.replace(/\/$/, '');
      const targetPath = bp === '' ? `/${filename}` : `${bp}/${filename}`;
      showToast(`正在异步上传 ${filename}...`);
      invoke('sftp_upload_file', {
          sessionId: props.sessionId,
          localPath: p,
          remotePath: targetPath,
      }).then(() => {
          showToast(`${filename} 上传完成!`);
          refresh();
      }).catch(err => {
          showToast(`${filename} 上传失败: ${err}`, 'error');
      });
  }
}

async function connect() {
  loading.value = true;
  try {
    const authType = props.host.private_key ? 'private_key' : 'password';
    await invoke('sftp_connect', {
      sessionId: props.sessionId,
      host: props.host.host,
      port: props.host.port,
      username: props.host.username,
      authType,
      password: props.host.password,
      privateKey: props.host.private_key
    });
    await loadDirectory('');
  } catch (err: any) {
    showToast('SFTP 连接失败: ' + err, 'error');
  } finally {
    loading.value = false;
  }
}

async function loadDirectory(path: string, showLoading = false) {
  if (!props.sessionId) return;
  if (showLoading) loading.value = true;
  try {
    const res = await invoke<{ current_path: string, files: FileMetadata[] }>('sftp_read_dir', {
      sessionId: props.sessionId,
      path: path
    });
    currentPath.value = res.current_path;
    files.value = res.files;
  } catch (err: any) {
    showToast('读取目录失败: ' + err, 'error');
  } finally {
    if (showLoading) loading.value = false;
  }
}

const sortedFiles = computed(() => {
  if (!files.value) return [];
  return [...files.value].sort((a, b) => {
    if (a.is_dir && !b.is_dir) return -1;
    if (!a.is_dir && b.is_dir) return 1;
    return a.name.localeCompare(b.name);
  });
});

function formatSize(bytes: number) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

async function handleDoubleClick(file: FileMetadata) {
  if (file.is_dir) {
    let nextPath = currentPath.value.endsWith('/') ? `${currentPath.value}${file.name}` : `${currentPath.value}/${file.name}`;
    await loadDirectory(nextPath);
  }
}

async function goUp() {
  if (!currentPath.value || currentPath.value === '/') return;
  const parts = currentPath.value.replace(/\/$/, '').split('/');
  parts.pop();
  let nextPath = parts.join('/') || '/';
  await loadDirectory(nextPath);
}

function startEditPath() {
  editPathValue.value = currentPath.value;
  isEditingPath.value = true;
  nextTick(() => {
    pathInputRef.value?.focus();
  });
}

async function finishEditPath() {
  if (!isEditingPath.value) return;
  isEditingPath.value = false;
  if (editPathValue.value !== currentPath.value) {
    await loadDirectory(editPathValue.value);
  }
}

async function refresh() {
  await loadDirectory(currentPath.value, false);
}

async function mkdir() {
  const name = await customPrompt('请输入新文件夹名称:');
  if (!name || !props.sessionId) return;
  const targetPath = currentPath.value.endsWith('/') ? `${currentPath.value}${name}` : `${currentPath.value}/${name}`;
  try {
    await invoke('sftp_mkdir', { sessionId: props.sessionId, path: targetPath });
    showToast('新建文件夹成功');
    await refresh();
  } catch (err: any) {
    showToast('新建失败: ' + err, 'error');
  }
}

async function renameFile(file: FileMetadata) {
  const newName = await customPrompt('重命名为:', file.name);
  if (!newName || newName === file.name || !props.sessionId) return;
  const oldPath = currentPath.value.endsWith('/') ? `${currentPath.value}${file.name}` : `${currentPath.value}/${file.name}`;
  const newPath = currentPath.value.endsWith('/') ? `${currentPath.value}${newName}` : `${currentPath.value}/${newName}`;
  try {
    await invoke('sftp_rename', { sessionId: props.sessionId, oldPath, newPath });
    showToast('重命名成功');
    await refresh();
  } catch (err: any) {
    showToast('重命名失败: ' + err, 'error');
  }
}

async function deleteFile(file: FileMetadata) {
  if (!(await customConfirm(`确定要删除 ${file.name} 吗？`))) return;
  if (!props.sessionId) return;
  const targetPath = currentPath.value.endsWith('/') ? `${currentPath.value}${file.name}` : `${currentPath.value}/${file.name}`;
  try {
    await invoke('sftp_delete', { sessionId: props.sessionId, path: targetPath, isDir: file.is_dir });
    showToast('删除成功');
    await refresh();
  } catch (err: any) {
    showToast('删除失败: ' + err, 'error');
  }
}

async function downloadFile(file: FileMetadata) {
  if (!props.sessionId) return;
  try {
    const defaultPath = file.name;
    const savePath = await save({ defaultPath });
    if (!savePath) return; // user cancelled
    showToast('开始下载...');
    await invoke('sftp_download_file', { 
      sessionId: props.sessionId, 
      remotePath: currentPath.value.endsWith('/') ? `${currentPath.value}${file.name}` : `${currentPath.value}/${file.name}`,
      localPath: savePath
    });
    showToast('下载完成');
  } catch (err: any) {
    showToast('下载失败: ' + err, 'error');
  }
}

async function uploadFile() {
  if (!props.sessionId) return;
  try {
    const selected = await open({ multiple: false });
    if (!selected) return; // user cancelled
    // get file name from local path (very basic extraction)
    const localPathStr = Array.isArray(selected) ? selected[0] : selected;
    if (typeof localPathStr !== 'string') return;
    
    // Fallback simple path parsing
    const localPath = localPathStr as string;
    const nameMatch = localPath.match(/[^\\/]+$/);
    const name = nameMatch ? nameMatch[0] : 'uploaded_file';
    
    const remotePath = currentPath.value.endsWith('/') ? `${currentPath.value}${name}` : `${currentPath.value}/${name}`;
    
    showToast('开始上传...');
    await invoke('sftp_upload_file', {
      sessionId: props.sessionId,
      localPath,
      remotePath
    });
    showToast('上传完成');
    await refresh();
  } catch (err: any) {
    showToast('上传失败: ' + err, 'error');
  }
}

async function compressFile(file: FileMetadata) {
  if (!props.sessionId) return;
  const parentPath = currentPath.value.replace(/\/$/, ''); 
  const pPath = parentPath === '' ? '/' : parentPath;
  try {
    showToast(`正在压缩 ${file.name}...`);
    await invoke('sftp_compress', { 
      sessionId: props.sessionId, 
      parentPath: pPath, 
      targetName: file.name 
    });
    showToast('压缩成功!');
    await refresh();
  } catch (err: any) {
    showToast('压缩失败: ' + err, 'error');
  }
}

async function extractFile(file: FileMetadata) {
  if (!props.sessionId) return;
  const parentPath = currentPath.value.replace(/\/$/, ''); 
  const pPath = parentPath === '' ? '/' : parentPath;
  try {
    showToast(`正在解压 ${file.name}...`);
    await invoke('sftp_extract', { 
      sessionId: props.sessionId, 
      parentPath: pPath, 
      targetName: file.name 
    });
    showToast('解压成功!');
    await refresh();
  } catch (err: any) {
    showToast('解压失败: ' + err, 'error');
  }
}
</script>

<style scoped>
.sftp-browser {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  background: var(--bg-main);
  color: var(--text-main);
  font-family: inherit;
}

.sftp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background: var(--bg-sidebar);
  border-bottom: 1px solid var(--border-color);
}

.path-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}

.path-display {
  padding: 6px 12px;
  background: rgba(255, 255, 255, 0.05); /* or var(--bg-hover) */
  border-radius: 6px;
  cursor: pointer;
  flex: 1;
  font-family: monospace;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  border: 1px solid transparent;
}

.path-display:hover {
  background: rgba(255, 255, 255, 0.08); /* or var(--bg-active) */
}

.path-input {
  flex: 1;
  padding: 6px 12px;
  background: var(--bg-main);
  border: 1px solid var(--primary-color, #6366f1);
  border-radius: 6px;
  color: var(--text-main);
  font-family: monospace;
  font-size: 13px;
  outline: none;
}

.sftp-actions {
  display: flex;
  gap: 8px;
  margin-left: 16px;
}

.icon-btn {
  background: transparent;
  border: 1px solid var(--border-color);
  color: var(--text-muted);
  padding: 6px;
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.icon-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-main);
}

.icon-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
  border-color: transparent;
}

.sftp-content {
  flex: 1;
  overflow: auto;
  position: relative;
}

.loading-overlay {
  position: absolute;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
  font-size: 14px;
  color: #fff;
  backdrop-filter: blur(2px);
}

.file-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.file-table th {
  text-align: left;
  padding: 8px 16px;
  color: var(--text-muted);
  font-weight: 500;
  border-bottom: 1px solid var(--border-color);
  position: sticky;
  top: 0;
  background: var(--bg-main, #fff);
  z-index: 10;
}

.file-table td {
  padding: 8px 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.03); /* var(--border-color) */
}

.file-table tr:hover td {
  background: rgba(255, 255, 255, 0.03); /* var(--bg-hover) */
}

.file-name-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.file-icon {
  width: 16px;
  height: 16px;
}

.file-icon.folder {
  color: #60a5fa; /* Blue */
  fill: rgba(96, 165, 250, 0.2);
}

.file-icon.file {
  color: #9ca3af; /* Gray */
}

.file-name {
  cursor: pointer;
}

.action-cell {
  display: flex;
  gap: 12px;
}

.text-btn {
  background: none;
  border: none;
  box-shadow: none; /* 移除可能的全局 button 样式 */
  font-size: 13px;
  color: #60a5fa; /* 蓝色 */
  cursor: pointer;
  padding: 0;
  opacity: 0.8;
  transition: opacity 0.2s;
}

.text-btn:hover {
  opacity: 1;
  text-decoration: underline;
}

.text-btn.danger {
  color: #ef4444; /* 红色 */
}

.empty-state {
  text-align: center;
  padding: 40px !important;
  color: var(--text-muted);
}

.sftp-prompt-overlay {
  position: absolute;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 999;
}

.sftp-prompt-box {
  background: var(--bg-main, #fff);
  padding: 20px;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  width: 320px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  border: 1px solid var(--border-color, #eee);
}

.prompt-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-main, #333);
}

.prompt-input {
  padding: 8px 12px;
  border: 1px solid var(--border-color, #ddd);
  border-radius: 6px;
  outline: none;
  font-size: 13px;
  background: var(--bg-hover, #f9f9f9);
  color: var(--text-main, #333);
}

.prompt-input:focus {
  border-color: #6366f1;
}

.prompt-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.btn-cancel {
  background: transparent;
  border: 1px solid var(--border-color, #ddd);
  color: var(--text-main, #333);
  padding: 6px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}

.btn-cancel:hover {
  background: var(--bg-hover, #f5f5f5);
}

.btn-confirm {
  background: #6366f1;
  border: none;
  color: #fff;
  padding: 6px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}

.btn-confirm:hover {
  background: #4f46e5;
}

.btn-confirm.danger {
  background: #ef4444;
}

.btn-confirm.danger:hover {
  background: #dc2626;
}

.confirm-message {
  font-size: 13px;
  color: var(--text-main, #333);
  margin-bottom: 8px;
}

.transfer-panel {
  position: absolute;
  bottom: 20px;
  right: 20px;
  width: 280px;
  background: var(--bg-main, #ffffff);
  border: 1px solid var(--border-color, #eee);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0,0,0,0.15);
  padding: 12px;
  z-index: 100;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.transfer-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.transfer-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
  color: var(--text-main, #333);
}

.transfer-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.transfer-size {
  color: var(--text-muted, #777);
  font-size: 11px;
  white-space: nowrap;
}

.transfer-name {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
  min-width: 0;
  margin-right: 8px;
}

.progress-bar {
  height: 4px;
  background: var(--border-color, #eee);
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: #6366f1;
  transition: width 0.1s linear;
}
tr.drag-over td {
  background: rgba(99, 102, 241, 0.15) !important;
  border-bottom-color: #6366f1;
}

.file-action-row:hover { background: var(--bg-hover, #f8f9fa); }

.drag-overlay {
  position: absolute; top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(99, 102, 241, 0.15); backdrop-filter: blur(4px);
  z-index: 100; display: flex; align-items: center; justify-content: center;
  border: 4px dashed var(--accent-color, #6366f1); border-radius: 12px; margin: 12px;
  pointer-events: none;
}
.drag-overlay-content {
  background: rgba(255,255,255,0.95);
  padding: 32px 48px; border-radius: 20px; box-shadow: 0 20px 40px rgba(0,0,0,0.1);
  display: flex; flex-direction: column; align-items: center; gap: 16px;
}
.drag-icon { width: 56px; height: 56px; color: var(--accent-color, #6366f1); }
</style>
