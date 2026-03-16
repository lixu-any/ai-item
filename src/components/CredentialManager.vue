<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
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
    // 处理空字符串为 null
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
  if (!confirm('确定删除该凭据吗？')) return;
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
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal-content credential-modal animate-pop">
        <div class="modal-header">
           <h3>凭据管理器</h3>
           <button class="win-close-btn" @click="emit('close')" title="关闭">
             <SvgIcon name="close" size="18" />
           </button>
        </div>

       <div v-if="!showAddForm" class="cred-list-area">
         <div class="list-header">
           <p>安全存储您的 SSH 登录信息，一键复用</p>
           <button class="add-btn-small" @click="showAddForm = true">+ 新建凭据</button>
         </div>
         
         <div class="cred-grid custom-scrollbar">
           <div v-for="c in credentials" :key="c.id" class="cred-card" @click="handleSelect(c)">
             <div class="cred-main">
               <div class="cred-type-icon">
                  <SvgIcon :name="c.private_key ? 'snippet' : 'credential'" size="20" />
                </div>
               <div class="cred-info">
                 <div class="cred-name">{{ c.name }}</div>
                 <div class="cred-user">{{ c.username }}</div>
               </div>
             </div>
             <div class="cred-actions">
               <button class="icon-btn edit" @click.stop="editCredential(c)">
                  <SvgIcon name="edit" size="14" />
                </button>
               <button class="icon-btn delete" @click.stop="deleteCredential(c.id!)">
                  <SvgIcon name="delete" size="14" />
                </button>
             </div>
           </div>
         </div>
         
         <div v-if="credentials.length === 0" class="empty-creds">
           <SvgIcon name="group" size="48" class="empty-icon" />
           <p>暂无保存的凭据</p>
         </div>
       </div>

       <div v-else class="add-cred-form">
         <div class="form-grid">
           <div class="form-group">
             <label>凭据标识名称</label>
             <input v-model="newCred.name" placeholder="e.g. 华为云-Root" autofocus />
           </div>
           
           <div class="form-row">
             <div class="form-group flex-2">
               <label>SSH 用户名</label>
               <input v-model="newCred.username" placeholder="root" />
             </div>
             <div class="form-group flex-3">
               <label>登录密码 (可选)</label>
               <input v-model="newCred.password" type="password" placeholder="••••••••" />
             </div>
           </div>

           <div class="form-group">
             <label>私钥内容 (可选)</label>
             <textarea 
                v-model="newCred.private_key" 
                placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
                rows="5"
                class="code-input"
             ></textarea>
           </div>
         </div>
         
         <div class="modal-footer">
           <button class="modal-btn secondary" @click="showAddForm = false">返回列表</button>
           <button class="modal-btn primary" @click="saveCredential">立即保存</button>
         </div>
       </div>
    </div>
  </div>
</template>

<style scoped>
.credential-modal {
  width: 520px;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  padding: 32px;
  position: relative; /* 为绝对定位的关闭按钮提供参考 */
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.header-main {
  display: flex;
  align-items: center;
  gap: 12px;
}

.win-close-btn {
  position: absolute;
  top: 24px;
  right: 24px;
  background: transparent;
  border: none;
  color: var(--text-dim);
  opacity: 0.5;
  cursor: pointer;
  font-size: 1.5rem;
  transition: all 0.2s;
  line-height: 1;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}

.win-close-btn:hover {
  opacity: 1;
  color: var(--danger);
  transform: scale(1.1);
  background: rgba(255, 77, 79, 0.05);
  border-radius: 50%;
}


h3 {
  margin: 0;
  font-size: 1.25rem;
  color: var(--text-main);
  font-weight: 700;
  line-height: 1.4rem;
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.list-header p {
  font-size: 0.85rem;
  color: var(--text-dim);
  margin: 0;
}

.add-btn-small {
  background: var(--accent-color);
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 8px;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  box-shadow: 0 4px 12px rgba(57, 108, 216, 0.2);
}

.add-btn-small:hover {
  background: var(--accent-hover);
  transform: translateY(-1px);
}

.cred-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  padding: 12px 6px; /* 增加足够的内边距，确保卡片浮起动画（-2px）不被父级裁剪 */
  margin: -12px -6px; /* 负边距抵消，保持布局整齐 */
  max-height: 400px;
  overflow-y: auto;
  overflow-x: hidden;
}

.cred-card {
  padding: 16px;
  border: 1px solid var(--border-color);
  background: #fcfcfc;
  border-radius: 14px;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.cred-card:hover {
  border-color: var(--accent-color);
  background: white;
  transform: translateY(-2px);
  box-shadow: 0 8px 20px rgba(0,0,0,0.06);
}

.cred-main {
  display: flex;
  gap: 12px;
  overflow: hidden;
}

.cred-type-icon {
  font-size: 1.4rem;
  background: var(--sidebar-bg);
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
}

.cred-info {
  overflow: hidden;
}

.cred-name {
  font-weight: 700;
  font-size: 0.9rem;
  color: var(--text-main);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cred-user {
  font-size: 0.75rem;
  color: var(--text-dim);
  margin-top: 2px;
}

.add-cred-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.icon-btn.edit:hover {
  color: var(--accent-color);
  background: rgba(57, 108, 216, 0.1);
}

.icon-btn.delete:hover {
  color: var(--danger);
  background: rgba(255, 77, 79, 0.1);
}

.cred-actions {
  display: flex;
  gap: 4px;
}

.code-input {
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.8rem;
  line-height: 1.4;
}

.empty-creds {
  text-align: center;
  padding: 60px 40px;
  color: var(--text-dim);
  opacity: 0.5;
}

.empty-icon {
  font-size: 3rem;
  margin-bottom: 12px;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 10px;
}

.animate-pop {
  animation: pop 0.3s cubic-bezier(0.18, 0.89, 0.32, 1.28);
}

@keyframes pop {
  from { transform: scale(0.9); opacity: 0; }
  to { transform: scale(1); opacity: 1; }
}
</style>
