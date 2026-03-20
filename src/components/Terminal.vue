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

    <div v-if="suggestions.length > 0" class="autocomplete-bar animate-pop">
      <div class="ac-hint">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>
        智能建议
      </div>
      <div class="ac-list">
        <span 
          v-for="(item, idx) in suggestions" 
          :key="idx" 
          class="ac-item" 
          :class="{ active: idx === selectedCompIdx }"
          @click="acceptCompletion(idx)"
        >{{ item }}</span>
      </div>
      <div class="ac-shortcuts">按 <kbd>Tab</kbd> 或 <kbd>→</kbd> 补全 · <kbd>↑</kbd><kbd>↓</kbd> 切换 · <kbd>Esc</kbd> 关闭</div>
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
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
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
let unlistenSettings: UnlistenFn | null = null;
let resizeObserver: ResizeObserver | null = null;

const emit = defineEmits(['data', 'resize', 'zmodem-detect', 'zmodem-sz']);

const showMenu = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const menuItems = ref<MenuItem[]>([]);

const showSearchBar = ref(false);
const searchQuery = ref('');
const searchInputRef = ref<HTMLInputElement | null>(null);

// ---- Auto Completion (智能提示) ----
const autoCompletionEnabled = ref(true);
const suggestions = ref<string[]>([]);
const selectedCompIdx = ref(0);
let currentInputStr = '';
let lastSubmittedCmd = '';
let lastSeenCwd = '~'; // Track cwd from terminal prompt
let zmodemCooldown = false; // Prevent re-triggering after abort
let compTimeout: any = null;

// Mock list of common commands for basic completion
const COMMON_CMDS = [
  'ls -la', 'cd ..', 'pwd', 'clear', 'exit', 'history',
  'git status', 'git add -A', 'git commit -m ""', 'git pull', 'git push',
  'docker ps', 'docker-compose up -d', 'docker images',
  'npm install', 'npm run dev', 'npm run build', 'yarn dev', 
  'htop', 'tail -f', 'systemctl status', 'systemctl restart'
];

async function fetchCompletions() {
  if (!autoCompletionEnabled.value) { suggestions.value = []; return; }
  const q = currentInputStr.trim().toLowerCase();
  if (!q || !props.sessionId) { suggestions.value = []; return; }
  
  try {
    const backendSuggestions = await invoke<string[]>('get_completions', {
      sessionId: props.sessionId,
      currentInput: currentInputStr,
    });
    
    // Fallback/append 简单的常用命令
    const matches = COMMON_CMDS.filter(c => c.startsWith(q) && c !== q);
    
    const combined = [...new Set([...backendSuggestions, ...matches])];
    suggestions.value = combined.slice(0, 5); // display up to 5
    selectedCompIdx.value = 0;
  } catch (err) {
    console.error('Failed to fetch completions', err);
    suggestions.value = [];
  }
}

function acceptCompletion(idx: number) {
  if (!suggestions.value[idx] || !term) return;
  const item = suggestions.value[idx];
  // Calculate what's missing
  const remainder = item.substring(currentInputStr.length);
  if (remainder) {
    sendDataToBackend(remainder);
  }
  currentInputStr = item; // update tracker
  suggestions.value = [];
}


// ---- Session 录制 ----
const isRecording = ref(false);
let recordStartTime = 0;
let recordBuffer: [number, string][] = []; // [elapsed_sec, text]

function startRecording() {
  recordBuffer = [];
  recordStartTime = Date.now();
  isRecording.value = true;
}

async function stopRecording(title?: string) {
  if (!isRecording.value) return;
  isRecording.value = false;
  // 生成 asciicast v2 格式
  const header = JSON.stringify({
    version: 2,
    width: term?.cols ?? 80,
    height: term?.rows ?? 24,
    timestamp: Math.floor(recordStartTime / 1000),
    title: title ?? 'Nixu Recording',
  });
  const events = recordBuffer.map(([t, d]) =>
    JSON.stringify([t, 'o', d])
  ).join('\n');
  const castContent = header + '\n' + events;

  try {
    const filePath = await save({
      title: '保存录制文件',
      defaultPath: `nixu-recording-${Date.now()}.cast`,
      filters: [{ name: 'asciicast', extensions: ['cast'] }],
    });
    if (filePath) {
      await writeTextFile(filePath, castContent);
    }
  } catch (err) {
    console.error('Save recording failed:', err);
  }
  recordBuffer = [];
}

onMounted(async () => {
  if (!terminalContainer.value) return;

  // 从数据库读取终端外观设置
  const savedFontSize   = await invoke<string | null>('get_setting', { key: 'term_font_size'    }).catch(() => null);
  const savedLineHeight = await invoke<string | null>('get_setting', { key: 'term_line_height'  }).catch(() => null);
  const savedFontFamily = await invoke<string | null>('get_setting', { key: 'term_font_family'  }).catch(() => null);
  const savedCursor     = await invoke<string | null>('get_setting', { key: 'term_cursor_style' }).catch(() => null);
  const savedAutoComp   = await invoke<string | null>('get_setting', { key: 'term_auto_completion'}).catch(() => null);

  const fontSize   = savedFontSize   ? parseInt(savedFontSize)     : 14;
  const lineHeight = savedLineHeight ? parseFloat(savedLineHeight)  : 1.2;
  const fontFamily = savedFontFamily || '"Cascadia Code", Menlo, Monaco, "Courier New", monospace';
  const cursorStyle = (savedCursor as any) || 'block';
  autoCompletionEnabled.value = savedAutoComp !== 'false';

  term = new Terminal({
    cursorBlink: true,
    cursorStyle,
    fontSize,
    lineHeight,
    fontFamily,
    scrollback: 10000,
    theme: {
      background: '#0a0a0a',
      foreground: '#d4d4d4',
      cursor: '#396cd8',
      selectionBackground: '#ffcc00',
      selectionForeground: '#000000',
      selectionInactiveBackground: '#ffcc00',
    },
    allowProposedApi: true,
    convertEol: true,
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

  // 拦截特定的前端按键，处理自动补全下拉框逻辑
  term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
    if (suggestions.value.length > 0 && e.type === 'keydown') {
      if (e.key === 'Tab' || e.key === 'ArrowRight') {
        acceptCompletion(selectedCompIdx.value);
        e.preventDefault();
        return false;
      } else if (e.key === 'ArrowDown') {
        selectedCompIdx.value = (selectedCompIdx.value + 1) % suggestions.value.length;
        e.preventDefault();
        return false;
      } else if (e.key === 'ArrowUp') {
        selectedCompIdx.value = (selectedCompIdx.value - 1 + suggestions.value.length) % suggestions.value.length;
        e.preventDefault();
        return false;
      } else if (e.key === 'Escape') {
        suggestions.value = [];
        e.preventDefault();
        return false;
      }
    }
    return true;
  });

  // 处理输入
  term.onData(async (data) => {
    // 粗略跟踪输入，如果不精确，可后续改进
    if (data === '\r' || data === '\n' || data === '\x03' /* Ctrl+C */) {
      // Read the actual command from xterm buffer (captures Tab completion too)
      try {
        const buf = term?.buffer.active;
        if (!buf) throw new Error('no buffer');
        const cursorRow = buf.cursorY + buf.viewportY;
        let lineText = '';
        for (let i = 0; i <= cursorRow && i < buf.length; i++) {
          const line = buf.getLine(i);
          if (line) lineText = line.translateToString(true);
        }
        // Extract command after prompt (after last $ or #)
        const afterPrompt = lineText.replace(/^.*[#$]\s*/, '').trim();
        if (afterPrompt.length > 0) {
          lastSubmittedCmd = afterPrompt;
        } else {
          lastSubmittedCmd = currentInputStr.trim();
        }
      } catch {
        lastSubmittedCmd = currentInputStr.trim();
      }
      currentInputStr = '';
      suggestions.value = [];
    } else if (data === '\x7f' || data === '\b') {
      currentInputStr = currentInputStr.slice(0, -1);
    } else if (!data.includes('\x1b') && data.length === 1) { // 忽略转义序列
      currentInputStr += data;
    } else if (data.length > 1 && !data.includes('\x1b')) {
      // 处理粘贴等多字符
      currentInputStr += data;
    }

    sendDataToBackend(data);

    // 防抖请求匹配
    if (currentInputStr.length >= 2) {
      if (compTimeout) clearTimeout(compTimeout);
      compTimeout = setTimeout(() => fetchCompletions(), 250);
    } else {
      suggestions.value = [];
    }
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
      } catch (err) {
        console.error("Backend resize failed:", err);
      }
    }
    emit('resize', size);
  });

  terminalContainer.value.addEventListener('click', () => {
    if (props.isActive) term?.focus();
  });

  term.write('\x1b[1;32mNixu\x1b[0m 终端就绪... ');
  if (props.sessionId) {
    setupSessionListener(props.sessionId);
  } else {
    term.writeln('正在等待会话启动...');
  }

  // 监听设置变更，实时更新终端外观（无需重启）
  unlistenSettings = await listen<{
    fontSize: number;
    lineHeight: number;
    fontFamily: string;
    cursorStyle: 'block' | 'underline' | 'bar';
    autoCompletion?: boolean;
  }>('terminal-settings-changed', (event) => {
    if (!term) return;
    const { fontSize, lineHeight, fontFamily, cursorStyle, autoCompletion } = event.payload;
    term.options.fontSize    = fontSize;
    term.options.lineHeight  = lineHeight;
    term.options.fontFamily  = fontFamily;
    term.options.cursorStyle = cursorStyle;
    if (autoCompletion !== undefined) {
      autoCompletionEnabled.value = autoCompletion;
      if (!autoCompletion) suggestions.value = [];
    }
    setTimeout(() => { try { fitAddon?.fit(); } catch(e){} }, 50);
  });
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
          const u8 = new Uint8Array(event.payload);
          
          // Save cooldown state BEFORE prompt parsing may reset it
          // (used later for filtering - must capture state before any mutation)
          const wasInCooldown = zmodemCooldown;

          // Always parse shell prompt for cwd tracking (on all SSH data)
          if (props.type === 'ssh') {
            const text = new TextDecoder().decode(u8);
            
            // Parse prompt: [user@host dir]# or user@host:dir$
            const promptMatch = text.match(/\[\w+@[\w.-]+\s+([^\]]+)\]\s*[#$]/) ||
                                text.match(/\w+@[\w.-]+:([^\$#]+)[\$#]/);
            if (promptMatch) {
              lastSeenCwd = promptMatch[1].trim();
              // Reset ZMODEM cooldown when shell prompt appears (operation is done)
              zmodemCooldown = false;
            }
          }

          // ZMODEM detection: look for "**\x18B0" (ZRQINIT) or rz waiting sequence
          if (props.type === 'ssh' && u8.length > 4 && !zmodemCooldown) {
            const text = new TextDecoder('utf-8', { fatal: false }).decode(u8);
            const isZmodem = text.includes('**\x18B0') || text.includes('rz waiting to receive');
            
            if (isZmodem) {
              // Set cooldown: ignore further ZMODEM until shell prompt reappears
              zmodemCooldown = true;
              const cmd = lastSubmittedCmd.trim();
              // Match 'sz' anywhere in the command (handles 'cd /tmp && sz file')
              const szMatch = cmd.match(/(?:^|\s|&&|\|)sz\s+(.+?)(?:\s*&&|\s*\||$)/i) ||
                              cmd.match(/\bsz\s+(.+)/i);
              const isSz = !!szMatch;
              const filenameFromCmd = szMatch ? szMatch[1].trim().split(/\s+/)[0] : '';
              lastSubmittedCmd = '';
              
              if (isSz) {
                // Use ps to get the EXACT command args of the running sz process
                // AND use /proc to get the real cwd of the shell - all in one exec
                invoke('sftp_exec', {
                  sessionId: id,
                  command: 'SZ_PID=$(pgrep -n sz 2>/dev/null); FILE=$(xargs -0 < /proc/$SZ_PID/cmdline 2>/dev/null | cut -d" " -f2-); CWD=$(readlink /proc/$SZ_PID/cwd 2>/dev/null || echo $HOME); echo "FILE:$FILE|CWD:$CWD"'
                }).then((result: any) => {
                  const res = String(result).trim();
                  const fileMatch = res.match(/FILE:(.+)\|CWD:(.+)/);
                  const szFilename = fileMatch ? fileMatch[1].trim() : filenameFromCmd;
                  const szCwd = fileMatch ? fileMatch[2].trim() : lastSeenCwd;
                  
                  invoke('write_to_ssh', { sessionId: id, data: [24,24,24,24,24,8,8,8,8,8] }).catch(() => {});
                  term?.writeln(`\x1b[1;33m[Nixu]\x1b[0m sz: ${szFilename} (${szCwd})`);
                  emit('zmodem-sz', { sessionId: id, filename: szFilename, cwd: szCwd });
                }).catch(() => {
                  invoke('write_to_ssh', { sessionId: id, data: [24,24,24,24,24,8,8,8,8,8] }).catch(() => {});
                  emit('zmodem-sz', { sessionId: id, filename: filenameFromCmd, cwd: lastSeenCwd });
                });
              } else {
                // rz: send Ctrl+C immediately and emit right away
                // Don't wait for sftp_exec - App.vue's resolveCwd will handle the path
                invoke('write_to_ssh', { sessionId: id, data: [24,24,24,24,24,8,8,8,8,8] }).catch(() => {});
                term?.write('\r\n');
                term?.writeln(`\x1b[1;33m[Nixu]\x1b[0m 检测到 rz，弹出上传窗口...`);
                emit('zmodem-detect', { sessionId: id, cwd: lastSeenCwd });
              }
              return;
            }
          }
          
          // During ZMODEM cooldown, strip ZMODEM control bytes but keep printable content
          // Use wasInCooldown (captured before promptMatch may have reset it) 
          if (wasInCooldown) {
            const t = new TextDecoder('utf-8', { fatal: false }).decode(u8);
            // Check if packet contains ZMODEM sequences
            if (t.includes('\x18') || t.includes('**B')) {
              // Strip ZMODEM bytes, keep printable chars and ANSI sequences
              const cleaned = t.replace(/[\x18]+/g, '')   // Remove CAN (ZMODEM abort) bytes
                               .replace(/\*\*B[0-9a-fA-F]*/g, ''); // Remove ZMODEM headers
              if (cleaned.length > 0) {
                term.write(cleaned);
              }
              return;
            }
          }

          term.write(u8);
          // 录制拦截
          if (isRecording.value) {
            const elapsed = (Date.now() - recordStartTime) / 1000;
            const text = new TextDecoder().decode(u8);
            recordBuffer.push([parseFloat(elapsed.toFixed(6)), text]);
          }
        }
      });
      
      // 会话监听建立后，强制同步一次尺寸，确保后端知道真实的 cols/rows
      setTimeout(() => {
        if (fitAddon && term) {
          try {
            fitAddon.fit();
            // 显式触发一次 resize 逻辑以同步后端
            const size = { cols: term.cols, rows: term.rows };
            const command = props.type === 'ssh' ? 'resize_ssh_session' : 'resize_pty_session';
            invoke(command, {
              sessionId: id,
              cols: size.cols,
              rows: size.rows
            }).catch(e => console.error("Initial resize failed:", e));
          } catch (e) {}
        }
      }, 100);
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
  if (unlistenSettings) unlistenSettings();
  term?.dispose();
});

defineExpose({
  write: (data: string) => sendDataToBackend(data),
  writeln: (data: string) => sendDataToBackend(data + '\n'),
  clear: () => term?.clear(),
  focus: () => term?.focus(),
  startRecording,
  stopRecording,
  isRecording,
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

/* ===== 自动补全浮层 ===== */
.autocomplete-bar {
  position: absolute;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(30, 30, 46, 0.95);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  padding: 10px 14px;
  border-radius: 12px;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.4);
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 200;
  color: white;
  pointer-events: auto;
  min-width: 320px;
  max-width: 600px;
}

.ac-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 700;
  color: #6366f1;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding-bottom: 6px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  width: 100%;
}

.ac-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.ac-item {
  padding: 6px 10px;
  border-radius: 6px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 13px;
  color: #d1d5db;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
  border: 1px solid transparent;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  width: 100%;
  box-sizing: border-box;
}

.ac-item:hover {
  background: rgba(255, 255, 255, 0.05);
}

.ac-item.active {
  background: rgba(99, 102, 241, 0.2);
  color: #fff;
  border-color: rgba(99, 102, 241, 0.5);
  font-weight: 600;
}

.ac-shortcuts {
  font-size: 11px;
  color: #9ca3af;
  display: flex;
  align-items: center;
  gap: 4px;
  padding-top: 6px;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  width: 100%;
}

.ac-shortcuts kbd {
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 4px;
  padding: 1px 4px;
  font-family: inherit;
  font-size: 10px;
  color: #e5e7eb;
}
</style>
