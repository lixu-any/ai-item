<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { confirm } from '@tauri-apps/plugin-dialog';
import SvgIcon from './SvgIcon.vue';

interface Credential {
  id?: number;
  name: string;
  username: string;
  password?: string | null;
  private_key?: string | null;
  description?: string | null;
}

const credentials = ref<Credential[]>([]);
const showAddForm = ref(false);
const isEditing = ref(false);
const newCred = ref<Credential>({
  name: '',
  username: 'root',
  password: '',
  private_key: '',
  description: ''
});

const emit = defineEmits(['close', 'select', 'toast']);

async function loadCredentials() {
  try {
    credentials.value = await invoke('get_credentials');
  } catch (err) {
    console.error('加载凭据失败:', err);
    emit('toast', { message: '加载凭据失败: ' + err, type: 'error' });
  }
}

async function saveCredential() {
  if (!newCred.value.name.trim()) {
    emit('toast', { message: '请输入凭据名称', type: 'error' });
    return;
  }
  if (!newCred.value.username.trim()) {
    emit('toast', { message: '请输入用户名', type: 'error' });
    return;
  }
  try {
    const credToSave = {
      ...newCred.value,
      password: newCred.value.password?.trim() || null,
      private_key: newCred.value.private_key?.trim() || null,
      description: newCred.value.description?.trim() || null
    };
    if (isEditing.value) {
      await invoke('update_credential', { cred: credToSave });
      emit('toast', { message: '凭据更新成功', type: 'success' });
    } else {
      await invoke('add_credential', { cred: credToSave });
      emit('toast', { message: '凭据保存成功', type: 'success' });
    }
    showAddForm.value = false;
    isEditing.value = false;
    newCred.value = { name: '', username: 'root', password: '', private_key: '', description: '' };
    loadCredentials();
  } catch (err) {
    console.error('保存凭据失败:', err);
    emit('toast', { message: '保存失败: ' + err, type: 'error' });
  }
}

function editCredential(c: Credential) {
  newCred.value = { ...c };
  isEditing.value = true;
  showAddForm.value = true;
}

async function deleteCredential(id: number) {
  const confirmed = await confirm('确定删除该凭据吗？', { title: 'AI-Term-Shell', kind: 'warning' });
  if (!confirmed) return;
  try {
    await invoke('delete_credential', { id });
    emit('toast', { message: '凭据已删除', type: 'success' });
    loadCredentials();
  } catch (err) {
    console.error('删除凭据失败:', err);
    emit('toast', { message: '删除失败: ' + err, type: 'error' });
  }
}

function handleSelect(c: Credential) {
  emit('select', c);
}

onMounted(loadCredentials);
</script>

<template>
  <div class="cred-root">
    <!-- 顶部渐变装饰条 -->
    <div class="accent-bar"></div>

    <!-- Header -->
    <header class="cred-header">
      <div class="header-left">
        <div class="header-icon-wrap">
          <SvgIcon name="credential" size="25" />
        </div>
        <div>
          <h1 class="header-title">凭据中心</h1>
          <p class="header-sub">SSH Credential Manager</p>
        </div>
      </div>
      <button v-if="!showAddForm" class="add-btn" @click="showAddForm = true" title="新建凭据">
        <SvgIcon name="add" size="25" />
      </button>
    </header>

    <div class="divider"></div>

    <!-- Body -->
    <div class="cred-body">
      <!-- 凭据列表 -->
      <div v-if="!showAddForm">
        <div class="cred-grid">
          <div v-for="c in credentials" :key="c.id" class="cred-card" @click="handleSelect(c)">
            <div class="cred-main">
              <div class="cred-type-icon">
                <SvgIcon :name="c.private_key ? 'snippet' : 'credential'" size="25" />
              </div>
              <div class="cred-info">
                <div class="cred-name">{{ c.name }}</div>
                <div class="cred-user">{{ c.username }}</div>
              </div>
            </div>
            <div class="cred-actions">
              <button class="icon-action edit" @click.stop="editCredential(c)" title="编辑">
                <SvgIcon name="edit" size="25" />
              </button>
              <button class="icon-action delete" @click.stop="deleteCredential(c.id!)" title="删除">
                <SvgIcon name="delete" size="25" />
              </button>
            </div>
          </div>
        </div>

        <div v-if="credentials.length === 0" class="empty-creds">
          <SvgIcon name="group" size="40" class="empty-icon" />
          <p>暂无保存的凭据</p>
          <p class="empty-hint">点击右上角「+ 新建凭据」开始添加</p>
        </div>
      </div>

      <!-- 添加/编辑表单 -->
      <div v-else class="add-form">
        <div class="form-section-title">{{ isEditing ? '编辑凭据' : '新建凭据' }}</div>
        <div class="form-field">
          <label>凭据名称</label>
          <input v-model="newCred.name" placeholder="e.g. 阿里云-Root" class="field-input" autofocus />
        </div>
        <div class="form-row">
          <div class="form-field flex-2">
            <label>SSH 用户名</label>
            <input v-model="newCred.username" placeholder="root" class="field-input" />
          </div>
          <div class="form-field flex-3">
            <label>登录密码 (可选)</label>
            <input v-model="newCred.password" type="password" placeholder="••••••••" class="field-input" />
          </div>
        </div>
        <div class="form-field">
          <label>私钥内容 (可选)</label>
          <textarea v-model="newCred.private_key" placeholder="-----BEGIN OPENSSH PRIVATE KEY-----" rows="5" class="field-input code-input"></textarea>
        </div>
      </div>
    </div>

    <div class="divider"></div>

    <!-- Footer -->
    <footer class="cred-footer">
      <template v-if="!showAddForm">
        <button class="btn-secondary" @click="emit('close')">关闭</button>
      </template>
      <template v-else>
        <button class="btn-secondary" @click="showAddForm = false; isEditing = false">返回列表</button>
        <button class="btn-primary" @click="saveCredential">立即保存</button>
      </template>
    </footer>
  </div>
</template>

<style scoped>
* { box-sizing: border-box; margin: 0; padding: 0; }

.cred-root {
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

.divider { height: 1px; background: #e2e8f0; flex-shrink: 0; }

/* Header */
.cred-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 24px 14px;
  background: #fff;
  flex-shrink: 0;
}
.add-btn:hover { background: #2563eb; transform: scale(1.05); }
.header-left { display: flex; align-items: center; gap: 12px; }
.header-icon-wrap {
  width: 44px; height: 44px;
  border-radius: 11px;
  background: linear-gradient(135deg, #dbeafe, #ede9fe);
  border: 1px solid #bfdbfe;
  display: flex; align-items: center; justify-content: center;
  color: #3b82f6;
  flex-shrink: 0;
}
.header-title { font-size: 15px; font-weight: 600; color: #0f172a; }
.header-sub { font-size: 11px; color: #94a3b8; margin-top: 2px; }

.add-btn {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  border: none;
  background: #3b82f6;
  color: white;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.15s, transform 0.15s;
  -webkit-appearance: none;
  appearance: none;
}
.add-btn:hover { background: #2563eb; }

/* Body */
.cred-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px 24px;
}
.cred-body::-webkit-scrollbar { width: 5px; }
.cred-body::-webkit-scrollbar-track { background: transparent; }
.cred-body::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 3px; }

/* Grid */
.cred-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.cred-card {
  padding: 14px;
  border: 1.5px solid #e2e8f0;
  background: #fff;
  border-radius: 12px;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.15s;
}
.cred-card:hover {
  border-color: #3b82f6;
  box-shadow: 0 4px 12px rgba(59,130,246,0.08);
  transform: translateY(-1px);
}
.cred-main { display: flex; gap: 10px; align-items: center; overflow: hidden; }
.cred-type-icon {
  width: 44px; height: 44px;
  border-radius: 10px;
  background: #f1f5f9;
  display: flex; align-items: center; justify-content: center;
  color: #3b82f6;
  flex-shrink: 0;
}
.cred-info { overflow: hidden; }
.cred-name {
  font-weight: 600; font-size: 13px; color: #0f172a;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.cred-user { font-size: 11.5px; color: #94a3b8; margin-top: 2px; }
.cred-actions { display: flex; gap: 4px; flex-shrink: 0; }

.icon-action {
  width: 32px; height: 32px;
  border: none; background: transparent;
  -webkit-appearance: none; appearance: none;
  border-radius: 7px;
  cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  color: #94a3b8;
  transition: all 0.15s;
}
.icon-action.edit:hover { color: #3b82f6; background: #eff6ff; }
.icon-action.delete:hover { color: #ef4444; background: #fee2e2; }

/* Empty */
.empty-creds {
  text-align: center;
  padding: 60px 20px;
  color: #94a3b8;
}
.empty-icon { opacity: 0.35; margin-bottom: 10px; }
.empty-hint { font-size: 12px; margin-top: 6px; }

/* Add form */
.add-form { display: flex; flex-direction: column; gap: 14px; }
.form-section-title {
  font-size: 13px; font-weight: 600; color: #374151;
  padding-bottom: 8px;
  border-bottom: 1px solid #e2e8f0;
}
.form-field { display: flex; flex-direction: column; gap: 5px; }
.form-row { display: flex; gap: 12px; }
.flex-2 { flex: 2; }
.flex-3 { flex: 3; }
label { font-size: 12.5px; font-weight: 500; color: #374151; }
.field-input {
  width: 100%;
  padding: 8px 11px;
  background: #fff;
  border: 1.5px solid #e2e8f0;
  border-radius: 7px;
  color: #1e293b;
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.field-input::placeholder { color: #cbd5e1; }
.field-input:focus { border-color: #3b82f6; box-shadow: 0 0 0 3px rgba(59,130,246,0.12); }
.code-input { font-family: 'JetBrains Mono', 'SF Mono', monospace; font-size: 12px; resize: vertical; }

/* Footer */
.cred-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 12px 24px;
  background: #fff;
  flex-shrink: 0;
}
.btn-secondary {
  padding: 8px 18px; border-radius: 7px;
  border: 1.5px solid #e2e8f0; background: #fff;
  color: #64748b; font-size: 13px; cursor: pointer;
  font-family: inherit; transition: all 0.15s;
  -webkit-appearance: none; appearance: none;
}
.btn-secondary:hover { border-color: #cbd5e1; color: #374151; }
.btn-primary {
  padding: 8px 22px; border-radius: 7px; border: none;
  background: #3b82f6; color: white;
  font-size: 13px; font-weight: 500; cursor: pointer;
  font-family: inherit; transition: all 0.15s;
  -webkit-appearance: none; appearance: none;
  box-shadow: 0 2px 8px rgba(59,130,246,0.25);
}
.btn-primary:hover { background: #2563eb; }
</style>
