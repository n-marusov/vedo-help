import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// RED phase: chat store actions (editMessage, deleteMessage, exportSession,
// loadSession enhancements, sendMessage temp-ID reconciliation) don't exist
// yet — they land in T11. All tests are `.skip` until then.
// The mock follows the existing documents.spec.ts pattern.

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
  editMessage: vi.fn(),
  deleteMessage: vi.fn(),
  exportSession: vi.fn(),
  get: vi.fn(),
  post: vi.fn(),
  del: vi.fn(),
  patch: vi.fn(),
  getSessionWithMessages: vi.fn(),
  generateSessionTitle: vi.fn(),
}));

vi.mock('@/api/client', () => ({
  api: apiMock,
  ApiError,
  getAccessToken: vi.fn(() => 'mock-token'),
}));

vi.mock('@/api/types', () => ({
  // Re-export types consumed by the store — the store is imported directly so
  // types must exist at runtime for the mock to work. Actual types are already
  // exported from types.ts and available via the import.
}));

import { useChatStore } from '@/stores/chat';

describe('chat store — v0.3.1 actions (RED)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    // jsdom does not provide navigator.clipboard
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: vi.fn() },
      writable: true,
      configurable: true,
    });
  });

  it('editMessage skips pending temp IDs before calling API', async () => {
    const store = useChatStore();
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    store.messages = [
      {
        id: 'temp-1782052621000',
        session_id: 'sess-1',
        role: 'user',
        content: 'draft',
        created_at: '2026-06-21T00:00:00Z',
      },
    ];

    await store.editMessage('sess-1', 'temp-1782052621000', 'updated');

    expect(apiMock.editMessage).not.toHaveBeenCalled();
    expect(store.messages[0].content).toBe('draft');
    expect(warnSpy).not.toHaveBeenCalled();
    warnSpy.mockRestore();
  });

  it('deleteMessage skips pending temp IDs before optimistic removal or API call', async () => {
    const store = useChatStore();
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    store.messages = [
      {
        id: 'temp-1782052621000',
        session_id: 'sess-1',
        role: 'user',
        content: 'draft',
        created_at: '2026-06-21T00:00:00Z',
      },
    ];

    await store.deleteMessage('sess-1', 'temp-1782052621000');

    expect(apiMock.deleteMessage).not.toHaveBeenCalled();
    expect(store.messages).toHaveLength(1);
    expect(store.messages[0].id).toBe('temp-1782052621000');
    expect(warnSpy).not.toHaveBeenCalled();
    warnSpy.mockRestore();
  });

  // --------------------------------------------------------------------------
  // editMessage
  // --------------------------------------------------------------------------

  it('editMessage calls api.editMessage and replaces content in messages.value', async () => {
    const store = useChatStore();
    const msgId = '550e8400-e29b-41d4-a716-446655440000';
    store.messages = [
      {
        id: msgId,
        session_id: 'sess-1',
        role: 'user',
        content: 'old',
        created_at: '2026-06-21T00:00:00Z',
      },
    ];

    apiMock.editMessage.mockResolvedValue({
      id: msgId,
      session_id: 'sess-1',
      role: 'user',
      content: 'new content',
      edited_at: '2026-06-21T01:00:00Z',
      original_content: 'old',
      created_at: '2026-06-21T00:00:00Z',
    });

    await store.editMessage('sess-1', msgId, 'new content');
    expect(apiMock.editMessage).toHaveBeenCalledWith('sess-1', msgId, {
      content: 'new content',
    });
    expect(store.messages[0].content).toBe('new content');
    expect(store.messages[0].edited_at).toBe('2026-06-21T01:00:00Z');
  });

  // --------------------------------------------------------------------------
  // deleteMessage
  // --------------------------------------------------------------------------

  it('deleteMessage optimistically removes and reverts on API error', async () => {
    const store = useChatStore();
    const msgId1 = '550e8400-e29b-41d4-a716-446655440010';
    const msgId2 = '550e8400-e29b-41d4-a716-446655440011';
    store.messages = [
      {
        id: msgId1,
        session_id: 'sess-1',
        role: 'user',
        content: 'a',
        created_at: '2026-06-21T00:00:00Z',
      },
      {
        id: msgId2,
        session_id: 'sess-1',
        role: 'assistant',
        content: 'b',
        created_at: '2026-06-21T00:01:00Z',
      },
    ];

    apiMock.deleteMessage.mockResolvedValue({});

    await store.deleteMessage('sess-1', msgId1);
    // Optimistic: msgId1 removed
    expect(store.messages.map((m) => m.id)).toEqual([msgId2]);

    // Now simulate API failure and check revert
    apiMock.deleteMessage.mockRejectedValue(new ApiError(500, 'API Error'));
    store.messages = [
      {
        id: msgId1,
        session_id: 'sess-1',
        role: 'user',
        content: 'a',
        created_at: '2026-06-21T00:00:00Z',
      },
    ];
    await store.deleteMessage('sess-1', msgId1);
    // After revert: message is back
    expect(store.messages.map((m) => m.id)).toContain(msgId1);
  });

  // --------------------------------------------------------------------------
  // exportSession
  // --------------------------------------------------------------------------

  it('exportSession triggers blob download and sets isExporting', async () => {
    const store = useChatStore();
    const blobContent = '# Session Title\n\n## user\nhello';
    const blob = new Blob([blobContent], { type: 'text/markdown' });

    // Mock URL.createObjectURL and <a download> click
    URL.createObjectURL = vi.fn().mockReturnValue('blob:mock');
    URL.revokeObjectURL = vi.fn();

    // Mock api.exportSession to return blob directly
    apiMock.exportSession.mockResolvedValue(blob);

    await store.exportSession('sess-1', 'md');
    expect(apiMock.exportSession).toHaveBeenCalledWith('sess-1', 'md');
    expect(URL.createObjectURL).toHaveBeenCalled();
    expect(URL.revokeObjectURL).toHaveBeenCalled();
    expect(store.isExporting).toBe(false);
  });

  // --------------------------------------------------------------------------
  // loadSession — sets isSessionLoading
  // --------------------------------------------------------------------------

  it('loadSession sets isSessionLoading during fetch then false after', async () => {
    const store = useChatStore();
    const messages = [
      {
        id: 'msg-1',
        session_id: 'sess-1',
        role: 'user',
        content: 'hi',
        created_at: '2026-06-21T00:00:00Z',
      },
    ];
    const response = {
      session: {
        id: 'sess-1',
        title: 'Test',
        created_at: '2026-06-21T00:00:00Z',
      },
      messages,
    };

    apiMock.get.mockResolvedValue(response);

    const promise = store.loadSession('sess-1');
    expect(store.isSessionLoading).toBe(true);
    await promise;
    expect(store.isSessionLoading).toBe(false);
    expect(store.messages).toEqual(messages);
  });

  // --------------------------------------------------------------------------
  // sendMessage — temp-ID reconciliation on done event
  // --------------------------------------------------------------------------

  it('sendMessage reconciles temp IDs on done event', async () => {
    const store = useChatStore();
    store.activeSessionId = 'sess-1';

    // Stub fetchSessions to avoid extra network calls
    apiMock.get.mockResolvedValue([]);

    // Stub fetch to produce a mock stream that includes server IDs in done
    const encoder = new TextEncoder();
    const donePayload = JSON.stringify({
      type: 'done',
      user_message_id: 'server-user-1',
      assistant_message_id: 'server-asst-1',
    });

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      body: new ReadableStream({
        start(controller) {
          controller.enqueue(encoder.encode(`data: ${donePayload}\n`));
          controller.close();
        },
      }),
    });

    await store.sendMessage('col-1', 'my query');

    // After done, the temp IDs should be replaced with server IDs
    const userMsg = store.messages.find((m) => m.role === 'user');
    const asstMsg = store.messages.find((m) => m.role === 'assistant');
    expect(userMsg?.id).toBe('server-user-1');
    expect(asstMsg?.id).toBe('server-asst-1');
  });

  // --------------------------------------------------------------------------
  // Chat UI polish: renameSession
  // --------------------------------------------------------------------------

  it('renameSession calls api.patch and updates session title', async () => {
    const store = useChatStore();
    store.sessions = [
      {
        id: 'sess-1',
        title: 'Old Title',
        message_count: 3,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];

    apiMock.patch.mockResolvedValue({
      id: 'sess-1',
      title: 'New Title',
      message_count: 3,
      created_at: '2026-06-23T00:00:00Z',
      updated_at: '2026-06-23T01:00:00Z',
    });

    await store.renameSession('sess-1', 'New Title');
    expect(apiMock.patch).toHaveBeenCalledWith('/sessions/sess-1', {
      title: 'New Title',
    });
    expect(store.sessions[0].title).toBe('New Title');
  });

  it('renameSession stores error on API failure', async () => {
    const store = useChatStore();
    store.sessions = [
      {
        id: 'sess-1',
        title: 'Old',
        message_count: 1,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];

    apiMock.patch.mockRejectedValue(new ApiError(500, 'Rename failed'));

    await store.renameSession('sess-1', 'New');
    expect(store.sessions[0].title).toBe('Old');
    expect(store.error).toBe('Rename failed');
  });

  // --------------------------------------------------------------------------
  // Chat UI polish: togglePinSession
  // --------------------------------------------------------------------------

  it('togglePinSession calls api.patch and flips pinned state', async () => {
    const store = useChatStore();
    store.sessions = [
      {
        id: 'sess-1',
        title: 'Chat',
        message_count: 2,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
        pinned: false,
      },
    ];

    apiMock.patch.mockResolvedValue({
      id: 'sess-1',
      title: 'Chat',
      pinned: true,
      message_count: 2,
      created_at: '2026-06-23T00:00:00Z',
      updated_at: '2026-06-23T01:00:00Z',
    });

    await store.togglePinSession('sess-1');
    expect(apiMock.patch).toHaveBeenCalledWith('/sessions/sess-1', {
      pinned: true,
    });
    expect(store.sessions[0].pinned).toBe(true);
  });

  it('togglePinSession stores error on API failure and reverts pin state', async () => {
    const store = useChatStore();
    store.sessions = [
      {
        id: 'sess-1',
        title: 'Chat',
        message_count: 2,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
        pinned: false,
      },
    ];

    // Optimistic: flip to true, then revert on failure
    apiMock.patch.mockRejectedValue(new ApiError(500, 'Pin failed'));

    await store.togglePinSession('sess-1');
    expect(store.error).toBe('Pin failed');
    // Should revert back to false after failure
    expect(store.sessions[0].pinned).toBe(false);
  });

  // --------------------------------------------------------------------------
  // Chat UI polish: regenerateMessage
  // --------------------------------------------------------------------------

  it('regenerateMessage resends last user query and replaces assistant response', async () => {
    const store = useChatStore();
    store.activeSessionId = 'sess-1';
    store.messages = [
      {
        id: 'msg-1',
        session_id: 'sess-1',
        role: 'user',
        content: 'my question',
        created_at: '2026-06-23T00:00:00Z',
      },
      {
        id: 'msg-2',
        session_id: 'sess-1',
        role: 'assistant',
        content: 'old answer',
        created_at: '2026-06-23T00:01:00Z',
      },
    ];

    const encoder = new TextEncoder();
    const donePayload = JSON.stringify({
      type: 'done',
      user_message_id: 'msg-1',
      assistant_message_id: 'msg-3',
    });
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      body: new ReadableStream({
        start(controller) {
          controller.enqueue(encoder.encode(`data: ${donePayload}\n`));
          controller.close();
        },
      }),
    });
    // Stub fetchSessions to avoid extra network calls
    apiMock.get.mockResolvedValue([]);

    store.lastCollectionId = 'col-1';

    await store.regenerateMessage('msg-2');

    // Should have called POST /api/query with the last user query
    expect(globalThis.fetch).toHaveBeenCalledWith(
      '/api/query',
      expect.objectContaining({
        method: 'POST',
        body: expect.stringContaining('my question'),
      }),
    );
    // The old assistant message should have been replaced
    const asstMsg = store.messages.find((m) => m.role === 'assistant');
    expect(asstMsg).toBeDefined();
    expect(asstMsg?.id).toBe('msg-3');
  });

  it('regenerateMessage stores error on fetch failure', async () => {
    const store = useChatStore();
    store.activeSessionId = 'sess-1';
    store.messages = [
      {
        id: 'msg-1',
        session_id: 'sess-1',
        role: 'user',
        content: 'my question',
        created_at: '2026-06-23T00:00:00Z',
      },
      {
        id: 'msg-2',
        session_id: 'sess-1',
        role: 'assistant',
        content: 'old answer',
        created_at: '2026-06-23T00:01:00Z',
      },
    ];

    globalThis.fetch = vi.fn().mockRejectedValue(new Error('Network error'));

    await store.regenerateMessage('msg-2');
    expect(store.error).toBeTruthy();
  });

  // --------------------------------------------------------------------------
  // Chat UI polish: copyMessage
  // --------------------------------------------------------------------------

  it('copyMessage copies message content to clipboard', async () => {
    const store = useChatStore();
    const writeTextSpy = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue(undefined);

    store.messages = [
      {
        id: 'msg-1',
        session_id: 'sess-1',
        role: 'user',
        content: 'copy this text',
        created_at: '2026-06-23T00:00:00Z',
      },
    ];

    await store.copyMessage('msg-1');
    expect(writeTextSpy).toHaveBeenCalledWith('copy this text');
    writeTextSpy.mockRestore();
  });

  it('copyMessage handles missing message gracefully', async () => {
    const store = useChatStore();
    const writeTextSpy = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue(undefined);

    store.messages = [];
    await store.copyMessage('non-existent-id');
    expect(writeTextSpy).not.toHaveBeenCalled();
    writeTextSpy.mockRestore();
  });

  it('copyMessage handles clipboard API failure gracefully', async () => {
    const store = useChatStore();
    const writeTextSpy = vi
      .spyOn(navigator.clipboard, 'writeText')
      .mockRejectedValue(new Error('Clipboard blocked'));
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => undefined);

    store.messages = [
      {
        id: 'msg-1',
        session_id: 'sess-1',
        role: 'user',
        content: 'text',
        created_at: '2026-06-23T00:00:00Z',
      },
    ];

    await store.copyMessage('msg-1');
    expect(writeTextSpy).toHaveBeenCalled();
    expect(warnSpy).toHaveBeenCalled();
    writeTextSpy.mockRestore();
    warnSpy.mockRestore();
  });

  // --------------------------------------------------------------------------
  // Chat UI polish: searchSessions
  // --------------------------------------------------------------------------

  it('setSearchQuery filters sessions by title', () => {
    const store = useChatStore();
    store.sessions = [
      {
        id: 'sess-1',
        title: 'How to deploy',
        message_count: 3,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
      {
        id: 'sess-2',
        title: 'API documentation',
        message_count: 5,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
      {
        id: 'sess-3',
        title: 'Deploy to production',
        message_count: 2,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];

    store.setSearchQuery('deploy');
    const result = store.filteredSessions;
    expect(result).toHaveLength(2);
    expect(result.map((s) => s.id)).toEqual(['sess-1', 'sess-3']);
  });

  it('setSearchQuery returns all sessions when query is empty', () => {
    const store = useChatStore();
    store.sessions = [
      {
        id: 'sess-1',
        title: 'Chat 1',
        message_count: 1,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
      {
        id: 'sess-2',
        title: 'Chat 2',
        message_count: 2,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];

    store.setSearchQuery('');
    expect(store.filteredSessions).toHaveLength(2);
    store.setSearchQuery('');
    expect(store.filteredSessions).toHaveLength(2);
  });

  it('setSearchQuery is case-insensitive', () => {
    const store = useChatStore();
    store.sessions = [
      {
        id: 'sess-1',
        title: 'How to DEPLOY',
        message_count: 1,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
      {
        id: 'sess-2',
        title: 'Getting started',
        message_count: 2,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
      },
    ];

    store.setSearchQuery('deploy');
    expect(store.filteredSessions).toHaveLength(1);
    store.setSearchQuery('DEPLOY');
    expect(store.filteredSessions).toHaveLength(1);
  });

  // --------------------------------------------------------------------------
  // Pipeline persistence: cancelStream clears localStorage
  // --------------------------------------------------------------------------

  it('cancelStream clears pipeline state from localStorage', () => {
    const store = useChatStore();

    localStorage.setItem('chat_pipeline_active', 'true');
    localStorage.setItem('chat_pipeline_session_id', 'sess-1');
    localStorage.setItem('chat_pipeline_collection_id', 'col-1');
    localStorage.setItem('chat_pipeline_stage', 'generating');
    localStorage.setItem('chat_pipeline_temp_title', 'test');
    localStorage.setItem('chat_pipeline_user_query', 'test query');

    store.cancelStream();

    expect(localStorage.getItem('chat_pipeline_active')).toBeNull();
    expect(localStorage.getItem('chat_pipeline_session_id')).toBeNull();
    expect(localStorage.getItem('chat_pipeline_collection_id')).toBeNull();
    expect(localStorage.getItem('chat_pipeline_stage')).toBeNull();
    expect(localStorage.getItem('chat_pipeline_temp_title')).toBeNull();
    expect(localStorage.getItem('chat_pipeline_user_query')).toBeNull();
  });

  // --------------------------------------------------------------------------
  // Pipeline persistence: checkPendingPipeline
  // --------------------------------------------------------------------------

  it('checkPendingPipeline does nothing when no state in localStorage', async () => {
    const store = useChatStore();
    await store.checkPendingPipeline();
    expect(store.activeSessionId).toBeNull();
    expect(store.isLoading).toBe(false);
    expect(store.pipelineStage).toBeNull();
  });

  it('checkPendingPipeline restores state from localStorage synchronously', () => {
    const store = useChatStore();
    store.sessions = [
      {
        id: 'session-1',
        title: 'New Chat',
        collection_id: 'col-1',
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
        message_count: 0,
      },
    ];

    localStorage.setItem('chat_pipeline_active', 'true');
    localStorage.setItem('chat_pipeline_session_id', 'session-1');
    localStorage.setItem('chat_pipeline_collection_id', 'col-1');
    localStorage.setItem('chat_pipeline_stage', 'embedding');
    localStorage.setItem('chat_pipeline_temp_title', 'test query');
    localStorage.setItem('chat_pipeline_user_query', 'What is RAG?');

    // Call without await — synchronous restoration happens before first await
    store.checkPendingPipeline();

    // Synchronous state restoration (before dynamic import & timers)
    expect(store.activeSessionId).toBe('session-1');
    // The original stage was saved before reload, but the new page cannot receive
    // further SSE stage events. Recovery uses a neutral progress label while polling.
    expect(store.pipelineStage).toBe('generating');
    expect(store.isLoading).toBe(true);
    expect(store.messages.length).toBe(2);
    expect(store.messages[0].role).toBe('user');
    expect(store.messages[0].content).toBe('What is RAG?');
    expect(store.messages[1].role).toBe('assistant');
    expect(store.messages[1].content).toBe('');
    expect(store.lastCollectionId).toBe('col-1');
    // Session title restored from localStorage, not default "New Chat" from backend
    const restoredSession = store.sessions.find((s) => s.id === 'session-1');
    expect(restoredSession?.title).toBe('test query');
    expect(restoredSession?.title).not.toBe('New Chat');
  });

  it('checkPendingPipeline recovers messages via polling', async () => {
    const store = useChatStore();
    store.sessions = [
      {
        id: 'session-1',
        title: 'New Chat',
        collection_id: 'col-1',
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
        message_count: 0,
      },
    ];

    // Mock the polling API to return completed messages
    apiMock.getSessionWithMessages.mockResolvedValue({
      session: {
        id: 'session-1',
        title: 'New Chat',
        collection_id: 'col-1',
      },
      messages: [
        {
          id: 'user-1',
          session_id: 'session-1',
          role: 'user',
          content: 'What is RAG?',
          created_at: '2026-06-23T00:00:00Z',
        },
        {
          id: 'asst-1',
          session_id: 'session-1',
          role: 'assistant',
          content: 'RAG stands for Retrieval-Augmented Generation',
          sources: undefined,
          created_at: '2026-06-23T00:01:00Z',
        },
      ],
    });

    // Mock fetchSessions called after recovery
    apiMock.get.mockResolvedValue([]);

    localStorage.setItem('chat_pipeline_active', 'true');
    localStorage.setItem('chat_pipeline_session_id', 'session-1');
    localStorage.setItem('chat_pipeline_collection_id', 'col-1');
    localStorage.setItem('chat_pipeline_stage', 'searching');
    localStorage.setItem('chat_pipeline_temp_title', 'test');
    localStorage.setItem('chat_pipeline_user_query', 'What is RAG?');

    store.checkPendingPipeline();

    // Wait for the 2s polling interval to fire and recover messages
    await vi.waitFor(
      () => {
        expect(store.isLoading).toBe(false);
      },
      { timeout: 5000, interval: 200 },
    );

    expect(store.pipelineStage).toBeNull();
    expect(store.messages.length).toBe(2);
    expect(store.messages[1].content).toBe('RAG stands for Retrieval-Augmented Generation');
    expect(localStorage.getItem('chat_pipeline_active')).toBeNull();
  }, 10000);

  it('checkPendingPipeline stops recovery when polling only returns the user message', async () => {
    vi.useFakeTimers();
    const store = useChatStore();
    store.sessions = [
      {
        id: 'session-user-only',
        title: 'New Chat',
        collection_id: 'col-1',
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
        message_count: 1,
      },
    ];

    apiMock.getSessionWithMessages.mockResolvedValue({
      session: {
        id: 'session-user-only',
        title: 'Recovered title',
        collection_id: 'col-1',
      },
      messages: [
        {
          id: 'user-1',
          session_id: 'session-user-only',
          role: 'user',
          content: 'What is RAG?',
          created_at: '2026-06-23T00:00:00Z',
        },
      ],
    });

    localStorage.setItem('chat_pipeline_active', 'true');
    localStorage.setItem('chat_pipeline_session_id', 'session-user-only');
    localStorage.setItem('chat_pipeline_collection_id', 'col-1');
    localStorage.setItem('chat_pipeline_stage', 'multi_query');
    localStorage.setItem('chat_pipeline_temp_title', 'test');
    localStorage.setItem('chat_pipeline_user_query', 'What is RAG?');

    store.checkPendingPipeline();
    expect(store.pipelineStage).toBe('generating');

    await vi.advanceTimersByTimeAsync(92000);

    expect(store.isLoading).toBe(false);
    expect(store.pipelineStage).toBeNull();
    expect(store.messages).toHaveLength(1);
    expect(store.error).toBe('Response generation was interrupted. Please retry the query.');
    expect(localStorage.getItem('chat_pipeline_active')).toBeNull();

    vi.useRealTimers();
  });

  it('checkPendingPipeline handles 404 from polling gracefully', async () => {
    const store = useChatStore();
    store.sessions = [
      {
        id: 'session-404',
        title: 'Deleted Session',
        collection_id: 'col-1',
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
        message_count: 0,
      },
    ];

    apiMock.getSessionWithMessages.mockRejectedValue(new ApiError(404, 'Session not found'));

    localStorage.setItem('chat_pipeline_active', 'true');
    localStorage.setItem('chat_pipeline_session_id', 'session-404');
    localStorage.setItem('chat_pipeline_collection_id', 'col-1');
    localStorage.setItem('chat_pipeline_stage', 'generating');
    localStorage.setItem('chat_pipeline_temp_title', 'test');
    localStorage.setItem('chat_pipeline_user_query', 'test query');

    store.checkPendingPipeline();

    // Wait for the 2s polling interval to fire; the 404 should clean up state
    await vi.waitFor(
      () => {
        expect(store.isLoading).toBe(false);
      },
      { timeout: 5000, interval: 200 },
    );

    expect(store.pipelineStage).toBeNull();
    expect(localStorage.getItem('chat_pipeline_active')).toBeNull();
  }, 10000);

  it('sendMessage saves then clears pipeline state in localStorage after done event', async () => {
    const store = useChatStore();
    store.activeSessionId = 'sess-1';

    // Stub fetch to produce a minimal stream (done event only)
    const encoder = new TextEncoder();
    const donePayload = JSON.stringify({ type: 'done' });
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      body: new ReadableStream({
        start(controller) {
          controller.enqueue(encoder.encode(`data: ${donePayload}\n`));
          controller.close();
        },
      }),
    });

    apiMock.get.mockResolvedValue([]);
    apiMock.generateSessionTitle.mockResolvedValue({ title: 'Generated Title' });

    await store.sendMessage('col-1', 'test query');

    expect(localStorage.getItem('chat_pipeline_active')).toBeNull();
    expect(localStorage.getItem('chat_pipeline_session_id')).toBeNull();
  });

  it('sendMessage preserves pipeline state when stream aborts during page reload', async () => {
    const store = useChatStore();
    store.activeSessionId = 'sess-1';

    globalThis.fetch = vi
      .fn()
      .mockRejectedValue(new DOMException('The operation was aborted.', 'AbortError'));

    await store.sendMessage('col-1', 'reload query');

    expect(localStorage.getItem('chat_pipeline_active')).toBe('true');
    expect(localStorage.getItem('chat_pipeline_session_id')).toBe('sess-1');
    expect(localStorage.getItem('chat_pipeline_collection_id')).toBe('col-1');
    expect(localStorage.getItem('chat_pipeline_temp_title')).toBe('reload query');
  });

  it('cancelStream clears pipeline state for explicit user cancellation', async () => {
    const store = useChatStore();
    store.activeSessionId = 'sess-1';

    let rejectFetch: (reason?: unknown) => void = () => undefined;
    globalThis.fetch = vi.fn().mockImplementation(
      () =>
        new Promise((_resolve, reject) => {
          rejectFetch = reject;
        }),
    );

    const sendPromise = store.sendMessage('col-1', 'cancel query');
    expect(localStorage.getItem('chat_pipeline_active')).toBe('true');

    store.cancelStream();
    rejectFetch(new DOMException('The operation was aborted.', 'AbortError'));
    await sendPromise;

    expect(localStorage.getItem('chat_pipeline_active')).toBeNull();
    expect(localStorage.getItem('chat_pipeline_session_id')).toBeNull();
  });
});
