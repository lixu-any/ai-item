<template>
  <div class="terminal-outer" @contextmenu.prevent="onContextMenu">
    <!-- 时光机悬浮投影 (Session Recap Overlay) — 必须在最外层以逃脱 overflow:hidden 裁剪 -->
    <transition name="recap-fade">
      <div v-if="recapMessage" class="session-recap-overlay">
        <span class="sparkle">✨</span>
        <span>{{ recapMessage }}</span>
      </div>
    </transition>

    <div class="terminal-padding-wrapper" @click="focusTerminal">
          <div style="position:relative; width:100%; height:100%;">
            <div ref="terminalContainer" class="terminal-container" @wheel="handleWheel"></div>

            <div v-if="ghostRemainder" class="ghost-text" :style="ghostTextStyle">{{ ghostRemainder }}</div>
            
            <div v-if="aiLockedCmd" class="ai-lock-overlay animate-pop" @click.stop>
              <div class="lock-box">
                <div class="lock-header">
                  <svg viewBox="0 0 24 24" fill="none" class="lock-icon" stroke="currentColor" stroke-width="2"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0zM12 9v4m0 4h.01"/></svg>
                  <h3>[安全结界] 高危指令拦截</h3>
                </div>
                <p class="lock-desc">您正准备执行具有毁灭性的操作：</p>
                <div class="lock-cmd"><span>{{ aiLockedCmd }}</span></div>
                <div class="lock-actions">
                  <span class="math-captcha">{{ mathQuestion }}</span>
                  <div style="position: relative;">
                    <input 
                      v-model="aiLockConfirmText" 
                      placeholder="输入结果放行" 
                      @keydown.enter="confirmAiLock"
                      class="lock-input"
                      ref="aiLockInputRef"
                    />
                    <div class="lock-error-toast" :class="{ 'show': showLockError }">计算错误！</div>
                  </div>
                  <button class="lock-btn danger" @click="confirmAiLock">确认</button>
                  <button class="lock-btn safe" @click="cancelAiLock">取消</button>
                </div>
              </div>
            </div>
          </div>
          <NanoHUD v-if="props.type === 'ssh' && termEnableNanoHud" :session="props.sessionObj" :is-active="!!props.isActive" />
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

    <!-- 全息聚光灯搜索 (Holographic Semantic Search) -->
    <div v-if="showHoloSearch" class="holo-search-overlay animate-pop" @click="closeHoloSearch">
      <div class="holo-search-box" @click.stop>
        <div class="holo-search-header">
          <svg class="holo-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><path d="M11 8v6M8 11h6"/></svg>
          <input 
            ref="holoSearchInputRef"
            v-model="holoQuery" 
            placeholder="用自然语言召唤历史指令 (例如：上周查大文件的命令是啥？)" 
            class="holo-search-input"
            @keydown.enter="executeHoloSearch"
            @keydown.esc="closeHoloSearch"
            :disabled="isHoloSearching"
          />
          <div v-if="isHoloSearching" class="holo-spinner"></div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, shallowRef, onMounted, onBeforeUnmount, watch, nextTick, computed } from 'vue';
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
import NanoHUD from './NanoHUD.vue';

const recapMessage = ref<string | null>(null);

const props = defineProps<{
  sessionId?: string;
  isActive?: boolean;
  type?: 'ssh' | 'local';
  sessionObj?: any;
}>();

const terminalContainer = ref<HTMLElement | null>(null);
let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;
const searchAddon = shallowRef<SearchAddon | null>(null);
let webglAddon: WebglAddon | null = null;
let unlisten: UnlistenFn | null = null;
let unlistenSettings: UnlistenFn | null = null;
let resizeObserver: ResizeObserver | null = null;
let resizeTimer: any = null;

const emit = defineEmits(['data', 'resize', 'zmodem-detect', 'zmodem-sz', 'ai-diagnose', 'toast']);

const showMenu = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const menuItems = ref<MenuItem[]>([]);

const showSearchBar = ref(false);
const searchQuery = ref('');
const searchInputRef = ref<HTMLInputElement | null>(null);

// ---- 全息语意搜索 (Holographic Search) ----
const showHoloSearch = ref(false);
const holoQuery = ref('');
const isHoloSearching = ref(false);
const holoSearchInputRef = ref<HTMLInputElement | null>(null);

function closeHoloSearch() {
  showHoloSearch.value = false;
  holoQuery.value = '';
  term?.focus();
}

async function executeHoloSearch() {
  const queryText = holoQuery.value.trim();
  if (!queryText || isHoloSearching.value || !props.sessionObj?.host?.id) return;
  isHoloSearching.value = true;
  
  try {
    const logs: any = await invoke('get_recent_session_logs', { hostId: props.sessionObj.host.id, limit: 300 }).catch(() => []);
    const commands = logs?.map((l: any) => l.command).join('\n') || '无历史记录';
    
    // We add explicitly "千万不要包裹 ```bash" to avoid syntax issues when pasted directly into terminal
    const systemPrompt = "你是极其冷酷干练的黑客终端指令检索引擎。如果用户找寻的意图命中历史库，请返回原指令；若未完全一致，依据意图直接写出符合且能立即执行的bash命令。只输出这句唯一的纯文本指令，绝对不包含任何Markdown块标记、反引号、提示前缀或废话！\n\n终端机记录库：\n" + commands;
    
    const foundCmd: string = await invoke('ask_ai', { prompt: queryText, systemPrompt });
    
    if (foundCmd && foundCmd.trim()) {
      // 通过底层通道写入真正的字符码以模拟用户键入（替代前端 paste）
      const u8 = new TextEncoder().encode(foundCmd.trim());
      const sessionIdStr = props.sessionId || props.sessionObj.id.toString();
      await invoke(props.type === 'ssh' ? 'write_to_ssh' : 'write_to_pty', {
        sessionId: sessionIdStr,
        data: Array.from(u8)
      });
      emit('toast', { message: '✨ 指令已吸附至光标，请按回车键 (Enter) 执行', type: 'success' });
    }
  } catch (err) {
    console.error('Holographic search failed:', err);
    term?.writeln(`\r\n\x1b[31m[Nixu 全息检索失败] ${err}\x1b[0m`);
  } finally {
    isHoloSearching.value = false;
    closeHoloSearch();
  }
}

// ---- Auto Completion (智能提示) ----
const autoCompletionEnabled = ref(true);
const termEnableNanoHud = ref(true);
const suggestions = ref<string[]>([]);
let currentInputStr = '';
let lastSubmittedCmd = '';
let lastSeenCwd = '~'; // Track cwd from terminal prompt
let zmodemCooldown = false; // Prevent re-triggering after abort
let compTimeout: any = null;

// ---- 高危死锁拦截 ----
const aiLockedCmd = ref('');
const aiLockConfirmText = ref('');
const aiLockInputRef = ref<HTMLInputElement | null>(null);
const showLockError = ref(false);
let pendingAiPayload = '';

const mathQuestion = ref('');
let mathAnswer = 0;

function generateMathCaptcha() {
  const ops = ['+', '-', '*', '/'];
  const op = ops[Math.floor(Math.random() * ops.length)];
  let a = Math.floor(Math.random() * 9) + 1; // 1-9
  let b = Math.floor(Math.random() * 9) + 1; // 1-9
  
  if (op === '+') {
    mathAnswer = a + b;
    mathQuestion.value = `${a} + ${b} = ?`;
  } else if (op === '-') {
    if (a < b) [a, b] = [b, a]; // 保证结果为正数
    mathAnswer = a - b;
    mathQuestion.value = `${a} - ${b} = ?`;
  } else if (op === '*') {
    mathAnswer = a * b;
    mathQuestion.value = `${a} × ${b} = ?`;
  } else if (op === '/') {
    // a * b = c -> c / a = b，保证整除
    const c = a * b;
    mathAnswer = b;
    mathQuestion.value = `${c} ÷ ${a} = ?`;
  }
}

const DANGEROUS_PATTERNS = [
  /\brm\s+-r?[fF]?\s+\//,         // rm -rf /
  /\bmkfs\./,                     // format disk
  /\bdd\s+if=.*of=\/dev\//,       // overwrite block device
  /\bdrop\s+database\b/i,         // drop database
  /\btruncate\s+table\b/i,        // truncate table
  /\bflushall\b/i,                // redis flushall
];

function isCommandDangerous(cmd: string) {
  return DANGEROUS_PATTERNS.some(p => p.test(cmd));
}

function confirmAiLock() {
  if (aiLockConfirmText.value.trim() === String(mathAnswer)) {
    sendDataToBackend(pendingAiPayload); // 发送被挂起的载荷（回车或完整的外部注入片段）
    aiLockedCmd.value = '';
    aiLockConfirmText.value = '';
    pendingAiPayload = '';
    currentInputStr = ''; // reset local tracker
    showLockError.value = false;
  } else {
    // 答错立刻清空，必须重填，并显示错误Toast
    aiLockConfirmText.value = '';
    showLockError.value = true;
    setTimeout(() => {
      showLockError.value = false;
    }, 2000);
  }
}

function cancelAiLock() {
  aiLockedCmd.value = '';
  aiLockConfirmText.value = '';
  pendingAiPayload = '';
  showLockError.value = false;
  sendDataToBackend('\x03'); // Send Ctrl+C to abort line in terminal!
  currentInputStr = ''; // reset local tracker
}

// Mock list of common commands for basic completion
const COMMON_CMDS = [
  'echo "hello world"', 'echo test', 'cd /var/log', 'cd /etc',
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
    suggestions.value = combined.slice(0, 5); // take best matches
    updateGhostTextPosition();
  } catch (err) {
    console.error('Failed to fetch completions', err);
    suggestions.value = [];
  }
}

const ghostRemainder = computed(() => {
  if (suggestions.value.length === 0) return '';
  const item = suggestions.value[0];
  if (item.toLowerCase().startsWith(currentInputStr.toLowerCase()) && currentInputStr.length > 0) {
    return item.substring(currentInputStr.length);
  }
  return '';
});

const ghostTextStyle = ref<Record<string, string>>({
  left: '0px',
  top: '0px',
  fontSize: '14px',
  fontFamily: 'monospace',
  lineHeight: '1.2'
});

function updateGhostTextPosition() {
  if (!term || !terminalContainer.value) return;
  const cw = terminalContainer.value.clientWidth / term.cols;
  const ch = terminalContainer.value.clientHeight / term.rows;
  
  const buf = term.buffer.active;
  ghostTextStyle.value = {
    left: `${buf.cursorX * cw}px`,
    top: `${buf.cursorY * ch}px`,
    height: `${ch}px`,
    fontSize: `${term.options.fontSize}px`,
    fontFamily: term.options.fontFamily || 'monospace',
    lineHeight: `${ch}px`
  };
}

function acceptCompletion() {
  const remainder = ghostRemainder.value;
  if (!remainder || !term) return;
  sendDataToBackend(remainder);
  currentInputStr += remainder; // update tracker
  suggestions.value = [];
}


// ---- Session 录制 ----
// ---- Focus Handling ----
function focusTerminal() {
  if (props.isActive) term?.focus();
}

function handleWheel(event: WheelEvent) {
  // Prevent scrolling the entire page when terminal has its own scrollbar
  if (term && term.buffer.active.baseY > 0 && event.deltaY < 0) { // Scrolling up and not at top
    event.preventDefault();
  }
  if (term && term.buffer.active.baseY < term.buffer.active.viewportY && event.deltaY > 0) { // Scrolling down and not at bottom
    event.preventDefault();
  }
}

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
  const savedRenderer   = await invoke<string | null>('get_setting', { key: 'term_renderer'       }).catch(() => null);
  const savedHud        = await invoke<string | null>('get_setting', { key: 'term_enable_nano_hud'}).catch(() => null);

  const fontSize   = savedFontSize   ? parseInt(savedFontSize)     : 14;
  const lineHeight = savedLineHeight ? parseFloat(savedLineHeight)  : 1.2;
  const fontFamily = savedFontFamily || '"Cascadia Code", Menlo, Monaco, "Courier New", monospace';
  const cursorStyle = (savedCursor as any) || 'block';
  autoCompletionEnabled.value = savedAutoComp !== 'false';
  termEnableNanoHud.value = savedHud !== 'false';

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
  
  // 仅在明确选择 WebGL 或默认时加载
  if (savedRenderer === 'webgl' || !savedRenderer) {
    try {
      webglAddon = new WebglAddon();
      webglAddon.onContextLoss(() => {
        console.warn('Xterm WebGL context lost, disposing addon to fallback to DOM renderer.');
        webglAddon?.dispose();
        webglAddon = null;
      });
      term.loadAddon(webglAddon);
    } catch (e) {
      console.warn('WebGL addon failed to load, falling back to DOM renderer', e);
      webglAddon = null;
    }
  }
  
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
    // 全息聚光灯拦截 (Cmd+Shift+R 或 Ctrl+Shift+R)
    if (e.type === 'keydown' && (e.metaKey || e.ctrlKey) && e.shiftKey && e.code === 'KeyR') {
      showHoloSearch.value = true;
      nextTick(() => { holoSearchInputRef.value?.focus() });
      e.preventDefault();
      return false;
    }

    if (ghostRemainder.value && e.type === 'keydown') {
      if (e.key === 'ArrowRight') {
        acceptCompletion();
        e.preventDefault();
        return false;
      } else if (e.key === 'Tab') {
        acceptCompletion();
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
    // 如果已经被锁死，屏蔽任何终端输入
    if (aiLockedCmd.value) {
      return;
    }

    // 触发前置死锁拦截前，精准捕捉终端画面中的整行文本（防 ArrowUp 历史记录和 Tab 补全绕过）
    if (data === '\r' || data === '\n') {
      let actualCommand = currentInputStr;
      try {
        const buf = term?.buffer.active;
        if (buf) {
          const cursorRow = buf.cursorY + buf.viewportY;
          let lineText = '';
          // 向上遍历直到遇到换行符，或者简单读取当前行（此处粗略读取光标所在行上方的内容）
          for (let i = Math.max(0, cursorRow - 1); i <= cursorRow && i < buf.length; i++) {
            const line = buf.getLine(i);
            if (line) lineText += line.translateToString(true);
          }
          const afterPrompt = lineText.replace(/^.*[#$]\s*/, '').trim();
          if (afterPrompt.length > 0) {
            actualCommand = afterPrompt;
          }
        }
      } catch (err) {}

      if (isCommandDangerous(actualCommand) && !aiLockedCmd.value) {
        aiLockedCmd.value = actualCommand;
        pendingAiPayload = data; // 缓存真正的发送字符
        generateMathCaptcha();
        nextTick(() => {
          aiLockInputRef.value?.focus();
        });
        return; // BLOCK SENDING ENTER!
      }
    }

    // 粗略跟踪输入，如果不精确，可后续改进
    if (data === '\r' || data === '\n' || data === '\x03' /* Ctrl+C */) {
      // Read the actual command from xterm buffer (captures Tab completion too)
      try {
        const buf = term?.buffer.active;
        if (!buf) throw new Error('no buffer');
        const cursorRow = buf.cursorY + buf.viewportY;
        let lineText = '';
        // 向上最多聚合 5 行以防超长命令折行被截断
        const maxScrollback = Math.max(0, cursorRow - 5);
        for (let i = maxScrollback; i <= cursorRow && i < buf.length; i++) {
          const line = buf.getLine(i);
          if (line) lineText += line.translateToString(true);
        }
        // Extract command after prompt (Support bash #, $, and zsh %) 
        // Use non-greedy .*? so we don't accidentally truncate user commands that naturally contain # or $
        const afterPrompt = lineText.replace(/^.*?[#$%]\s+/, '').trim();
        if (afterPrompt.length > 0) {
          lastSubmittedCmd = afterPrompt;
        } else {
          lastSubmittedCmd = currentInputStr.trim();
        }
      } catch {
        lastSubmittedCmd = currentInputStr.trim();
      }

      // 【强制侦测探针】输出当前采集到的入库要素，用于排查静默无记录 Bug
      if (data === '\r' || data === '\n') {
        const _dbgMissing = [];
        if (!lastSubmittedCmd) _dbgMissing.push("提取的Command为空");
        if (!props.sessionObj?.host?.id) _dbgMissing.push("缺失底层主机HostID");
        if (_dbgMissing.length > 0 && props.type !== 'local') {
           // local session doesn't need to log, so we only warn if it's ssh
           term?.writeln(`\r\n\x1b[33m[Nixu 截流警告] DB阻断: ${_dbgMissing.join(' | ')} (Type: ${props.type})\x1b[0m`);
        }
      }

      // 回车时，将真实下发的命令异步保存进时光机记录（为后续的会话提要与全息搜索提供弹药）
      if ((data === '\r' || data === '\n') && lastSubmittedCmd && props.sessionObj?.host?.id) {
        invoke('log_terminal_command', { 
          hostId: props.sessionObj.host.id, 
          command: lastSubmittedCmd 
        }).catch(err => {
          term?.writeln(`\r\n\x1b[31m[Nixu DB写库失败] ${err}\x1b[0m`);
        });
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

  // Vue @click handler on terminal-padding-wrapper takes care of focus now.

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
    renderer?: 'webgl' | 'dom';
    enableNanoHud?: boolean;
  }>('terminal-settings-changed', (event) => {
    if (!term) return;
    const { fontSize, lineHeight, fontFamily, cursorStyle, autoCompletion, renderer, enableNanoHud } = event.payload;
    term.options.fontSize    = fontSize;
    term.options.lineHeight  = lineHeight;
    term.options.fontFamily  = fontFamily;
    term.options.cursorStyle = cursorStyle;
    if (autoCompletion !== undefined) {
      autoCompletionEnabled.value = autoCompletion;
      if (!autoCompletion) suggestions.value = [];
    }
    if (enableNanoHud !== undefined) {
      termEnableNanoHud.value = enableNanoHud;
    }
    if (renderer !== undefined) {
      if (renderer === 'webgl' && !webglAddon) {
        try {
          webglAddon = new WebglAddon();
          webglAddon.onContextLoss(() => { webglAddon?.dispose(); webglAddon = null; });
          term.loadAddon(webglAddon);
        } catch (e) {
          console.warn('WebGL enable failed', e);
          webglAddon = null;
        }
      } else if (renderer === 'dom' && webglAddon) {
        webglAddon.dispose();
        webglAddon = null;
      }
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
    
    // 触发时光机会话提要 (Session Recap)
    if (props.type === 'ssh' && props.sessionObj?.host?.id) {
      invoke('get_recent_session_logs', { hostId: props.sessionObj.host.id, limit: 30 })
        .then(async (logs: any) => {
          if (!logs || logs.length === 0) {
            return; // 无历史记录时静默不显示
          }

          const commands = logs.map((l: any) => l.command).join('\n');
          const systemPrompt = "你是一个极客终端AI助手。这里是用户在过去几天对此台服务器执行的最新的历史命令记录。请用一句话中文，以极其极简冷酷的黑客口吻总结用户上一段工作周期的主线意图。绝对不要打招呼，直接输出诸如：'检测到您上个栈周期主体为重构 Nginx 代理和拉取 Docker 镜像。' 这句话，不超过 45 个字。";
          const prompt = `历史命令如下：\n\`\`\`bash\n${commands}\n\`\`\``;
          
          recapMessage.value = null;
          try {
            const summary: any = await invoke('ask_ai', { prompt, systemPrompt });
            recapMessage.value = summary;
            setTimeout(() => { recapMessage.value = null; }, 10000);
          } catch (e) {
            recapMessage.value = null;
          }
        })
        .catch(() => {});
    }

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
    { label: '🤖 AI 诊断', icon: '🤖', action: 'ai_diagnose', disabled: !hasSelection },
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
    case 'ai_diagnose':
      if (term.hasSelection()) {
        emit('ai-diagnose', term.getSelection());
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
  // Disconnect observers and listeners
  if (resizeObserver) resizeObserver.disconnect();
  if (unlisten) unlisten();
  if (unlistenSettings) unlistenSettings();
  
  // Clear any pending timers
  if (resizeTimer) clearTimeout(resizeTimer);
  if (compTimeout) clearTimeout(compTimeout);

  // Strictly dispose all active Xterm Addons first to free memory
  if (fitAddon) { fitAddon.dispose(); fitAddon = null; }
  if (searchAddon.value) { searchAddon.value.dispose(); searchAddon.value = null; }
  if (webglAddon) { webglAddon.dispose(); webglAddon = null; }

  // Clear suggestions completely
  suggestions.value = [];
  autoCompletionEnabled.value = false;

  // Finally dispose the main Terminal instance
  term?.dispose();
  term = null;
});

defineExpose({
  write: (data: string) => {
    // 拦截来自外部插件（如 Snippet 侧边栏）传入的带回车的毁灭指令
    if ((data.endsWith('\n') || data.endsWith('\r')) && !aiLockedCmd.value) {
       const cmdToCheck = data.trim();
       if (isCommandDangerous(cmdToCheck)) {
          aiLockedCmd.value = cmdToCheck;
          pendingAiPayload = data; // 将整条外部命令挂起
          generateMathCaptcha();
          nextTick(() => aiLockInputRef.value?.focus());
          return;
       }
    }
    sendDataToBackend(data);
  },
  writeln: (data: string) => {
    if (!aiLockedCmd.value && isCommandDangerous(data.trim())) {
      aiLockedCmd.value = data.trim();
      pendingAiPayload = data + '\n';
      generateMathCaptcha();
      nextTick(() => aiLockInputRef.value?.focus());
      return;
    }
    sendDataToBackend(data + '\n');
  },
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

.ghost-text {
  position: absolute;
  color: rgba(255, 255, 255, 0.4);
  pointer-events: none;
  z-index: 10;
  white-space: pre;
  display: flex;
  align-items: center;
}

.ai-lock-overlay {
  position: absolute;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(15, 0, 0, 0.85);
  backdrop-filter: blur(4px);
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
}
.lock-box {
  background: #1a0505;
  border: 1px solid #ef4444;
  border-radius: 8px;
  padding: 24px;
  width: 400px;
  box-shadow: 0 0 40px rgba(239, 68, 68, 0.2);
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.lock-header {
  display: flex;
  align-items: center;
  gap: 12px;
  color: #ef4444;
}
.lock-icon { width: 28px; height: 28px; }
.lock-header h3 { margin: 0; font-size: 18px; font-weight: 600; letter-spacing: 1px; }
.lock-desc { color: #fca5a5; font-size: 14px; margin: 0; }
.lock-cmd {
  background: #000;
  border-left: 3px solid #ef4444;
  padding: 12px;
  font-family: monospace;
  color: #f87171;
  word-break: break-all;
}
.lock-actions {
  display: flex;
  gap: 12px;
  margin-top: 8px;
}
.math-captcha {
  background: rgba(239, 68, 68, 0.15);
  color: #fca5a5;
  padding: 0 12px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  font-family: monospace;
  font-size: 16px;
  font-weight: bold;
  border: 1px solid rgba(239, 68, 68, 0.3);
  user-select: none;
  white-space: nowrap;
}
.lock-input {
  width: 80px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(239, 68, 68, 0.3);
  color: #fff;
  padding: 0 12px;
  height: 38px;
  border-radius: 4px;
  font-family: monospace;
  outline: none;
  pointer-events: auto;
}
.lock-input:focus { border-color: #ef4444; }

.lock-error-toast {
  position: absolute;
  top: -30px;
  left: 50%;
  transform: translateX(-50%) translateY(10px);
  background: #ef4444;
  color: #fff;
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 12px;
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 4px 12px rgba(239, 68, 68, 0.3);
}
.lock-error-toast::after {
  content: '';
  position: absolute;
  bottom: -4px;
  left: 50%;
  transform: translateX(-50%);
  border-left: 4px solid transparent;
  border-right: 4px solid transparent;
  border-top: 4px solid #ef4444;
}
.lock-error-toast.show {
  opacity: 1;
  transform: translateX(-50%) translateY(0);
}

.lock-btn.safe {
  background: #3b82f6;
  color: #fff;
  border: none;
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
  transition: background 0.2s;
  white-space: nowrap;
}
.lock-btn.danger {
  background: #ef4444;
  color: #fff;
  border: none;
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
  transition: background 0.2s;
  white-space: nowrap;
}
.lock-btn.danger:hover { background: #dc2626; }
.lock-btn.safe:hover { background: #2563eb; }

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

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* 全息聚光灯搜索 (Holographic Search) */
.holo-search-overlay {
  position: absolute;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(15, 23, 42, 0.4);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 20vh;
  z-index: 200;
}
.holo-search-box {
  width: 55vw;
  min-width: 450px;
  max-width: 800px;
  background: rgba(15, 23, 42, 0.85);
  border: 1px solid rgba(148, 163, 184, 0.2);
  box-shadow: 0 24px 50px rgba(0, 0, 0, 0.5), inset 0 1px 1px rgba(255,255,255,0.05);
  border-radius: 16px;
  overflow: hidden;
}
.holo-search-header {
  display: flex;
  align-items: center;
  padding: 20px 24px;
  gap: 16px;
}
.holo-icon {
  width: 28px; height: 28px;
  color: #38bdf8;
  filter: drop-shadow(0 0 8px rgba(56,189,248,0.5));
}
.holo-search-input {
  flex: 1;
  background: transparent;
  border: none;
  color: #f8fafc;
  font-size: 18px;
  font-family: 'Inter', -apple-system, sans-serif;
  outline: none;
}
.holo-search-input::placeholder {
  color: #475569;
}
.holo-spinner {
  width: 20px; height: 20px;
  border: 2px solid rgba(255,255,255,0.1);
  border-top-color: #38bdf8;
  border-radius: 50%;
  animation: spin 1s linear infinite;
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

/* ============================================================
   时光机简报浮层 — Nixu Session Recap Overlay
   ============================================================ */
.session-recap-overlay {
  position: absolute;
  top: 12px;
  left: 16px;
  z-index: 9999;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 7px 14px;
  max-width: 480px;
  background: rgba(2, 6, 23, 0.82);
  border: 1px solid rgba(56, 189, 248, 0.28);
  border-radius: 8px;
  box-shadow:
    0 0 0 1px rgba(56, 189, 248, 0.08),
    0 4px 24px rgba(0, 0, 0, 0.5),
    0 0 16px rgba(56, 189, 248, 0.08);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  pointer-events: none;
  font-family: 'JetBrains Mono', 'SFMono-Regular', Consolas, monospace;
  font-size: 12px;
  font-weight: 500;
  color: #7dd3fc;
  letter-spacing: 0.02em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.session-recap-overlay .sparkle {
  font-size: 13px;
  flex-shrink: 0;
  animation: recap-pulse 2.5s ease-in-out infinite;
  filter: drop-shadow(0 0 3px rgba(125, 211, 252, 0.8));
}

@keyframes recap-pulse {
  0%, 100% { opacity: 0.5; transform: scale(0.9); }
  50% { opacity: 1; transform: scale(1.1); }
}

.recap-fade-enter-active {
  transition: opacity 0.4s ease, transform 0.4s cubic-bezier(0.16, 1, 0.3, 1);
}
.recap-fade-leave-active {
  transition: opacity 0.4s ease, transform 0.35s ease;
}
.recap-fade-enter-from {
  opacity: 0;
  transform: translateY(-8px) scale(0.96);
}
.recap-fade-leave-to {
  opacity: 0;
  transform: translateY(-8px) scale(0.96);
}
</style>
