<script setup lang="ts">
import { ref, watch } from 'vue';
import SvgIcon from './SvgIcon.vue';

interface Group {
  id?: number;
  name: string;
  parent_id?: number | null;
}

const props = defineProps<{
  modelValue: Group;
  isEditing: boolean;
  saveError: string;
  isStandalone: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', val: Group): void;
  (e: 'save'): void;
  (e: 'cancel'): void;
}>();

const localGroup = ref<Group>({ ...props.modelValue });

let syncingFromProp = false;

watch(() => props.modelValue, (val) => {
  syncingFromProp = true;
  localGroup.value = { ...val };
  syncingFromProp = false;
}, { deep: true, flush: 'sync' });

watch(localGroup, (val) => {
  if (syncingFromProp) return;
  emit('update:modelValue', { ...val });
}, { deep: true, flush: 'sync' });

</script>

<template>
  <div class="modal-content" :class="{ 'standalone-window': isStandalone }">
    <div class="form-header">
      <SvgIcon :name="isEditing ? 'edit' : 'group'" size="20" class="header-icon" />
      <h3>{{ isEditing ? '编辑业务分组' : '创建新分组' }}</h3>
    </div>
    
    <div class="form-scroll-area">
      <div v-if="isStandalone" class="group-form-hint">
        分组可帮助你组织和管理多台主机，例如按环境、地域画分。
      </div>
      <div class="form-grid">
        <div class="form-group animate-in" style="--delay: 0.1s">
          <label>分组名称</label>
          <input v-model="localGroup.name" placeholder="e.g. 生产环境" @keyup.enter="$emit('save')" autofocus />
        </div>
      </div>
    </div>

    <div v-if="saveError" class="modal-error animate-in">{{ saveError }}</div>

    <div class="modal-footer">
      <button class="modal-btn secondary" @click="$emit('cancel')">取消</button>
      <button class="modal-btn primary" @click="$emit('save')">
        {{ isEditing ? '更新分组' : '立即创建' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
/* Scoped css not necessarily needed since global App.vue css applies, but added for modularity */
</style>
