<template>
  <div class="session-player">
    <div class="player-toolbar">
      <button class="play-btn" @click="togglePlay" :title="isPlaying ? '暂停' : '播放'">
        <svg v-if="!isPlaying" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="16" height="16"><path fill="currentColor" stroke="none" d="M8 5v14l11-7z"/></svg>
        <svg v-else xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="16" height="16"><path fill="currentColor" stroke="none" d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>
      </button>
      <div class="time-display">{{ Math.floor(currentTime) }}s / {{ Math.floor(totalTime) }}s</div>
      <input type="range" class="timeline-slider" min="0" :max="totalTime" step="0.1" v-model="seekTime" @input="onSeekInput" @change="onSeekChange" />
      <span class="speed-control">
        倍速 
        <select v-model="playbackSpeed">
          <option :value="0.5">0.5x</option>
          <option :value="1">1.0x</option>
          <option :value="2">2.0x</option>
          <option :value="4">4.0x</option>
        </select>
      </span>
    </div>
    <div ref="terminalContainer" class="terminal-container"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { readTextFile } from '@tauri-apps/plugin-fs';
import '@xterm/xterm/css/xterm.css';

const props = defineProps<{ filePath: string }>();

const terminalContainer = ref<HTMLElement | null>(null);
let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;

const isPlaying = ref(false);
const currentTime = ref(0);
const totalTime = ref(0);
const seekTime = ref(0);
const playbackSpeed = ref(1);

interface CastEvent {
  time: number;
  type: string;
  data: string;
}

const header = ref<any>(null);
const events = ref<CastEvent[]>([]);
let lastRealTime = 0;
let animationFrame = 0;
let currentIndex = 0;

async function loadCast() {
  try {
    const text = await readTextFile(props.filePath);
    const lines = text.split('\n').filter(l => l.trim() && l.startsWith('[') || l.startsWith('{'));
    if (lines.length === 0) return;
    
    // 首行 Header
    try {
      header.value = JSON.parse(lines[0]);
    } catch(e) { console.error('Player: invalid header'); }

    const evts: CastEvent[] = [];
    for (let i = 1; i < lines.length; i++) {
      try {
        const arr = JSON.parse(lines[i]);
        if (Array.isArray(arr) && arr.length >= 3) {
          evts.push({ time: arr[0], type: arr[1], data: arr[2] });
        }
      } catch (e) {}
    }
    events.value = evts;
    if (evts.length > 0) {
      totalTime.value = evts[evts.length - 1].time;
    }
  } catch (err) {
    console.error('Failed to load recording file:', err);
  }
}

function initTerminal() {
  term = new Terminal({
    cursorBlink: false,
    theme: { background: '#000000', foreground: '#ffffff' },
    fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, monospace",
    fontSize: 14,
    cols: header.value?.width || 80,
    rows: header.value?.height || 24,
    disableStdin: true
  });
  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(terminalContainer.value!);
  fitAddon.fit();
}

function renderFrame() {
  if (!isPlaying.value) return;
  const now = performance.now();
  const delta = (now - lastRealTime) / 1000 * playbackSpeed.value;
  lastRealTime = now;
  currentTime.value += delta;
  seekTime.value = currentTime.value;
  
  // catch up events
  while (currentIndex < events.value.length && events.value[currentIndex].time <= currentTime.value) {
    const ev = events.value[currentIndex];
    if (ev.type === 'o') {
      term?.write(ev.data);
    }
    currentIndex++;
  }

  if (currentTime.value >= totalTime.value) {
    isPlaying.value = false;
    currentTime.value = totalTime.value;
    seekTime.value = currentTime.value;
    return;
  }
  
  animationFrame = requestAnimationFrame(renderFrame);
}

function togglePlay() {
  if (events.value.length === 0) return;
  if (currentTime.value >= totalTime.value) {
    currentTime.value = 0;
    seekTime.value = 0;
    currentIndex = 0;
    term?.clear();
  }
  isPlaying.value = !isPlaying.value;
  if (isPlaying.value) {
    lastRealTime = performance.now();
    animationFrame = requestAnimationFrame(renderFrame);
  } else {
    cancelAnimationFrame(animationFrame);
  }
}

function onSeekInput() {
  if (isPlaying.value) {
    isPlaying.value = false;
    cancelAnimationFrame(animationFrame);
  }
  currentTime.value = parseFloat(seekTime.value as unknown as string);
}

function onSeekChange() {
  currentTime.value = parseFloat(seekTime.value as unknown as string);
  term?.clear();
  currentIndex = 0;
  // 直接写入直到该时间点的所有数据
  let chunk = '';
  while (currentIndex < events.value.length && events.value[currentIndex].time <= currentTime.value) {
    const ev = events.value[currentIndex];
    if (ev.type === 'o') chunk += ev.data;
    currentIndex++;
  }
  term?.write(chunk);
}

onMounted(async () => {
  await loadCast();
  if (terminalContainer.value) {
    initTerminal();
    window.addEventListener('resize', () => fitAddon?.fit());
    // 自动渲染第一帧
    if (events.value.length > 0) {
      onSeekChange();
    }
  }
});

onBeforeUnmount(() => {
  term?.dispose();
  cancelAnimationFrame(animationFrame);
});
</script>

<style scoped>
.session-player {
  display: flex; flex-direction: column; width: 100%; height: 100%;
  background: var(--bg-main, #0d0d14); border-radius: 12px; overflow: hidden;
}
.player-toolbar {
  display: flex; align-items: center; gap: 12px; padding: 10px 16px;
  background: #1a1a2e; border-bottom: 2px solid #2a2a3e;
  color: #fff; font-size: 13px; flex-shrink: 0;
}
.play-btn {
  background: #6366f1; border: none; color: white; border-radius: 8px;
  width: 32px; height: 32px; display: flex; align-items: center; justify-content: center;
  cursor: pointer; transition: 0.15s; flex-shrink: 0;
}
.play-btn:hover { background: #4f46e5; }
.time-display { 
  font-family: 'JetBrains Mono', monospace; font-size: 11px; opacity: 0.8; 
  min-width: 60px; text-align: center; font-weight: 700;
}
.timeline-slider {
  flex: 1; accent-color: #6366f1; cursor: pointer; height: 4px;
  background: #2a2a3e; border-radius: 2px; outline: none; -webkit-appearance: none;
}
.timeline-slider::-webkit-slider-thumb {
  -webkit-appearance: none; appearance: none;
  width: 14px; height: 14px; background: #6366f1; border-radius: 50%;
}
.speed-control { display: flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 700; opacity: 0.9; }
.speed-control select {
  background: #111; color: white; border: 1px solid #333; border-radius: 6px; 
  padding: 4px; font-size: 11px; outline: none; font-weight: 700;
}
.terminal-container { flex: 1; width: 100%; height: calc(100% - 50px); padding: 10px; box-sizing: border-box; }
</style>
