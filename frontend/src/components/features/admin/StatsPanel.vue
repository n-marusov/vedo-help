<script setup lang="ts">
import VBadge from '@/components/ui/VBadge.vue';
import { useCollectionStore } from '@/stores/collections';
import { useStatsStore } from '@/stores/stats';
import { watch } from 'vue';

const statsStore = useStatsStore();
const collectionStore = useCollectionStore();

watch(
  () => collectionStore.activeCollectionId,
  (newId) => {
    if (newId) {
      statsStore.fetchStats(newId);
    } else {
      statsStore.clearStats();
    }
  },
  { immediate: true },
);

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / 1024 ** i).toFixed(1)} ${units[i]}`;
}
</script>

<template>
  <div class="stats-panel" data-testid="stats-panel">
    <h3 class="stats-title">Collection Statistics</h3>

    <!-- Loading -->
    <div v-if="statsStore.isStatsLoading" class="stats-loading">
      <div v-for="i in 4" :key="i" class="skeleton-card" />
    </div>

    <!-- Error -->
    <div v-else-if="statsStore.error" class="stats-error">
      {{ statsStore.error }}
    </div>

    <!-- Empty / No Selection -->
    <div v-else-if="!collectionStore.activeCollectionId" class="stats-empty">
      Select a collection
    </div>

    <!-- Stats Cards -->
    <div v-else-if="statsStore.stats" class="stats-grid">
      <div class="stat-card">
        <span class="stat-label">Total Documents</span>
        <span class="stat-value">{{ statsStore.stats.total_documents }}</span>
      </div>
      <div class="stat-card">
        <span class="stat-label">Total Chunks</span>
        <span class="stat-value">{{ statsStore.stats.total_chunks }}</span>
      </div>
      <div class="stat-card">
        <span class="stat-label">Git Repos</span>
        <span class="stat-value">{{ statsStore.stats.total_git_repos }}</span>
      </div>
      <div class="stat-card">
        <span class="stat-label">Total Size</span>
        <span class="stat-value">{{
          formatBytes(statsStore.stats.total_file_size_bytes)
        }}</span>
      </div>

      <!-- Source Breakdown -->
      <div class="stat-card stat-card--breakdown">
        <span class="stat-label">Documents by Source</span>
        <div class="stat-breakdown">
          <div class="breakdown-row">
            <VBadge variant="info">Upload</VBadge>
            <span>{{ statsStore.stats.upload_documents }}</span>
          </div>
          <div class="breakdown-row">
            <VBadge variant="success">Git</VBadge>
            <span>{{ statsStore.stats.git_documents }}</span>
          </div>
        </div>
      </div>

      <div class="stat-card stat-card--breakdown">
        <span class="stat-label">Chunks by Source</span>
        <div class="stat-breakdown">
          <div class="breakdown-row">
            <VBadge variant="info">Upload</VBadge>
            <span>{{ statsStore.stats.upload_chunks }}</span>
          </div>
          <div class="breakdown-row">
            <VBadge variant="success">Git</VBadge>
            <span>{{ statsStore.stats.git_chunks }}</span>
          </div>
        </div>
      </div>

      <!-- File Types -->
      <div class="stat-card stat-card--wide">
        <span class="stat-label">File Types</span>
        <div class="stat-breakdown">
          <div
            v-for="(count, type) in statsStore.stats.document_types"
            :key="type"
            class="breakdown-row"
          >
            <VBadge>{{ type }}</VBadge>
            <span>{{ count }}</span>
          </div>
          <div
            v-if="Object.keys(statsStore.stats.document_types).length === 0"
            class="breakdown-row"
          >
            <span class="stat-muted">No files</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.stats-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 0;
}

.stats-title {
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  font-weight: 600;
  color: var(--color-foreground);
  margin: 0;
}

.stats-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.stat-card {
  background: var(--color-card, #ffffff);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg, 12px);
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.stat-card--breakdown,
.stat-card--wide {
  grid-column: 1 / -1;
}

.stat-label {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 11px);
  font-weight: 500;
  color: var(--color-muted-foreground);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.stat-value {
  font-family: var(--font-family);
  font-size: 24px;
  font-weight: 700;
  color: var(--color-foreground);
}

.stat-breakdown {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.breakdown-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  color: var(--color-foreground);
}

.stat-muted {
  color: var(--color-muted-foreground);
  font-style: italic;
}

/* Skeleton */
.stats-loading {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.skeleton-card {
  height: 80px;
  background: var(--color-border, #e0e0e0);
  border-radius: var(--radius-lg, 12px);
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.stats-error {
  color: var(--color-destructive, #e53935);
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
}

.stats-empty {
  color: var(--color-muted-foreground);
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  font-style: italic;
}
</style>
