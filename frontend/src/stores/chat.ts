import { ApiError, api, getAccessToken } from '@/api/client';
import type {
  EditMessageRequest,
  Message,
  Session,
  SessionSummary,
  StreamEvent,
} from '@/api/types';
import { defineStore } from 'pinia';
import { ref } from 'vue';

function normalizeStreamLine(line: string): string {
  const trimmed = line.trim();
  if (!trimmed || trimmed.startsWith(':')) return '';
  return trimmed.startsWith('data:') ? trimmed.slice(5).trim() : trimmed;
}

function isPendingMessageId(messageId: string): boolean {
  return messageId.startsWith('temp-');
}

function isUuid(messageId: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
    messageId,
  );
}

function canPersistMessageAction(messageId: string): boolean {
  return !isPendingMessageId(messageId) && isUuid(messageId);
}

export const useChatStore = defineStore('chat', () => {
  const messages = ref<Message[]>([]);
  const isLoading = ref(false);
  const activeSessionId = ref<string | null>(null);
  const sessions = ref<SessionSummary[]>([]);
  const error = ref<string | null>(null);
  const isSessionLoading = ref(false);
  const isExporting = ref(false);
  const isLoadingSessions = ref(false);

  let abortController: AbortController | null = null;

  function parseNDJSON(
    reader: ReadableStreamDefaultReader<Uint8Array>,
  ): ReadableStream<StreamEvent> {
    return new ReadableStream({
      async start(controller) {
        const decoder = new TextDecoder();
        let buffer = '';

        try {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            buffer += decoder.decode(value, { stream: true });
            const lines = buffer.split('\n');
            buffer = lines.pop() || '';

            for (const line of lines) {
              const trimmed = normalizeStreamLine(line);
              if (!trimmed) continue;
              try {
                const event: StreamEvent = JSON.parse(trimmed);
                controller.enqueue(event);
              } catch {
                // skip malformed lines
              }
            }
          }

          // Process remaining buffer
          const trimmed = normalizeStreamLine(buffer);
          if (trimmed) {
            try {
              const event: StreamEvent = JSON.parse(trimmed);
              controller.enqueue(event);
            } catch {
              // skip malformed
            }
          }
        } catch (err) {
          controller.error(err);
        } finally {
          controller.close();
        }
      },
    });
  }

  async function sendMessage(collectionId: string, query: string) {
    isLoading.value = true;
    error.value = null;
    abortController = new AbortController();

    // Add user message optimistically
    const tempUserMsg: Message = {
      id: `temp-${Date.now()}`,
      session_id: activeSessionId.value || '',
      role: 'user',
      content: query,
      created_at: new Date().toISOString(),
    };
    messages.value.push(tempUserMsg);

    // Add placeholder assistant message
    const assistantMsg: Message = {
      id: `temp-assist-${Date.now()}`,
      session_id: activeSessionId.value || '',
      role: 'assistant',
      content: '',
      created_at: new Date().toISOString(),
    };
    messages.value.push(assistantMsg);

    try {
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
      };
      const token = getAccessToken();
      if (token) {
        headers.Authorization = `Bearer ${token}`;
      }

      const body: Record<string, unknown> = {
        collection_id: collectionId,
        query,
      };
      if (activeSessionId.value) {
        body.session_id = activeSessionId.value;
      }

      const response = await fetch('/api/query', {
        method: 'POST',
        headers,
        body: JSON.stringify(body),
        signal: abortController.signal,
      });

      if (!response.ok) {
        throw new ApiError(response.status, `Query failed: ${response.statusText}`);
      }

      const reader = response.body?.getReader();
      if (!reader) throw new Error('No response body');

      const stream = parseNDJSON(reader);
      const streamReader = stream.getReader();
      let fullContent = '';
      let sources: string | undefined;

      while (true) {
        const { done, value } = await streamReader.read();
        if (done) break;

        switch (value.type) {
          case 'chunk': {
            const chunkText = value.data?.text || value.text || '';
            fullContent += chunkText;
            // Update the last assistant message content
            const lastMsg = messages.value[messages.value.length - 1];
            if (lastMsg?.role === 'assistant') {
              lastMsg.content = fullContent;
              // Force reactivity by replacing the array
              messages.value = [...messages.value];
            }
            break;
          }
          case 'sources':
            sources = JSON.stringify(value.data?.sources || value.sources);
            break;
          case 'error':
            error.value = value.data?.text || value.text || 'An error occurred';
            // Remove the placeholder assistant message
            messages.value.pop();
            break;
          case 'done': {
            // Finalize the assistant message
            const finalMsg = messages.value[messages.value.length - 1];
            if (finalMsg?.role === 'assistant') {
              finalMsg.content = fullContent;
              finalMsg.sources = sources;
              messages.value = [...messages.value];
            }

            // Temp-ID reconciliation: fields are inside `data` (Rust SSE format)
            const doneData = value.data || value;
            if (doneData.user_message_id || doneData.assistant_message_id) {
              for (let i = 0; i < messages.value.length; i++) {
                const msg = messages.value[i];
                if (msg.role === 'user' && msg.id.startsWith('temp-') && doneData.user_message_id) {
                  console.debug(
                    '[chat.sendMessage] reconciled temp IDs user=%s->%s',
                    msg.id,
                    doneData.user_message_id,
                  );
                  messages.value[i] = {
                    ...msg,
                    id: doneData.user_message_id,
                  };
                }
                if (
                  msg.role === 'assistant' &&
                  msg.id.startsWith('temp-assist-') &&
                  doneData.assistant_message_id
                ) {
                  console.debug(
                    '[chat.sendMessage] reconciled temp IDs assist=%s->%s',
                    msg.id,
                    doneData.assistant_message_id,
                  );
                  messages.value[i] = {
                    ...msg,
                    id: doneData.assistant_message_id,
                  };
                }
              }
            }
            break;
          }
        }
      }

      // Refresh sessions after a new query (might have created a session)
      await fetchSessions();
    } catch (err) {
      if (err instanceof Error && err.name === 'AbortError') {
        // User cancelled
      } else if (err instanceof ApiError) {
        error.value = err.message;
      } else if (err instanceof Error) {
        error.value = err.message;
      }
      // Remove the placeholder assistant message on error
      if (
        messages.value.length > 0 &&
        messages.value[messages.value.length - 1]?.role === 'assistant'
      ) {
        messages.value.pop();
      }
    } finally {
      isLoading.value = false;
      abortController = null;
    }
  }

  function cancelStream() {
    if (abortController) {
      abortController.abort();
      abortController = null;
    }
  }

  async function fetchSessions() {
    isLoadingSessions.value = true;
    try {
      const result = await api.get<SessionSummary[]>('/sessions');
      sessions.value = result;
      console.debug('[chat.fetchSessions] loaded %d sessions', result.length);
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    } finally {
      isLoadingSessions.value = false;
    }
  }

  async function createSession(collectionId?: string) {
    try {
      const session = await api.post<Session>('/sessions', {
        collection_id: collectionId,
      });
      activeSessionId.value = session.id;
      messages.value = [];
      await fetchSessions();
      return session;
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
      return null;
    }
  }

  async function deleteSession(sessionId: string) {
    try {
      await api.del(`/sessions/${sessionId}`);
      if (activeSessionId.value === sessionId) {
        activeSessionId.value = null;
        messages.value = [];
      }
      await fetchSessions();
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    }
  }

  async function loadSession(sessionId: string) {
    isSessionLoading.value = true;
    try {
      const result = await api.get<{ session: Session; messages: Message[] }>(
        `/sessions/${sessionId}`,
      );
      messages.value = result.messages;
      activeSessionId.value = sessionId;
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    } finally {
      isSessionLoading.value = false;
    }
  }

  async function editMessage(sessionId: string, messageId: string, content: string) {
    console.debug('[chat.editMessage] session=%s msg=%s', sessionId, messageId);
    if (!canPersistMessageAction(messageId)) {
      console.warn(
        '[FIX:chat-temp-id] skipped edit for pending/non-UUID message id=%s session=%s',
        messageId,
        sessionId,
      );
      return;
    }

    try {
      const req: EditMessageRequest = { content };
      const updated = await api.editMessage(sessionId, messageId, req);
      const idx = messages.value.findIndex((m) => m.id === messageId);
      if (idx !== -1) {
        messages.value[idx] = { ...messages.value[idx], ...updated };
      }
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    }
  }

  async function deleteMessage(sessionId: string, messageId: string) {
    if (!canPersistMessageAction(messageId)) {
      console.warn(
        '[FIX:chat-temp-id] skipped delete for pending/non-UUID message id=%s session=%s',
        messageId,
        sessionId,
      );
      return;
    }

    const idx = messages.value.findIndex((m) => m.id === messageId);
    if (idx === -1) return;

    // Optimistic remove
    const prev = messages.value[idx];
    messages.value.splice(idx, 1);
    console.debug('[chat.deleteMessage] optimistic remove idx=%d', idx);

    try {
      await api.deleteMessage(sessionId, messageId);
    } catch (err) {
      // Revert on failure
      messages.value.splice(idx, 0, prev);
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    }
  }

  async function exportSession(sessionId: string, format: 'md' | 'json') {
    isExporting.value = true;
    console.debug('[chat.exportSession] format=%s', format);
    try {
      const blob = await api.exportSession(sessionId, format);
      const url = URL.createObjectURL(blob);
      const extension = format === 'md' ? 'md' : 'json';
      const a = document.createElement('a');
      a.href = url;
      a.download = `session-${sessionId}.${extension}`;
      a.click();
      URL.revokeObjectURL(url);
      console.debug('[chat.exportSession] format=%s bytes=%d', format, blob.size);
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    } finally {
      isExporting.value = false;
    }
  }

  function clearMessages() {
    messages.value = [];
    activeSessionId.value = null;
    error.value = null;
  }

  return {
    messages,
    isLoading,
    activeSessionId,
    sessions,
    error,
    isSessionLoading,
    isExporting,
    isLoadingSessions,
    sendMessage,
    cancelStream,
    fetchSessions,
    createSession,
    deleteSession,
    loadSession,
    editMessage,
    deleteMessage,
    exportSession,
    clearMessages,
    canPersistMessageAction,
  };
});
