<template>
  <div ref="terminalContainer" class="terminal-container"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import '@xterm/xterm/css/xterm.css';

const props = defineProps<{
  sessionId?: string;
  isActive?: boolean;
}>();

const terminalContainer = ref<HTMLElement | null>(null);
let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let unlisten: UnlistenFn | null = null;
let resizeObserver: ResizeObserver | null = null;

const emit = defineEmits(['data', 'resize']);

onMounted(async () => {
  if (!terminalContainer.value) return;

  term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: '"Cascadia Code", Menlo, Monaco, "Courier New", monospace',
    theme: {
      background: '#1e1e1e',
      foreground: '#d4d4d4',
    },
    allowProposedApi: true
  });

  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.loadAddon(new WebLinksAddon());

  term.open(terminalContainer.value);
  
  // 使用 ResizeObserver 代替 window.resize 监听，处理 v-show 导致的维度变化
  resizeObserver = new ResizeObserver(() => {
    if (props.isActive && fitAddon) {
      try {
        fitAddon.fit();
      } catch (e) {
        // 忽略隐藏时的计算异常
      }
    }
  });
  resizeObserver.observe(terminalContainer.value);

  // 处理输入：从 xterm 到 SSH 后端
  term.onData(async (data) => {
    if (props.sessionId) {
      try {
        const encoder = new TextEncoder();
        await invoke('write_to_ssh', {
          sessionId: props.sessionId,
          data: Array.from(encoder.encode(data))
        });
      } catch (err) {
        console.error('发送数据失败:', err);
      }
    }
  });

  term.onResize(async (size) => {
    if (props.sessionId) {
      try {
        await invoke('resize_ssh_session', {
          sessionId: props.sessionId,
          cols: size.cols,
          rows: size.rows
        });
        console.log(`已将 PTY 尺寸同步到后端: ${size.cols}x${size.rows}`);
      } catch (err) {
        console.error('同步 PTY 尺寸失败:', err);
      }
    }
    emit('resize', size);
  });

  terminalContainer.value.addEventListener('click', () => {
    if (props.isActive) term?.focus();
  });

  // 监听初始状态
  term.writeln('\x1b[1;32mAI-Term-Shell\x1b[0m 终端就绪...');
  if (props.sessionId) {
    setupSessionListener(props.sessionId);
  } else {
    term.writeln('正在等待会话启动...');
  }
});

async function setupSessionListener(id: string) {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
  if (term) {
    term.writeln(`\x1b[1;34m系统:\x1b[0m 已连接到会话 ${id.substring(0, 8)}`);
    if (props.isActive) term.focus();
    
    // 处理输出：从 SSH 后端到 xterm
    unlisten = await listen<number[]>(`sse-data-${id}`, (event) => {
      if (term) {
        term.write(new Uint8Array(event.payload));
      }
    });
  }
}

// 监听活动状态，切换标签时聚焦
watch(() => props.isActive, (active) => {
  if (active && term && fitAddon) {
    setTimeout(() => {
      try { fitAddon!.fit(); } catch(e){}
      term!.focus();
    }, 50); // 给 DOM 一点显示的时间
  }
});

// 仅在 sessionId 改变时重新监听（不支持热重载的覆盖，仅为安全起见）
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
.terminal-container {
  width: 100%;
  height: 100%;
  background-color: #1e1e1e;
  cursor: text;
}

:deep(.xterm-viewport) {
  background-color: #1e1e1e !important;
}
</style>
