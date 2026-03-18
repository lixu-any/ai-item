<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  name: string;
  size?: number | string;
  color?: string;
}>();

// 预先导入所有 svg 的原始字符串，避开运行时 URL 解析和 mask 兼容问题
const icons = import.meta.glob('../assets/icons/*.svg', { query: '?raw', import: 'default', eager: true });

const svgContent = computed(() => {
  const path = `../assets/icons/${props.name}.svg`;
  return (icons[path] as string) || '';
});

const style = computed(() => ({
  width: typeof props.size === 'number' ? `${props.size}px` : props.size || '1em',
  height: typeof props.size === 'number' ? `${props.size}px` : props.size || '1em',
  color: props.color || 'currentColor',
}));
</script>

<template>
  <span class="svg-icon" :style="style" v-html="svgContent"></span>
</template>

<style scoped>
.svg-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

/* 强制内部的 SVG 继承父级的宽高尺寸并自动居中 */
.svg-icon :deep(svg) {
  width: 100%;
  height: 100%;
}
</style>
