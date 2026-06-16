<script setup lang="ts">
defineProps<{
  value: number;
  variant?: string;
}>();
</script>

<template>
  <div class="v-progress-bar">
    <div
      class="v-progress-bar__fill"
      :style="{ width: `${Math.min(100, Math.max(0, value))}%` }"
      :class="variant ? `v-progress-bar__fill--${variant}` : ''"
    />
  </div>
</template>

<style scoped>
.v-progress-bar {
  width: 100%;
  height: 4px;
  background: var(--color-border);
  border-radius: 2px;
  overflow: hidden;
}

.v-progress-bar__fill {
  height: 100%;
  background: var(--color-primary);
  border-radius: 2px;
  transition: width var(--transition-normal, 200ms);
}

.v-progress-bar__fill--success {
  background: var(--color-success, #10b981);
}

.v-progress-bar__fill--warning {
  background: var(--color-warning, #f59e0b);
}

.v-progress-bar__fill--error {
  background: var(--color-destructive);
}
</style>
