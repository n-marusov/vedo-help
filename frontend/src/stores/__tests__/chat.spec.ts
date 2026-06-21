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
  });

  // --------------------------------------------------------------------------
  // editMessage
  // --------------------------------------------------------------------------

  it.skip('editMessage calls api.patch and replaces content in messages.value', async () => {
    const store = useChatStore();
    store.messages = [
      {
        id: 'msg-1',
        session_id: 'sess-1',
        role: 'user',
        content: 'old',
        created_at: '2026-06-21T00:00:00Z',
      },
    ];

    apiMock.patch.mockResolvedValue({
      id: 'msg-1',
      session_id: 'sess-1',
      role: 'user',
      content: 'new content',
      edited_at: '2026-06-21T01:00:00Z',
      original_content: 'old',
      created_at: '2026-06-21T00:00:00Z',
    });

    await store.editMessage('sess-1', 'msg-1', 'new content');
    expect(apiMock.patch).toHaveBeenCalledWith('/sessions/sess-1/messages/msg-1', {
      content: 'new content',
    });
    expect(store.messages[0].content).toBe('new content');
    expect(store.messages[0].edited_at).toBe('2026-06-21T01:00:00Z');
  });

  // --------------------------------------------------------------------------
  // deleteMessage
  // --------------------------------------------------------------------------

  it.skip('deleteMessage optimistically removes and reverts on API error', async () => {
    const store = useChatStore();
    store.messages = [
      {
        id: 'msg-1',
        session_id: 'sess-1',
        role: 'user',
        content: 'a',
        created_at: '2026-06-21T00:00:00Z',
      },
      {
        id: 'msg-2',
        session_id: 'sess-1',
        role: 'assistant',
        content: 'b',
        created_at: '2026-06-21T00:01:00Z',
      },
    ];

    apiMock.del.mockResolvedValue({});

    await store.deleteMessage('sess-1', 'msg-1');
    // Optimistic: msg-1 removed
    expect(store.messages.map((m) => m.id)).toEqual(['msg-2']);

    // Now simulate API failure and check revert
    apiMock.del.mockRejectedValue(new ApiError(500, 'API Error'));
    store.messages = [
      {
        id: 'msg-1',
        session_id: 'sess-1',
        role: 'user',
        content: 'a',
        created_at: '2026-06-21T00:00:00Z',
      },
    ];
    await store.deleteMessage('sess-1', 'msg-1');
    // After revert: message is back
    expect(store.messages.map((m) => m.id)).toContain('msg-1');
  });

  // --------------------------------------------------------------------------
  // exportSession
  // --------------------------------------------------------------------------

  it.skip('exportSession triggers blob download and sets isExporting', async () => {
    const store = useChatStore();
    const blobContent = '# Session Title\n\n## user\nhello';
    const blob = new Blob([blobContent], { type: 'text/markdown' });

    // Mock URL.createObjectURL and <a download> click
    const createObjectURL = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:mock');
    const revokeObjectURL = vi.spyOn(URL, 'revokeObjectURL').mockReturnValue();

    // Mock global fetch for the export-session blob
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      blob: () => Promise.resolve(blob),
    });

    await store.exportSession('sess-1', 'md');
    expect(globalThis.fetch).toHaveBeenCalledWith(
      '/api/sessions/sess-1/export?format=md',
      expect.any(Object),
    );
    expect(createObjectURL).toHaveBeenCalled();
    expect(revokeObjectURL).toHaveBeenCalled();
    expect(store.isExporting).toBe(false);

    vi.restoreAllMocks();
  });

  // --------------------------------------------------------------------------
  // loadSession — sets isSessionLoading
  // --------------------------------------------------------------------------

  it.skip('loadSession sets isSessionLoading during fetch then false after', async () => {
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

    apiMock.get.mockResolvedValue(messages);

    const promise = store.loadSession('sess-1');
    expect(store.isSessionLoading).toBe(true);
    await promise;
    expect(store.isSessionLoading).toBe(false);
    expect(store.messages).toEqual(messages);
  });

  // --------------------------------------------------------------------------
  // sendMessage — temp-ID reconciliation on done event
  // --------------------------------------------------------------------------

  it.skip('sendMessage reconciles temp IDs on done event', async () => {
    const store = useChatStore();
    store.activeSessionId = 'sess-1';

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
});
