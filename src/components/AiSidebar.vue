<script setup lang="ts">
import { ref, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import SvgIcon from './SvgIcon.vue';

const emit = defineEmits(['close', 'run-command', 'toast']);

const props = defineProps<{
  context?: { os: string; shell: string; pwd?: string; }
}>();

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  isCommand?: boolean;
}

const query = ref('');
const messages = ref<ChatMessage[]>([]);
const isWaiting = ref(false);
const messagesContainer = ref<HTMLElement | null>(null);

function extractCommand(text: string): string | null {
  const match = text.match(/```(?:bash|sh|shell)?\s*([\s\S]*?)```/);
  return match?.[1]?.trim() || null;
}

async function sendQuery() {
  const currentQuery = query.value.trim();
  if (!currentQuery) return;

  messages.value.push({ id: Date.now().toString(), role: 'user', content: currentQuery });
  query.value = '';
  isWaiting.value = true;
  await scrollToBottom();

  const osInfo = props.context?.os || 'Linux';
  const shellInfo = props.context?.shell || 'bash';
  const systemPrompt = `你是一个资深的 Linux 系统管理员。\n当前环境: ${osInfo}, Shell: ${shellInfo}。\n请直接返回最适合的命令，无需过多解释。使用简体中文进行简短说明。如果提供命令，必须放在 \`\`\`bash 和 \`\`\` 之间。`;

  try {
    const response = await invoke<string>('ask_ai', { prompt: currentQuery, systemPrompt });
    const command = extractCommand(response);
    messages.value.push({ id: Date.now().toString(), role: 'assistant', content: response, isCommand: !!command });
  } catch (err: any) {
    messages.value.push({ id: Date.now().toString(), role: 'assistant', content: `⚠️ 请求失败: ${err}\n请检查设置中的 API Key 配置。` });
    emit('toast', { message: 'AI 请求失败', type: 'error' });
  } finally {
    isWaiting.value = false;
    await scrollToBottom();
  }
}

function runAiCommand(content: string) {
  const cmd = extractCommand(content);
  if (cmd) emit('run-command', cmd);
}

async function copyToClipboard(content: string) {
  const cmd = extractCommand(content) || content;
  try {
    await navigator.clipboard.writeText(cmd);
    emit('toast', { message: '已复制到剪贴板', type: 'success' });
  } catch {}
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    emit('toast', { message: '代码已复制', type: 'success' });
  } catch {}
}

async function scrollToBottom() {
  await nextTick();
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    sendQuery();
  }
}

// 将消息内容解析为文本段落 + 代码块片段
interface Segment {
  type: 'text' | 'code';
  content: string;
  lang?: string;
}

function diagnoseError(errorText: string) {
  if (!errorText.trim()) return;
  const diagnosePrompt = `详细分析下列终端报错日志，查出根本原因，并在回复中直接提供可以一键粘贴执行的修复脚本代码（必须使用 \`\`\`bash 包裹）。如果可能，尽量给出副作用最小的修补方案：\n\n${errorText}`;
  messages.value.push({ id: Date.now().toString(), role: 'user', content: `[AI 一键诊断报告]\n${errorText.length > 200 ? errorText.substring(0, 200) + '...' : errorText}` });
  
  isWaiting.value = true;
  scrollToBottom();

  const osInfo = props.context?.os || 'Linux';
  const shellInfo = props.context?.shell || 'bash';
  const systemPrompt = `你是一个终极 Linux 专家架构师。\n当前发生故障的环境: ${osInfo}, Shell: ${shellInfo}。\n请直接返回最适合的修复命令或者脚本，无需过多解释。提供的命令必须严格包裹在 \`\`\`bash 和 \`\`\` 之间。`;

  invoke<string>('ask_ai', { prompt: diagnosePrompt, systemPrompt }).then(response => {
    const command = extractCommand(response);
    messages.value.push({ id: Date.now().toString(), role: 'assistant', content: response, isCommand: !!command });
  }).catch((err: any) => {
    messages.value.push({ id: Date.now().toString(), role: 'assistant', content: `⚠️ AI 诊断调用出现故障: ${err}` });
    emit('toast', { message: 'AI 诊断失败', type: 'error' });
  }).finally(() => {
    isWaiting.value = false;
    scrollToBottom();
  });
}

defineExpose({
  diagnoseError
});

function parseContent(text: string): Segment[] {
  const segments: Segment[] = [];
  const regex = /```(\w*)\n?([\s\S]*?)```/g;
  let lastIndex = 0;
  let match;
  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      const before = text.slice(lastIndex, match.index).trim();
      if (before) segments.push({ type: 'text', content: before });
    }
    segments.push({ type: 'code', lang: match[1] || 'bash', content: match[2].trim() });
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < text.length) {
    const after = text.slice(lastIndex).trim();
    if (after) segments.push({ type: 'text', content: after });
  }
  return segments;
}
</script>

<template>
  <div class="ai-sidebar">
    <!-- Header -->
    <div class="ai-header">
      <div class="header-title">
        <SvgIcon name="ai" size="16" />
        <span>AI 助手</span>
      </div>
      <button class="close-btn" @click="emit('close')" title="关闭" style="padding: 0;">
        <SvgIcon name="close" size="14" />
      </button>
    </div>

    <!-- Messages -->
    <div ref="messagesContainer" class="messages-area">
      <div v-if="messages.length === 0" class="empty-state">
        <div class="empty-icon-wrap">
          <SvgIcon name="ai" size="32" />
        </div>
        <p class="empty-title">有什么我可以帮你的吗？</p>
        <p class="empty-hint">向 AI 发问，或点击下方示例快速开始：</p>
        <div class="suggestion-chips">
          <span class="chip" @click="query = '列出当前目录大于100MB的文件'">📄 查找大文件</span>
          <span class="chip" @click="query = '显示CPU和内存使用率'">📊 系统状态</span>
          <span class="chip" @click="query = '查看最近的系统日志错误'">🔍 查看日志</span>
          <span class="chip" @click="query = '列出所有运行中的进程'">⚙️ 进程列表</span>
        </div>
      </div>

      <div v-for="msg in messages" :key="msg.id" class="msg-row" :class="msg.role">
        <div class="bubble" :class="msg.role">
          <!-- 解析渲染带代码块的内容 -->
          <template v-for="(seg, i) in parseContent(msg.content)" :key="i">
            <div v-if="seg.type === 'text'" class="bubble-text">{{ seg.content }}</div>
            <div v-else class="code-block">
              <div class="code-header">
                <span class="code-lang">{{ seg.lang }}</span>
                <button class="code-copy-btn" @click="copyText(seg.content)">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" width="12" height="12"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>
                  复制
                </button>
              </div>
              <pre class="code-body"><code>{{ seg.content }}</code></pre>
            </div>
          </template>
          <div v-if="msg.role === 'assistant' && msg.isCommand" class="cmd-actions">
            <button class="action-btn" @click="copyToClipboard(msg.content)">
              <SvgIcon name="copy" size="12" /> 复制
            </button>
            <button class="action-btn run" @click="runAiCommand(msg.content)">
              <SvgIcon name="play" size="12" /> 插入终端
            </button>
          </div>
        </div>
      </div>

      <div v-if="isWaiting" class="msg-row assistant">
        <div class="bubble assistant waiting">
          <span class="dot"></span>
          <span class="dot" style="animation-delay: 0.15s;"></span>
          <span class="dot" style="animation-delay: 0.3s;"></span>
        </div>
      </div>
    </div>

    <!-- Input -->
    <div class="input-area">
      <div class="input-wrap">
        <textarea
          v-model="query"
          @keydown="handleKeydown"
          placeholder="给 AI 发送指令 (Enter 发送, Shift+Enter 换行)..."
          rows="2"
          class="query-input"
        ></textarea>
        <button class="send-btn" @click="sendQuery" :disabled="isWaiting || !query.trim()">
          {{ isWaiting ? '...' : '发送' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ai-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-sidebar, #f8f9fa);
  border-left: 1px solid var(--border-color, #e2e8f0);
  overflow: hidden;
  width: 320px;
  
  /* Detach from normal flow to prevent DOM resize jank */
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  z-index: 50;
  box-shadow: -4px 0 24px rgba(0,0,0,0.08);
}

/* Header */
.ai-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color, #e2e8f0);
  flex-shrink: 0;
}
.header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-main, #1e293b);
}
.close-btn {
  width: 26px; height: 26px;
  border: none; background: transparent;
  -webkit-appearance: none;
  appearance: none;
  color: var(--text-dim, #94a3b8);
  cursor: pointer;
  border-radius: 6px;
  display: flex; align-items: center; justify-content: center;
  transition: background 0.15s, color 0.15s;
}
.close-btn:hover { background: var(--bg-hover, #f1f5f9); color: var(--text-main, #1e293b); }

/* Messages */
.messages-area {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.messages-area::-webkit-scrollbar { width: 5px; }
.messages-area::-webkit-scrollbar-track { background: transparent; }
.messages-area::-webkit-scrollbar-thumb { background: var(--border-color, #e2e8f0); border-radius: 3px; }

/* Empty state */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  text-align: center;
  gap: 8px;
  color: var(--text-dim, #94a3b8);
  padding: 0 16px;
}
.empty-icon-wrap {
  width: 60px; height: 60px;
  border-radius: 18px;
  background: linear-gradient(135deg, rgba(57,108,216,0.08), rgba(139,92,246,0.08));
  border: 1px solid rgba(57,108,216,0.12);
  display: flex; align-items: center; justify-content: center;
  color: #3b82f6;
  margin-bottom: 4px;
}
.empty-icon { opacity: 0.4; }
.empty-title { font-size: 13px; font-weight: 600; color: #374151; }
.empty-hint { font-size: 11.5px; line-height: 1.5; max-width: 90%; }

.suggestion-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: center;
  margin-top: 4px;
}
.chip {
  font-size: 11px;
  padding: 5px 10px;
  border-radius: 20px;
  background: rgba(57,108,216,0.06);
  border: 1px solid rgba(57,108,216,0.15);
  color: #3b82f6;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}
.chip:hover {
  background: rgba(57,108,216,0.12);
  border-color: rgba(57,108,216,0.3);
  transform: translateY(-1px);
}

/* Messages */
.msg-row {
  display: flex;
}
.msg-row.user { justify-content: flex-end; }
.msg-row.assistant { justify-content: flex-start; }

.bubble {
  max-width: 85%;
  padding: 9px 12px;
  border-radius: 10px;
  font-size: 12.5px;
  line-height: 1.5;
  border: 1px solid transparent;
}
.bubble.user {
  background: var(--accent-bg, #eff6ff);
  border-color: var(--accent-border, #bfdbfe);
  color: var(--accent-text, #1d4ed8);
}
.bubble.assistant {
  background: var(--surface-color, #fff);
  border-color: var(--border-color, #e2e8f0);
  color: var(--text-main, #1e293b);
}
.bubble-text { white-space: pre-wrap; word-break: break-word; }

/* Code block */
.code-block {
  margin: 6px 0;
  border-radius: 7px;
  overflow: hidden;
  border: 1px solid #e2e8f0;
  font-size: 12px;
}
.code-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 10px;
  background: #f1f5f9;
  border-bottom: 1px solid #e2e8f0;
}
.code-lang {
  font-size: 10.5px;
  color: #64748b;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.code-copy-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  color: #64748b;
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  font-family: inherit;
  transition: color 0.15s;
}
.code-copy-btn:hover { color: #1e293b; }
.code-body {
  margin: 0;
  padding: 10px 12px;
  background: #f8fafc;
  color: #0f4c81;
  font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
  font-size: 12px;
  line-height: 1.6;
  overflow-x: auto;
  white-space: pre;
}
.code-body code { font-family: inherit; }

/* Command actions */
.cmd-actions {
  display: flex;
  gap: 6px;
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border-color, #e2e8f0);
}
.action-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  padding: 3px 9px;
  border-radius: 5px;
  border: 1px solid var(--border-color, #e2e8f0);
  background: var(--bg-main, #f5f7fa);
  color: var(--text-main, #374151);
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
}
.action-btn:hover { border-color: #93c5fd; color: #1d4ed8; }
.action-btn.run {
  background: #dcfce7;
  border-color: #86efac;
  color: #166534;
}
.action-btn.run:hover { background: #bbf7d0; }

/* Waiting dots */
.bubble.waiting {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 10px 14px;
}
.dot {
  display: inline-block;
  width: 6px; height: 6px;
  border-radius: 50%;
  background: var(--accent-color, #3b82f6);
  animation: bounce 0.7s ease-in-out infinite;
}
@keyframes bounce {
  0%, 100% { transform: translateY(0); opacity: 0.4; }
  50% { transform: translateY(-4px); opacity: 1; }
}

/* Input area */
.input-area {
  flex-shrink: 0;
  padding: 10px 12px;
  border-top: 1px solid var(--border-color, #e2e8f0);
  background: var(--surface-color, #fff);
}
.input-wrap {
  display: flex;
  gap: 8px;
  align-items: flex-end;
}
.query-input {
  flex: 1;
  padding: 8px 12px;
  background: var(--bg-main, #f5f7fa);
  border: 1.5px solid var(--border-color, #e2e8f0);
  border-radius: 8px;
  color: var(--text-main, #1e293b);
  font-size: 12.5px;
  font-family: inherit;
  resize: none;
  outline: none;
  transition: border-color 0.15s;
  line-height: 1.5;
}
.query-input::placeholder { color: var(--text-dim, #94a3b8); }
.query-input:focus { border-color: var(--accent-color, #3b82f6); }

.send-btn {
  flex-shrink: 0;
  padding: 0 14px;
  height: 34px;
  border-radius: 8px;
  border: none;
  background: var(--accent-color, #3b82f6);
  color: white;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  font-family: inherit;
  white-space: nowrap;
  transition: all 0.15s;
}
.send-btn:hover:not(:disabled) { background: #2563eb; }
.send-btn:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
