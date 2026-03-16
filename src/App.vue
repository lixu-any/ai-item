<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow, getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen, emit } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import Terminal from "./components/Terminal.vue";
import ContextMenu, { MenuItem } from "./components/ContextMenu.vue";
import SpotlightSearch from "./components/SpotlightSearch.vue";
import SnippetSidebar from "./components/SnippetSidebar.vue";
import CredentialManager from "./components/CredentialManager.vue";
import SvgIcon from "./components/SvgIcon.vue";
import SettingsModal from "./components/SettingsModal.vue";
import AiSidebar from "./components/AiSidebar.vue";

interface SessionTab {
  id: string;
  title: string;
  type: 'ssh' | 'local';
  host?: Host;
  connected?: boolean;
}

interface Group {
  id?: number;
  name: string;
  parent_id?: number | null;
}

interface Host {
  id?: number;
  name: string;
  group_id?: number | null;
  host: string;
  port: number;
  username: string;
  password?: string | null;
  private_key?: string | null;
}

const sessions = ref<SessionTab[]>([]);
const activeSessionId = ref<string | null>(null);
const connecting = ref(false);
const connError = ref("");

const savedHosts = ref<Host[]>([]);
const groups = ref<Group[]>([]);
const showAddModal = ref(false);
const isEditing = ref(false);
const showGroupModal = ref(false);
const saveError = ref("");
const deletingHostId = ref<number | null>(null);
const authType = ref<'password' | 'private_key'>('password');
const pkType = ref<'path' | 'content'>('path');
const searchQuery = ref("");
const collapsedGroups = ref<number[]>([]);
const collapsedUnGrouped = ref(false);
const toasts = ref<{id: number, message: string, type: 'success' | 'error'}[]>([]);
let toastIdCounter = 0;

const showSpotlight = ref(false);
const showSnippetSidebar = ref(false);
const showAiSidebar = ref(false);
const showCredentialManager = ref(false);
const terminalRefs = ref<Record<string, any>>({});

function setTerminalRef(id: string, el: any) {
  if (el) terminalRefs.value[id] = el;
  else delete terminalRefs.value[id];
}

function handleRunSnippet(command: string) {
  if (activeSessionId.value && terminalRefs.value[activeSessionId.value]) {
    terminalRefs.value[activeSessionId.value].write(command + '\n');
    showToast('已发送命令片段');
  } else {
    showToast('没有活动的终端窗口', 'error');
  }
}

function handleSelectCredential(c: any) {
  newHost.value.username = c.username;
  if (c.password) {
    newHost.value.password = c.password;
    authType.value = 'password';
  }
  if (c.private_key) {
    newHost.value.private_key = c.private_key;
    authType.value = 'private_key';
    pkType.value = c.private_key.includes('BEGIN') ? 'content' : 'path';
  }
  showCredentialManager.value = false;
  showToast(`已应用凭据: ${c.name}`);
}

const viewMode = ref<'main' | 'add-host' | 'edit-host' | 'add-group' | 'edit-group' | 'settings' | 'credentials'>('main');
const hostId = ref<number | null>(null);
const groupId = ref<number | null>(null);
const deletingGroupId = ref<number | null>(null);

// 窗口大小持久化相关
const appWindow = getCurrentWebviewWindow();
let resizeTimer: any = null;

// 右键菜单相关
const showTabMenu = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const contextTabId = ref<string | null>(null);
const tabMenuItems = ref<MenuItem[]>([]);

const newGroup = ref<Group>({
  name: '',
  parent_id: null,
});

function showToast(message: string, type: 'success' | 'error' = 'success') {
  const id = ++toastIdCounter;
  toasts.value.push({ id, message, type });
  setTimeout(() => {
    toasts.value = toasts.value.filter(t => t.id !== id);
  }, 3000);
}

const newHost = ref<Host>({
  name: '',
  host: '',
  port: 22,
  username: 'root',
  password: '',
  private_key: '',
  group_id: null,
});

const filteredHosts = () => {
  if (!searchQuery.value) return savedHosts.value;
  const q = searchQuery.value.toLowerCase();
  return savedHosts.value.filter(h => 
    h.name.toLowerCase().includes(q) || 
    h.host.toLowerCase().includes(q) ||
    h.username.toLowerCase().includes(q)
  );
};

function toggleGroup(groupId: number) {
  const index = collapsedGroups.value.indexOf(groupId);
  if (index > -1) {
    collapsedGroups.value.splice(index, 1);
  } else {
    collapsedGroups.value.push(groupId);
  }
}

function isGroupCollapsed(groupId: number) {
  return collapsedGroups.value.includes(groupId);
}

function isHostActive(host: Host) {
  return sessions.value.some(s => s.type === 'ssh' && s.host?.id === host.id);
}

async function loadHosts() {
  try {
    savedHosts.value = await invoke("get_hosts");
  } catch (err) {
    console.error("加载主机列表失败:", err);
  }
}

async function loadGroups() {
  try {
    groups.value = await invoke("get_groups");
  } catch (err) {
    console.error("加载分组失败:", err);
  }
}

onMounted(async () => {
  const params = new URLSearchParams(window.location.search);
  const view = params.get('view');
  
  // 无论什么模式，都需要加载业务分组以便选择
  loadGroups();

  if (view === 'add-host') {
    viewMode.value = 'add-host';
    openAddModal();
  } else if (view === 'edit-host') {
    const id = params.get('id');
    if (id) {
      viewMode.value = 'edit-host';
      hostId.value = parseInt(id);
      try {
        const h = await invoke<Host>("get_host", { id: hostId.value });
        editHost(h);
      } catch (err) {
        showToast("加载主机信息失败: " + err, "error");
      }
    }
  } else if (view === 'add-group') {
    viewMode.value = 'add-group';
    openAddGroupModal();
  } else if (view === 'edit-group') {
    const id = params.get('id');
    if (id) {
      viewMode.value = 'edit-group';
      groupId.value = parseInt(id);
      try {
        const g = await invoke<Group>("get_group", { id: groupId.value });
        editGroup(g);
      } catch (err) {
        showToast("加载分组信息失败: " + err, "error");
      }
    }
  } else if (view === 'settings') {
    viewMode.value = 'settings';
  } else if (view === 'credentials') {
    viewMode.value = 'credentials';
  } else {
    viewMode.value = 'main';
    loadHosts();
    loadGroups();
    listen('host-changed', () => {
      console.log("收到 host-changed 事件，刷新列表");
      loadHosts();
      loadGroups();
    });
    
    appWindow.onResized(() => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(saveWindowSize, 500);
    });

    // 注册全局快捷键
    window.addEventListener('keydown', (e) => {
      // Cmd+K or Cmd+P (Mac) / Ctrl+K or Ctrl+P (Others)
      const isMod = window.navigator.platform.includes('Mac') ? (e.metaKey || e.ctrlKey) : e.ctrlKey;
      if (isMod && (e.key === 'k' || e.key === 'p')) {
        e.preventDefault();
        showSpotlight.value = !showSpotlight.value;
      }
    });
  }
});

async function saveHost() {
  saveError.value = "";
  try {
    const hostToSave = { ...newHost.value };
    if (authType.value === 'password') hostToSave.private_key = null;
    else hostToSave.password = null;

    if (isEditing.value) {
      await invoke("update_host", { host: hostToSave });
    } else {
      await invoke("add_host", { host: hostToSave });
    }
    
    // 发出全局通知并关闭当前窗口（如果是独立模式）
    await emit('host-changed');
    
    if (viewMode.value !== 'main') {
      const win = getCurrentWebviewWindow();
      await win.close();
    } else {
      // 兼容旧模式或内建模式（如果有的话）
      showToast("保存成功");
      showAddModal.value = false;
      isEditing.value = false;
      newHost.value = { name: '', host: '', port: 22, username: 'root', password: '', private_key: '', group_id: null };
      await loadHosts();
    }
  } catch (err) {
    saveError.value = "保存失败: " + String(err);
  }
}

async function openAddModal() {
  console.log("尝试打开‘添加主机’窗口，当前模式:", viewMode.value);
  if (viewMode.value === 'main') {
    try {
      const webview = new WebviewWindow('add-host', {
        url: 'index.html?view=add-host',
        title: '添加主机',
        width: 440,
        height: 580,
        resizable: false,
        maximizable: false,
        minimizable: false,
      });
      
      webview.once('tauri://error', (e) => {
        console.error('窗口创建失败事件:', e);
        showToast("无法创建窗口，请检查权限", "error");
      });
      
      webview.once('tauri://created', () => {
        console.log("‘添加主机’窗口创建成功");
      });
    } catch (err) {
      console.error("创建窗口时抛出异常:", err);
      showToast("创建窗口失败: " + err, "error");
    }
  } else {
    isEditing.value = false;
    saveError.value = '';
    newHost.value = { name: '', host: '', port: 22, username: 'root', password: '', private_key: '', group_id: null };
    showAddModal.value = true;
  }
}

function openSettingsWindow() {
  if (viewMode.value === 'main') {
    try {
      const webview = new WebviewWindow('settings', {
        url: 'index.html?view=settings',
        title: '全局设置',
        width: 480,
        height: 520,
        resizable: false,
        maximizable: false,
        minimizable: false,
      });
      
      webview.once('tauri://error', (e) => {
        console.error('设置窗口创建失败事件:', e);
        showToast("无法创建窗口，请检查权限或确认窗口已打开", "error");
      });
    } catch (err) {
      console.error("创建设置窗口时抛出异常:", err);
    }
  }
}

function openCredentialWindow() {
  if (viewMode.value === 'main') {
    try {
      const webview = new WebviewWindow('credentials', {
        url: 'index.html?view=credentials',
        title: '凭据中心',
        width: 560,
        height: 560,
        resizable: false,
        maximizable: false,
        minimizable: false,
      });
      webview.once('tauri://error', () => {
        showToast("无法创建凭据中心窗口，请确认窗口已打开", "error");
      });
    } catch (err) {
      console.error("创建凭据中心窗口时抛出异常:", err);
    }
  }
}

async function editHost(h: Host) {
  console.log("尝试打开‘编辑主机’窗口, id:", h.id);
  if (viewMode.value === 'main') {
    try {
      const webview = new WebviewWindow(`edit-host-${h.id}`, {
        url: `index.html?view=edit-host&id=${h.id}`,
        title: `编辑主机: ${h.name}`,
        width: 440,
        height: 580,
        resizable: false,
        maximizable: false,
        minimizable: false,
      });
      
      webview.once('tauri://error', (e) => {
        console.error('编辑窗口创建失败事件:', e);
      });
    } catch (err) {
      console.error("创建编辑窗口时抛出异常:", err);
    }
  } else {
    isEditing.value = true;
    saveError.value = '';
    newHost.value = { ...h };
    authType.value = h.private_key ? 'private_key' : 'password';
    showAddModal.value = true;
  }
}

async function saveGroup() {
  saveError.value = "";
  if (!newGroup.value.name.trim()) {
    saveError.value = "分组名称不能为空";
    return;
  }
  try {
    if (viewMode.value === 'edit-group' || (viewMode.value === 'main' && isEditing.value)) {
      await invoke("update_group", { group: newGroup.value });
    } else {
      await invoke("add_group", { name: newGroup.value.name, parentId: newGroup.value.parent_id });
    }
    
    await emit('host-changed');
    
    if (viewMode.value !== 'main') {
      const win = getCurrentWebviewWindow();
      await win.close();
    } else {
      showToast("分组保存成功");
      showGroupModal.value = false;
      newGroup.value = { name: '', parent_id: null };
      await loadGroups();
    }
  } catch (err) {
    saveError.value = "保存分组失败: " + String(err);
  }
}

async function openAddGroupModal() {
  if (viewMode.value === 'main') {
    new WebviewWindow('add-group', {
      url: 'index.html?view=add-group',
      title: '添加分组',
      width: 440,
      height: 280,
      resizable: false,
      maximizable: false,
      minimizable: false,
    });
  } else {
    isEditing.value = false;
    saveError.value = '';
    newGroup.value = { name: '', parent_id: null };
    showGroupModal.value = true;
  }
}

async function editGroup(g: Group) {
  if (viewMode.value === 'main') {
    new WebviewWindow(`edit-group-${g.id}`, {
      url: `index.html?view=edit-group&id=${g.id}`,
      title: `编辑分组: ${g.name}`,
      width: 440,
      height: 280,
      resizable: false,
      maximizable: false,
      minimizable: false,
    });
  } else {
    isEditing.value = true;
    saveError.value = '';
    newGroup.value = { ...g };
    showGroupModal.value = true;
  }
}

async function deleteGroup(id: number) {
  console.log("触发分组删除, ID:", id);
  deletingGroupId.value = id;
}

async function confirmDeleteGroup() {
  if (deletingGroupId.value === null) return;
  try {
    console.log("正在执行分组删除, ID:", deletingGroupId.value);
    await invoke("delete_group", { id: deletingGroupId.value });
    showToast("分组已删除");
    await emit('host-changed');
    await loadGroups();
    await loadHosts();
  } catch (err) {
    showToast("删除分组失败: " + err, "error");
  } finally {
    deletingGroupId.value = null;
  }
}

async function deleteHost(id: number) {
  console.log("触发主机删除, ID:", id);
  deletingHostId.value = id;
}

async function confirmDelete() {
  if (deletingHostId.value === null) return;
  const id = deletingHostId.value;
  try {
    console.log("正在从数据库删除主机, ID:", id);
    await invoke("delete_host", { id });
    showToast("已删除主机");
    await loadHosts();
  } catch (err) {
    showToast("删除失败: " + err, "error");
  } finally {
    deletingHostId.value = null;
  }
}

async function newLocalTerminal() {
  const sessionId = crypto.randomUUID();
  const newSession: SessionTab = {
    id: sessionId,
    title: '本地终端',
    type: 'local',
    connected: true
  };
  
  sessions.value.push(newSession);
  activeSessionId.value = sessionId;
  
  try {
    // 默认开启尺寸，稍后 Terminal.vue 会触发 resize 同步更准确的值
    await invoke('open_pty_session', {
      sessionId: sessionId, // PTY session is tied to sessionId
      cols: 80,
      rows: 24
    });
  } catch (err) {
    showToast(`无法开启本地终端: ${err}`, 'error');
    sessions.value = sessions.value.filter(s => s.id !== sessionId);
  }
}

async function connectToHost(host: Host) {
  connecting.value = true;
  connError.value = "";
  
  const sessionId = crypto.randomUUID();
  sessions.value.push({
    id: sessionId,
    title: `${host.username}@${host.host}`,
    type: 'ssh',
    host: host,
    connected: true
  });
  activeSessionId.value = sessionId;

  await nextTick();
  await new Promise(r => setTimeout(r, 100));

  try {
    console.log("正在调用 open_ssh_session...");
    await invoke("open_ssh_session", {
      sessionId: sessionId,
      host: host.host,
      port: host.port,
      username: host.username,
      password: host.password || null,
      privateKey: host.private_key || null,
      cols: 80,
      rows: 24
    });
    showToast(`成功连接到 ${host.name}`);
  } catch (err) {
    console.error("连接异常:", err);
    showToast(`连接失败: ${err}`, "error");
    closeSession(sessionId);
  } finally {
    connecting.value = false;
  }
}

async function reconnectSession(session: SessionTab) {
  // 先断开
  await disconnectSession(session);
  
  // 重新连接：由于 connectToHost 会创建新 SessionID，这里我们需要一种原地重连的方式，
  // 或者简单地把旧的删了加个新的。这里为了简单，我们采用“删除并新建”的逻辑。
  closeSession(session.id);
  if (session.type === 'ssh' && session.host) {
    connectToHost(session.host);
  } else {
    newLocalTerminal();
  }
}

async function disconnectSession(session: SessionTab) {
  try {
    if (session.type === 'ssh') {
      await invoke('close_ssh_session', { sessionId: session.id });
    } else {
      await invoke('close_pty_session', { sessionId: session.id });
    }
    session.connected = false;
    showToast(`已断开会话: ${session.title}`);
  } catch (err) {
    console.error("断开会话失败:", err);
  }
}

// 右键菜单处理
function onTabContextMenu(e: MouseEvent, session: SessionTab) {
  menuX.value = e.clientX;
  menuY.value = e.clientY;
  contextTabId.value = session.id;
  
  const isConnected = session.connected !== false;
  
  tabMenuItems.value = [
    { label: '复制标签', icon: '👯', action: 'duplicate' },
    { divider: true },
    { label: '连接', icon: '🔗', action: 'connect', disabled: isConnected },
    { label: '重新连接', icon: '🔄', action: 'reconnect' },
    { label: '断开连接', icon: '🔌', action: 'disconnect', disabled: !isConnected },
    { divider: true },
    { label: '关闭', icon: '❌', action: 'close' },
    { label: '关闭其他', icon: '🧹', action: 'closeOthers' },
    { label: '关闭全部', icon: '🗑️', action: 'closeAll', danger: true },
  ];
  
  showTabMenu.value = true;
}

async function handleTabMenuAction(action: string) {
  const targetId = contextTabId.value;
  if (!targetId) return;
  
  const session = sessions.value.find(s => s.id === targetId);
  if (!session) return;

  switch (action) {
    case 'duplicate':
      if (session.type === 'ssh' && session.host) {
        connectToHost(session.host);
      } else {
        newLocalTerminal();
      }
      break;
    case 'connect':
      if (session.type === 'ssh' && session.host) {
        // 由于目前的 open_ssh_session 等逻辑都是异步且依赖新生成 ID 的，
        // 原地重连比较复杂。为了 UX 的一致性，这里我们直接调用重连逻辑即“重开”。
        reconnectSession(session);
      } else {
        reconnectSession(session);
      }
      break;
    case 'reconnect':
      reconnectSession(session);
      break;
    case 'disconnect':
      disconnectSession(session);
      break;
    case 'close':
      closeSession(targetId);
      break;
    case 'closeOthers':
      sessions.value = sessions.value.filter(s => s.id === targetId);
      activeSessionId.value = targetId;
      break;
    case 'closeAll':
      sessions.value = [];
      activeSessionId.value = null;
      break;
  }
}

// 移除 splitPane 函数

function onDragStart(event: DragEvent, hostId: number) {
  if (event.dataTransfer) {
    event.dataTransfer.setData("hostId", hostId.toString());
    event.dataTransfer.effectAllowed = "move";
  }
}

async function onDrop(event: DragEvent, groupId: number | null) {
  event.preventDefault();
  const hostIdStr = event.dataTransfer?.getData("hostId");
  if (!hostIdStr) return;
  
  const hostId = parseInt(hostIdStr);
  const host = savedHosts.value.find(h => h.id === hostId);
  if (host && host.group_id !== groupId) {
    try {
      const updatedHost = { ...host, group_id: groupId };
      await invoke("update_host", { host: updatedHost });
      await loadHosts();
      console.log(`主机 ${hostId} 已移动到分组 ${groupId}`);
    } catch (err) {
      alert("移动失败: " + err);
    }
  }
}

function closeSession(id: string) {
  sessions.value = sessions.value.filter(s => s.id !== id);
  if (activeSessionId.value === id) {
    activeSessionId.value = sessions.value.length > 0 ? sessions.value[sessions.value.length - 1].id : null;
  }
}

async function cancelForm() {
  if (viewMode.value !== 'main') {
    const win = getCurrentWebviewWindow();
    await win.close();
  } else {
    showAddModal.value = false;
  }
}

async function selectPrivateKeyFile() {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      title: '选择私钥文件',
    });
    if (selected) {
      newHost.value.private_key = selected as string;
    }
  } catch (err) {
    showToast("选择文件失败: " + err, "error");
  }
}

async function saveWindowSize() {
  try {
    const size = await appWindow.innerSize();
    await invoke("save_setting", { key: "window_width", value: size.width.toString() });
    await invoke("save_setting", { key: "window_height", value: size.height.toString() });
  } catch (err) {
    console.error("Failed to save window size:", err);
  }
}

</script>

<template>
  <!-- Main Application Mode -->
  <div v-if="viewMode === 'main'" class="app-layout">
    <aside class="sidebar">
      <div class="sidebar-header">
        <span class="sidebar-title">主机列表</span>
        <div class="header-actions">
          <button class="add-btn primary" @click="openAddGroupModal" title="添加分组">
            <SvgIcon name="group" size="14" />
          </button>
          <button class="add-btn accent" @click="openAddModal" title="添加主机">
            <SvgIcon name="add" size="14" />
          </button>
        </div>
      </div>
      
      <div class="search-container">
        <div class="search-input-wrapper">
          <span class="search-icon">🔍</span>
          <input v-model="searchQuery" placeholder="搜索名称 / IP / 用户..." />
          <span v-if="searchQuery" class="clear-search" @click="searchQuery = ''">×</span>
        </div>
      </div>

      <div class="host-list">
        <!-- 渲染分组及其主机 -->
        <div v-for="g in groups" :key="g.id!" 
          v-show="!searchQuery || filteredHosts().some(h => h.group_id === g.id)"
          class="group-section"
          :class="{ collapsed: isGroupCollapsed(g.id!) }"
          @dragover.prevent
          @drop="onDrop($event, g.id!)"
        >
          <div class="group-header" @click="toggleGroup(g.id!)">
            <span class="chevron">›</span>
            <SvgIcon name="group" size="16" class="folder-icon" />
            <span class="group-name">{{ g.name }}</span>
            <div class="group-actions">
              <button class="icon-btn" @click.stop="editGroup(g)" title="编辑分组">
                <SvgIcon name="edit" size="14" />
              </button>
              <button class="icon-btn delete-btn" @click.stop="deleteGroup(g.id!)" title="删除分组">
                <SvgIcon name="delete" size="14" />
              </button>
            </div>
          </div>
          <div class="group-content" v-show="!isGroupCollapsed(g.id!) || searchQuery">
            <div 
              v-for="h in filteredHosts().filter((h: Host) => h.group_id === g.id)" 
              :key="h.id!" 
              class="host-item" 
              @dblclick="connectToHost(h)"
              draggable="true"
              @dragstart="onDragStart($event, h.id!)"
            >
              <div class="host-info">
                <div class="host-name-row">
                  <span v-if="isHostActive(h)" class="status-dot"></span>
                  <span class="host-name">{{ h.name }}</span>
                </div>
                <span class="host-ip">{{ h.username }}@{{ h.host }}</span>
              </div>
              <div class="host-actions">
                 <button class="icon-btn" @click.stop="connectToHost(h)" title="连接">
                   <SvgIcon name="play" size="14" />
                 </button>
                 <button class="icon-btn" @click.stop="editHost(h)" title="编辑">
                   <SvgIcon name="edit" size="14" />
                 </button>
                 <button class="icon-btn delete-btn" @click.stop="deleteHost(h.id!)" title="删除">
                   <SvgIcon name="delete" size="14" />
                 </button>
              </div>
            </div>
          </div>
        </div>

        <!-- 未分组主机 -->
        <div v-if="filteredHosts().some(h => !h.group_id)" class="group-section"
          :class="{ collapsed: collapsedUnGrouped }"
          @dragover.prevent
          @drop="onDrop($event, null)"
        >
          <div class="group-header" @click="collapsedUnGrouped = !collapsedUnGrouped">
            <span class="chevron">›</span>
            <span class="folder-icon">📂</span>
            <span class="group-name">未分组</span>
          </div>
          <div class="group-content" v-show="!collapsedUnGrouped || searchQuery">
            <div 
              v-for="h in filteredHosts().filter((h: Host) => !h.group_id)" 
              :key="h.id!" 
              class="host-item" 
              @dblclick="connectToHost(h)"
              draggable="true"
              @dragstart="onDragStart($event, h.id!)"
            >
              <div class="host-info">
                <div class="host-name-row">
                  <span v-if="isHostActive(h)" class="status-dot"></span>
                  <span class="host-name">{{ h.name }}</span>
                </div>
                <span class="host-ip">{{ h.username }}@{{ h.host }}</span>
              </div>
              <div class="host-actions">
                 <button class="icon-btn" @click.stop="connectToHost(h)" title="连接">
                   <SvgIcon name="play" size="14" />
                 </button>
                 <button class="icon-btn" @click.stop="editHost(h)" title="编辑">
                   <SvgIcon name="edit" size="14" />
                 </button>
                 <button class="icon-btn delete-btn" @click.stop="deleteHost(h.id!)" title="删除">
                   <SvgIcon name="delete" size="14" />
                 </button>
              </div>
            </div>
          </div>
        </div>

        <div v-if="filteredHosts().length === 0 && searchQuery" class="empty-state">
          <SvgIcon name="search" size="48" class="empty-icon" />
          <p>没有找到匹配的主机</p>
        </div>
        <div v-else-if="savedHosts.length === 0" class="empty-state">
          <SvgIcon name="host" size="48" class="empty-icon" />
          <p>点击上方 + 开始添加第一台服务器</p>
        </div>
      </div>
      <div class="sidebar-footer">
        <div class="footer-item" @click="newLocalTerminal" title="本地终端">
          <SvgIcon name="host" size="18" class="footer-icon" />
          <span>本地终端</span>
        </div>
        <div class="footer-item" @click="showSnippetSidebar = !showSnippetSidebar" :class="{ active: showSnippetSidebar }" title="命令片段">
          <SvgIcon name="snippet" size="18" class="footer-icon" />
          <span>命令片段</span>
        </div>
        <div class="footer-item" @click="showAiSidebar = !showAiSidebar" :class="{ active: showAiSidebar }" title="AI 助手">
          <SvgIcon name="ai" size="18" class="footer-icon" />
          <span>AI 助手</span>
        </div>
        <div class="footer-item" title="凭据管理" @click="openCredentialWindow">
          <SvgIcon name="credential" size="18" class="footer-icon" />
          <span>凭据中心</span>
        </div>
        <div class="footer-item" title="设置" @click="openSettingsWindow">
          <SvgIcon name="settings" size="18" class="footer-icon" />
          <span>设置</span>
        </div>
      </div>
    </aside>
    <main class="main-content">
      <header class="top-bar">
        <div v-if="sessions.length === 0" class="no-tabs">未连接</div>
        <div 
          v-for="session in sessions" 
          :key="session.id" 
          class="tab"
          :class="{ 
            active: activeSessionId === session.id,
            disconnected: session.connected === false
          }"
          @click="activeSessionId = session.id"
          @contextmenu.prevent="onTabContextMenu($event, session)"
        >
          <span>{{ session.title }}</span>
          <div class="tab-actions">
            <SvgIcon name="close" size="12" class="close-icon" @click.stop="closeSession(session.id)" />
          </div>
        </div>
      </header>
      <div class="terminal-wrapper">
        <div v-if="connecting" class="terminal-overlay">
          <div class="loader"></div>
          <p>正在建立安全连接...</p>
        </div>
        <div v-if="sessions.length === 0 && !connecting" class="terminal-empty">
          <div class="empty-hero">
            <h1>AI-Term Shell</h1>
            <p>基于人工智能的高效 SSH 终端</p>
            <div class="hero-actions">
              <button @click="openAddModal">新增服务器</button>
              <button class="secondary-btn" @click="newLocalTerminal">本地终端</button>
            </div>
          </div>
        </div>
        <div class="main-terminal-area">
          <Terminal 
            v-for="session in sessions" 
            :key="session.id"
            v-show="activeSessionId === session.id"
            :ref="(el) => setTerminalRef(session.id, el)"
            :session-id="session.id" 
            :is-active="activeSessionId === session.id"
            :type="session.type"
          />
        </div>
      </div>
    </main>
    <SnippetSidebar v-if="showSnippetSidebar" @run-snippet="handleRunSnippet" @close="showSnippetSidebar = false" />
    <AiSidebar v-if="showAiSidebar" @run-command="handleRunSnippet" @toast="showToast" @close="showAiSidebar = false" />
  </div>

  <!-- Standalone Form / Modal -->
  <div v-if="viewMode !== 'main' || showAddModal || showGroupModal" 
    :class="viewMode === 'main' ? 'modal-overlay' : 'standalone-page'">
    
    <!-- Host Form -->
    <div v-if="viewMode === 'add-host' || viewMode === 'edit-host' || showAddModal" 
      class="modal-content" :class="{ 'premium-modal': isEditing, 'standalone-window': viewMode !== 'main' }">
      <div class="modal-header-accent"></div>
      <div class="form-header">
        <SvgIcon :name="isEditing ? 'edit' : 'add'" size="20" class="header-icon" />
        <h3>{{ isEditing ? '编辑主机配置' : '添加新主机' }}</h3>
        <button class="use-cred-btn" @click="showCredentialManager = true">复用凭据</button>
      </div>
      
      <div class="form-scroll-area">
        <div class="form-grid">
          <div class="form-group animate-in" style="--delay: 0.1s">
            <label>标识名称</label>
            <input v-model="newHost.name" placeholder="e.g. 生产环境-Web01" autofocus />
          </div>
          
          <div class="form-row animate-in" style="--delay: 0.2s">
            <div class="form-group flex-2">
              <label>主机地址 (IP/Domain)</label>
              <input v-model="newHost.host" placeholder="192.168.1.100" />
            </div>
            <div class="form-group flex-1">
              <label>端口</label>
              <input v-model.number="newHost.port" type="number" />
            </div>
          </div>

          <div class="form-group animate-in" style="--delay: 0.3s">
            <label>业务分组</label>
            <select v-model="newHost.group_id" class="group-select">
              <option :value="null">未分组 (Default)</option>
              <option v-for="g in groups" :key="g.id!" :value="g.id">{{ g.name }}</option>
            </select>
          </div>

          <div class="form-group animate-in" style="--delay: 0.4s">
            <label>SSH 用户名</label>
            <input v-model="newHost.username" placeholder="root" />
          </div>

          <div class="form-group animate-in" style="--delay: 0.5s">
            <label>认证方式</label>
            <div class="auth-tabs">
              <div class="auth-tab" :class="{ active: authType === 'password' }" @click="authType = 'password'">
                <span>🔑</span> 密码认证
              </div>
              <div class="auth-tab" :class="{ active: authType === 'private_key' }" @click="authType = 'private_key'">
                <span>📜</span> 私钥认证
              </div>
            </div>
          </div>

          <div v-if="authType === 'password'" class="form-group animate-in" style="--delay: 0.6s">
            <label>登录密码</label>
            <input v-model="newHost.password" type="password" placeholder="••••••••" />
          </div>
          
          <div v-else class="form-group animate-in" style="--delay: 0.6s">
            <div style="display: flex; justify-content: space-between; align-items: center;">
              <label>私钥认证</label>
              <div class="pk-toggle">
                <span :class="{ active: pkType === 'path' }" @click="pkType = 'path'">路径</span>
                <span :class="{ active: pkType === 'content' }" @click="pkType = 'content'">直接粘贴</span>
              </div>
            </div>
            <div v-if="pkType === 'path'" class="input-with-btn">
              <input v-model="newHost.private_key" placeholder="~/.ssh/id_rsa" />
              <button class="browse-btn" @click="selectPrivateKeyFile">浏览</button>
            </div>
            <textarea v-else v-model="newHost.private_key" placeholder="-----BEGIN OPENSSH PRIVATE KEY-----&#10;..." rows="4" style="font-family: 'JetBrains Mono', monospace; font-size: 0.75rem;"></textarea>
          </div>
        </div>
      </div>

      <div v-if="saveError" class="modal-error animate-in">{{ saveError }}</div>

      <div class="modal-footer">
        <button class="modal-btn secondary" @click="cancelForm">取消</button>
        <button class="modal-btn primary" @click="saveHost">
          {{ isEditing ? '更新配置' : '立即保存' }}
        </button>
      </div>
    </div>

    <!-- Group Form -->
    <div v-if="viewMode === 'add-group' || viewMode === 'edit-group' || showGroupModal" 
      class="modal-content" :class="{ 'standalone-window': viewMode !== 'main' }">
      <div v-if="viewMode === 'main'" class="form-header">
        <span class="header-icon">{{ isEditing ? '📁' : '📁' }}</span>
        <h3>{{ isEditing ? '编辑业务分组' : '创建新分组' }}</h3>
      </div>
      
      <div class="form-scroll-area">
        <div class="form-grid">
          <div class="form-group animate-in" style="--delay: 0.1s">
            <label>分组名称</label>
            <input v-model="newGroup.name" placeholder="e.g. 生产环境" @keyup.enter="saveGroup" autofocus />
          </div>
        </div>
      </div>

      <div v-if="saveError" class="modal-error animate-in">{{ saveError }}</div>

      <div class="modal-footer">
        <button class="modal-btn secondary" @click="cancelForm">取消</button>
        <button class="modal-btn primary" @click="saveGroup">
          {{ isEditing ? '更新分组' : '立即创建' }}
        </button>
      </div>
    </div>
        <div v-if="viewMode === 'settings'" class="modal-content standalone-window premium-modal" style="width: 100%; height: 100vh; border-radius: 0; padding: 0; background-color: var(--bg-main);">
      <SettingsModal @close="cancelForm" @toast="showToast" />
    </div>

    <!-- Credentials Standalone -->
    <div v-if="viewMode === 'credentials'" class="modal-content standalone-window premium-modal" style="width: 100%; height: 100vh; border-radius: 0; padding: 0; background-color: var(--bg-main);">
      <CredentialManager @close="cancelForm" @select="() => {}" @toast="(t: any) => showToast(t.message, t.type)" />
    </div>
  </div>

  <!-- 分组删除确认 Modal -->
  <div v-if="deletingGroupId !== null" class="modal-overlay">
    <div class="modal-content confirm-modal">
      <h3 style="color: var(--danger)">确认删除分组</h3>
      <p style="color: var(--text-dim); line-height: 1.6;">确定要删除该分组吗？分组内的虚拟主机会被移至“未分组”。</p>
      <div class="modal-actions" style="margin-top: 24px;">
        <button class="modal-btn secondary" @click="deletingGroupId = null">取消</button>
        <button class="modal-btn primary danger" @click="confirmDeleteGroup">确认删除</button>
      </div>
    </div>
  </div>

  <!-- 删除确认 Modal -->
  <div v-if="deletingHostId !== null" class="modal-overlay">
    <div class="modal-content confirm-modal">
      <h3 style="color: var(--danger)">确认删除</h3>
      <p style="color: var(--text-dim); line-height: 1.6;">确定要删除该主机吗？此操作不可撤销。</p>
      <div class="modal-actions" style="margin-top: 24px;">
        <button class="modal-btn secondary" @click="deletingHostId = null">取消</button>
        <button class="modal-btn primary danger" @click="confirmDelete">确认删除</button>
      </div>
    </div>
  </div>

  <!-- Toast Container -->
  <SpotlightSearch 
    v-model:visible="showSpotlight" 
    :hosts="savedHosts" 
    @select="connectToHost"
  />

  <div class="toast-container">
    <div v-for="t in toasts" :key="t.id" class="toast" :class="t.type">
      {{ t.message }}
    </div>
  </div>

  <!-- Tab Context Menu -->
  <ContextMenu 
    v-model:visible="showTabMenu"
    :x="menuX"
    :y="menuY"
    :items="tabMenuItems"
    @action="handleTabMenuAction"
  />

  <CredentialManager 
    v-if="showCredentialManager" 
    @close="showCredentialManager = false"
    @select="handleSelectCredential"
    @toast="(t) => showToast(t.message, t.type)"
  />
</template>

<style>
:root {
  --bg-dark: hsl(0, 0%, 98%);
  --sidebar-bg: hsl(0, 0%, 94%);
  --sidebar-hover: hsl(0, 0%, 90%);
  --border-color: hsla(0, 0%, 0%, 0.1);
  --accent-color: hsl(210, 100%, 52%);
  --accent-hover: hsl(210, 100%, 45%);
  --text-main: hsl(0, 0%, 12%);
  --text-dim: hsl(0, 0%, 45%);
  --danger: hsl(0, 80%, 50%);
  --glass-bg: hsla(0, 0%, 0%, 0.03);
  --glass-border: hsla(0, 0%, 0%, 0.08);
}

body, html, #app {
  margin: 0; padding: 0;
  height: 100%; width: 100%;
  overflow: hidden;
  background-color: var(--bg-dark);
  color: var(--text-main);
  font-family: 'Inter', -apple-system, sans-serif;
}

.app-layout {
  display: flex;
  height: 100vh; width: 100vw;
}

/* Sidebar Styling */
.sidebar {
  width: 260px;
  background-color: var(--sidebar-bg);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
}

.sidebar-header {
  padding: 24px 16px 12px;
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

.search-container {
  padding: 0 12px 16px;
}
.search-input-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}
.search-icon {
  position: absolute;
  left: 10px;
  font-size: 0.75rem;
  opacity: 0.5;
}
.search-input-wrapper input {
  width: 100%;
  background: var(--bg-dark);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 8px 30px 8px 28px;
  color: var(--text-main);
  font-size: 0.75rem;
  box-sizing: border-box;
  transition: all 0.2s;
}
.search-input-wrapper input:focus { 
  border-color: var(--accent-color); 
  background: var(--sidebar-hover);
  outline: none; 
}
.clear-search {
  position: absolute; 
  right: 10px; 
  color: var(--text-dim); 
  cursor: pointer; 
  font-size: 0.9rem;
}

.header-actions { display: flex; gap: 4px; padding: 2px; background: rgba(0,0,0,0.03); border-radius: 8px; }

.add-btn {
  background: white;
  border: 1px solid var(--border-color);
  color: var(--text-dim);
  width: 28px; height: 28px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  padding: 0;
  font-size: 0.85rem;
  box-shadow: 0 1px 2px rgba(0,0,0,0.05);
}
.add-btn:hover {
  background: var(--bg-dark);
  color: var(--text-main);
  transform: translateY(-1px);
  box-shadow: 0 2px 4px rgba(0,0,0,0.08);
}
.add-btn.primary { 
  background: var(--accent-color); 
  color: white; 
  border: none;
  font-weight: bold;
  font-size: 1rem;
}
.add-btn.primary:hover {
  background: var(--accent-hover);
  box-shadow: 0 4px 12px rgba(57, 108, 216, 0.4);
} 

.host-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 4px;
}

.group-section { margin-bottom: 4px; }
.group-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  cursor: pointer;
  border-radius: 8px;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}
.group-header:hover { background: var(--sidebar-hover); }

.chevron {
  font-size: 0.9rem;
  color: var(--text-dim);
  transition: transform 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  width: 12px;
  display: flex;
  justify-content: center;
  transform: rotate(90deg);
}
.collapsed .chevron { transform: rotate(0deg); }

.folder-icon { 
  font-size: 0.9rem; 
  opacity: 0.8; 
  color: var(--accent-color);
}
.group-actions {
  margin-left: auto;
  display: flex;
  gap: 6px;
  opacity: 0;
  transition: opacity 0.2s;
}
.group-header:hover .group-actions { opacity: 1; }
.group-name { 
  flex: 1; 
  overflow: hidden; 
  text-overflow: ellipsis; 
  white-space: nowrap; 
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--text-main);
  text-transform: uppercase;
  letter-spacing: 0.03em;
  opacity: 0.9;
}

.group-content {
  margin-left: 20px;
  padding-left: 12px;
  border-left: 1px solid rgba(0,0,0,0.06);
  display: flex;
  flex-direction: column;
  gap: 0;
  margin-top: 2px;
}

.host-item {
  padding: 2px 10px;
  border-radius: 8px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.2s ease;
  cursor: pointer;
}
.host-item:hover { background: var(--sidebar-hover); transform: translateX(2px); }

.host-info { display: flex; flex-direction: column; overflow: hidden; }
.host-name-row { display: flex; align-items: center; gap: 6px; }
.status-dot {
  min-width: 8px; height: 8px; border-radius: 50%;
  background: #34c759;
  box-shadow: 0 0 8px rgba(52, 199, 89, 0.4);
  animation: pulse 2s infinite;
}
@keyframes pulse { 0% { opacity: 1; } 50% { opacity: 0.4; } 100% { opacity: 1; } }
.host-name { font-size: 0.85rem; font-weight: 500; color: #1a1a1a; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.host-item:hover .host-name { color: var(--accent-color); }
.host-ip { font-size: 0.65rem; color: var(--text-dim); opacity: 0.6; font-family: 'JetBrains Mono', monospace; margin-top: 1px; }
.host-item:hover .host-ip { opacity: 0.9; }

.host-actions { display: flex; gap: 8px; opacity: 0; }
.host-item:hover .host-actions { opacity: 1; }

.icon-btn {
  background: none; border: none; padding: 4px; border-radius: 4px;
  color: var(--text-dim); cursor: pointer; transition: 0.2s;
  font-size: 0.85rem;
}
.icon-btn:hover { color: var(--accent-color); background: var(--border-color); }
.delete-btn:hover { color: var(--danger) !important; }

.sidebar-footer {
  padding: 12px 8px;
  border-top: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.footer-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 0.8rem;
  color: var(--text-dim);
  cursor: pointer;
  transition: all 0.2s;
}
.footer-item:hover {
  background: var(--sidebar-hover);
  color: var(--text-main);
}
.footer-item.active {
  background: var(--sidebar-hover);
  color: var(--accent-color);
  font-weight: 600;
}
.footer-icon { font-size: 1rem; opacity: 0.8; }
.action-icon {
  width: 14px;
  height: 14px;
  border: 1.5px solid currentColor;
  opacity: 0.6;
  position: relative;
  transition: all 0.2s;
  cursor: pointer;
  display: inline-block;
}
.action-icon:hover { opacity: 1; color: var(--accent-color); border-color: var(--accent-color); }

/* Custom Split Icons */
.split-icon-v::after {
  content: '';
  position: absolute;
  top: 0; left: 50%; bottom: 0;
  width: 1.5px; background: currentColor;
  transform: translateX(-50%);
}
.split-icon-h::after {
  content: '';
  position: absolute;
  left: 0; top: 50%; right: 0;
  height: 1.5px; background: currentColor;
  transform: translateY(-50%);
}

.session-container {
  display: flex;
  width: 100%;
  height: 100%;
  gap: 2px;
  background-color: #1a1a1a;
}

.layout-horizontal {
  flex-direction: column;
}

.layout-vertical {
  flex-direction: row;
}

.session-container > * {
  flex: 1;
  min-width: 0;
  min-height: 0;
  height: 100%;
}

.terminal-pane-active {
  outline: 2px solid #396cd8;
  z-index: 1;
}

/* Empty State */
.empty-state {
  display: flex; flex-direction: column; align-items: center;
  justify-content: center; padding: 40px 20px; color: var(--text-dim);
  text-align: center;
}
.empty-icon { font-size: 2rem; margin-bottom: 12px; opacity: 0.5; }
.empty-state p { font-size: 0.85rem; margin: 0; }

/* Main Content Area */
.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.top-bar {
  height: 40px;
  background-color: var(--sidebar-bg);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: flex-end;
  padding: 0 10px;
  gap: 2px;
}

.tab {
  height: 32px;
  padding: 0 14px;
  background: var(--sidebar-bg);
  border: 1px solid var(--border-color);
  border-bottom: none;
  border-radius: 6px 6px 0 0;
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-dim);
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.2s;
  margin-bottom: -1px;
  position: relative;
}
.tab:hover { background: var(--sidebar-hover); color: var(--text-main); }
.tab.disconnected { opacity: 0.6; }
.tab.disconnected span { text-decoration: line-through; opacity: 0.8; }
.tab.active {
  background: var(--bg-dark);
  color: var(--text-main);
  border-color: var(--border-color);
  z-index: 2;
}
.tab.active::after {
  content: '';
  position: absolute;
  top: 0; left: 0; right: 0; height: 2px;
  background: var(--accent-color);
  border-radius: 6px 6px 0 0;
}

.tab-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: 8px;
}

.action-icon {
  width: 16px;
  height: 16px;
  border: 1px solid currentColor;
  opacity: 0.6;
  position: relative;
  transition: all 0.2s;
  cursor: pointer;
}
.action-icon:hover { opacity: 1; color: var(--accent-color); border-color: var(--accent-color); }

/* Custom Split Icons */
.split-icon-v::after {
  content: '';
  position: absolute;
  top: 0; left: 50%; bottom: 0;
  width: 1px; background: currentColor;
}
.split-icon-h::after {
  content: '';
  position: absolute;
  left: 0; top: 50%; right: 0;
  height: 1px; background: currentColor;
}

.close-icon { 
  font-size: 14px; opacity: 0.5; transition: 0.2s;
  background: none; border: none; color: inherit; cursor: pointer;
}
.close-icon:hover { opacity: 1; transform: scale(1.1); }

.terminal-wrapper { 
  flex: 1; 
  position: relative; 
  background: #000; 
  overflow: hidden; 
  display: flex;
}

.main-terminal-area {
  flex: 1;
  position: relative;
  overflow: hidden;
  height: 100%;
}

.session-container {
  display: flex;
  width: 100%;
  height: 100%;
  gap: 0;
  background-color: #1a1a1a;
}

.layout-horizontal {
  flex-direction: column;
}

.layout-vertical {
  flex-direction: row;
}

.session-container > * {
  flex: 1;
  min-width: 0;
  min-height: 0;
  height: 100%;
}

.terminal-pane-active {
  outline: 2px solid #396cd8;
  z-index: 1;
}

.terminal-wrapper > div:not(.terminal-overlay):not(.terminal-empty) {
  flex: 1;
}

/* Terminal Empty & Loading */
.terminal-overlay {
  position: absolute; inset: 0; background: rgba(255,255,255,0.7);
  display: flex; flex-direction: column; align-items: center;
  justify-content: center; z-index: 100; backdrop-filter: blur(8px);
}
.loader {
  width: 40px; height: 40px; border: 3px solid var(--glass-border);
  border-top-color: var(--accent-color); border-radius: 50%;
  animation: spin 1s linear infinite; margin-bottom: 16px;
}
@keyframes spin { to { transform: rotate(360deg); } }

.terminal-empty {
  position: absolute; inset: 0; display: flex; align-items: center; justify-content: center;
  background: radial-gradient(circle at center, hsl(210, 30%, 94%) 0%, var(--bg-dark) 100%);
}
.empty-hero { text-align: center; }
.empty-hero h1 { 
  font-size: 3rem; margin-bottom: 8px; font-weight: 800; 
  line-height: 1.3;
  padding: 10px 0;
  background: linear-gradient(135deg, var(--text-main) 0%, var(--accent-color) 100%); 
  -webkit-background-clip: text; 
  background-clip: text;
  -webkit-text-fill-color: transparent; 
}
.hero-actions {
  display: flex;
  justify-content: center;
  gap: 16px;
}
.hero-actions button {
  background: var(--accent-color); color: white; border: none;
  padding: 12px 32px; border-radius: 12px; font-weight: 600;
  cursor: pointer; transition: 0.2s;
}
.hero-actions .secondary-btn {
  background: rgba(57, 108, 216, 0.1);
  color: var(--accent-color);
  border: 1px solid var(--accent-color);
}
.hero-actions button:hover { background: var(--accent-hover); transform: translateY(-2px); box-shadow: 0 8px 20px rgba(57, 108, 216, 0.4); }
.hero-actions .secondary-btn:hover { background: rgba(57, 108, 216, 0.15); color: var(--accent-hover); }

/* Modal Styling */
.modal-overlay {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: white;
  border: 1px solid var(--border-color);
  padding: 32px;
  border-radius: 20px;
  width: 440px;
  box-shadow: 0 40px 100px rgba(0,0,0,0.1);
}


.modal-content h3 {
  margin-top: 0;
  margin-bottom: 24px;
  font-size: 1.2rem;
  color: var(--text-main);
}

.form-grid {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.modal-content input, .modal-content textarea, .group-select {
  background: #fdfdfd;
  border: 1px solid var(--border-color);
  color: var(--text-main);
  padding: 12px 14px;
  border-radius: 12px;
  font-size: 0.95rem;
  transition: all 0.2s;
}
.modal-content input:focus { border-color: var(--accent-color); outline: none; box-shadow: 0 0 0 3px hsla(210, 100%, 50%, 0.1); }

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 20px;
  margin-top: 32px;
}

.modal-actions button {
  padding: 10px 24px;
  border-radius: 10px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  font-size: 0.9rem;
  border: 1px solid var(--border-color);
  background: var(--glass-bg);
  color: var(--text-main);
}
.modal-actions button.primary { background: var(--accent-color); color: white; border: none; }
.modal-actions button.danger { background: var(--danger); color: white; border: none; }

/* Toast Styling */
.toast-container {
  position: fixed;
  top: 24px;
  right: 24px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  z-index: 2000;
  pointer-events: none;
}
.toast {
  background: white;
  border: 1px solid var(--border-color);
  padding: 14px 24px;
  border-radius: 14px;
  color: var(--text-main);
  font-size: 0.85rem;
  box-shadow: 0 16px 48px rgba(0,0,0,0.08);
  animation: slideInRight 0.4s cubic-bezier(0.18, 0.89, 0.32, 1.28);
  min-width: 240px;
  max-width: 360px;
  pointer-events: auto;
  border-left-width: 5px;
}
.toast.success { border-left-color: var(--accent-color); }
.toast.error { border-left-color: var(--danger); }

@keyframes slideInRight {
  from { transform: translateX(100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}

/* Standalone Page & Modern Form */
.standalone-page {
  height: 100vh; width: 100vw;
  display: flex; background: var(--bg-dark);
  align-items: center; justify-content: center;
}
.standalone-window {
  width: 100% !important; height: 100% !important;
  border-radius: 0 !important; box-shadow: none !important;
  margin: 0 !important; display: flex; flex-direction: column;
  padding: 0 !important;
  background: var(--sidebar-bg);
}
.form-header {
  padding: 24px 32px 8px;
  display: flex; align-items: center; gap: 8px;
  position: relative;
  user-select: none;
}
.win-close-btn {
  position: absolute; right: 24px; top: 24px;
  background: transparent; border: none; color: var(--text-dim);
  opacity: 0.5; cursor: pointer; font-size: 1rem;
  transition: all 0.2s;
}
.win-close-btn:hover { opacity: 1; color: var(--danger); transform: scale(1.1); }
.header-icon { font-size: 1.2rem; filter: grayscale(0.5); }

.use-cred-btn {
  margin-left: auto;
  margin-right: 32px;
  background: var(--glass-bg);
  border: 1px solid var(--border-color);
  padding: 6px 12px;
  border-radius: 8px;
  font-size: 0.75rem;
  color: var(--accent-color);
  cursor: pointer;
}
.use-cred-btn:hover { background: var(--sidebar-hover); }

.standalone-window h3 {
  border: none; margin: 0; padding: 0;
  font-size: 1rem; font-weight: 700;
  color: var(--text-main);
  opacity: 0.8;
}
.form-scroll-area {
  flex: 1; overflow-y: auto; overflow-x: hidden; padding: 0 32px 24px;
}
/* Custom Scrollbar */
.form-scroll-area::-webkit-scrollbar {
  width: 4px;
}
.form-scroll-area::-webkit-scrollbar-track {
  background: transparent;
}
.form-scroll-area::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.03);
  border-radius: 10px;
}
.form-scroll-area:hover::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.08);
}

.form-grid { display: flex; flex-direction: column; gap: 18px; padding-top: 5px; }
.form-row { display: flex; gap: 12px; }
.flex-2 { flex: 2; }
.flex-1 { flex: 1; }
.form-group { display: flex; flex-direction: column; gap: 6px; }
.form-group label { font-size: 0.7rem; color: #4a4a4a; font-weight: 700; opacity: 0.9; text-transform: uppercase; letter-spacing: 0.02em; }

.modal-content input, .modal-content textarea, .group-select {
  background: var(--bg-dark);
  border: 1px solid var(--border-color);
  padding: 0 12px; 
  height: 38px;
  border-radius: 8px; 
  font-size: 0.85rem;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: none;
  box-sizing: border-box;
  width: 100%;
  line-height: 36px;
  color: var(--text-main);
}
.modal-content textarea { height: auto; padding: 10px 12px; line-height: 1.4; resize: vertical; }
.input-with-btn {
  display: flex;
  gap: 8px;
}
.input-with-btn input {
  flex: 1;
}
.browse-btn {
  background: white;
  border: 1px solid var(--border-color);
  color: var(--text-main);
  padding: 0 12px;
  border-radius: 8px;
  font-size: 0.75rem;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.2s;
}
.browse-btn:hover {
  background: var(--bg-dark);
  border-color: var(--accent-color);
}

.pk-toggle { font-size: 0.65rem; display: flex; gap: 8px; color: var(--text-dim); }
.pk-toggle span { cursor: pointer; padding: 2px 6px; border-radius: 4px; opacity: 0.5; }
.pk-toggle span.active { background: var(--border-color); opacity: 1; color: var(--text-main); font-weight: 600; }
.modal-content input:focus, .group-select:focus {
  border-color: var(--accent-color); background: white;
  box-shadow: 0 0 0 2px hsla(210, 100%, 52%, 0.05);
  outline: none;
}
/* Standardize select appearance */
.group-select {
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='10' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M2.5 3.5l2.5 2.5 2.5-2.5'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  padding-right: 32px;
}

.auth-tabs {
  display: flex; gap: 4px; padding: 3px;
  background: rgba(0,0,0,0.03); border-radius: 10px;
}
.auth-tab {
  flex: 1; padding: 6px; text-align: center; font-size: 0.75rem;
  border-radius: 7px; cursor: pointer; transition: all 0.2s;
  display: flex; align-items: center; justify-content: center; gap: 6px;
  color: var(--text-dim); font-weight: 500; border: none; background: transparent;
}
.auth-tab.active {
  background: white; color: var(--accent-color);
  box-shadow: 0 1px 4px rgba(0,0,0,0.05); font-weight: 600;
}

.modal-footer {
  padding: 16px 32px; border-top: 1px solid var(--border-color);
  display: flex; justify-content: flex-end; gap: 10px;
  background: transparent;
}
.modal-btn { 
  padding: 8px 18px; border-radius: 8px; font-weight: 600; 
  cursor: pointer; transition: all 0.2s; font-size: 0.8rem;
  border: 1px solid var(--border-color);
}
.modal-btn.primary { 
  background: var(--accent-color); color: white; border: none;
}
.modal-btn.primary:hover { opacity: 0.95; }
.modal-btn.secondary { background: white; color: var(--text-dim); }
.modal-btn.secondary:hover { color: var(--text-main); background: var(--bg-dark); }

.animate-in {
  animation: slideUp 0.4s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  opacity: 0; transform: translateY(5px);
  animation-delay: var(--delay, 0s);
}
@keyframes slideUp { to { opacity: 1; transform: translateY(0); } }
.modal-error { color: var(--danger); font-size: 0.75rem; font-weight: 600; padding: 8px 32px; background: hsla(0, 80%, 50%, 0.05); margin-bottom: 8px; }
</style>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

</style>
<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}

</style>