<template>
  <div v-if="visible" class="spotlight-overlay" @click.self="close">
    <div class="spotlight-modal animate-pop">
      <div class="search-input-wrapper">
        <span class="search-icon">🔍</span>
        <input 
          ref="inputRef"
          v-model="query" 
          placeholder="输入服务器名称、IP 或用户名..." 
          class="search-input"
          @keydown.down="moveDown"
          @keydown.up="moveUp"
          @keydown.enter="confirm"
          @keydown.esc="close"
        />
      </div>

      <div v-if="filteredHosts.length > 0" class="results-list">
        <div 
          v-for="(host, index) in filteredHosts" 
          :key="host.id"
          class="result-item"
          :class="{ active: selectedIndex === index }"
          @mouseenter="selectedIndex = index"
          @click="selectHost(host)"
        >
          <div class="result-icon">📟</div>
          <div class="result-info">
            <div class="result-name">{{ host.name }}</div>
            <div class="result-details">
              {{ host.username }}@{{ host.host }}:{{ host.port }}
            </div>
          </div>
          <div v-if="selectedIndex === index" class="result-hint">回车连接</div>
        </div>
      </div>
      <div v-else-if="query" class="no-results">
        没有找到匹配的服务器
      </div>
      <div v-else class="results-list">
        <div class="results-header">我的主机</div>
        <div 
          v-for="(host, index) in hosts.slice(0, 5)" 
          :key="host.id"
          class="result-item"
          :class="{ active: selectedIndex === index }"
          @mouseenter="selectedIndex = index"
          @click="selectHost(host)"
        >
          <div class="result-icon">📟</div>
          <div class="result-info">
            <div class="result-name">{{ host.name }}</div>
            <div class="result-details">
              {{ host.username }}@{{ host.host }}:{{ host.port }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue';

interface Host {
  id?: number;
  name: string;
  host: string;
  port: number;
  username: string;
}

const props = defineProps<{
  visible: boolean;
  hosts: Host[];
}>();

const emit = defineEmits(['update:visible', 'select']);

const query = ref('');
const selectedIndex = ref(0);
const inputRef = ref<HTMLInputElement | null>(null);

const filteredHosts = computed(() => {
  if (!query.value) return [];
  const q = query.value.toLowerCase();
  return props.hosts.filter(h => 
    h.name.toLowerCase().includes(q) || 
    h.host.toLowerCase().includes(q) || 
    h.username.toLowerCase().includes(q)
  );
});

watch(() => props.visible, (val) => {
  if (val) {
    query.value = '';
    selectedIndex.value = 0;
    nextTick(() => {
      inputRef.value?.focus();
    });
  }
});

watch(filteredHosts, () => {
  selectedIndex.value = 0;
});

const close = () => {
  emit('update:visible', false);
};

const selectHost = (host: Host) => {
  emit('select', host);
  close();
};

const moveDown = (e: Event) => {
  e.preventDefault();
  const max = filteredHosts.value.length > 0 ? filteredHosts.value.length : Math.min(props.hosts.length, 5);
  selectedIndex.value = (selectedIndex.value + 1) % max;
};

const moveUp = (e: Event) => {
  e.preventDefault();
  const max = filteredHosts.value.length > 0 ? filteredHosts.value.length : Math.min(props.hosts.length, 5);
  selectedIndex.value = (selectedIndex.value - 1 + max) % max;
};

const confirm = () => {
  const currentList = filteredHosts.value.length > 0 ? filteredHosts.value : props.hosts.slice(0, 5);
  if (currentList[selectedIndex.value]) {
    selectHost(currentList[selectedIndex.value]);
  }
};
</script>

<style scoped>
.spotlight-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(8px);
  z-index: 10000;
  display: flex;
  justify-content: center;
  padding-top: 20vh;
}

.spotlight-modal {
  width: 100%;
  max-width: 600px;
  background: white;
  border-radius: 12px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  overflow: hidden;
  height: max-content;
  max-height: 400px;
  display: flex;
  flex-direction: column;
}

.search-input-wrapper {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  background: #f5f5f5;
  margin: 12px 16px 4px;
  border-radius: 10px;
  border: 1px solid rgba(0, 0, 0, 0.05);
}

.search-icon {
  font-size: 1.1rem;
  margin-right: 10px;
  opacity: 0.6;
  display: flex;
  align-items: center;
}

.search-input {
  flex: 1;
  border: none;
  outline: none;
  font-size: 1rem;
  background: transparent;
  color: #333;
  padding: 4px 0;
}

.results-list {
  padding: 8px;
  overflow-y: auto;
}

.results-header {
  padding: 8px 12px;
  font-size: 0.75rem;
  text-transform: uppercase;
  color: #999;
  font-weight: 600;
  letter-spacing: 0.5px;
}

.result-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.result-item:hover, .result-item.active {
  background-color: #396cd8;
  color: white;
}

.result-icon {
  font-size: 1.4rem;
  margin-right: 12px;
}

.result-info {
  flex: 1;
}

.result-name {
  font-weight: 600;
  font-size: 0.95rem;
}

.result-details {
  font-size: 0.75rem;
  opacity: 0.7;
  margin-top: 2px;
}

.result-item:hover .result-details, .result-item.active .result-details {
  opacity: 0.9;
}

.result-hint {
  font-size: 0.7rem;
  opacity: 0.5;
  background: rgba(255, 255, 255, 0.2);
  padding: 2px 6px;
  border-radius: 4px;
}

.no-results {
  padding: 40px;
  text-align: center;
  color: #999;
}

.animate-pop {
  animation: pop-in 0.2s ease-out;
}

@keyframes pop-in {
  from {
    transform: scale(0.95);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

/* 自定义滚动条样式 */
.results-list::-webkit-scrollbar {
  width: 2px;
}

.results-list::-webkit-scrollbar-track {
  background: transparent;
}

.results-list::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.1);
  border-radius: 4px;
}

.results-list::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.2);
}
</style>
