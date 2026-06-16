<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(
  defineProps<{
    size?: 'sm' | 'xs';
    variant?: 'default' | 'success' | 'warning' | 'info';
  }>(),
  {
    size: 'sm',
    variant: 'default',
  },
);

const classes = computed(() => ['v-badge', `v-badge--${props.size}`, `v-badge--${props.variant}`]);
</script>

<template>
  <span :class="classes">
    <slot />
  </span>
</template>

<style scoped>
.v-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-family: var(--font-family);
  font-weight: 600;
  letter-spacing: 0.5px;
  white-space: nowrap;
  border-radius: var(--radius-full, 9999px);
}

/* Sizes */
.v-badge--sm {
  padding: 2px 8px;
  font-size: var(--font-size-2xs, 11px);
}

.v-badge--xs {
  padding: 1px 6px;
  font-size: var(--font-size-3xs, 10px);
}

/* Variants */
.v-badge--default {
  background: var(--color-secondary);
  color: var(--color-muted-foreground);
}

.v-badge--success {
  background: color-mix(
    in srgb,
    var(--color-success, #10b981) 20%,
    transparent
  );
  color: var(--color-success, #10b981);
}

.v-badge--warning {
  background: color-mix(
    in srgb,
    var(--color-warning, #f59e0b) 20%,
    transparent
  );
  color: var(--color-warning, #f59e0b);
}

.v-badge--info {
  background: color-mix(in srgb, var(--color-info, #3b82f6) 20%, transparent);
  color: var(--color-info, #3b82f6);
}
</style>
