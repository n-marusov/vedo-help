<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(
  defineProps<{
    variant?: 'text' | 'circle' | 'card';
    rows?: number;
  }>(),
  {
    variant: 'text',
    rows: 1,
  },
);

const rowsArray = computed(() => Array.from({ length: props.rows }));
</script>

<template>
  <div
    class="skeleton"
    :class="[`skeleton-${variant}`]"
    :data-testid="'skeleton'"
  >
    <!-- Text variant: stacked lines -->
    <template v-if="variant === 'text'">
      <div
        v-for="(_, i) in rowsArray"
        :key="i"
        class="skeleton-line"
        :style="{
          width: i === rows - 1 ? '60%' : '100%',
        }"
      />
    </template>

    <!-- Circle variant -->
    <template v-else-if="variant === 'circle'">
      <div class="skeleton-circle-shape" />
    </template>

    <!-- Card variant: a single block per row -->
    <template v-else-if="variant === 'card'">
      <div v-for="(_, i) in rowsArray" :key="i" class="skeleton-card-row" />
    </template>
  </div>
</template>

<style scoped>
.skeleton {
  display: flex;
  flex-direction: column;
  gap: var(--space-2, 8px);
}

/* ─── Text lines ─── */
.skeleton-text {
  gap: var(--space-1, 4px);
}

.skeleton-line {
  height: 12px;
  border-radius: var(--radius-sm, 6px);
  background: var(--color-muted);
  animation: skeletonShimmer 1.6s ease-in-out infinite;
}

/* ─── Circle ─── */
.skeleton-circle {
  align-items: center;
}

.skeleton-circle-shape {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-full, 9999px);
  background: var(--color-muted);
  animation: skeletonShimmer 1.6s ease-in-out infinite;
}

/* ─── Card rows ─── */
.skeleton-card-row {
  height: 48px;
  border-radius: var(--radius-md, 8px);
  background: var(--color-muted);
  animation: skeletonShimmer 1.6s ease-in-out infinite;
}

/* ─── Shimmer animation ─── */
@keyframes skeletonShimmer {
  0% {
    opacity: 0.6;
  }
  50% {
    opacity: 1;
  }
  100% {
    opacity: 0.6;
  }
}
</style>
