<script setup lang="ts">
defineProps<{
  modelValue: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
  change: [];
}>();

function handleInput(e: Event) {
  const target = e.target as HTMLInputElement;
  emit('update:modelValue', target.value);
}

function handleChange() {
  emit('change');
}
</script>

<template>
  <input
    :value="modelValue"
    :disabled="disabled"
    type="date"
    class="v-date-picker"
    @input="handleInput"
    @change="handleChange"
  />
</template>

<style scoped>
.v-date-picker {
  background: transparent;
  border: none;
  color: var(--color-foreground);
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
  outline: none;
  cursor: pointer;
  width: 90px;
  color-scheme: dark;
}

.v-date-picker::-webkit-calendar-picker-indicator {
  filter: invert(0.7);
  cursor: pointer;
  padding: 0;
  margin: 0;
}

.v-date-picker:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
