<script setup lang="ts">
import { ref } from 'vue';
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

interface Group {
  id?: number;
  name: string;
  parent_id?: number | null;
}

const props = defineProps<{
  hosts: Host[];
  groups: Group[];
  recentHosts: Host[];
  activeHostIds: Set<number>;
  showSnippetSidebar: boolean;
  showAiSidebar: boolean;
}>();

const emit = defineEmits<{
  (e: 'add-group'): void;
  (e: 'add-host'): void;
  (e: 'edit-group', g: Group): void;
  (e: 'delete-group', id: number): void;
  (e: 'connect', h: Host): void;
  (e: 'edit-host', h: Host): void;
  (e: 'delete-host', id: number): void;
  (e: 'clear-recent'): void;
  (e: 'drop-host', hostId: number, groupId: number | null): void;
  (e: 'new-local'): void;
  (e: 'toggle-snippet'): void;
  (e: 'toggle-ai'): void;
  (e: 'open-credential'): void;
  (e: 'open-recording'): void;
  (e: 'open-settings'): void;
}>();

const searchQuery = ref("");
const collapsedGroups = ref<Set<number>>(new Set());
const collapsedUnGrouped = ref(false);

function filteredHosts() {
  if (!searchQuery.value) return props.hosts;
  const q = searchQuery.value.toLowerCase();
  return props.hosts.filter(h => 
    h.name.toLowerCase().includes(q) || 
    h.host.toLowerCase().includes(q) || 
    h.username.toLowerCase().includes(q)
  );
}

function toggleGroup(id: number) {
  if (collapsedGroups.value.has(id)) {
    collapsedGroups.value.delete(id);
  } else {
    collapsedGroups.value.add(id);
  }
}

function isGroupCollapsed(id: number) {
  return collapsedGroups.value.has(id);
}

function groupHostCount(groupId: number | null) {
  return props.hosts.filter(h => h.group_id === groupId).length;
}

function hostAvatarText(name: string) {
  return name ? name.substring(0, 1).toUpperCase() : '?';
}

function hostAvatarColor(name: string) {
  let hash = 0;
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
  const hue = Math.abs(hash % 360);
  return `hsl(${hue}, 65%, 45%)`;
}

function isHostActive(h: Host) {
  return h.id !== undefined && props.activeHostIds.has(h.id);
}

function onDragStart(event: DragEvent, hostId: number) {
  if (event.dataTransfer) {
    event.dataTransfer.setData("hostId", hostId.toString());
    event.dataTransfer.effectAllowed = "move";
  }
}

function onDrop(event: DragEvent, groupId: number | null) {
  event.preventDefault();
  const hostIdStr = event.dataTransfer?.getData("hostId");
  if (!hostIdStr) return;
  const hostId = parseInt(hostIdStr);
  emit('drop-host', hostId, groupId);
}
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <span class="sidebar-title">主机列表</span>
      <div class="header-actions">
        <button class="add-btn primary" @click="$emit('add-group')" title="添加分组">
          <SvgIcon name="group" size="14" />
        </button>
        <button class="add-btn accent" @click="$emit('add-host')" title="添加主机">
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

    <!-- 最近连接 -->
    <div v-if="recentHosts.length > 0 && !searchQuery" class="recent-section">
      <div class="recent-header">
        <span>最近连接</span>
        <button class="clear-recent-btn" @click="$emit('clear-recent')" title="清空最近连接">
          <SvgIcon name="close" size="12" /> 清空
        </button>
      </div>
      <div
        v-for="h in recentHosts"
        :key="`recent-${h.id}`"
        class="recent-item"
        @click="$emit('connect', h)"
        :title="`双击连接 ${h.name}`"
      >
        <div class="host-avatar recent-avatar" :style="{ background: hostAvatarColor(h.name) }">
          {{ hostAvatarText(h.name) }}
          <span v-if="isHostActive(h)" class="avatar-online"></span>
        </div>
        <div class="host-meta">
          <span class="host-name">{{ h.name }}</span>
          <span class="host-addr">{{ h.username }}@{{ h.host }}</span>
        </div>
        <button class="recent-connect-btn" @click.stop="$emit('connect', h)" title="连接">
          <SvgIcon name="play" size="12" />
        </button>
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
          <SvgIcon name="group" size="15" class="folder-icon" />
          <span class="group-name">{{ g.name }}</span>
          <div class="group-actions">
            <button class="icon-btn" @click.stop="$emit('edit-group', g)" title="编辑分组">
              <SvgIcon name="edit" size="13" />
            </button>
            <button class="icon-btn delete-btn" @click.stop="$emit('delete-group', g.id!)" title="删除分组">
              <SvgIcon name="delete" size="13" />
            </button>
          </div>
          <span class="group-count">{{ groupHostCount(g.id!) }}</span>
        </div>
        <div class="group-content" v-show="!isGroupCollapsed(g.id!) || searchQuery">
          <div
            v-for="h in filteredHosts().filter((h: Host) => h.group_id === g.id)"
            :key="h.id!"
            class="host-card"
            @dblclick="$emit('connect', h)"
            draggable="true"
            @dragstart="onDragStart($event, h.id!)"
          >
            <div class="host-avatar" :style="{ background: hostAvatarColor(h.name) }">
              {{ hostAvatarText(h.name) }}
              <span v-if="isHostActive(h)" class="avatar-online"></span>
            </div>
            <div class="host-meta">
              <span class="host-name">{{ h.name }}</span>
              <span class="host-addr">{{ h.username }}@{{ h.host }}</span>
            </div>
            <div class="host-actions">
              <button class="icon-btn" @click.stop="$emit('connect', h)" title="连接">
                <SvgIcon name="play" size="13" />
              </button>
              <button class="icon-btn" @click.stop="$emit('edit-host', h)" title="编辑">
                <SvgIcon name="edit" size="13" />
              </button>
              <button class="icon-btn delete-btn" @click.stop="$emit('delete-host', h.id!)" title="删除">
                <SvgIcon name="delete" size="13" />
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
          <SvgIcon name="group" size="15" class="folder-icon" />
          <span class="group-name">未分组</span>
          <span class="group-count">{{ groupHostCount(null) }}</span>
        </div>
        <div class="group-content" v-show="!collapsedUnGrouped || searchQuery">
          <div
            v-for="h in filteredHosts().filter((h: Host) => !h.group_id)"
            :key="h.id!"
            class="host-card"
            @dblclick="$emit('connect', h)"
            draggable="true"
            @dragstart="onDragStart($event, h.id!)"
          >
            <div class="host-avatar" :style="{ background: hostAvatarColor(h.name) }">
              {{ hostAvatarText(h.name) }}
              <span v-if="isHostActive(h)" class="avatar-online"></span>
            </div>
            <div class="host-meta">
              <span class="host-name">{{ h.name }}</span>
              <span class="host-addr">{{ h.username }}@{{ h.host }}</span>
            </div>
            <div class="host-actions">
              <button class="icon-btn" @click.stop="$emit('connect', h)" title="连接">
                <SvgIcon name="play" size="13" />
              </button>
              <button class="icon-btn" @click.stop="$emit('edit-host', h)" title="编辑">
                <SvgIcon name="edit" size="13" />
              </button>
              <button class="icon-btn delete-btn" @click.stop="$emit('delete-host', h.id!)" title="删除">
                <SvgIcon name="delete" size="13" />
              </button>
            </div>
          </div>
        </div>
      </div>

      <div v-if="filteredHosts().length === 0 && searchQuery" class="empty-state">
        <SvgIcon name="search" size="48" class="empty-icon" />
        <p>没有找到匹配的主机</p>
      </div>
      <div v-else-if="hosts.length === 0" class="empty-state">
        <SvgIcon name="host" size="48" class="empty-icon" />
        <p>点击上方 + 开始添加第一台服务器</p>
      </div>
    </div>
    <div class="sidebar-footer">
      <div class="footer-item" @click="$emit('new-local')" title="本地终端">
        <SvgIcon name="host" size="18" class="footer-icon" />
        <span>本地终端</span>
      </div>
      <div class="footer-item" @click="$emit('toggle-snippet')" :class="{ active: showSnippetSidebar }" title="命令片段">
        <SvgIcon name="snippet" size="18" class="footer-icon" />
        <span>命令片段</span>
      </div>
      <div class="footer-item" @click="$emit('toggle-ai')" :class="{ active: showAiSidebar }" title="AI 助手">
        <SvgIcon name="ai" size="18" class="footer-icon" />
        <span>AI 助手</span>
      </div>
      <div class="footer-item" title="凭据管理" @click="$emit('open-credential')">
        <SvgIcon name="credential" size="18" class="footer-icon" />
        <span>凭据中心</span>
      </div>
      <div class="footer-item" title="本地录像" @click="$emit('open-recording')">
        <SvgIcon name="play" size="18" class="footer-icon" />
        <span>播放回放</span>
      </div>
      <div class="footer-item" title="设置" @click="$emit('open-settings')">
        <SvgIcon name="settings" size="18" class="footer-icon" />
        <span>设置</span>
      </div>
    </div>
  </aside>
</template>

<style scoped>
/* Keeping scoped empty for now, relying on global App.vue styles */
</style>
