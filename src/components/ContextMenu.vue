<template>
  <div 
    v-if="visible" 
    class="context-menu" 
    :style="{ top: `${y}px`, left: `${x}px` }"
    @click.stop
    @contextmenu.prevent
  >
    <div 
      v-for="(item, index) in items" 
      :key="index"
      class="menu-item-wrapper"
    >
      <div v-if="item.divider" class="menu-divider"></div>
      <div 
        v-else 
        class="menu-item" 
        :class="{ 'disabled': item.disabled, 'danger': item.danger, 'active': item.active }"
        @click="handleAction(item)"
      >
        <span v-if="item.icon" class="item-icon">{{ item.icon }}</span>
        <span class="item-label">{{ item.label }}</span>
        <span v-if="item.shortcut" class="item-shortcut">{{ item.shortcut }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';

export interface MenuItem {
  label?: string;
  icon?: string;
  action?: string;
  disabled?: boolean;
  danger?: boolean;
  active?: boolean;
  divider?: boolean;
  shortcut?: string;
}

const props = defineProps<{
  visible: boolean;
  x: number;
  y: number;
  items: MenuItem[];
}>();

const emit = defineEmits(['update:visible', 'action']);

const handleAction = (item: MenuItem) => {
  if (item.disabled || item.divider) return;
  emit('action', item.action);
  emit('update:visible', false);
};

const closeMenu = () => {
  if (props.visible) {
    emit('update:visible', false);
  }
};

onMounted(() => {
  window.addEventListener('click', closeMenu);
});

onUnmounted(() => {
  window.removeEventListener('click', closeMenu);
});
</script>

<style scoped>
.context-menu {
  position: fixed;
  z-index: 9999;
  background: white;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  border: 1px solid rgba(0, 0, 0, 0.08);
  padding: 2px 0;
  min-width: 120px;
  backdrop-filter: blur(10px);
  animation: menu-appear 0.1s ease-out;
  overflow: hidden;
}

@keyframes menu-appear {
  from {
    opacity: 0;
    transform: scale(0.98) translateY(-5px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.menu-item {
  display: flex;
  align-items: center;
  padding: 6px 12px;
  cursor: pointer;
  transition: all 0.2s;
  color: #333;
  font-size: 0.85rem;
  user-select: none;
}

.menu-item:hover {
  background-color: #f5f5f5;
}

.menu-item.active {
  background-color: #396cd8;
  color: white;
}

.menu-item.disabled {
  opacity: 0.4;
  cursor: not-allowed;
  pointer-events: none;
}

.menu-item.danger {
  color: #ff4d4f;
}

.menu-item.danger:hover {
  background-color: #fff1f0;
}

.menu-divider {
  height: 1px;
  background-color: #eee;
  margin: 4px 0;
}

.item-icon {
  margin-right: 8px;
  font-size: 1rem;
}

.item-label {
  flex: 1;
}

.item-shortcut {
  margin-left: 16px;
  font-size: 0.75rem;
  opacity: 0.5;
}
</style>
