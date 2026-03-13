<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Terminal from "./components/Terminal.vue";

interface SessionTab {
  id: string;
  title: string;
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
const newGroupName = ref("");
const saveError = ref("");
const deletingHostId = ref<number | null>(null);
const authType = ref<'password' | 'private_key'>('password');
const searchQuery = ref("");
const toasts = ref<{id: number, message: string, type: 'success' | 'error'}[]>([]);
let toastIdCounter = 0;

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

function isHostActive(host: Host) {
  return sessions.value.some(s => s.title.includes(`${host.username}@${host.host}`));
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

onMounted(() => {
  loadHosts();
  loadGroups();
});

async function saveHost() {
  saveError.value = "";
  try {
    const hostToSave = { ...newHost.value };
    if (authType.value === 'password') hostToSave.private_key = null;
    else hostToSave.password = null;

    if (isEditing.value) {
      await invoke("update_host", { host: hostToSave });
      showToast("更新成功");
    } else {
      await invoke("add_host", { host: hostToSave });
      showToast("保存成功");
    }
    
    showAddModal.value = false;
    isEditing.value = false;
    newHost.value = { name: '', host: '', port: 22, username: 'root', password: '', private_key: '', group_id: null };
    authType.value = 'password';
    await loadHosts();
  } catch (err) {
    saveError.value = "保存失败: " + String(err);
  }
}

function openAddModal() {
  isEditing.value = false;
  saveError.value = '';
  newHost.value = { name: '', host: '', port: 22, username: 'root', password: '', private_key: '', group_id: null };
  showAddModal.value = true;
}

function editHost(h: Host) {
  isEditing.value = true;
  saveError.value = '';
  newHost.value = { ...h };
  authType.value = h.private_key ? 'private_key' : 'password';
  showAddModal.value = true;
}

async function saveGroup() {
  if (!newGroupName.value.trim()) return;
  try {
    await invoke("add_group", { name: newGroupName.value, parentId: null });
    newGroupName.value = "";
    showGroupModal.value = false;
    await loadGroups();
  } catch (err) {
    alert("创建分组失败: " + err);
  }
}

async function deleteHost(id: number) {
  deletingHostId.value = id;
}

async function confirmDelete() {
  if (deletingHostId.value === null) return;
  const id = deletingHostId.value;
  try {
    console.log("正在从数据库删除主机:", id);
    await invoke("delete_host", { id });
    showToast("已删除主机");
    await loadHosts();
  } catch (err) {
    showToast("删除失败: " + err, "error");
  } finally {
    deletingHostId.value = null;
  }
}

async function connectToHost(host: Host) {
  connecting.value = true;
  connError.value = "";
  
  const id = crypto.randomUUID();
  sessions.value.push({
    id: id,
    title: `${host.username}@${host.host}`
  });
  activeSessionId.value = id;

  await nextTick();
  await new Promise(r => setTimeout(r, 100));

  try {
    console.log("正在调用 open_ssh_session...");
    await invoke("open_ssh_session", {
      sessionId: id,
      host: host.host,
      port: host.port,
      username: host.username,
      password: host.password || null,
      privateKey: host.private_key || null
    });
    console.log("连接成功, sessionId:", id);
    showToast(`成功连接到 ${host.name}`);
  } catch (err) {
    console.error("连接异常:", err);
    showToast(`连接失败: ${err}`, "error");
    closeSession(id);
  } finally {
    connecting.value = false;
  }
}

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
</script>

<template>
  <div class="app-layout">
    <aside class="sidebar">
      <div class="sidebar-header">
        <span class="sidebar-title">主机列表</span>
        <div class="header-actions">
          <button class="add-btn" @click="showGroupModal = true" title="添加分组">
            <span>📁</span>
          </button>
          <button class="add-btn primary" @click="openAddModal" title="添加主机">
            <span>+</span>
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
        <div v-for="g in groups" :key="g.id!" class="group-section"
          @dragover.prevent
          @drop="onDrop($event, g.id!)"
        >
          <div class="group-header">
            <span class="folder-icon">📂</span>
            <span class="group-name">{{ g.name }}</span>
          </div>
          <div class="group-content">
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
                 <button class="icon-btn" @click.stop="connectToHost(h)" title="连接">▶</button>
                 <button class="icon-btn" @click.stop="editHost(h)" title="编辑">✎</button>
                 <button class="icon-btn delete-btn" @click.stop="deleteHost(h.id!)" title="删除">✗</button>
              </div>
            </div>
          </div>
        </div>

        <!-- 未分组主机 -->
        <div v-if="filteredHosts().some(h => !h.group_id)" class="group-section"
          @dragover.prevent
          @drop="onDrop($event, null)"
        >
          <div class="group-header">
            <span class="folder-icon">📂</span>
            <span class="group-name">未分组</span>
          </div>
          <div class="group-content">
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
                 <button class="icon-btn" @click.stop="connectToHost(h)" title="连接">▶</button>
                 <button class="icon-btn" @click.stop="editHost(h)" title="编辑">✎</button>
                 <button class="icon-btn delete-btn" @click.stop="deleteHost(h.id!)" title="删除">✗</button>
              </div>
            </div>
          </div>
        </div>

        <div v-if="filteredHosts().length === 0 && searchQuery" class="empty-state">
          <span class="empty-icon">🔍</span>
          <p>没有找到匹配的主机</p>
        </div>
        <div v-else-if="savedHosts.length === 0" class="empty-state">
          <span class="empty-icon">🚀</span>
          <p>点击上方 + 开始添加第一台服务器</p>
        </div>
      </div>
      <div class="sidebar-footer">
        <div class="footer-item" title="AI 助手">
          <span class="footer-icon">✨</span>
          <span>AI 助手</span>
        </div>
        <div class="footer-item" title="设置">
          <span class="footer-icon">⚙️</span>
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
          :class="{ active: activeSessionId === session.id }"
          @click="activeSessionId = session.id"
        >
          <span>{{ session.title }}</span>
          <button class="close-btn" @click.stop="closeSession(session.id)">×</button>
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
            </div>
          </div>
        </div>
        <Terminal 
          v-for="session in sessions" 
          :key="session.id"
          v-show="activeSessionId === session.id"
          :session-id="session.id" 
          :is-active="activeSessionId === session.id"
        />
      </div>
    </main>
    <div v-if="showAddModal" class="modal-overlay">
      <div class="modal-content premium-modal">
        <h3>{{ isEditing ? '编辑主机' : '添加主机' }}</h3>
        <input v-model="newHost.name" placeholder="备忘名称 (如：生产服务器一)" />
        <input v-model="newHost.host" placeholder="IP 地址或域名" />
        <input v-model.number="newHost.port" placeholder="SSH 端口" type="number" />
        <input v-model="newHost.username" placeholder="登录用户名" />
        
        <select v-model="newHost.group_id" class="group-select">
          <option :value="null">-- 选择分组 --</option>
          <option v-for="g in groups" :key="g.id!" :value="g.id">
            {{ g.name }}
          </option>
        </select>
        
        <div class="auth-toggle">
          <label><input type="radio" value="password" v-model="authType"> 密码登录</label>
          <label><input type="radio" value="private_key" v-model="authType"> 密钥登录</label>
        </div>

        <input v-if="authType === 'password'" v-model="newHost.password" placeholder="登录密码" type="password" />
        <textarea v-else v-model="newHost.private_key" placeholder="粘贴私钥内容 (-----BEGIN PRIVATE KEY-----...)"></textarea>

        <div v-if="saveError" class="error-msg">{{ saveError }}</div>

        <div class="modal-actions">
          <button @click="showAddModal = false">取消</button>
          <button class="primary" @click="saveHost">{{ isEditing ? '更新' : '保存' }}</button>
        </div>
      </div>
    </div>
    <div v-if="showGroupModal" class="modal-overlay">
      <div class="modal-content confirm-modal">
        <h3>创建新分组</h3>
        <input v-model="newGroupName" placeholder="分组名称 (如：生产环境)" @keyup.enter="saveGroup" />
        <div class="modal-actions">
          <button @click="showGroupModal = false">取消</button>
          <button class="primary" @click="saveGroup">创建</button>
        </div>
      </div>
    </div>
    <!-- 删除确认 Modal -->
    <div v-if="deletingHostId !== null" class="modal-overlay">
      <div class="modal-content confirm-modal">
        <h3>确认删除</h3>
        <p>确定要从主机列表中永久删除该配置吗？</p>
        <div class="modal-actions">
          <button @click="deletingHostId = null">取消</button>
          <button class="primary danger" @click="confirmDelete">确认删除</button>
        </div>
      </div>
    </div>
    <!-- Toast Notifications -->
    <div class="toast-container">
      <div v-for="t in toasts" :key="t.id" class="toast" :class="t.type">
        {{ t.message }}
      </div>
    </div>
  </div>
</template>

<style>
:root {
  --bg-dark: hsl(222, 18%, 13%);
  --sidebar-bg: hsl(222, 20%, 10%);
  --sidebar-hover: hsl(222, 20%, 18%);
  --border-color: hsla(222, 15%, 25%, 0.6);
  --accent-color: hsl(210, 100%, 60%);
  --accent-hover: hsl(210, 100%, 68%);
  --text-main: hsl(220, 10%, 94%);
  --text-dim: hsl(220, 10%, 70%);
  --danger: hsl(0, 84%, 65%);
  --glass-bg: hsla(0, 0%, 100%, 0.05);
  --glass-border: hsla(0, 0%, 100%, 0.12);
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

.header-actions { display: flex; gap: 6px; }

.add-btn {
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  color: var(--text-main);
  width: 26px; height: 26px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  padding: 0;
  font-size: 0.7rem;
}
.add-btn:hover {
  background: var(--sidebar-hover);
  border-color: var(--accent-color);
}
.add-btn.primary { background: var(--accent-color); color: white; border: none; }

.host-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 4px;
}

.group-section { margin-bottom: 2px; }
.group-header {
  display: flex;
  align-items: center;
  padding: 6px 12px;
  border-radius: 6px;
  color: var(--text-dim);
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
}
.group-header:hover { background: var(--glass-bg); color: var(--text-main); }
.folder-icon { margin-right: 8px; font-size: 0.9rem; opacity: 0.7; }

.group-content {
  margin-left: 18px;
  padding-left: 10px;
  border-left: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
}

.host-item {
  padding: 4px 10px;
  border-radius: 6px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.15s ease;
  cursor: pointer;
}
.host-item:hover { background: var(--sidebar-hover); }

.host-info { display: flex; flex-direction: column; overflow: hidden; }
.host-name-row { display: flex; align-items: center; gap: 6px; }
.status-dot {
  min-width: 6px; height: 6px; border-radius: 50%;
  background: #4caf50;
  box-shadow: 0 0 6px #4caf50;
  animation: pulse 2s infinite;
}
@keyframes pulse { 0% { opacity: 1; } 50% { opacity: 0.4; } 100% { opacity: 1; } }
.host-name { font-size: 0.8rem; font-weight: 500; color: var(--text-dim); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.host-item:hover .host-name { color: var(--text-main); }
.host-ip { font-size: 0.65rem; color: var(--text-dim); opacity: 0.4; font-family: 'JetBrains Mono', monospace; }

.host-actions { display: flex; gap: 4px; opacity: 0; }
.host-item:hover .host-actions { opacity: 1; }

.icon-btn {
  background: none; border: none; padding: 2px; border-radius: 4px;
  color: var(--text-dim); cursor: pointer; transition: 0.2s;
  font-size: 0.7rem;
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
.footer-icon { font-size: 1rem; opacity: 0.8; }

.modal-content.confirm-modal {
  border-color: var(--border-color);
  width: 400px;
}
.confirm-modal p {
  color: var(--text-dim);
  line-height: 1.6;
  font-size: 0.9rem;
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

.close-btn { 
  font-size: 14px; opacity: 0.5; transition: 0.2s;
  background: none; border: none; color: inherit; cursor: pointer;
}
.close-btn:hover { opacity: 1; transform: scale(1.1); }

.terminal-wrapper { flex: 1; position: relative; background: #000; overflow: hidden; }

/* Terminal Empty & Loading */
.terminal-overlay {
  position: absolute; inset: 0; background: rgba(0,0,0,0.8);
  display: flex; flex-direction: column; align-items: center;
  justify-content: center; z-index: 100; backdrop-filter: blur(4px);
}
.loader {
  width: 40px; height: 40px; border: 3px solid var(--glass-border);
  border-top-color: var(--accent-color); border-radius: 50%;
  animation: spin 1s linear infinite; margin-bottom: 16px;
}
@keyframes spin { to { transform: rotate(360deg); } }

.terminal-empty {
  position: absolute; inset: 0; display: flex; align-items: center; justify-content: center;
  background: radial-gradient(circle at center, hsla(215, 30%, 15%, 1) 0%, var(--bg-dark) 100%);
}
.empty-hero { text-align: center; }
.empty-hero h1 { font-size: 2.5rem; margin-bottom: 8px; font-weight: 800; background: linear-gradient(135deg, #fff 0%, #396cd8 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
.empty-hero p { color: var(--text-dim); font-size: 1.1rem; margin-bottom: 32px; }
.hero-actions button {
  background: var(--accent-color); color: white; border: none;
  padding: 12px 32px; border-radius: 12px; font-weight: 600;
  cursor: pointer; transition: 0.2s;
}
.hero-actions button:hover { background: var(--accent-hover); transform: translateY(-2px); box-shadow: 0 8px 20px rgba(57, 108, 216, 0.4); }

/* Modal Styling */
.modal-overlay {
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(12px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: var(--sidebar-bg);
  border: 1px solid var(--border-color);
  padding: 32px;
  border-radius: 16px;
  width: 440px;
  box-shadow: 0 24px 64px rgba(0,0,0,0.6);
}

.modal-content.premium-modal { border-color: var(--accent-color); }

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
  background: var(--bg-dark);
  border: 1px solid var(--border-color);
  color: var(--text-main);
  padding: 12px 14px;
  border-radius: 10px;
  font-size: 0.95rem;
  transition: all 0.2s;
}
.modal-content input:focus { border-color: var(--accent-color); outline: none; }

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
  background: var(--sidebar-bg);
  border: 1px solid var(--border-color);
  padding: 12px 20px;
  border-radius: 10px;
  color: var(--text-main);
  font-size: 0.85rem;
  box-shadow: 0 12px 32px rgba(0,0,0,0.4);
  animation: slideInRight 0.3s cubic-bezier(0.18, 0.89, 0.32, 1.28);
  min-width: 200px;
  max-width: 320px;
  pointer-events: auto;
  border-left-width: 4px;
}
.toast.success { border-left-color: var(--accent-color); }
.toast.error { border-left-color: var(--danger); }

@keyframes slideInRight {
  from { transform: translateX(100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}
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