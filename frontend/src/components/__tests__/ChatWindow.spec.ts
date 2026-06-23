import ChatView from '@/views/ChatView.vue';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';

const apiMock = vi.hoisted(() => ({
  get: vi.fn((path: string) => {
    if (path === '/collections') {
      return Promise.resolve([
        {
          id: 'collection-1',
          name: 'Technical Docs',
          created_at: '2026-06-19T00:00:00Z',
          document_count: 2,
        },
      ]);
    }
    if (path === '/sessions') {
      return Promise.resolve([]);
    }
    return Promise.resolve([]);
  }),
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
  getAccessToken: vi.fn(() => null),
}));

describe('ChatWindow (ChatView)', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    vi.clearAllMocks();
    setActivePinia(createPinia());
  });

  it('renders welcome screen when no messages', () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="welcome-message"]').exists()).toBe(true);
  });

  it('has a send button', () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="btn-send"]').exists()).toBe(true);
  });

  it('has input textarea', () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="chat-input"]').exists()).toBe(true);
  });

  it('does not show cancel button when not loading', () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="btn-cancel"]').exists()).toBe(false);
  });

  it('shows cancel button when loading', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();
    chatStore.isLoading = true;
    await wrapper.vm.$nextTick();
    expect(wrapper.find('[data-testid="btn-cancel"]').exists()).toBe(true);
  });

  it('shows collection options when the chat collection selector is opened', async () => {
    mount(ChatView, {
      attachTo: document.body,
    });

    await nextTick();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await nextTick();

    const trigger = document.body.querySelector<HTMLElement>(
      '[data-testid="collection-select"] .v-select__trigger',
    );
    if (!trigger) {
      throw new Error('Expected chat collection selector trigger to be rendered.');
    }

    trigger.click();
    await nextTick();

    const dropdown = document.body.querySelector<HTMLElement>(
      '[data-testid="collection-select-dropdown"]',
    );
    expect(dropdown).not.toBeNull();
    expect(dropdown?.textContent).toContain('Technical Docs');
  });

  // ==========================================================================
  // Chat UI polish: session sidebar and chat view changes
  // All tests are `.skip` until the features are implemented.
  // ==========================================================================

  it.skip('renders centered New Session button below sidebar header', async () => {
    const wrapper = mount(ChatView);
    const newBtn = wrapper.find('[data-testid="btn-new-chat"]');
    expect(newBtn.exists()).toBe(true);
    // Should not be inside the header row — find its position context
    const sidebar = wrapper.find('[data-testid="session-sidebar"]');
    const header = sidebar.find('.session-header');
    expect(header.exists()).toBe(true);
    // New button should be outside .session-header in a dedicated row
    const newBtnRow = sidebar.find('[data-testid="new-session-row"]');
    expect(newBtnRow.exists()).toBe(true);
  });

  it.skip('renders search icon button in sidebar header', async () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="sidebar-search-btn"]').exists()).toBe(true);
  });

  it.skip('renders collapse sidebar button', async () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="sidebar-collapse-btn"]').exists()).toBe(true);
  });

  it.skip('collapse button toggles sidebar visibility', async () => {
    const wrapper = mount(ChatView);
    const collapseBtn = wrapper.find('[data-testid="sidebar-collapse-btn"]');
    await collapseBtn.trigger('click');
    await nextTick();
    const sidebar = wrapper.find('[data-testid="session-sidebar"]');
    expect(sidebar.classes('session-sidebar--collapsed')).toBe(true);
  });

  it.skip('session search filters session list by title', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();
    chatStore.sessions = [
      {
        id: 's1',
        title: 'Deploy instructions',
        message_count: 3,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
      {
        id: 's2',
        title: 'API usage',
        message_count: 5,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];
    chatStore.isLoadingSessions = false;
    await nextTick();

    const searchInput = wrapper.find('[data-testid="session-search-input"]');
    expect(searchInput.exists()).toBe(true);
    await searchInput.setValue('deploy');
    await nextTick();

    const items = wrapper.findAll('.session-item');
    expect(items.length).toBe(1);
    expect(items[0].text()).toContain('Deploy');
  });

  it.skip('rename session dialog opens and submits new title', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();
    chatStore.sessions = [
      {
        id: 's1',
        title: 'Old title',
        message_count: 1,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];
    chatStore.isLoadingSessions = false;
    await nextTick();

    const renameBtn = wrapper.find('[data-testid="session-rename-btn"]');
    await renameBtn.trigger('click');
    await nextTick();

    // Dialog should be visible
    const dialog = wrapper.find('[data-testid="rename-session-dialog"]');
    expect(dialog.exists()).toBe(true);
  });

  it.skip('pin session toggles pin icon', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();
    chatStore.sessions = [
      {
        id: 's1',
        title: 'Chat',
        message_count: 2,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
        pinned: false,
      },
    ];
    chatStore.isLoadingSessions = false;
    await nextTick();

    const pinBtn = wrapper.find('[data-testid="session-pin-btn"]');
    expect(pinBtn.exists()).toBe(true);
    await pinBtn.trigger('click');
    await nextTick();

    expect(chatStore.togglePinSession).toHaveBeenCalledWith('s1');
  });

  it.skip('displays collection tag with session name and collection name', async () => {
    const wrapper = mount(ChatView);
    const { useCollectionStore } = await import('@/stores/collections');
    const collectionStore = useCollectionStore();
    collectionStore.collections = [
      {
        id: 'col-1',
        name: 'My Docs',
        created_at: '2026-06-23T00:00:00Z',
        document_count: 5,
      },
    ];
    collectionStore.activeCollectionId = 'col-1';
    await nextTick();

    const tag = wrapper.find('[data-testid="collection-tag"]');
    expect(tag.exists()).toBe(true);
    expect(tag.text()).toContain('My Docs');
  });

  it.skip('auto-selects first collection on chat load when none selected', async () => {
    const wrapper = mount(ChatView);
    const { useCollectionStore } = await import('@/stores/collections');
    const collectionStore = useCollectionStore();
    collectionStore.collections = [
      {
        id: 'col-1',
        name: 'Docs',
        created_at: '2026-06-23T00:00:00Z',
        document_count: 3,
      },
      {
        id: 'col-2',
        name: 'Manual',
        created_at: '2026-06-23T00:00:00Z',
        document_count: 1,
      },
    ];
    // activeCollectionId should be null initially
    expect(collectionStore.activeCollectionId).toBeNull();
    await nextTick();

    // After mount, should auto-select first collection
    expect(collectionStore.activeCollectionId).toBe('col-1');
  });

  it.skip('auto-sets collection when selecting existing session', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const { useCollectionStore } = await import('@/stores/collections');
    const chatStore = useChatStore();
    const collectionStore = useCollectionStore();

    collectionStore.collections = [
      {
        id: 'col-1',
        name: 'Docs',
        created_at: '2026-06-23T00:00:00Z',
        document_count: 3,
      },
      {
        id: 'col-2',
        name: 'Manual',
        created_at: '2026-06-23T00:00:00Z',
        document_count: 1,
      },
    ];

    // Simulate selecting a session with collection_id
    chatStore.sessions = [
      {
        id: 's1',
        title: 'My chat',
        message_count: 2,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
        collection_id: 'col-2',
      },
    ];
    await wrapper.vm.handleSelectSession('s1');
    await nextTick();

    expect(collectionStore.activeCollectionId).toBe('col-2');
  });
});
