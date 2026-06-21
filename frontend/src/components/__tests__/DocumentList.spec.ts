import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';

const { mockBatchDelete } = vi.hoisted(() => ({
  mockBatchDelete: vi.fn(),
}));

vi.mock('@/api/client', () => ({
  api: {
    get: vi.fn(),
    batchDeleteDocuments: mockBatchDelete,
    del: vi.fn(),
    upload: vi.fn(),
  },
  getAccessToken: () => null,
}));

import { useCollectionStore } from '@/stores/collections';
import DocumentList from '../DocumentList.vue';

const fixture = [
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

// biome-ignore lint/suspicious/noExplicitAny: test mock type inference limitation
function mockGet(apiModule: any) {
  apiModule.api.get.mockResolvedValue(fixture);
}

describe('DocumentList - bulk delete', () => {
  it('renders checkbox for each document', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const collectionStore = useCollectionStore();
    collectionStore.collections = [
      {
        id: 'col-1',
        name: 'Test',
        created_at: '2026-01-01T00:00:00Z',
        document_count: 3,
      },
    ];
    collectionStore.setActiveCollection('col-1');

    const apiModule = await import('@/api/client');
    mockGet(apiModule);

    const wrapper = mount(DocumentList, { global: { plugins: [pinia] } });
    await flushPromises();
    await nextTick();

    expect(wrapper.findAll('.dl-item input[type="checkbox"]')).toHaveLength(3);
  });

  it('toggle all checkbox selects/deselects all documents', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const collectionStore = useCollectionStore();
    collectionStore.collections = [
      {
        id: 'col-1',
        name: 'Test',
        created_at: '2026-01-01T00:00:00Z',
        document_count: 3,
      },
    ];
    collectionStore.setActiveCollection('col-1');

    const apiModule = await import('@/api/client');
    mockGet(apiModule);

    const wrapper = mount(DocumentList, { global: { plugins: [pinia] } });
    await flushPromises();
    await nextTick();

    const toggle = wrapper.find('.dl-toggle-all input[type="checkbox"]');
    expect(toggle.exists()).toBe(true);

    await toggle.setValue(true);
    for (const cb of wrapper.findAll('.dl-item input[type="checkbox"]')) {
      expect((cb.element as HTMLInputElement).checked).toBe(true);
    }
    await toggle.setValue(false);
    for (const cb of wrapper.findAll('.dl-item input[type="checkbox"]')) {
      expect((cb.element as HTMLInputElement).checked).toBe(false);
    }
  });

  it('shows Delete N selected button when documents are checked', async () => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const collectionStore = useCollectionStore();
    collectionStore.collections = [
      {
        id: 'col-1',
        name: 'Test',
        created_at: '2026-01-01T00:00:00Z',
        document_count: 3,
      },
    ];
    collectionStore.setActiveCollection('col-1');

    const apiModule = await import('@/api/client');
    mockGet(apiModule);

    const wrapper = mount(DocumentList, { global: { plugins: [pinia] } });
    await flushPromises();
    await nextTick();

    const boxes = wrapper.findAll('.dl-item input[type="checkbox"]');
    await boxes[0].setValue(true);
    await boxes[1].setValue(true);
    expect(wrapper.find('.dl-header__actions').text()).toMatch(/Delete.*2/);
  });

  it('calls api.batchDeleteDocuments on confirm', async () => {
    mockBatchDelete.mockResolvedValue({
      deleted_count: 2,
      ids: ['doc-1', 'doc-2'],
    });

    const pinia = createPinia();
    setActivePinia(pinia);

    const collectionStore = useCollectionStore();
    collectionStore.collections = [
      {
        id: 'col-1',
        name: 'Test',
        created_at: '2026-01-01T00:00:00Z',
        document_count: 3,
      },
    ];
    collectionStore.setActiveCollection('col-1');

    const apiModule = await import('@/api/client');
    mockGet(apiModule);

    const wrapper = mount(DocumentList, { global: { plugins: [pinia] } });
    await flushPromises();
    await nextTick();

    const boxes = wrapper.findAll('.dl-item input[type="checkbox"]');
    await boxes[0].setValue(true);
    await boxes[1].setValue(true);

    await wrapper.find('.dl-header__actions [class*="destructive"]').trigger('click');
    await nextTick();

    const allBtns = document.body.querySelectorAll('button');
    for (const b of allBtns) {
      if (b.textContent?.includes('Delete') && !b.textContent?.includes('selected')) {
        (b as HTMLElement).click();
        break;
      }
    }
    await nextTick();

    expect(mockBatchDelete).toHaveBeenCalledWith(['doc-1', 'doc-2']);
  });
});
