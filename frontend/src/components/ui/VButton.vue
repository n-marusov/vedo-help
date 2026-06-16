<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(
  defineProps<{
    variant?: 'primary' | 'outline' | 'ghost' | 'small' | 'destructive';
    disabled?: boolean;
  }>(),
  {
    variant: 'primary',
    disabled: false,
  },
);

const emit = defineEmits<{
  click: [e: MouseEvent];
}>();

const classes = computed(() => [
  'v-btn',
  `v-btn--${props.variant}`,
  { 'v-btn--disabled': props.disabled },
]);

function handleClick(e: MouseEvent) {
  if (!props.disabled) {
    emit('click', e);
  }
}
</script>

<template>
  <button :class="classes" :disabled="disabled" @click="handleClick">
    <slot />
  </button>
</template>

<style scoped>
.v-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
  font-weight: 600;
  line-height: 1;
  cursor: pointer;
  border: 1px solid transparent;
  border-radius: var(--radius-sm, 6px);
  padding: 8px 14px;
  white-space: nowrap;
  transition:
    background var(--transition-fast, 150ms),
    border-color var(--transition-fast, 150ms),
    color var(--transition-fast, 150ms);
  user-select: none;
}

.v-btn--disabled {
  opacity: 0.4;
  cursor: not-allowed;
  pointer-events: none;
}

/* Primary */
.v-btn--primary {
  background: var(--color-primary);
  color: var(--color-primary-foreground);
  border-color: var(--color-primary);
}
.v-btn--primary:hover:not(.v-btn--disabled) {
  background: color-mix(in srgb, var(--color-primary) 85%, white);
  border-color: color-mix(in srgb, var(--color-primary) 85%, white);
}

/* Outline */
.v-btn--outline {
  background: transparent;
  color: var(--color-muted-foreground);
  border-color: var(--color-border);
}
.v-btn--outline:hover:not(.v-btn--disabled) {
  background: var(--color-secondary);
  color: var(--color-foreground);
}

/* Ghost */
.v-btn--ghost {
  background: transparent;
  color: var(--color-muted-foreground);
  border-color: transparent;
}
.v-btn--ghost:hover:not(.v-btn--disabled) {
  background: var(--color-secondary);
  color: var(--color-foreground);
}

/* Small */
.v-btn--small {
  padding: 4px 10px;
  font-size: var(--font-size-2xs, 11px);
  border-radius: var(--radius-xs, 4px);
  background: var(--color-primary);
  color: var(--color-primary-foreground);
  border-color: var(--color-primary);
}
.v-btn--small:hover:not(.v-btn--disabled) {
  background: color-mix(in srgb, var(--color-primary) 85%, white);
}

/* Destructive */
.v-btn--destructive {
  background: var(--color-destructive);
  color: var(--color-destructive-foreground);
  border-color: var(--color-destructive);
}
.v-btn--destructive:hover:not(.v-btn--disabled) {
  background: color-mix(in srgb, var(--color-destructive) 85%, black);
  border-color: color-mix(in srgb, var(--color-destructive) 85%, black);
}
</style>
