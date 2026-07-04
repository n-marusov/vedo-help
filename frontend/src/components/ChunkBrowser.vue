<script setup lang="ts">
import VBadge from '@/components/ui/VBadge.vue';
import { useCollectionStore } from '@/stores/collections';
import { useStatsStore } from '@/stores/stats';
import { ref, watch } from 'vue';

const statsStore = useStatsStore();
const collectionStore = useCollectionStore();

const searchQuery = ref('');
const searchType = ref<'text' | 'semantic'>('text');
const currentPage = ref(0);
const pageSize = 20;

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function doSearch() {
  currentPage.value = 0;
  triggerSearch();
}

function triggerSearch() {
  if (!collectionStore.activeCollectionId) return;

  const params: Record<string, unknown> = {};
  if (searchQuery.value.trim()) {
    params.q = searchQuery.value.trim();
  }
  params.search_type = searchType.value;

  if (searchType.value === 'text') {
    params.limit = pageSize;
    params.offset = currentPage.value * pageSize;
  } else {
    params.top_k = pageSize;
  }

  statsStore.searchChunks(
    collectionStore.activeCollectionId,
    params as import('@/api/types').ChunkSearchParams,
  );
}

function debouncedSearch() {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    doSearch();
  }, 300);
}

function nextPage() {
  currentPage.value++;
  triggerSearch();
}

function prevPage() {
  if (currentPage.value > 0) {
    currentPage.value--;
    triggerSearch();
  }
}

function clearSearch() {
  searchQuery.value = '';
  statsStore.clearChunks();
}

// Watch for collection changes to reset
watch(
  () => collectionStore.activeCollectionId,
  () => {
    searchQuery.value = '';
    currentPage.value = 0;
    statsStore.clearChunks();
  },
);

function truncateText(text: string, max = 300): string {
  if (text.length <= max) return text;
  return `${text.slice(0, max)}...`;
}
</script>

<template>
  <div class="chunk-browser" data-testid="chunk-browser">
    <h3 class="chunk-browser-title">Chunk Browser</h3>

    <!-- Search Controls -->
    <div class="search-controls">
      <!-- Search Mode Toggle -->
      <div class="search-mode-toggle">
        <button
          class="pill-btn"
          :class="{ 'pill-btn--active': searchType === 'text' }"
          @click="searchType = 'text'"
        >
          Text Search
        </button>
        <button
          class="pill-btn"
          :class="{ 'pill-btn--active': searchType === 'semantic' }"
          @click="searchType = 'semantic'"
        >
          Semantic Search
        </button>
      </div>
    </div>

    <!-- Search Input -->
    <div class="search-input-row">
      <input
        v-model="searchQuery"
        type="text"
        class="search-input"
        placeholder="Search chunks..."
        @input="debouncedSearch"
        @keyup.enter="doSearch"
      />
      <button v-if="searchQuery" class="search-clear-btn" @click="clearSearch">
        &times;
      </button>
    </div>

    <!-- Loading -->
    <div v-if="statsStore.isChunksLoading" class="chunks-loading">
      <div v-for="i in 3" :key="i" class="skeleton-chunk" />
    </div>

    <!-- Error -->
    <div v-else-if="statsStore.error" class="chunks-error">
      {{ statsStore.error }}
    </div>

    <!-- Empty State -->
    <div v-else-if="statsStore.chunks.length === 0" class="chunks-empty">
      <template v-if="searchQuery"> No chunks found </template>
      <template v-else> Enter a search query </template>
    </div>

    <!-- Results -->
    <div v-else class="chunks-list">
      <div
        v-for="chunk in statsStore.chunks"
        :key="chunk.chunk_id"
        class="chunk-card"
      >
        <div class="chunk-header">
          <span class="chunk-doc-name">{{ chunk.document_name }}</span>
          <VBadge
            :variant="chunk.source === 'git' ? 'success' : 'info'"
            size="xs"
          >
            {{ chunk.source === "git" ? "Git" : "Upload" }}
          </VBadge>
          <span class="chunk-index">#{{ chunk.chunk_index }}</span>
          <span v-if="chunk.score !== null" class="chunk-score">
            {{ (chunk.score * 100).toFixed(1) }}%
          </span>
        </div>
        <div class="chunk-text">
          {{ truncateText(chunk.text) }}
        </div>
        <div v-if="chunk.file_path" class="chunk-footer">
          {{ chunk.file_path }}
        </div>
      </div>
    </div>

    <!-- Pagination (text search only) -->
    <div
      v-if="searchType === 'text' && statsStore.chunks.length > 0"
      class="pagination"
    >
      <button class="pag-btn" :disabled="currentPage === 0" @click="prevPage">
        Previous
      </button>
      <span class="page-info">Page {{ currentPage + 1 }}</span>
      <button
        class="pag-btn"
        :disabled="statsStore.chunks.length < pageSize"
        @click="nextPage"
      >
        Next
      </button>
    </div>
  </div>
</template>

<style scoped>
.chunk-browser {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 0;
}

.chunk-browser-title {
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  font-weight: 600;
  color: var(--color-foreground);
  margin: 0;
}

/* ── Search Controls ── */
.search-controls {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.search-mode-toggle {
  display: flex;
  gap: 4px;
}

.pill-btn {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 11px);
  font-weight: 500;
  padding: 6px 14px;
  border: 1px solid var(--color-border);
  border-radius: 20px;
  background: transparent;
  color: var(--color-muted-foreground);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.pill-btn:hover {
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.pill-btn--active {
  background: var(--color-primary);
  color: var(--color-primary-foreground, #ffffff);
  border-color: var(--color-primary);
}

/* ── Search Input ── */
.search-input-row {
  position: relative;
  display: flex;
}

.search-input {
  flex: 1;
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  padding: 10px 36px 10px 14px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg, 12px);
  background: var(--color-background, #ffffff);
  color: var(--color-foreground);
  outline: none;
  transition: border-color var(--transition-fast);
}

.search-input:focus {
  border-color: var(--color-primary);
}

.search-clear-btn {
  position: absolute;
  right: 10px;
  top: 50%;
  transform: translateY(-50%);
  background: none;
  border: none;
  font-size: 18px;
  color: var(--color-muted-foreground);
  cursor: pointer;
  line-height: 1;
}

/* ── Chunk Cards ── */
.chunks-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
  max-height: 400px;
}

.chunk-card {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg, 12px);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.chunk-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.chunk-doc-name {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 11px);
  font-weight: 600;
  color: var(--color-foreground);
}

.chunk-index {
  font-family: var(--font-family);
  font-size: 10px;
  color: var(--color-muted-foreground);
}

.chunk-score {
  font-family: var(--font-family);
  font-size: 10px;
  color: var(--color-primary);
  font-weight: 600;
}

.chunk-text {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 11px);
  color: var(--color-foreground);
  line-height: 1.5;
  word-break: break-word;
}

.chunk-footer {
  font-family: var(--font-family);
  font-size: 10px;
  color: var(--color-muted-foreground);
  font-style: italic;
}

/* ── Skeletons ── */
.chunks-loading {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.skeleton-chunk {
  height: 60px;
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

.chunks-error {
  color: var(--color-destructive, #e53935);
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
}

.chunks-empty {
  color: var(--color-muted-foreground);
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  font-style: italic;
}

/* ── Pagination ── */
.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
}

.pag-btn {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 11px);
  font-weight: 500;
  padding: 6px 14px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg, 12px);
  background: transparent;
  color: var(--color-foreground);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.pag-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.pag-btn:not(:disabled):hover {
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.page-info {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 11px);
  color: var(--color-muted-foreground);
}
</style>
