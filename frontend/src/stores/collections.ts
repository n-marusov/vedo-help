import { ApiError, api } from '@/api/client';
import type { Collection, CreateCollectionRequest } from '@/api/types';
import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useCollectionStore = defineStore('collections', () => {
  const collections = ref<Collection[]>([]);
  const activeCollectionId = ref<string | null>(null);
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  async function fetchCollections() {
    isLoading.value = true;
    error.value = null;
    try {
      const result = await api.get<Collection[]>('/collections');
      collections.value = result;
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    } finally {
      isLoading.value = false;
    }
  }

  async function createCollection(req: CreateCollectionRequest) {
    error.value = null;
    try {
      const collection = await api.post<Collection>('/collections', req);
      collections.value.push(collection);
      return collection;
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
      return null;
    }
  }

  async function deleteCollection(collectionId: string) {
    error.value = null;
    try {
      await api.del(`/collections/${collectionId}`);
      collections.value = collections.value.filter((c) => c.id !== collectionId);
      if (activeCollectionId.value === collectionId) {
        activeCollectionId.value = null;
      }
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    }
  }

  function setActiveCollection(id: string | null) {
    activeCollectionId.value = id;
  }

  return {
    collections,
    activeCollectionId,
    isLoading,
    error,
    fetchCollections,
    createCollection,
    deleteCollection,
    setActiveCollection,
  };
});
