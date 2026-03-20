<script setup lang="ts">
import { ref } from 'vue';
import SvgIcon from './SvgIcon.vue';
import ContextMenu, { MenuItem } from './ContextMenu.vue';

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

const showMenu = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const menuItems = ref<MenuItem[]>([]);
const contextTargetType = ref<'host' | 'group' | null>(null);
const contextTargetData = ref<Host | Group | null>(null);

function openHostMenu(e: MouseEvent, h: Host) {
  contextTargetType.value = 'host';
  contextTargetData.value = h;
  menuItems.value = [
    { label: '连接', action: 'connect', icon: '▶️' },
    { label: '编辑', action: 'edit', icon: '✏️' },
    { divider: true },
    { label: '删除', action: 'delete', icon: '🗑️', danger: true },
  ];
  menuX.value = e.clientX;
  menuY.value = e.clientY;
  showMenu.value = true;
}

function openGroupMenu(e: MouseEvent, g: Group) {
  contextTargetType.value = 'group';
  contextTargetData.value = g;
  const count = groupHostCount(g.id!);
  menuItems.value = [
    { label: '编辑', action: 'edit', icon: '✏️' },
    { divider: true },
    {
      label: '删除',
      action: 'delete',
      icon: '🗑️',
      danger: true,
      disabled: count > 0 // 有内容的目录不允许删除
    },
  ];
  menuX.value = e.clientX;
  menuY.value = e.clientY;
  showMenu.value = true;
}

function handleMenuAction(action: string) {
  if (contextTargetType.value === 'host') {
    const h = contextTargetData.value as Host;
    if (action === 'connect') emit('connect', h);
    if (action === 'edit') emit('edit-host', h);
    if (action === 'delete') emit('delete-host', h.id!);
  } else if (contextTargetType.value === 'group') {
    const g = contextTargetData.value as Group;
    if (action === 'edit') emit('edit-group', g);
    if (action === 'delete') emit('delete-group', g.id!);
  }
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
        @contextmenu.prevent="openHostMenu($event, h)"
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
      </div>
    </div>

    <div class="host-list">
      <!-- 渲染分组及其主机 -->
      <transition-group name="list" tag="div" class="host-groups-wrapper">
        <div v-for="g in groups" :key="'group-'+g.id" 
          v-show="!searchQuery || filteredHosts().some(h => h.group_id === g.id)"
          class="group-section"
          :class="{ collapsed: isGroupCollapsed(g.id!) }"
          @dragover.prevent
          @drop="onDrop($event, g.id!)"
        >
          <div class="group-header" @click="toggleGroup(g.id!)" @contextmenu.prevent="openGroupMenu($event, g)">
            <span class="chevron">›</span>
            <SvgIcon name="group" size="15" class="folder-icon" />
            <span class="group-name">{{ g.name }}</span>
            <span class="group-count">{{ groupHostCount(g.id!) }}</span>
          </div>
          <transition name="fade-slide">
            <div class="group-content-wrapper" v-show="!isGroupCollapsed(g.id!) || !!searchQuery">
              <transition-group name="list" tag="div" class="group-content">
                <div
                  v-for="h in filteredHosts().filter((h: Host) => h.group_id === g.id)"
                  :key="'host-'+h.id"
                  class="host-card"
                  @dblclick="$emit('connect', h)"
                  @contextmenu.prevent="openHostMenu($event, h)"
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
                </div>
              </transition-group>
            </div>
          </transition>
        </div>

        <!-- 未分组主机 -->
        <div v-if="filteredHosts().some(h => !h.group_id)" class="group-section" :key="'group-ungrouped'"
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
          <transition name="fade-slide">
            <div class="group-content-wrapper" v-show="!collapsedUnGrouped || !!searchQuery">
              <transition-group name="list" tag="div" class="group-content">
                <div
                  v-for="h in filteredHosts().filter((h: Host) => !h.group_id)"
                  :key="'host-'+h.id"
                  class="host-card"
                  @dblclick="$emit('connect', h)"
                  @contextmenu.prevent="openHostMenu($event, h)"
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
                </div>
              </transition-group>
            </div>
          </transition>
        </div>
      </transition-group>

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
    <ContextMenu 
      v-model:visible="showMenu"
      :x="menuX"
      :y="menuY"
      :items="menuItems"
      @action="handleMenuAction"
    />
  </aside>
</template>

<style scoped>
/* Animation for list items */
.list-enter-active, .list-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
.list-enter-from, .list-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
.list-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
.fade-slide-enter-active, .fade-slide-leave-active {
  transition: max-height 0.3s ease-in-out, opacity 0.3s ease-in-out;
  overflow: hidden;
  max-height: 1000px;
}
.fade-slide-enter-from, .fade-slide-leave-to {
  opacity: 0;
  max-height: 0;
}

/* Sidebar Local Overrides */
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
  border: 1px solid transparent;
  border-radius: 10px;
  padding: 8px 30px 8px 28px;
  color: var(--text-main);
  font-size: 0.75rem;
  box-sizing: border-box;
  box-shadow: inset 0 1px 2px rgba(0,0,0,0.02);
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}
.search-input-wrapper input:focus { 
  border-color: rgba(14, 165, 233, 0.4); 
  background: #ffffff;
  box-shadow: inset 0 1px 3px rgba(0,0,0,0.03), 0 0 0 3px rgba(14, 165, 233, 0.15);
  outline: none; 
}
.clear-search {
  position: absolute; 
  right: 10px; 
  color: var(--text-dim); 
  cursor: pointer; 
  font-size: 0.9rem;
}

.host-list {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 8px;
  position: relative;
}

 

.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  cursor: pointer;
  border-radius: 8px;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}
.group-header:hover { 
  background: rgba(0, 0, 0, 0.04); 
}

.chevron {
  font-size: 1rem;
  color: var(--text-dim);
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  width: 12px;
  display: flex;
  justify-content: center;
  transform: rotate(90deg);
}
.collapsed .chevron { transform: rotate(0deg); }

.folder-icon { 
  opacity: 0.8; 
  color: var(--accent-color);
}

.group-name { 
  flex: 1; 
  min-width: 0;
  overflow: hidden; 
  text-overflow: ellipsis; 
  white-space: nowrap; 
  font-size: 0.8rem;
  font-weight: 700;
  color: var(--text-main);
  opacity: 0.9;
}
.group-count {
  font-size: 0.7rem;
  color: var(--text-dim);
  background: rgba(0, 0, 0, 0.05);
  padding: 2px 6px;
  border-radius: 12px;
  font-weight: 600;
}

/* Host Card Polihing */
.host-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 3px 12px;
  cursor: pointer;
  border-radius: 10px; /* Reduced from full width to rounded card */
  border: 1px solid transparent;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  background: transparent;
}
.host-card:hover {
  background: #ffffff;
  border-color: rgba(0, 0, 0, 0.03);
  box-shadow: 0 4px 12px rgba(0,0,0,0.03), 0 1px 2px rgba(0,0,0,0.01);
  transform: scale(0.99); /* Press/Hover elasticity */
}

/* Avatar Polish */
.host-avatar {
  position: relative;
  width: 34px;
  height: 34px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-weight: 700;
  font-size: 0.85rem;
  flex-shrink: 0;
  box-shadow: inset 0 -2px 4px rgba(0,0,0,0.15), 0 2px 4px rgba(0,0,0,0.1);
  text-shadow: 0 1px 2px rgba(0,0,0,0.2);
}
.avatar-online {
  position: absolute;
  bottom: -2px;
  right: -2px;
  width: 10px;
  height: 10px;
  background-color: var(--success);
  border: 2px solid white;
  border-radius: 50%;
  box-shadow: 0 0 6px rgba(16, 185, 129, 0.6);
}

/* Host Meta */
.host-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.host-name {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-main);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.host-addr {
  font-size: 0.7rem;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: 'JetBrains Mono', monospace;
}


/* Sidebar Footer */
.sidebar-footer {
  padding: 12px;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  border-top: 1px solid var(--border-color);
  background: rgba(255,255,255,0.3);
}
.footer-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  cursor: pointer;
  color: var(--text-dim);
  border-radius: 8px;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}
.footer-item.active { background: rgba(14, 165, 233, 0.1); color: var(--accent-color); }
.footer-item.active .footer-icon { color: var(--accent-color); }
.footer-item:hover { background: rgba(0,0,0,0.04); color: var(--text-main); }
.footer-icon { opacity: 0.7; font-size: 1rem; }
.footer-item span { font-size: 0.75rem; font-weight: 500; }

/* Empty state styling override */
.empty-state {
  display: flex; flex-direction: column; align-items: center;
  justify-content: center; padding: 40px 20px; color: var(--text-dim);
  text-align: center;
}
.empty-icon { font-size: 2rem; margin-bottom: 12px; opacity: 0.5; }
.empty-state p { font-size: 0.85rem; margin: 0; font-weight: 500;}

/* Recent Section */
.recent-section {
  padding: 0 12px 16px;
  border-bottom: 1px solid var(--border-color);
  margin-bottom: 10px;
}
.recent-header {
  font-size: 0.75rem; font-weight: 700; color: var(--text-dim);
  margin-bottom: 8px; display: flex; justify-content: space-between; align-items: center;
  text-transform: uppercase; letter-spacing: 0.05em;
}
.recent-item {
  display: flex; align-items: center; gap: 10px; padding: 8px 10px; cursor: pointer;
  border-radius: 8px; transition: all 0.2s;
  background: transparent;
}
.recent-item:hover { 
  background: #ffffff; 
  transform: scale(0.99);
  box-shadow: 0 2px 8px rgba(0,0,0,0.04);
}
.recent-avatar { width: 30px; height: 30px; font-size: 0.8rem; border-radius: 8px;}
.clear-recent-btn {
  background: none; border: none; color: var(--text-dim); font-size: 0.7rem; cursor: pointer; display: flex; align-items: center; gap: 3px; padding: 2px 6px; border-radius: 4px; transition: 0.2s;
}
.clear-recent-btn:hover { background: rgba(0,0,0,0.06); color: var(--text-main); }
</style>
