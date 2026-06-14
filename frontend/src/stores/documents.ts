import { ApiError, api } from '@/api/client';
import type { Document, UploadResponse } from '@/api/types';
import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useDocumentStore = defineStore('documents', () => {
  const documents = ref<Document[]>([]);
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  async function fetchDocuments(collectionId?: string) {
    isLoading.value = true;
    error.value = null;
    try {
      const path = collectionId ? `/documents?collection_id=${collectionId}` : '/documents';
      const result = await api.get<Document[]>(path);
      documents.value = result;
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    } finally {
      isLoading.value = false;
    }
  }

  async function uploadDocument(
    file: File,
    collectionId: string,
    onProgress?: (percent: number) => void,
  ): Promise<UploadResponse | null> {
    error.value = null;
    const formData = new FormData();
    formData.append('file', file);
    formData.append('collection_id', collectionId);

    try {
      // Use XMLHttpRequest for progress tracking
      if (onProgress) {
        return await new Promise<UploadResponse | null>((resolve) => {
          const xhr = new XMLHttpRequest();
          xhr.open('POST', '/api/documents/upload');

          const apiKey = localStorage.getItem('vedo_api_key');
          if (apiKey) {
            xhr.setRequestHeader('Authorization', `Bearer ${apiKey}`);
          }

          xhr.upload.onprogress = (event) => {
            if (event.lengthComputable) {
              onProgress(Math.round((event.loaded / event.total) * 100));
            }
          };

          xhr.onload = () => {
            if (xhr.status >= 200 && xhr.status < 300) {
              resolve(JSON.parse(xhr.responseText));
            } else {
              try {
                const body = JSON.parse(xhr.responseText);
                error.value = body?.error?.message || 'Upload failed';
              } catch {
                error.value = 'Upload failed';
              }
              resolve(null);
            }
          };

          xhr.onerror = () => {
            error.value = 'Network error during upload';
            resolve(null);
          };

          xhr.send(formData);
        });
      }

      const result = await api.upload<UploadResponse>('/documents/upload', formData);
      return result;
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
      return null;
    } finally {
      await fetchDocuments(collectionId);
    }
  }

  async function deleteDocument(documentId: string) {
    error.value = null;
    try {
      await api.del(`/documents/${documentId}`);
      documents.value = documents.value.filter((d) => d.id !== documentId);
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    }
  }

  return {
    documents,
    isLoading,
    error,
    fetchDocuments,
    uploadDocument,
    deleteDocument,
  };
});
