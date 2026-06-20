import { ApiError, api, getAccessToken } from '@/api/client';
import type { BatchDeleteResponse, Document, UploadResponse, ZipUploadResponse } from '@/api/types';
import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useDocumentStore = defineStore('documents', () => {
  const documents = ref<Document[]>([]);
  const isLoading = ref(false);
  const isDeleting = ref(false);
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

          const token = getAccessToken();
          if (token) {
            xhr.setRequestHeader('Authorization', `Bearer ${token}`);
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

  async function deleteDocument(documentId: string): Promise<boolean> {
    error.value = null;
    try {
      await api.del(`/documents/${documentId}`);
      // Refresh the full list from server after successful deletion
      const collectionId = documents.value.find((d) => d.id === documentId)?.collection_id;
      if (collectionId) {
        await fetchDocuments(collectionId);
      } else {
        documents.value = documents.value.filter((d) => d.id !== documentId);
      }
      return true;
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
      return false;
    }
  }

  async function deleteDocumentsBatch(ids: string[]): Promise<BatchDeleteResponse | null> {
    error.value = null;
    if (isDeleting.value) return null;
    isDeleting.value = true;

    // Snapshot for rollback
    const snapshot = [...documents.value];

    // Optimistic removal
    documents.value = documents.value.filter((d) => !ids.includes(d.id));

    try {
      return await api.batchDeleteDocuments(ids);
    } catch (err) {
      // Rollback: restore documents
      documents.value = snapshot;
      if (err instanceof ApiError) {
        error.value = err.message;
      } else {
        error.value = 'Failed to delete documents';
      }
      return null;
    } finally {
      isDeleting.value = false;
    }
  }

  async function uploadZip(
    file: File,
    collectionId: string,
    onProgress?: (percent: number) => void,
  ): Promise<ZipUploadResponse | null> {
    error.value = null;
    const formData = new FormData();
    formData.append('file', file);
    formData.append('collection_id', collectionId);

    try {
      return await new Promise<ZipUploadResponse | null>((resolve) => {
        const xhr = new XMLHttpRequest();
        xhr.open('POST', '/api/documents/upload-zip');

        const token = getAccessToken();
        if (token) {
          xhr.setRequestHeader('Authorization', `Bearer ${token}`);
        }

        xhr.upload.onprogress = (event) => {
          if (event.lengthComputable) {
            onProgress?.(Math.round((event.loaded / event.total) * 100));
          }
        };

        xhr.onload = () => {
          if (xhr.status >= 200 && xhr.status < 300) {
            resolve(JSON.parse(xhr.responseText));
          } else if (xhr.status === 413) {
            error.value =
              'ZIP содержит более 10 файлов. Пожалуйста, уменьшите количество файлов в архиве.';
            resolve(null);
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
    } finally {
      await fetchDocuments(collectionId);
    }
  }

  return {
    documents,
    isLoading,
    error,
    fetchDocuments,
    uploadDocument,
    uploadZip,
    deleteDocument,
    deleteDocumentsBatch,
    isDeleting,
  };
});
