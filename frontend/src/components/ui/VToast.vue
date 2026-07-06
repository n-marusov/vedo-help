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
      setTimeout(() => emit('close'), 3500);
    }
  },
);

// ── Icon per variant ──
const iconMap: Record<string, string> = {
  success: 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z',
  error: 'M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z',
  info: 'M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z',
  warning:
    'M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z',
};
</script>

<template>
  <Transition name="v-toast">
    <div v-if="show" :class="['v-toast', `v-toast--${type}`]">
      <svg
        class="v-toast__icon"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        stroke-width="2"
        aria-hidden="true"
      >
        <path
          :d="iconMap[type]"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
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
  gap: 8px;
  padding: 8px 12px;
  border-radius: var(--radius-md, 8px);
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
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

.v-toast__icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.v-toast--success .v-toast__icon {
  color: var(--color-success, #10b981);
}

.v-toast--error .v-toast__icon {
  color: var(--color-destructive);
}

.v-toast--info .v-toast__icon {
  color: var(--color-info, #3b82f6);
}

.v-toast--warning .v-toast__icon {
  color: var(--color-warning, #f59e0b);
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
  font-size: 12px;
  padding: 0;
  line-height: 1;
  width: 14px;
  height: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
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
