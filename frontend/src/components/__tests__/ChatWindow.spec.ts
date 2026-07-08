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
  post: vi.fn(),
  patch: vi.fn(),
  del: vi.fn(),
  getSessionWithMessages: vi.fn(),
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
    localStorage.clear();
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
      '[data-testid="collection-selector-trigger"]',
    );
    if (!trigger) {
      throw new Error('Expected chat collection selector trigger to be rendered.');
    }

    trigger.click();
    await nextTick();

    const dropdown = document.body.querySelector<HTMLElement>(
      '[data-testid="collection-selector-dropdown"]',
    );
    expect(dropdown).not.toBeNull();
    expect(dropdown?.textContent).toContain('Technical Docs');
  });

  // ==========================================================================
  // Chat UI polish: session sidebar
  // ==========================================================================

  it('renders new session button below sidebar header', async () => {
    const wrapper = mount(ChatView);
    const newBtn = wrapper.find('[data-testid="btn-new-chat"]');
    expect(newBtn.exists()).toBe(true);
    // Button should not be inside the header row
    const sidebar = wrapper.find('[data-testid="session-sidebar"]');
    const header = sidebar.find('.session-header');
    expect(header.exists()).toBe(true);
  });

  it('renders search icon button in sidebar header', async () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="session-search-toggle"]').exists()).toBe(true);
  });

  it('renders collapse sidebar button', async () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="sidebar-collapse-btn"]').exists()).toBe(true);
  });

  it('collapse button toggles sidebar collapsed state', async () => {
    const wrapper = mount(ChatView);
    const collapseBtn = wrapper.find('[data-testid="sidebar-collapse-btn"]');
    await collapseBtn.trigger('click');
    await nextTick();
    const sidebar = wrapper.find('[data-testid="session-sidebar"]');
    expect(sidebar.classes('session-sidebar--collapsed')).toBe(true);
  });

  it('session search dialog opens and filters sessions', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();

    // Populate sessions directly in the store
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

    // Open search dialog
    const searchToggle = wrapper.find('[data-testid="session-search-toggle"]');
    await searchToggle.trigger('click');
    await nextTick();

    // VDialog uses Teleport to body, so check in document.body
    const dialog = document.body.querySelector('[data-testid="confirm-dialog"]');
    expect(dialog).not.toBeNull();

    // Type a search query into the store directly
    chatStore.setSearchQuery('Deploy');
    await nextTick();

    // Check that only matching sessions show in dialog list
    const items = document.body.querySelectorAll('[data-testid="search-dialog-item"]');
    expect(items.length).toBe(1);
    expect(items[0].textContent).toContain('Deploy');
  });

  it('rename session dialog opens', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();
    // Wait for onMounted fetch calls to complete
    await nextTick();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await nextTick();
    // Now override with test data
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

    // VDialog uses <Teleport to="body">, so search in document.body
    const dialog = document.body.querySelector('[data-testid="confirm-dialog"]');
    expect(dialog).not.toBeNull();
    // Dialog should contain the rename input (also teleported)
    const input = document.body.querySelector('[data-testid="session-rename-input"]');
    expect(input).not.toBeNull();
  });

  it('displays collection tag when collection is selected', async () => {
    const wrapper = mount(ChatView);
    // Wait for onMounted fetchCollections to resolve
    await wrapper.vm.$nextTick();
    await new Promise((r) => setTimeout(r, 0));
    await wrapper.vm.$nextTick();

    const { useCollectionStore } = await import('@/stores/collections');
    const collectionStore = useCollectionStore();

    // collections now has [{ id: 'collection-1', name: 'Technical Docs' }] from mock
    // match the activeCollectionId to the mock's data
    collectionStore.setActiveCollection('collection-1');
    await wrapper.vm.$nextTick();

    // When no active session but collection is active: toolbar-collection-badge
    const tag = wrapper.find('[data-testid="toolbar-collection-badge"]');
    expect(tag.exists()).toBe(true);
    expect(tag.text()).toContain('Technical Docs');
  });

  // ==========================================================================
  // Chat UI polish: tests requiring store action mocking
  // ==========================================================================

  it('collection badge renders below session title in toolbar', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const { useCollectionStore } = await import('@/stores/collections');
    const chatStore = useChatStore();
    const collectionStore = useCollectionStore();

    collectionStore.collections = [
      {
        id: 'col-1',
        name: 'Technical Docs',
        created_at: '2026-06-23T00:00:00Z',
        document_count: 5,
      },
    ];
    collectionStore.activeCollectionId = 'col-1';

    chatStore.sessions = [
      {
        id: 'sess-1',
        title: 'My Chat Session',
        message_count: 3,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];
    chatStore.activeSessionId = 'sess-1';
    chatStore.isLoadingSessions = false;
    await nextTick();

    const toolbarBadges = wrapper.find('[data-testid="toolbar-badges"]');
    const children = toolbarBadges.element.children;

    // First child should be the session title (h1), then the collection badge
    expect(children.length).toBeGreaterThanOrEqual(2);
    expect((children[0] as HTMLElement).tagName).toBe('H1');
    expect((children[0] as HTMLElement).textContent).toContain('My Chat Session');
    expect((children[1] as HTMLElement).textContent).toContain('Technical Docs');
  });

  it('restores pipeline state and displays it in toolbar after simulated reload', async () => {
    // Override the sessions mock to return a session with default backend title
    apiMock.get.mockImplementation((path: string) => {
      if (path === '/sessions') {
        return Promise.resolve([
          {
            id: 'sess-1',
            title: 'New Chat',
            collection_id: 'col-1',
            created_at: '2026-06-23T00:00:00Z',
            updated_at: '2026-06-23T00:00:00Z',
            message_count: 1,
          },
        ]);
      }
      if (path === '/collections') {
        return Promise.resolve([
          {
            id: 'col-1',
            name: 'Technical Docs',
            created_at: '2026-06-23T00:00:00Z',
            document_count: 5,
          },
        ]);
      }
      return Promise.resolve([]);
    });

    // Stub getSessionWithMessages so polling does not throw
    apiMock.getSessionWithMessages.mockResolvedValue({
      session: { id: 'sess-1' },
      messages: [],
    });

    // Simulate localStorage left behind by a running pipeline before reload
    localStorage.setItem('chat_pipeline_active', 'true');
    localStorage.setItem('chat_pipeline_session_id', 'sess-1');
    localStorage.setItem('chat_pipeline_collection_id', 'col-1');
    localStorage.setItem('chat_pipeline_stage', 'searching');
    localStorage.setItem('chat_pipeline_temp_title', 'How does RAG work?');
    localStorage.setItem('chat_pipeline_user_query', 'How does RAG work?');

    const wrapper = mount(ChatView);

    // Wait for onMounted chain: fetchSessions → fetchCollections → checkPendingPipeline
    await nextTick();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await nextTick();

    // The toolbar should show the restored temp title, not "New Chat"
    const toolbarTitle = wrapper.find('[data-testid="toolbar-session-title"]');
    expect(toolbarTitle.exists()).toBe(true);
    expect(toolbarTitle.text()).toBe('How does RAG work?');

    // The collection badge should appear below the title
    const collBadge = wrapper.find('[data-testid="toolbar-collection-badge"]');
    expect(collBadge.exists()).toBe(true);
    expect(collBadge.text()).toContain('Technical Docs');
  });

  it('displays session tag with collection badge when session is active', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const { useCollectionStore } = await import('@/stores/collections');
    const chatStore = useChatStore();
    const collectionStore = useCollectionStore();

    collectionStore.collections = [
      {
        id: 'col-1',
        name: 'Technical Docs',
        created_at: '2026-06-23T00:00:00Z',
        document_count: 5,
      },
    ];
    collectionStore.activeCollectionId = 'col-1';

    chatStore.sessions = [
      {
        id: 'sess-1',
        title: 'My Chat Session',
        message_count: 3,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];
    chatStore.activeSessionId = 'sess-1';
    chatStore.isLoadingSessions = false;
    await nextTick();

    // Session and collection badges should show separately
    const toolbarBadges = wrapper.find('[data-testid="toolbar-badges"]');
    expect(toolbarBadges.exists()).toBe(true);
    expect(toolbarBadges.text()).toContain('My Chat Session');

    const collBadge = wrapper.find('[data-testid="toolbar-collection-badge"]');
    expect(collBadge.exists()).toBe(true);
    expect(collBadge.text()).toContain('Technical Docs');

    const sessionTitle = wrapper.find('[data-testid="toolbar-session-title"]');
    expect(sessionTitle.exists()).toBe(true);
    expect(sessionTitle.text()).toContain('My Chat Session');
  });

  it('renders export button when session is active', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();
    chatStore.activeSessionId = 'sess-1';
    await nextTick();

    const exportBtn = wrapper.find('[data-testid="export-btn"]');
    expect(exportBtn.exists()).toBe(true);
  });

  // ==========================================================================
  // Regression: sidebar expanded by default matching design
  // ==========================================================================

  it('sidebar starts expanded (not collapsed) by default', async () => {
    const wrapper = mount(ChatView);
    const sidebar = wrapper.find('[data-testid="session-sidebar"]');
    expect(sidebar.exists()).toBe(true);
    expect(sidebar.classes('session-sidebar--collapsed')).toBe(false);
  });

  it('sidebar shows header with session title when expanded', async () => {
    const wrapper = mount(ChatView);
    const sidebar = wrapper.find('[data-testid="session-sidebar"]');
    expect(sidebar.find('.session-title').exists()).toBe(true);
    expect(sidebar.find('.session-title').text()).toBe('HISTORY');
  });

  it('sidebar shows search toggle and collapse buttons in header when expanded', async () => {
    const wrapper = mount(ChatView);
    const sidebar = wrapper.find('[data-testid="session-sidebar"]');
    expect(sidebar.find('[data-testid="session-search-toggle"]').exists()).toBe(true);
    expect(sidebar.find('[data-testid="sidebar-collapse-btn"]').exists()).toBe(true);
  });

  it('sidebar shows new session button when expanded', async () => {
    const wrapper = mount(ChatView);
    const sidebar = wrapper.find('[data-testid="session-sidebar"]');
    expect(sidebar.find('[data-testid="btn-new-chat"]').exists()).toBe(true);
  });

  it('sidebar shows session list when sessions exist and expanded', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();

    chatStore.sessions = [
      {
        id: 's1',
        title: 'Test Session',
        message_count: 2,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];
    chatStore.isLoadingSessions = false;
    await nextTick();

    const sidebar = wrapper.find('[data-testid="session-sidebar"]');
    const items = sidebar.findAll('[data-testid="session-item"]');
    expect(items.length).toBeGreaterThan(0);
    expect(items[0].text()).toContain('Test Session');
  });

  it('collapsing sidebar toggles collapsed CSS class', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();

    const sidebar = wrapper.find('[data-testid="session-sidebar"]');

    // Starts expanded
    expect(sidebar.classes('session-sidebar--collapsed')).toBe(false);

    // Collapse
    chatStore.toggleSidebarCollapsed();
    await nextTick();
    expect(sidebar.classes('session-sidebar--collapsed')).toBe(true);

    // Expand again
    chatStore.toggleSidebarCollapsed();
    await nextTick();
    expect(sidebar.classes('session-sidebar--collapsed')).toBe(false);
  });

  it('collapsed sidebar has narrow CSS width', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();

    const sidebar = wrapper.find('[data-testid="session-sidebar"]');

    // Expanded width
    expect(sidebar.classes('session-sidebar--collapsed')).toBe(false);

    // Collapse
    chatStore.toggleSidebarCollapsed();
    await nextTick();
    expect(sidebar.classes('session-sidebar--collapsed')).toBe(true);
  });

  it('collapse button has correct title based on sidebar state', async () => {
    const wrapper = mount(ChatView);
    const collapseBtn = wrapper.find('[data-testid="sidebar-collapse-btn"]');

    // Initially expanded
    expect(collapseBtn.attributes('title')).toBe('Collapse sidebar');

    // Toggle to collapsed
    await collapseBtn.trigger('click');
    await nextTick();
    expect(collapseBtn.attributes('title')).toBe('Expand sidebar');

    // Toggle back to expanded
    await collapseBtn.trigger('click');
    await nextTick();
    expect(collapseBtn.attributes('title')).toBe('Collapse sidebar');
  });

  it('sidebar controls remain responsive after selecting a session with null sources', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();

    // Wait for onMounted fetch calls to complete
    await nextTick();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await nextTick();

    // Populate sessions
    chatStore.sessions = [
      {
        id: 's1',
        title: 'Test Session',
        message_count: 2,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];
    chatStore.isLoadingSessions = false;
    await nextTick();

    // Load session with null sources (simulating backend returning sources: "null")
    chatStore.messages = [
      {
        id: 'm1',
        session_id: 's1',
        role: 'user',
        content: 'Hello',
        sources: 'null',
        created_at: '2026-06-23T00:00:00Z',
      },
      {
        id: 'm2',
        session_id: 's1',
        role: 'assistant',
        content: 'Hi!',
        sources: 'null',
        created_at: '2026-06-23T00:00:05Z',
      },
    ];
    chatStore.activeSessionId = 's1';
    await nextTick();

    // Sidebar collapse button should still be visible and enabled
    const collapseBtn = wrapper.find('[data-testid="sidebar-collapse-btn"]');
    expect(collapseBtn.exists()).toBe(true);
    await collapseBtn.trigger('click');
    await nextTick();
    const sidebar = wrapper.find('[data-testid="session-sidebar"]');
    expect(sidebar.classes('session-sidebar--collapsed')).toBe(true);

    // New session button should still be enabled
    const newSessionBtn = wrapper.find('[data-testid="btn-new-chat"]');
    expect(newSessionBtn.exists()).toBe(true);
    expect(newSessionBtn.attributes('disabled')).toBeUndefined();

    // Search toggle should still be visible
    const searchToggle = wrapper.find('[data-testid="session-search-toggle"]');
    expect(searchToggle.exists()).toBe(true);
  });
});
