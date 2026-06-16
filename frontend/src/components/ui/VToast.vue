<script setup lang="ts">
import { watch } from 'vue';

const props = withDefaults(
  defineProps<{
    message: string;
    type?: 'info' | 'success' | 'error' | 'warning';
    show: boolean;
  }>(),
  {
    type: 'info',
    show: false,
  },
);

const emit = defineEmits<{
  close: [];
}>();

watch(
  () => props.show,
  (val) => {
    if (val) {
      setTimeout(() => emit('close'), 4000);
    }
  },
);
</script>

<template>
  <Transition name="v-toast">
    <div v-if="show" :class="['v-toast', `v-toast--${type}`]">
      <span class="v-toast__message">{{ message }}</span>
      <button class="v-toast__close" @click="emit('close')">×</button>
    </div>
  </Transition>
</template>

<style scoped>
.v-toast {
  position: fixed;
  bottom: 24px;
  right: 24px;
  z-index: 9999;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-radius: var(--radius-md, 8px);
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  box-shadow: var(--shadow-lg, 0 10px 40px rgba(0, 0, 0, 0.3));
  border: 1px solid var(--color-border);
  min-width: 280px;
  max-width: 420px;
}

.v-toast--info {
  background: var(--color-card);
  color: var(--color-foreground);
  border-left: 3px solid var(--color-info, #3b82f6);
}

.v-toast--success {
  background: var(--color-card);
  color: var(--color-foreground);
  border-left: 3px solid var(--color-success, #10b981);
}

.v-toast--error {
  background: var(--color-card);
  color: var(--color-foreground);
  border-left: 3px solid var(--color-destructive);
}

.v-toast--warning {
  background: var(--color-card);
  color: var(--color-foreground);
  border-left: 3px solid var(--color-warning, #f59e0b);
}

.v-toast__message {
  flex: 1;
  line-height: 1.4;
}

.v-toast__close {
  background: none;
  border: none;
  color: var(--color-muted-foreground);
  cursor: pointer;
  font-size: 16px;
  padding: 0;
  line-height: 1;
  transition: color var(--transition-fast, 150ms);
}

.v-toast__close:hover {
  color: var(--color-foreground);
}

/* Transition */
.v-toast-enter-active {
  transition: all var(--transition-normal, 200ms);
}
.v-toast-leave-active {
  transition: all var(--transition-slow, 300ms);
}
.v-toast-enter-from {
  opacity: 0;
  transform: translateY(20px);
}
.v-toast-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
