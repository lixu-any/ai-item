<script setup lang="ts">
import { ref, nextTick, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Terminal from "./components/Terminal.vue";

interface SessionTab {
  id: string;
  title: string;
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
const showAddModal = ref(false);
const authType = ref<'password' | 'private_key'>('password');
const newHost = ref<Host>({
  name: '',
  host: '',
  port: 22,
  username: 'root',
  password: '',
  private_key: '',
});

async function loadHosts() {
  try {
    savedHosts.value = await invoke("get_hosts");
  } catch (err) {
    console.error("加载主机列表失败:", err);
  }
}

onMounted(() => {
  loadHosts();
});

async function saveHost() {
  try {
    const hostToSave = { ...newHost.value };
    if (authType.value === 'password') hostToSave.private_key = null;
    else hostToSave.password = null;

    await invoke("add_host", { host: hostToSave });
    showAddModal.value = false;
    newHost.value = { name: '', host: '', port: 22, username: 'root', password: '', private_key: '' };
    authType.value = 'password';
    await loadHosts();
  } catch (err) {
    alert("保存失败: " + err);
  }
}

async function deleteHost(id: number) {
  if (confirm("确定要删除这个主机吗？")) {
    try {
      await invoke("delete_host", { id });
      await loadHosts();
    } catch (err) {
      alert("删除失败: " + err);
    }
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
  } catch (err) {
    console.error("连接异常:", err);
    connError.value = String(err);
    closeSession(id);
  } finally {
    connecting.value = false;
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
        <h3>主机列表</h3>
        <button class="add-btn" @click="showAddModal = true">+</button>
      </div>
      <div class="host-list">
        <div v-for="h in savedHosts" :key="h.id!" class="host-item" @dblclick="connectToHost(h)">
          <div class="host-info">
            <span class="host-name">{{ h.name }}</span>
            <span class="host-ip">{{ h.username }}@{{ h.host }}:{{ h.port }}</span>
          </div>
          <div class="host-actions">
             <button class="icon-btn" @click.stop="connectToHost(h)">▶</button>
             <button class="icon-btn delete-btn" @click.stop="deleteHost(h.id!)">✗</button>
          </div>
        </div>
        <div v-if="savedHosts.length === 0" class="empty-hosts">
          暂无保存的服务器
        </div>
        <div v-if="connError" class="error-msg">{{ connError }}</div>
      </div>
      <nav>
        <ul>
          <li> AI 助手</li>
          <li> 设置</li>
        </ul>
      </nav>
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
      <div class="modal-content">
        <h3>添加服务器</h3>
        <input v-model="newHost.name" placeholder="备忘名称 (如：生产服务器一)" />
        <input v-model="newHost.host" placeholder="IP 地址或域名" />
        <input v-model.number="newHost.port" placeholder="SSH 端口" type="number" />
        <input v-model="newHost.username" placeholder="登录用户名" />
        
        <div class="auth-toggle">
          <label><input type="radio" value="password" v-model="authType"> 密码登录</label>
          <label><input type="radio" value="private_key" v-model="authType"> 密钥登录</label>
        </div>

        <input v-if="authType === 'password'" v-model="newHost.password" placeholder="登录密码" type="password" />
        <textarea v-else v-model="newHost.private_key" placeholder="粘贴私钥内容 (-----BEGIN PRIVATE KEY-----...)"></textarea>

        <div class="modal-actions">
          <button @click="showAddModal = false">取消</button>
          <button class="primary" @click="saveHost">保存到数据库</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
/* 全局重置 */
body, html, #app {
  margin: 0;
  padding: 0;
  height: 100%;
  width: 100%;
  overflow: hidden;
  background-color: #1e1e1e;
  color: #d4d4d4;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
}

.app-layout {
  display: flex;
  height: 100vh;
  width: 100vw;
}

.sidebar {
  width: 250px;
  background-color: #252526;
  border-right: 1px solid #3c3c3c;
  padding: 1rem;
  display: flex;
  flex-direction: column;
}

.sidebar-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}
.sidebar-header h3 { margin: 0; }
.add-btn {
  background: #396cd8;
  color: white;
  border: none;
  border-radius: 4px;
  width: 24px;
  height: 24px;
  cursor: pointer;
  display: flex;
  justify-content: center;
  align-items: center;
}
.host-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.host-item {
  background: #333;
  padding: 10px;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.host-item:hover {
  background: #444;
}
.host-info {
  display: flex;
  flex-direction: column;
}
.host-name {
  font-weight: bold;
  font-size: 0.9rem;
  color: #fff;
}
.host-ip {
  font-size: 0.75rem;
  color: #aaa;
  margin-top: 4px;
}
.host-actions {
  display: flex;
  gap: 4px;
}
.icon-btn {
  background: transparent;
  border: none;
  color: #ccc;
  cursor: pointer;
  padding: 4px;
}
.icon-btn:hover { color: #fff; }
.delete-btn:hover { color: #ff5555; }
.empty-hosts {
  color: #888;
  font-size: 0.85rem;
  text-align: center;
  margin-top: 2rem;
}

.modal-overlay {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0,0,0,0.5);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
}
.modal-content {
  background: #252526;
  padding: 20px;
  border-radius: 8px;
  width: 350px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  border: 1px solid #3c3c3c;
}
.modal-content h3 { margin: 0 0 10px 0; color: #fff; }
.modal-content input, .modal-content textarea {
  background: #3c3c3c;
  border: 1px solid #555;
  color: white;
  padding: 8px;
  border-radius: 4px;
}
.modal-content textarea {
  height: 80px;
  resize: vertical;
}
.auth-toggle {
  display: flex;
  gap: 15px;
  color: #ccc;
  font-size: 0.85rem;
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 10px;
}
.modal-actions button {
  background: transparent;
  color: #ccc;
  border: 1px solid #555;
  padding: 6px 12px;
  border-radius: 4px;
  cursor: pointer;
}
.modal-actions button.primary {
  background: #007acc;
  color: white;
  border: none;
}
.modal-actions button:hover {
  background: #333;
}
.modal-actions button.primary:hover {
  background: #0098ff;
}

.error-msg {
  color: #ff5555;
  font-size: 0.8rem;
  margin-top: 5px;
  word-break: break-all;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.top-bar {
  height: 35px;
  background-color: #252526;
  display: flex;
  align-items: flex-end;
  padding: 0;
  font-size: 0.8rem;
  border-bottom: 1px solid #3c3c3c;
  overflow-x: auto;
}
.no-tabs {
  padding: 0 1rem;
  line-height: 35px;
}
.tab {
  height: 30px;
  padding: 0 15px;
  display: flex;
  align-items: center;
  background-color: #2d2d2d;
  color: #969696;
  border-right: 1px solid #1e1e1e;
  border-top: 1px solid transparent;
  cursor: pointer;
  user-select: none;
}
.tab.active {
  background-color: #1e1e1e;
  color: #cccccc;
  border-top: 1px solid #007acc;
}
.tab:hover:not(.active) {
  background-color: #323233;
}
.close-btn {
  background: transparent;
  border: none;
  color: inherit;
  font-size: 1.2em;
  padding: 0;
  margin-left: 8px;
  cursor: pointer;
  box-shadow: none;
}
.close-btn:hover {
  color: white;
  background: transparent;
  border-color: transparent;
}

.terminal-wrapper {
  flex: 1;
  position: relative;
  background-color: #1e1e1e;
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