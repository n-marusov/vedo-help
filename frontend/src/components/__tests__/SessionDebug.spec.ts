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
  it('loads all sessions on mount', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue(mockSessions);

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    await flushPromises();

    expect(api.adminSearchSessions).toHaveBeenCalledTimes(1);
    expect(api.adminSearchSessions).toHaveBeenCalledWith({
      search: undefined,
      user_name: undefined,
      from: undefined,
      to: undefined,
    });
    const items = wrapper.findAll('[data-testid="session-list-item"]');
    expect(items.length).toBe(2);
  });

  it('renders session list after search', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue(mockSessions);

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    await flushPromises();
    vi.mocked(api.adminSearchSessions).mockClear();
    vi.mocked(api.adminSearchSessions).mockResolvedValue(mockSessions);

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

  it('user name filter input is present and calls API', async () => {
    vi.mocked(api.adminSearchSessions).mockResolvedValue([]);

    const wrapper = mount(SessionDebug, {
      global: {
        stubs: {
          SessionDebug: false,
        },
      },
    });

    // Check the user ID filter input exists
    const userIdInput = wrapper.find('[data-testid="session-debug-user-search"]');
    expect(userIdInput.exists()).toBe(true);

    // Type a user ID and verify API is called with it
    await userIdInput.setValue('user-123');
    await userIdInput.trigger('input');
    await flushPromises();

    expect(api.adminSearchSessions).toHaveBeenCalledWith(
      expect.objectContaining({ user_name: 'user-123' }),
    );
  });

  it('dynamic step status shows active for steps with data', async () => {
    const debugMsg = {
      id: 'msg-debug',
      session_id: 'sess-1',
      role: 'assistant' as const,
      content: 'Test response with all pipeline steps.',
      created_at: '2026-06-24T10:00:05Z',
      debug_data: JSON.stringify({
        query_text: 'How do I install VEDO?',
        multi_query: {
          original_query: 'How do I install VEDO?',
          variants: ['How to install VEDO?', 'VEDO installation steps', 'Setup VEDO guide'],
          latency_ms: 200,
        },
        hyde: {
          per_query: [
            {
              query: 'How to install VEDO?',
              hypothetical_doc: 'To install VEDO, run docker compose up...',
              latency_ms: 500,
            },
          ],
        },
        embedding_search: {
          query_snippet: 'How do I install VEDO?',
          embedding_dimension: 384,
          latency_ms: 45,
          collection_name: 'abc-123',
          top_k: 5,
          result_count: 3,
          retries: 0,
          results: [],
        },
        keyword_search: {
          query_tokens: ['install', 'vedo'],
          total_matches: 2,
          results: [],
          latency_ms: 10,
        },
        merge_dedup: {
          input_chunks: 10,
          after_dedup: 7,
          source_breakdown: { vector_chunks: 5, keyword_chunks: 2 },
        },
        reranking: {
          input_count: 7,
          accepted: 5,
          rejected: 2,
          results: [],
        },
        final_answer: {
          model: 'gpt-4',
          max_retries: 3,
          chunks_in_context: 5,
          history_message_count: 0,
          history_token_estimate: 0,
          token_budget: 4000,
          total_tokens_estimate: 2000,
          latency_ms: 1200,
          prompt_preview: 'Answer...',
        },
      }),
    };

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
      messages: [debugMsg],
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

    // Toggle debug panel
    const debugToggle = wrapper.find('[data-testid="session-debug-toggle"]');
    if (debugToggle.exists()) {
      await debugToggle.trigger('click');
    }
    await flushPromises();

    // Verify all 7 step titles visible
    const stepTitles = wrapper.findAll('[data-testid="debug-step-title"]');
    expect(stepTitles.length).toBe(7);

    // Verify that steps with data show active badges (no "v0.5" future badges)
    // All 7 steps have data now, but only embedding_search (3) and final_answer (7)
    // are in the mock. The new logic shows "active" when getStepData returns data.
    const futureBadges = wrapper.findAll('[data-testid="debug-step-future"]');
    // Steps without data in the mock will show v0.5 badge (none if all have data)
    // Actually the test mock now has ALL 7 steps populated
    expect(futureBadges.length).toBe(0);
  });

  it('renders multi-query and hyde step data when present', async () => {
    const fullDebugMsg = {
      id: 'msg-full',
      session_id: 'sess-1',
      role: 'assistant' as const,
      content: 'Test response.',
      created_at: '2026-06-24T10:00:05Z',
      debug_data: JSON.stringify({
        query_text: 'Test query',
        multi_query: {
          original_query: 'Test query',
          variants: ['Variant 1', 'Variant 2'],
          latency_ms: 150,
        },
        hyde: {
          per_query: [
            {
              query: 'Variant 1',
              hypothetical_doc: 'Hypothetical document about variant 1.',
              latency_ms: 300,
            },
          ],
        },
        embedding_search: null,
        keyword_search: null,
        merge_dedup: null,
        reranking: null,
        final_answer: null,
      }),
    };

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
      messages: [fullDebugMsg],
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

    // Steps 1 (multi_query) and 2 (hyde) should have active badge
    // Steps 3-7 should have v0.5 future badge (except steps 3 and 7 which have status: 'active')
    // Actual: steps 4,5,6 have future badges = 3
    const futureBadges = wrapper.findAll('[data-testid="debug-step-future"]');
    expect(futureBadges.length).toBe(3); // keyword_search, merge_dedup, reranking have no data + future status
  });
});
