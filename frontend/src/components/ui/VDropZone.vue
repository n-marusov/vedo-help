<script setup lang="ts">
import { ref } from 'vue';

withDefaults(
  defineProps<{
    accept?: string;
    label?: string;
  }>(),
  {
    accept: '.pdf,.md,.txt,.html,.json,.zip',
    label: 'Drop files here',
  },
);

const emit = defineEmits<{
  'files-selected': [files: File[]];
}>();

const isDragOver = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

function handleDragOver(e: DragEvent) {
  e.preventDefault();
  isDragOver.value = true;
}

function handleDragLeave() {
  isDragOver.value = false;
}

function handleDrop(e: DragEvent) {
  e.preventDefault();
  isDragOver.value = false;
  if (e.dataTransfer?.files) {
    emit('files-selected', Array.from(e.dataTransfer.files));
  }
}

function handleClick() {
  fileInput.value?.click();
}

function handleInputChange(e: Event) {
  const target = e.target as HTMLInputElement;
  if (target.files) {
    emit('files-selected', Array.from(target.files));
    target.value = '';
  }
}
</script>

<template>
  <div
    class="drop-zone"
    :class="{ 'drop-zone--active': isDragOver }"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
    @click="handleClick"
    role="button"
    :tabindex="0"
    @keydown.enter="handleClick"
    @keydown.space.prevent="handleClick"
  >
    <input
      ref="fileInput"
      type="file"
      :accept="accept"
      multiple
      class="drop-zone__input"
      @change="handleInputChange"
    />
    <span class="drop-zone__label">{{ label }}</span>
  </div>
</template>

<style scoped>
.drop-zone {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 128px;
  border: 2px dashed var(--color-border);
  border-radius: var(--radius-lg, 12px);
  background: var(--color-secondary);
  cursor: pointer;
  transition: all var(--transition-fast, 150ms);
  user-select: none;
}

.drop-zone:hover,
.drop-zone--active {
  border-color: var(--color-primary);
  background: var(--color-accent);
}

.drop-zone__input {
  display: none;
}

.drop-zone__label {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
  color: var(--color-muted-foreground);
  pointer-events: none;
}

.drop-zone--active .drop-zone__label {
  color: var(--color-primary);
}
</style>
