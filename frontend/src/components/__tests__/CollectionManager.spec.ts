import CollectionManager from '@/components/CollectionManager.vue';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';

const apiMock = vi.hoisted(() => ({
  get: vi.fn((path: string) => {
    if (path === '/collections') {
      return Promise.resolve([
        {
          id: 'col-1',
          name: 'Technical Docs',
          description: 'Product documentation',
          document_count: 5,
          created_at: '2026-06-19T00:00:00Z',
        },
        {
          id: 'col-2',
          name: 'API Reference',
          description: undefined,
          document_count: 12,
          created_at: '2026-06-20T00:00:00Z',
        },
      ]);
    }
    return Promise.resolve([]);
  }),
  post: vi.fn(),
  del: vi.fn(),
}));

vi.mock('@/api/client', () => ({
  api: apiMock,
  ApiError: class ApiError extends Error {
    constructor(
      public status: number,
      message: string,
    ) {
      super(message);
    }
  },
}));

describe('CollectionManager', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    vi.clearAllMocks();
    setActivePinia(createPinia());
  });

  // ==========================================================================
  // Rendering collection list
  // ==========================================================================

  it('renders COLLECTIONS header', async () => {
    const wrapper = mount(CollectionManager);
    expect(wrapper.text()).toContain('COLLECTIONS');
  });

  it('renders + New button', async () => {
    const wrapper = mount(CollectionManager);
    const newBtn = wrapper.findComponent({ name: 'VButton' });
    expect(newBtn.exists()).toBe(true);
    expect(newBtn.text()).toContain('+ New');
  });

  it('renders collection cards when collections are loaded', async () => {
    const wrapper = mount(CollectionManager);
    // Wait for onMounted fetchCollections
    await nextTick();
    await new Promise((r) => setTimeout(r, 0));
    await nextTick();

    const cards = wrapper.findAll('.cm-card');
    expect(cards.length).toBe(2);
    expect(cards[0].text()).toContain('Technical Docs');
    expect(cards[0].text()).toContain('5 documents');
    expect(cards[1].text()).toContain('API Reference');
    expect(cards[1].text()).toContain('12 documents');
  });

  it('displays collection description when present', async () => {
    const wrapper = mount(CollectionManager);
    await nextTick();
    await new Promise((r) => setTimeout(r, 0));
    await nextTick();

    const cards = wrapper.findAll('.cm-card');
    expect(cards[0].text()).toContain('Product documentation');
  });

  it('renders singular "document" for count of 1', async () => {
    const wrapper = mount(CollectionManager);
    // Override store to have a single document
    const { useCollectionStore } = await import('@/stores/collections');
    const store = useCollectionStore();
    store.collections = [
      {
        id: 'col-1',
        name: 'Single Doc',
        description: undefined,
        document_count: 1,
        created_at: '2026-06-19T00:00:00Z',
      },
    ];
    store.isLoading = false;
    await nextTick();

    expect(wrapper.text()).toContain('1 document');
  });

  // ==========================================================================
  // Inline trash icon presence on collection cards
  // ==========================================================================

  it('renders delete button on each collection card', async () => {
    const wrapper = mount(CollectionManager);
    await nextTick();
    await new Promise((r) => setTimeout(r, 0));
    await nextTick();

    const deleteBtns = wrapper.findAll('.cm-card__delete');
    expect(deleteBtns.length).toBe(2);
    // Each delete button should have the delete title
    for (const btn of deleteBtns) {
      expect(btn.attributes('title')).toBe('Delete collection');
    }
  });

  // ==========================================================================
  // Delete dialog opens on trash icon click
  // ==========================================================================

  it('opens delete dialog when trash icon is clicked', async () => {
    const wrapper = mount(CollectionManager, {
      attachTo: document.body,
    });
    await nextTick();
    await new Promise((r) => setTimeout(r, 0));
    await nextTick();

    // Click delete button on first card
    const deleteBtn = wrapper.find('.cm-card__delete');
    await deleteBtn.trigger('click');
    await nextTick();

    // VDialog teleports to body, so check in document.body
    const dialog = document.body.querySelector('[data-testid="confirm-dialog"]');
    expect(dialog).not.toBeNull();
    expect(dialog?.textContent).toContain('Technical Docs');
    expect(dialog?.textContent).toContain('5 documents');
    expect(dialog?.textContent).toContain('This action cannot be undone');
  });

  it('calls deleteCollection when delete dialog is confirmed', async () => {
    const wrapper = mount(CollectionManager, {
      attachTo: document.body,
    });
    await nextTick();
    await new Promise((r) => setTimeout(r, 0));
    await nextTick();

    const { useCollectionStore } = await import('@/stores/collections');
    const store = useCollectionStore();
    const deleteSpy = vi.spyOn(store, 'deleteCollection');

    // Click delete on first card
    await wrapper.find('.cm-card__delete').trigger('click');
    await nextTick();

    // Find and click the confirm button inside the teleported dialog
    const confirmBtn = document.body.querySelector(
      '[data-testid="confirm-dialog"] [data-testid="btn-dialog-confirm"]',
    ) as HTMLElement;
    expect(confirmBtn).not.toBeNull();
    await confirmBtn.click();
    await nextTick();

    expect(deleteSpy).toHaveBeenCalledWith('col-1');
  });

  // ==========================================================================
  // Active collection highlighting
  // ==========================================================================

  it('highlights active collection card', async () => {
    const wrapper = mount(CollectionManager);
    await nextTick();
    await new Promise((r) => setTimeout(r, 0));
    await nextTick();

    const { useCollectionStore } = await import('@/stores/collections');
    const store = useCollectionStore();
    store.setActiveCollection('col-1');
    await nextTick();

    const cards = wrapper.findAll('.cm-card');
    expect(cards[0].classes('cm-card--active')).toBe(true);
    expect(cards[1].classes('cm-card--active')).toBe(false);
  });

  // ==========================================================================
  // Empty state
  // ==========================================================================

  it('shows empty state when no collections exist', async () => {
    const wrapper = mount(CollectionManager);
    await nextTick();
    await new Promise((r) => setTimeout(r, 0));
    await nextTick();

    const { useCollectionStore } = await import('@/stores/collections');
    const store = useCollectionStore();
    store.collections = [];
    store.isLoading = false;
    await nextTick();

    expect(wrapper.text()).toContain('No collections yet');
    expect(wrapper.text()).toContain('Create one to start organizing documents');
  });

  // ==========================================================================
  // Loading state
  // ==========================================================================

  it('shows loading state when fetching', async () => {
    const wrapper = mount(CollectionManager);
    const { useCollectionStore } = await import('@/stores/collections');
    const store = useCollectionStore();
    store.collections = [];
    store.isLoading = true;
    await nextTick();

    expect(wrapper.text()).toContain('Loading...');
  });
});
