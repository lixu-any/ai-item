<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  name: string;
  size?: number | string;
  color?: string;
}>();

const iconUrl = computed(() => {
  return new URL(`../assets/icons/${props.name}.svg`, import.meta.url).href;
});

const iconStyle = computed(() => ({
  width: typeof props.size === 'number' ? `${props.size}px` : props.size || '1em',
  height: typeof props.size === 'number' ? `${props.size}px` : props.size || '1em',
  maskImage: `url(${iconUrl.value})`,
  WebkitMaskImage: `url(${iconUrl.value})`,
  maskRepeat: 'no-repeat',
  WebkitMaskRepeat: 'no-repeat',
  maskPosition: 'center',
  WebkitMaskPosition: 'center',
  maskSize: 'contain',
  WebkitMaskSize: 'contain',
  backgroundColor: props.color || 'currentColor',
  display: 'inline-block'
}));
</script>

<template>
  <div class="svg-icon" :style="iconStyle"></div>
</template>

<style scoped>
.svg-icon {
  flex-shrink: 0;
}
</style>
