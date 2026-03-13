<template>
  <div class="terminal-outer" @contextmenu.prevent="onContextMenu">
    <div class="terminal-padding-wrapper">
      <div ref="terminalContainer" class="terminal-container"></div>
    </div>
    
    <div v-if="showSearchBar" class="terminal-search-bar animate-pop">
      <input 
        ref="searchInputRef"
        v-model="searchQuery" 
        placeholder="在终端中查找..." 
        class="search-input"
        @input="onSearch(false)"
        @keydown.enter="onSearch(true)"
        @keydown.esc="showSearchBar = false"
      />
      <div class="search-btns">
        <button @click="searchAddon?.findPrevious(searchQuery)">上一个</button>
        <button @click="searchAddon?.findNext(searchQuery)">下一个</button>
        <button class="close-btn" @click="showSearchBar = false">×</button>
      </div>
    </div>

    <ContextMenu
      v-model:visible="showMenu"
      :x="menuX"
      :y="menuY"
      :items="menuItems"
      @action="handleMenuAction"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';
import { SearchAddon } from '@xterm/addon-search';
import { Unicode11Addon } from '@xterm/addon-unicode11';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import ContextMenu, { MenuItem } from './ContextMenu.vue';
import '@xterm/xterm/css/xterm.css';

const props = defineProps<{
  sessionId?: string;
  isActive?: boolean;
  type?: 'ssh' | 'local';
}>();

const terminalContainer = ref<HTMLElement | null>(null);
let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;
const searchAddon = ref<SearchAddon | null>(null);
let unlisten: UnlistenFn | null = null;
let resizeObserver: ResizeObserver | null = null;

const emit = defineEmits(['data', 'resize']);

const showMenu = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const menuItems = ref<MenuItem[]>([]);

const showSearchBar = ref(false);
const searchQuery = ref('');
const searchInputRef = ref<HTMLInputElement | null>(null);

onMounted(async () => {
  if (!terminalContainer.value) return;

  term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: '"Cascadia Code", Menlo, Monaco, "Courier New", monospace',
    scrollback: 10000,
    theme: {
      background: '#0a0a0a',
      foreground: '#d4d4d4',
      cursor: '#396cd8',
      selectionBackground: '#ffcc00', // 使用更醒目的金黄色作为当前搜索/选中背景
      selectionForeground: '#000000', // 选中文字颜色为黑色
      selectionInactiveBackground: '#ffcc00', // 即使失去焦点也保持高亮
    },
    allowProposedApi: true,
    convertEol: true, // 确保自动处理换行
  });

  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  
  searchAddon.value = new SearchAddon();
  term.loadAddon(searchAddon.value);
  
  term.loadAddon(new Unicode11Addon());
  term.unicode.activeVersion = '11';
  
  term.loadAddon(new WebLinksAddon((_event, uri) => {
    // 自动识别并打开链接
    window.open(uri, '_blank');
  }));

  term.open(terminalContainer.value);
  
  // 尝试加载 WebGL 加速
  try {
    const webglAddon = new WebglAddon();
    term.loadAddon(webglAddon);
  } catch (e) {}
  
  let resizeTimer: any = null;
  resizeObserver = new ResizeObserver(() => {
    if (props.isActive && fitAddon) {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        try {
          fitAddon!.fit();
          console.log('Terminal: Refitted to handle wrapping -', term?.cols, 'cols');
        } catch (e) {}
      }, 50);
    }
  });
  resizeObserver.observe(terminalContainer.value!.parentElement!); // 观察包含内边距的容器

  // 处理输入
  term.onData(async (data) => {
    sendDataToBackend(data);
  });

  term.onResize(async (size) => {
    if (props.sessionId) {
      try {
        const command = props.type === 'ssh' ? 'resize_ssh_session' : 'resize_pty_session';
        await invoke(command, {
          sessionId: props.sessionId,
          cols: size.cols,
          rows: size.rows
        });
      } catch (err) {}
    }
    emit('resize', size);
  });

  terminalContainer.value.addEventListener('click', () => {
    if (props.isActive) term?.focus();
  });

  term.writeln('\x1b[1;32mAI-Term-Shell\x1b[0m 终端就绪...');
  if (props.sessionId) {
    setupSessionListener(props.sessionId);
  } else {
    term.writeln('正在等待会话启动...');
  }
});

async function setupSessionListener(id: string) {
  if (unlisten) {
    const oldUnlisten = unlisten;
    unlisten = null;
    oldUnlisten();
  }

  if (term) {
    term.writeln(`\x1b[1;34m系统:\x1b[0m 已连接到会话 ${id.substring(0, 8)}`);
    if (props.isActive) term.focus();
    
    try {
      unlisten = await listen<number[]>(`sse-data-${id}`, (event) => {
        if (term) {
          term.write(new Uint8Array(event.payload));
        }
      });
    } catch (err) {}
  }
}

function onContextMenu(e: MouseEvent) {
  menuX.value = e.clientX;
  menuY.value = e.clientY;
  
  const hasSelection = term ? term.hasSelection() : false;
  
  menuItems.value = [
    { label: '复制', icon: '📋', action: 'copy', shortcut: '⌘C', disabled: !hasSelection },
    { label: '粘贴', icon: '📥', action: 'paste', shortcut: '⌘V' },
    { label: '粘贴选中', icon: '🖱️', action: 'pasteSelection', disabled: !hasSelection },
    { divider: true },
    { label: '查找', icon: '🔍', action: 'find', shortcut: '⌘F' },
    { divider: true },
    { label: '清空缓存', icon: '🧹', action: 'clear' },
  ];
  
  showMenu.value = true;
}

async function handleMenuAction(action: string) {
  if (!term) return;

  switch (action) {
    case 'copy':
      if (term.hasSelection()) {
        await navigator.clipboard.writeText(term.getSelection());
      }
      break;
    case 'paste':
      try {
        const text = await navigator.clipboard.readText();
        if (text) sendDataToBackend(text);
      } catch (err) {}
      break;
    case 'pasteSelection':
      if (term.hasSelection()) {
        sendDataToBackend(term.getSelection());
      }
      break;
    case 'find':
      {
        const selection = term.getSelection();
        if (selection) {
          searchQuery.value = selection;
        }
        showSearchBar.value = true;
        nextTick(() => {
          searchInputRef.value?.focus();
          if (searchQuery.value) {
            onSearch(false); // 自动执行一次搜索以高亮
          }
        });
      }
      break;
    case 'clear':
      term.clear();
      break;
  }
  term.focus();
}

async function sendDataToBackend(data: string) {
  if (props.sessionId) {
    try {
      const encoder = new TextEncoder();
      const command = props.type === 'ssh' ? 'write_to_ssh' : 'write_to_pty';
      await invoke(command, {
        sessionId: props.sessionId,
        data: Array.from(encoder.encode(data))
      });
    } catch (err) {}
  }
}

function onSearch(isNext = false) {
  if (searchAddon.value && searchQuery.value) {
    if (isNext) {
      searchAddon.value.findNext(searchQuery.value);
    } else {
      searchAddon.value.findNext(searchQuery.value, { incremental: true });
    }
  }
}

watch(() => props.isActive, (active) => {
  if (active && term && fitAddon) {
    setTimeout(() => {
      try { fitAddon!.fit(); } catch(e){}
      term!.focus();
    }, 50);
  }
});

watch(() => props.sessionId, (newId) => {
  if (newId) {
    setupSessionListener(newId);
  }
});

onBeforeUnmount(() => {
  if (resizeObserver) resizeObserver.disconnect();
  if (unlisten) unlisten();
  term?.dispose();
});

defineExpose({
  write: (data: string | Uint8Array) => term?.write(data),
  writeln: (data: string) => term?.writeln(data),
  clear: () => term?.clear(),
  focus: () => term?.focus(),
});
</script>

<style scoped>
.terminal-outer {
  width: 100%;
  height: 100%;
  position: relative;
  overflow: hidden;
}

.terminal-padding-wrapper {
  width: 100%;
  height: 100%;
  padding: 12px;
  box-sizing: border-box;
  background-color: #0a0a0a;
  display: flex;
}

.terminal-container {
  flex: 1;
  width: 100%;
  height: 100%;
  overflow: hidden;
  cursor: text;
}

.terminal-search-bar {
  position: absolute;
  top: 12px;
  right: 40px;
  background: white;
  padding: 4px 12px;
  border-radius: 10px;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  z-index: 100;
  border: 1px solid rgba(0, 0, 0, 0.08);
  min-width: 320px;
}

.animate-pop {
  animation: pop-in 0.2s cubic-bezier(0.18, 0.89, 0.32, 1.28);
}

@keyframes pop-in {
  from { transform: scale(0.9) translateY(-10px); opacity: 0; }
  to { transform: scale(1) translateY(0); opacity: 1; }
}

.terminal-search-bar .search-input {
  flex: 1;
  border: none;
  outline: none;
  font-size: 0.95rem;
  background: transparent;
  color: #333;
  padding: 6px 4px;
}

.search-btns {
  display: flex;
  gap: 4px;
  margin-left: 8px;
  border-left: 1px solid #eee;
  padding-left: 8px;
}

.search-btns button {
  padding: 4px 8px;
  background: transparent;
  border: none;
  color: #396cd8;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  border-radius: 4px;
}

.search-btns button:hover {
  background: rgba(57, 108, 216, 0.1);
}

.search-btns button.close-btn {
  color: #999;
  font-size: 1.1rem;
}

.search-btns button.close-btn:hover {
  color: #ff4d4f;
  background: rgba(255, 77, 79, 0.1);
}

:deep(.xterm-viewport) {
  background-color: #0a0a0a !important;
}

/* 自定义滚动条样式 */
:deep(.xterm-viewport::-webkit-scrollbar) {
  width: 10px !important; /* 总宽度设大一点方便点击，视觉上通过边框变窄 */
}

:deep(.xterm-viewport::-webkit-scrollbar-track) {
  background: transparent;
}

:deep(.xterm-viewport::-webkit-scrollbar-thumb) {
  background-color: rgba(255, 255, 255, 0.15) !important;
  background-clip: padding-box !important; /* 关键：让背景只在内容区显示 */
  border: 4px solid transparent !important;   /* 四周透明边框，10 - 4*2 = 2px 视觉宽度 */
  border-radius: 10px !important;
}

:deep(.xterm-viewport::-webkit-scrollbar-thumb:hover) {
  background-color: rgba(255, 255, 255, 0.3) !important;
}
</style>
