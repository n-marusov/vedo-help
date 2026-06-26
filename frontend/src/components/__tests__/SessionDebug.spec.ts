import { api } from '@/api/client';
import type { Message, SessionSummary } from '@/api/types';
import SessionDebug from '@/components/SessionDebug.vue';
import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

// Mock the API client
vi.mock('@/api/client', () => ({
  api: {
    adminSearchSessions: vi.fn(),
    getSessionWithMessages: vi.fn(),
  },
}));

const mockSessions: SessionSummary[] = [
  {
    id: 'sess-1',
    title: 'Technical Docs Q&A',
    message_count: 12,
    created_at: '2026-06-24T10:00:00Z',
    updated_at: '2026-06-24T12:00:00Z',
  },
  {
    id: 'sess-2',
    title: 'Deployment Guide',
    message_count: 8,
    created_at: '2026-06-23T09:00:00Z',
    updated_at: '2026-06-23T11:00:00Z',
  },
];

const mockMessages: Message[] = [
  {
    id: 'msg-1',
    session_id: 'sess-1',
    role: 'user',
    content: 'How do I install VEDO?',
    created_at: '2026-06-24T10:00:00Z',
  },
  {
    id: 'msg-2',
    session_id: 'sess-1',
    role: 'assistant',
    content: 'To install VEDO run docker compose up...',
    created_at: '2026-06-24T10:00:05Z',
    debug_data: JSON.stringify({
      query_text: 'How do I install VEDO?',
      embedding_search: {
        query_snippet: 'How do I install VEDO?',
        embedding_dimension: 384,
        latency_ms: 45,
        collection_name: 'abc-123',
        top_k: 5,
        result_count: 3,
        retries: 0,
        results: [
          {
            chunk_id: 'chunk-1',
            document_name: 'docs.pdf',
            chunk_index: 3,
            score: 0.92,
            text_snippet: 'To install run docker compose up',
          },
        ],
      },
      final_answer: {
        model: 'gpt-4',
        max_retries: 3,
        chunks_in_context: 3,
        history_message_count: 0,
        history_token_estimate: 0,
        token_budget: 4000,
        total_tokens_estimate: 2000,
        latency_ms: 1200,
        prompt_preview: 'Answer the question based on the context...',
      },
    }),
  },
  {
    id: 'msg-3',
    session_id: 'sess-1',
    role: 'user',
    content: 'What are system requirements?',
    created_at: '2026-06-24T10:01:00Z',
  },
];

describe('SessionDebug', () => {
  it('renders session list after search', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue(mockSessions);

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    // Trigger search
    const searchInput = wrapper.find('[data-testid="session-debug-search"]');
    if (searchInput.exists()) {
      await searchInput.setValue('Technical');
      await searchInput.trigger('input');
    }

    await flushPromises();

    expect(api.adminSearchSessions).toHaveBeenCalled();
    const items = wrapper.findAll('[data-testid="session-list-item"]');
    expect(items.length).toBeGreaterThanOrEqual(1);
  });

  it('clicking session shows messages', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue(mockSessions);
    vi.mocked(api.getSessionWithMessages).mockResolvedValue({
      session: {
        id: 'sess-1',
        title: 'Technical Docs Q&A',
        message_count: 3,
        pinned: false,
        collection_id: 'col-1',
        created_at: '2026-06-24T10:00:00Z',
        updated_at: '2026-06-24T12:00:00Z',
      },
      messages: mockMessages,
    });

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    // First search to populate sessions
    const searchInput = wrapper.find('[data-testid="session-debug-search"]');
    if (searchInput.exists()) {
      await searchInput.setValue('Technical');
      await searchInput.trigger('input');
    }
    await flushPromises();

    // Click first session
    const firstItem = wrapper.find('[data-testid="session-list-item"]');
    if (firstItem.exists()) {
      await firstItem.trigger('click');
    }
    await flushPromises();

    expect(api.getSessionWithMessages).toHaveBeenCalledWith('sess-1');
    const messageBubbles = wrapper.findAll('[data-testid="session-msg"]');
    expect(messageBubbles.length).toBeGreaterThanOrEqual(1);
  });

  it('search input calls API with filter', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue([]);

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    const searchInput = wrapper.find('[data-testid="session-debug-search"]');
    if (searchInput.exists()) {
      await searchInput.setValue('API Reference');
      await searchInput.trigger('input');
    }

    await flushPromises();

    expect(api.adminSearchSessions).toHaveBeenCalledWith(
      expect.objectContaining({ search: 'API Reference' }),
    );
  });

  it('assistant message shows debug button', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue(mockSessions);
    vi.mocked(api.getSessionWithMessages).mockResolvedValue({
      session: {
        id: 'sess-1',
        title: 'Technical Docs Q&A',
        message_count: 3,
        pinned: false,
        collection_id: 'col-1',
        created_at: '2026-06-24T10:00:00Z',
        updated_at: '2026-06-24T12:00:00Z',
      },
      messages: mockMessages,
    });

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    // Load session
    const searchInput = wrapper.find('[data-testid="session-debug-search"]');
    if (searchInput.exists()) {
      await searchInput.setValue('Technical');
      await searchInput.trigger('input');
    }
    await flushPromises();

    const firstItem = wrapper.find('[data-testid="session-list-item"]');
    if (firstItem.exists()) {
      await firstItem.trigger('click');
    }
    await flushPromises();

    // Check assistant messages have debug toggle button
    const debugBtns = wrapper.findAll('[data-testid="session-debug-toggle"]');
    expect(debugBtns.length).toBeGreaterThanOrEqual(1);
  });

  it('debug panel shows 7 steps', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue(mockSessions);
    vi.mocked(api.getSessionWithMessages).mockResolvedValue({
      session: {
        id: 'sess-1',
        title: 'Technical Docs Q&A',
        message_count: 3,
        pinned: false,
        collection_id: 'col-1',
        created_at: '2026-06-24T10:00:00Z',
        updated_at: '2026-06-24T12:00:00Z',
      },
      messages: mockMessages,
    });

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    // Load session
    const searchInput = wrapper.find('[data-testid="session-debug-search"]');
    if (searchInput.exists()) {
      await searchInput.setValue('Technical');
      await searchInput.trigger('input');
    }
    await flushPromises();

    const firstItem = wrapper.find('[data-testid="session-list-item"]');
    if (firstItem.exists()) {
      await firstItem.trigger('click');
    }
    await flushPromises();

    // Click debug toggle on first assistant message
    const debugToggle = wrapper.find('[data-testid="session-debug-toggle"]');
    if (debugToggle.exists()) {
      await debugToggle.trigger('click');
    }
    await flushPromises();

    // Verify all 7 step titles visible
    const stepTitles = wrapper.findAll('[data-testid="debug-step-title"]');
    expect(stepTitles.length).toBe(7);
  });

  it('active steps 3 and 7 show data', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue(mockSessions);
    vi.mocked(api.getSessionWithMessages).mockResolvedValue({
      session: {
        id: 'sess-1',
        title: 'Technical Docs Q&A',
        message_count: 3,
        pinned: false,
        collection_id: 'col-1',
        created_at: '2026-06-24T10:00:00Z',
        updated_at: '2026-06-24T12:00:00Z',
      },
      messages: mockMessages,
    });

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    // Load and click
    const searchInput = wrapper.find('[data-testid="session-debug-search"]');
    if (searchInput.exists()) {
      await searchInput.setValue('Technical');
      await searchInput.trigger('input');
    }
    await flushPromises();

    const firstItem = wrapper.find('[data-testid="session-list-item"]');
    if (firstItem.exists()) {
      await firstItem.trigger('click');
    }
    await flushPromises();

    const debugToggle = wrapper.find('[data-testid="session-debug-toggle"]');
    if (debugToggle.exists()) {
      await debugToggle.trigger('click');
    }
    await flushPromises();

    // Step 3 (embedding search) and step 7 (final answer) should have data
    const stepData = wrapper.findAll('[data-testid="debug-step-data"]');
    expect(stepData.length).toBeGreaterThanOrEqual(2);
  });

  it('future steps show placeholder badge', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue(mockSessions);
    vi.mocked(api.getSessionWithMessages).mockResolvedValue({
      session: {
        id: 'sess-1',
        title: 'Technical Docs Q&A',
        message_count: 3,
        pinned: false,
        collection_id: 'col-1',
        created_at: '2026-06-24T10:00:00Z',
        updated_at: '2026-06-24T12:00:00Z',
      },
      messages: mockMessages,
    });

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    const searchInput = wrapper.find('[data-testid="session-debug-search"]');
    if (searchInput.exists()) {
      await searchInput.setValue('Technical');
      await searchInput.trigger('input');
    }
    await flushPromises();

    const firstItem = wrapper.find('[data-testid="session-list-item"]');
    if (firstItem.exists()) {
      await firstItem.trigger('click');
    }
    await flushPromises();

    const debugToggle = wrapper.find('[data-testid="session-debug-toggle"]');
    if (debugToggle.exists()) {
      await debugToggle.trigger('click');
    }
    await flushPromises();

    // Steps 1,2,4,5,6 should show v0.5 badge
    const futureBadges = wrapper.findAll('[data-testid="debug-step-future"]');
    expect(futureBadges.length).toBe(5);
  });

  it('renders empty state when no session selected', async () => {
    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    const emptyState = wrapper.find('[data-testid="session-debug-empty"]');
    expect(emptyState.exists()).toBe(true);
  });

  it('debug button not shown for user messages', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue(mockSessions);
    vi.mocked(api.getSessionWithMessages).mockResolvedValue({
      session: {
        id: 'sess-1',
        title: 'Technical Docs Q&A',
        message_count: 3,
        pinned: false,
        collection_id: 'col-1',
        created_at: '2026-06-24T10:00:00Z',
        updated_at: '2026-06-24T12:00:00Z',
      },
      messages: mockMessages,
    });

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    const searchInput = wrapper.find('[data-testid="session-debug-search"]');
    if (searchInput.exists()) {
      await searchInput.setValue('Technical');
      await searchInput.trigger('input');
    }
    await flushPromises();

    const firstItem = wrapper.find('[data-testid="session-list-item"]');
    if (firstItem.exists()) {
      await firstItem.trigger('click');
    }
    await flushPromises();

    // Debug toggle buttons should only be on assistant messages
    const allMsgs = wrapper.findAll('[data-testid="session-msg"]');
    const debugToggles = wrapper.findAll('[data-testid="session-debug-toggle"]');
    expect(debugToggles.length).toBeGreaterThanOrEqual(1);
    // Every message with a debug toggle should be assistant role
    for (let i = 0; i < allMsgs.length; i++) {
      const msgText = allMsgs[i].text();
      const hasToggle = msgText.includes('Debug') || msgText.includes('Generation Pipeline');
      const isUser = msgText.includes('How do I install') || msgText.includes('What are system');
      if (isUser) {
        expect(hasToggle).toBe(false);
      }
    }
  });
});
