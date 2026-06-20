import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { ApiError } = vi.hoisted(() => {
  class ApiError extends Error {
    status: number;
    constructor(status: number, message: string) {
      super(message);
      this.status = status;
    }
  }
  return { ApiError };
});

const apiMock = vi.hoisted(() => ({
  batchDeleteDocuments: vi.fn(),
}));

vi.mock('@/api/client', () => ({
  api: apiMock,
  ApiError,
}));

import { useDocumentStore } from '@/stores/documents';

describe('documents store — deleteDocumentsBatch', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it('optimistically removes documents from list on success', async () => {
    const store = useDocumentStore();
    store.documents = [
      {
        id: 'doc-1',
        name: 'one.md',
        file_type: 'text/markdown',
        file_size: 100,
        uploaded_at: '2026-01-01T00:00:00Z',
        collection_id: 'col-1',
      },
      {
        id: 'doc-2',
        name: 'two.md',
        file_type: 'text/markdown',
        file_size: 200,
        uploaded_at: '2026-01-01T00:00:00Z',
        collection_id: 'col-1',
      },
      {
        id: 'doc-3',
        name: 'three.md',
        file_type: 'text/markdown',
        file_size: 300,
        uploaded_at: '2026-01-01T00:00:00Z',
        collection_id: 'col-1',
      },
    ];

    apiMock.batchDeleteDocuments.mockResolvedValue({
      deleted_count: 2,
      ids: ['doc-1', 'doc-2'],
    });

    const result = await store.deleteDocumentsBatch(['doc-1', 'doc-2']);

    expect(store.documents.length).toBe(1);
    expect(store.documents[0].id).toBe('doc-3');
    expect(apiMock.batchDeleteDocuments).toHaveBeenCalledWith(['doc-1', 'doc-2']);
    expect(result).toEqual({ deleted_count: 2, ids: ['doc-1', 'doc-2'] });
  });

  it('rolls back on failure', async () => {
    const store = useDocumentStore();
    store.documents = [
      {
        id: 'doc-1',
        name: 'one.md',
        file_type: 'text/markdown',
        file_size: 100,
        uploaded_at: '2026-01-01T00:00:00Z',
        collection_id: 'col-1',
      },
      {
        id: 'doc-2',
        name: 'two.md',
        file_type: 'text/markdown',
        file_size: 200,
        uploaded_at: '2026-01-01T00:00:00Z',
        collection_id: 'col-1',
      },
    ];

    apiMock.batchDeleteDocuments.mockRejectedValue(new ApiError(500, 'API Error'));

    const result = await store.deleteDocumentsBatch(['doc-1']);

    expect(store.documents.length).toBe(2);
    expect(result).toBeNull();
    expect(store.error).toBe('API Error');
  });

  it('prevents double submission while deleting', async () => {
    const store = useDocumentStore();
    store.documents = [
      {
        id: 'doc-1',
        name: 'one.md',
        file_type: 'text/markdown',
        file_size: 100,
        uploaded_at: '2026-01-01T00:00:00Z',
        collection_id: 'col-1',
      },
    ];

    apiMock.batchDeleteDocuments.mockReturnValue(new Promise<never>(() => {}));

    store.deleteDocumentsBatch(['doc-1']);

    expect(store.isDeleting).toBe(true);
  });
});
