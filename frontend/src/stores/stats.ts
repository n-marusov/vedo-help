import { defineStore } from 'pinia';
import { ref } from 'vue';
import { api } from '../api/client';
import type { ChunkSearchParams, ChunkSearchResult, CollectionStats } from '../api/types';

export const useStatsStore = defineStore('stats', () => {
  const stats = ref<CollectionStats | null>(null);
  const chunks = ref<ChunkSearchResult[]>([]);
  const isStatsLoading = ref(false);
  const isChunksLoading = ref(false);
  const error = ref<string | null>(null);

  async function fetchStats(collectionId: string) {
    isStatsLoading.value = true;
    error.value = null;
    try {
      stats.value = await api.getCollectionStats(collectionId);
    } catch (e) {
      const message = e instanceof Error ? e.message : 'Failed to fetch stats';
      error.value = message;
      stats.value = null;
    } finally {
      isStatsLoading.value = false;
    }
  }

  async function searchChunks(collectionId: string, params: ChunkSearchParams) {
    isChunksLoading.value = true;
    error.value = null;
    try {
      const result = await api.searchChunks(collectionId, params);
      chunks.value = Array.isArray(result) ? result : [];
    } catch (e) {
      const message = e instanceof Error ? e.message : 'Failed to search chunks';
      error.value = message;
      chunks.value = [];
    } finally {
      isChunksLoading.value = false;
    }
  }

  function clearChunks() {
    chunks.value = [];
  }

  function clearStats() {
    stats.value = null;
    chunks.value = [];
    error.value = null;
  }

  return {
    stats,
    chunks,
    isStatsLoading,
    isChunksLoading,
    error,
    fetchStats,
    searchChunks,
    clearChunks,
    clearStats,
  };
});
