<script setup lang="ts">
import { ref, watch } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import SvgIcon from './SvgIcon.vue';

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

const props = defineProps<{
  modelValue: Host;
  isEditing: boolean;
  groups: any[];
  saveError: string;
  isStandalone: boolean;
  authTypeRef: 'password' | 'private_key';
  pkTypeRef: 'path' | 'content';
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', val: Host): void;
  (e: 'update:authTypeRef', val: 'password' | 'private_key'): void;
  (e: 'update:pkTypeRef', val: 'path' | 'content'): void;
  (e: 'save'): void;
  (e: 'cancel'): void;
  (e: 'use-credential'): void;
}>();

const localHost = ref<Host>({ ...props.modelValue });

// flush:'sync' 确保 watcher 同步执行，syncingFromProp 标志在回调时仍有效
// 否则 Vue 的异步 watcher 队列会让标志归零后回调才执行，造成 build 版本死循环
let syncingFromProp = false;

watch(() => props.modelValue, (val) => {
  syncingFromProp = true;
  localHost.value = { ...val };
  syncingFromProp = false;
}, { deep: true, flush: 'sync' });

watch(localHost, (val) => {
  if (syncingFromProp) return;
  emit('update:modelValue', { ...val });
}, { deep: true, flush: 'sync' });

function getAuthType() { return props.authTypeRef; }
function setAuthType(v: 'password' | 'private_key') { emit('update:authTypeRef', v); }

function getPkType() { return props.pkTypeRef; }
function setPkType(v: 'path' | 'content') { emit('update:pkTypeRef', v); }

async function selectPrivateKeyFile() {
  const selected = await open({
    multiple: false,
    title: '选择私钥文件'
  });
  if (selected && typeof selected === 'string') {
    localHost.value.private_key = selected;
  }
}
</script>

<template>
  <div class="modal-content" :class="{ 'premium-modal': isEditing, 'standalone-window': isStandalone }">
    <div class="modal-header-accent"></div>
    <div class="form-header">
      <SvgIcon :name="isEditing ? 'edit' : 'add'" size="20" class="header-icon" />
      <h3>{{ isEditing ? '编辑主机配置' : '添加新主机' }}</h3>
      <button class="use-cred-btn" @click="$emit('use-credential')">复用凭据</button>
    </div>
    
    <div class="form-scroll-area">
      <div class="form-grid">
        <div class="form-group animate-in" style="--delay: 0.1s">
          <label>标识名称</label>
          <input v-model="localHost.name" placeholder="e.g. 生产环境-Web01" autofocus />
        </div>
        
        <div class="form-row animate-in" style="--delay: 0.2s">
          <div class="form-group flex-2">
            <label>主机地址 (IP/Domain)</label>
            <input v-model="localHost.host" placeholder="192.168.1.100" />
          </div>
          <div class="form-group flex-1">
            <label>端口</label>
            <input v-model.number="localHost.port" type="number" />
          </div>
        </div>

        <div class="form-group animate-in" style="--delay: 0.3s">
          <label>业务分组</label>
          <select v-model="localHost.group_id" class="group-select">
            <option :value="null">未分组 (Default)</option>
            <option v-for="g in groups" :key="g.id" :value="g.id">{{ g.name }}</option>
          </select>
        </div>

        <div class="form-group animate-in" style="--delay: 0.4s">
          <label>SSH 用户名</label>
          <input v-model="localHost.username" placeholder="root" />
        </div>

        <div class="form-group animate-in" style="--delay: 0.5s">
          <label>认证方式</label>
          <div class="auth-tabs">
            <div class="auth-tab" :class="{ active: getAuthType() === 'password' }" @click="setAuthType('password')">
              <SvgIcon name="credential" size="14" />
              <span>密码认证</span>
            </div>
            <div class="auth-tab" :class="{ active: getAuthType() === 'private_key' }" @click="setAuthType('private_key')">
              <SvgIcon name="snippet" size="14" />
              <span>私钥认证</span>
            </div>
          </div>
        </div>

        <div v-if="getAuthType() === 'password'" class="form-group animate-in" style="--delay: 0.6s">
          <label>登录密码</label>
          <input v-model="localHost.password" type="password" placeholder="••••••••" />
        </div>
        
        <div v-else class="form-group animate-in" style="--delay: 0.6s">
          <div style="display: flex; justify-content: space-between; align-items: center;">
            <label>私钥认证</label>
            <div class="pk-toggle">
              <span :class="{ active: getPkType() === 'path' }" @click="setPkType('path')">路径</span>
              <span :class="{ active: getPkType() === 'content' }" @click="setPkType('content')">直接粘贴</span>
            </div>
          </div>
          <div v-if="getPkType() === 'path'" class="input-with-btn">
            <input v-model="localHost.private_key" placeholder="~/.ssh/id_rsa" />
            <button class="browse-btn" @click="selectPrivateKeyFile">浏览</button>
          </div>
          <textarea v-else v-model="localHost.private_key" placeholder="-----BEGIN OPENSSH PRIVATE KEY-----&#10;..." rows="4" style="font-family: 'JetBrains Mono', monospace; font-size: 0.75rem;"></textarea>
        </div>
      </div>
    </div>

    <div v-if="saveError" class="modal-error animate-in">{{ saveError }}</div>

    <div class="modal-footer">
      <button class="modal-btn secondary" @click="$emit('cancel')">取消</button>
      <button class="modal-btn primary" @click="$emit('save')">
        {{ isEditing ? '更新配置' : '立即保存' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
/* Scoped css not necessarily needed right now since most classes are global from App.vue, but we leave it as is. */
</style>
