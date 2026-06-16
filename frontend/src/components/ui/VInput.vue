<script setup lang="ts">
defineProps<{
  modelValue: string;
  placeholder?: string;
  disabled?: boolean;
  type?: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
  keydown: [e: KeyboardEvent];
}>();

function handleInput(e: Event) {
  const target = e.target as HTMLInputElement;
  emit('update:modelValue', target.value);
}
</script>

<template>
  <input
    :value="modelValue"
    :placeholder="placeholder"
    :disabled="disabled"
    :type="type || 'text'"
    class="v-input"
    @input="handleInput"
    @keydown="(e: KeyboardEvent) => emit('keydown', e)"
  />
</template>

<style scoped>
.v-input {
  width: 100%;
  background: var(--color-secondary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md, 8px);
  padding: 10px 12px;
  color: var(--color-foreground);
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  outline: none;
  transition: border-color var(--transition-fast, 150ms);
  box-sizing: border-box;
}

.v-input::placeholder {
  color: var(--color-muted-foreground);
  opacity: 0.6;
}

.v-input:focus {
  border-color: var(--color-primary);
}

.v-input:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
