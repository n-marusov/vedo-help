import { ApiError, api, getAccessToken } from '@/api/client';
import type {
  EditMessageRequest,
  Message,
  Session,
  SessionSummary,
  StreamEvent,
} from '@/api/types';
import { SeverityNumber, logger } from '@/telemetry';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

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

// ── Pipeline persistence across page reloads ──

const LS_PIPELINE_ACTIVE = 'chat_pipeline_active';
const LS_PIPELINE_SESSION_ID = 'chat_pipeline_session_id';
const LS_PIPELINE_COLLECTION_ID = 'chat_pipeline_collection_id';
const LS_PIPELINE_STAGE = 'chat_pipeline_stage';
const LS_PIPELINE_TEMP_TITLE = 'chat_pipeline_temp_title';
const LS_PIPELINE_USER_QUERY = 'chat_pipeline_user_query';
const RECOVERY_TOTAL_TIMEOUT_MS = 300000;
let pageIsUnloading = false;

if (typeof window !== 'undefined') {
  const markPageUnloading = () => {
    pageIsUnloading = true;
  };
  const markPageShown = () => {
    pageIsUnloading = false;
  };
  window.addEventListener('pagehide', markPageUnloading);
  window.addEventListener('beforeunload', markPageUnloading);
  window.addEventListener('pageshow', markPageShown);
}

function savePipelineState(sessionId: string, collectionId: string, query: string) {
  try {
    localStorage.setItem(LS_PIPELINE_ACTIVE, 'true');
    localStorage.setItem(LS_PIPELINE_SESSION_ID, sessionId);
    localStorage.setItem(LS_PIPELINE_COLLECTION_ID, collectionId);
    localStorage.setItem(LS_PIPELINE_STAGE, '');
    localStorage.setItem(LS_PIPELINE_TEMP_TITLE, query.slice(0, 45).trim());
    localStorage.setItem(LS_PIPELINE_USER_QUERY, query);
  } catch {
    // Storage can be disabled or full; recovery is best-effort.
  }
}

function updateSavedPipelineStage(stage: string) {
  if (localStorage.getItem(LS_PIPELINE_ACTIVE) === 'true') {
    localStorage.setItem(LS_PIPELINE_STAGE, stage);
  }
}

function clearPipelineState() {
  localStorage.removeItem(LS_PIPELINE_ACTIVE);
  localStorage.removeItem(LS_PIPELINE_SESSION_ID);
  localStorage.removeItem(LS_PIPELINE_COLLECTION_ID);
  localStorage.removeItem(LS_PIPELINE_STAGE);
  localStorage.removeItem(LS_PIPELINE_TEMP_TITLE);
  localStorage.removeItem(LS_PIPELINE_USER_QUERY);
}

export interface PendingPipelineState {
  sessionId: string;
  collectionId: string;
  stage: string;
  tempTitle: string;
  userQuery: string;
}

function getPipelineState(): PendingPipelineState | null {
  const active = localStorage.getItem(LS_PIPELINE_ACTIVE);
  if (active !== 'true') {
    return null;
  }
  const state = {
    sessionId: localStorage.getItem(LS_PIPELINE_SESSION_ID) || '',
    collectionId: localStorage.getItem(LS_PIPELINE_COLLECTION_ID) || '',
    stage: localStorage.getItem(LS_PIPELINE_STAGE) || '',
    tempTitle: localStorage.getItem(LS_PIPELINE_TEMP_TITLE) || 'New Chat',
    userQuery: localStorage.getItem(LS_PIPELINE_USER_QUERY) || '',
  };
  return state;
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
  const lastCollectionId = ref<string | null>(null);
  const searchQuery = ref('');
  const sidebarCollapsed = ref(false);
  const pipelineStage = ref<string | null>(null);
  const pipelineSessionId = ref<string | null>(null);

  const stageLabels: Record<string, string> = {
    embedding: 'Embedding query...',
    multi_query: 'Generating query variants...',
    hyde: 'Generating hypothetical document...',
    searching: 'Searching vector DB...',
    keyword_search: 'Searching by keywords...',
    reranking: 'Reranking results...',
    building_context: 'Building context...',
    generating: 'Generating response...',
  };

  const pipelineStageLabel = computed(() => {
    if (!pipelineStage.value) return null;
    return stageLabels[pipelineStage.value] || pipelineStage.value;
  });

  const filteredSessions = computed(() => {
    if (!searchQuery.value.trim()) return sessions.value;
    const q = searchQuery.value.toLowerCase();
    return sessions.value.filter((s) => s.title.toLowerCase().includes(q));
  });

  function getPeriodLabel(date: Date): { label: string; order: number } {
    const now = new Date();
    const startOfDay = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const startOfYesterday = new Date(startOfDay.getTime() - 86400000);
    const startOfWeek = new Date(startOfDay.getTime() - 6 * 86400000);

    if (date >= startOfDay) return { label: 'TODAY', order: 1 };
    if (date >= startOfYesterday) return { label: 'YESTERDAY', order: 2 };
    if (date >= startOfWeek) return { label: 'WEEK', order: 3 };

    // Same calendar month
    if (date.getMonth() === now.getMonth() && date.getFullYear() === now.getFullYear()) {
      return { label: 'MONTH', order: 4 };
    }

    // Older: format as YYYY-MM
    const month = String(date.getMonth() + 1).padStart(2, '0');
    return { label: `${date.getFullYear()}-${month}`, order: 5 };
  }

  interface SessionGroup {
    label: string | null; // null for pinned (no header)
    sessions: SessionSummary[];
  }

  const filteredSessionsByPeriod = computed<SessionGroup[]>(() => {
    const list = searchQuery.value.trim() ? filteredSessions.value : sessions.value;

    const pinned = list.filter((s) => s.pinned);
    const unpinned = list.filter((s) => !s.pinned);

    const groups: Map<string, { label: string; order: number; sessions: SessionSummary[] }> =
      new Map();

    // Add pinned as first group (no label shown)
    const result: SessionGroup[] = [];
    if (pinned.length > 0) {
      result.push({ label: null, sessions: pinned });
    }

    for (const session of unpinned) {
      const { label, order } = getPeriodLabel(new Date(session.updated_at));
      const key = `${order}:${label}`;
      if (!groups.has(key)) {
        groups.set(key, { label, order, sessions: [] });
      }
      groups.get(key)?.sessions.push(session);
    }

    // Sort groups by order, then add to result
    const sortedGroups = Array.from(groups.values()).sort((a, b) => a.order - b.order);
    for (const g of sortedGroups) {
      result.push({ label: g.label, sessions: g.sessions });
    }

    return result;
  });

  function setSearchQuery(query: string) {
    searchQuery.value = query;
  }

  function toggleSidebarCollapsed() {
    sidebarCollapsed.value = !sidebarCollapsed.value;
    localStorage.setItem('chat_sidebar_collapsed', String(sidebarCollapsed.value));
  }

  let abortController: AbortController | null = null;
  let streamCancelledByUser = false;
  let _isResending = false;
  let loadSessionRequestId = 0;

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

  async function refreshGeneratedSessionTitle(sessionId: string) {
    logger.emit({
      severityNumber: SeverityNumber.DEBUG,
      severityText: 'DEBUG',
      body: '[FIX] chat.title_generation.started',
      attributes: {
        component: 'frontend/chat-store',
        session_id: sessionId,
      },
    });

    try {
      const { title } = await api.generateSessionTitle(sessionId);
      const nextTitle = title?.trim();
      if (!nextTitle) {
        logger.emit({
          severityNumber: SeverityNumber.DEBUG,
          severityText: 'DEBUG',
          body: '[FIX] chat.title_generation.skipped_empty_title',
          attributes: {
            component: 'frontend/chat-store',
            session_id: sessionId,
          },
        });
        return;
      }

      const idx = sessions.value.findIndex((s) => s.id === sessionId);
      if (idx !== -1) {
        sessions.value[idx] = {
          ...sessions.value[idx],
          title: nextTitle,
        };
      } else {
        await fetchSessions();
      }

      logger.emit({
        severityNumber: SeverityNumber.DEBUG,
        severityText: 'DEBUG',
        body: '[FIX] chat.title_generation.succeeded',
        attributes: {
          component: 'frontend/chat-store',
          session_id: sessionId,
          title: nextTitle,
        },
      });
    } catch (err) {
      logger.emit({
        severityNumber: SeverityNumber.ERROR,
        severityText: 'ERROR',
        body: '[FIX] chat.title_generation.failed',
        attributes: {
          component: 'frontend/chat-store',
          session_id: sessionId,
          error: err instanceof Error ? err.message : String(err),
        },
      });
    }
  }

  async function sendMessage(collectionId: string, query: string) {
    isLoading.value = true;
    error.value = null;
    pipelineStage.value = null;
    pipelineSessionId.value = activeSessionId.value || null;
    streamCancelledByUser = false;
    abortController = new AbortController();

    // Optimistic title: show user query in sidebar and badge immediately
    const currentSession = activeSessionId.value
      ? sessions.value.find((s) => s.id === activeSessionId.value)
      : null;
    if (
      currentSession &&
      (currentSession.title === 'New Chat' || currentSession.title === 'New Session')
    ) {
      const idx = sessions.value.findIndex((s) => s.id === activeSessionId.value);
      if (idx !== -1) {
        const tempTitle = query.slice(0, 45).trim();
        sessions.value[idx] = { ...sessions.value[idx], title: tempTitle };
      }
    }

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

    // Save pipeline state so it survives page reload.
    // ChatView creates a session before sending, so in the common path this
    // stores the real backend session id. If a caller still sends without an
    // active session, checkPendingPipeline can recover by matching the newest
    // default-titled session in the selected collection.
    savePipelineState(activeSessionId.value || '', collectionId, query);

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
      let completedSessionId: string | null = null;

      while (true) {
        const { done, value } = await streamReader.read();
        if (done) break;

        switch (value.type) {
          case 'chunk': {
            const chunkText = value.data?.text || value.text || '';
            fullContent += chunkText;
            // Only write chunks to messages when viewing the pipeline session.
            // This prevents two bugs:
            // 1. Corrupting another session's messages with pipeline content
            // 2. Dumping all accumulated fullContent into the empty streaming
            //    placeholder when switching back, which fills it prematurely
            //    and causes the fallback pipeline-status-bar to appear.
            if (activeSessionId.value === pipelineSessionId.value) {
              const lastMsg = messages.value[messages.value.length - 1];
              if (lastMsg?.role === 'assistant') {
                // Incremental append — not fullContent — so the streaming
                // placeholder stays empty on switch-back until the next
                // real chunk from the backend arrives.
                lastMsg.content = (lastMsg.content || '') + chunkText;
                // Force reactivity by replacing the array
                messages.value = [...messages.value];
              }
            }
            break;
          }
          case 'sources':
            sources = JSON.stringify(value.data?.sources || value.sources);
            break;
          case 'pipeline_stage': {
            const stageName = value.data?.stage_name || '';
            if (stageName) {
              pipelineStage.value = stageName;
              updateSavedPipelineStage(stageName);
            }
            break;
          }
          case 'debug': {
            // Only write debug data when viewing the pipeline session
            if (activeSessionId.value === pipelineSessionId.value) {
              const lastMsg = messages.value[messages.value.length - 1];
              if (lastMsg?.role === 'assistant') {
                lastMsg.debug_data = JSON.stringify(value.data?.debug || value.data);
                messages.value = [...messages.value];
              }
            }
            break;
          }
          case 'error': {
            const errSessionId = pipelineSessionId.value;
            clearPipelineState();
            pipelineSessionId.value = null;
            pipelineStage.value = null;
            error.value = value.data?.text || value.text || 'An error occurred';
            // Remove the placeholder assistant message only if viewing
            // the pipeline session — otherwise we'd corrupt other sessions.
            if (activeSessionId.value === errSessionId) {
              messages.value.pop();
            }
            break;
          }
          case 'done': {
            pipelineStage.value = null;
            // Finalize the assistant message. Only write content when
            // viewing the pipeline session — if the user switched away,
            // the backend already persisted the complete response.
            const doneSessionId = pipelineSessionId.value;
            completedSessionId = doneSessionId;
            pipelineSessionId.value = null;
            clearPipelineState();
            if (activeSessionId.value === doneSessionId) {
              const finalMsg = messages.value[messages.value.length - 1];
              if (finalMsg?.role === 'assistant') {
                finalMsg.content = fullContent;
                finalMsg.sources = sources;
                messages.value = [...messages.value];
              }
            }

            // Temp-ID reconciliation: fields are inside `data` (Rust SSE format)
            const doneData = value.data || value;
            if (doneData.user_message_id || doneData.assistant_message_id) {
              for (let i = 0; i < messages.value.length; i++) {
                const msg = messages.value[i];
                if (msg.role === 'user' && msg.id.startsWith('temp-') && doneData.user_message_id) {
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

      // Refine title via LLM-based title generation for the completed
      // pipeline session. Do not depend on the currently selected session:
      // users can switch chats while the stream is still running.
      if (completedSessionId && fullContent.trim()) {
        await refreshGeneratedSessionTitle(completedSessionId);
      }
    } catch (err) {
      if (err instanceof Error && err.name === 'AbortError') {
        if (streamCancelledByUser) {
          clearPipelineState();
        }
        // AbortError during page navigation is expected. Keep localStorage so
        // the next page instance can restore title, stage, and SSE recovery.
      } else if (err instanceof ApiError) {
        error.value = err.message;
      } else if (err instanceof Error) {
        error.value = err.message;
      }
      // Remove the placeholder assistant message on error.
      // Guard by pipeline session to avoid corrupting other sessions.
      const catchErrSessionId = pipelineSessionId.value;
      if (
        activeSessionId.value === catchErrSessionId &&
        messages.value.length > 0 &&
        messages.value[messages.value.length - 1]?.role === 'assistant'
      ) {
        messages.value.pop();
      }
    } finally {
      isLoading.value = false;
      abortController = null;
      streamCancelledByUser = false;
    }
  }

  function cancelStream() {
    streamCancelledByUser = true;
    isLoading.value = false;
    pipelineStage.value = null;
    pipelineSessionId.value = null;
    clearPipelineState();
    if (abortController) {
      abortController.abort();
      abortController = null;
    }
  }

  /**
   * Resume a pipeline that was running when the page was reloaded.
   * Restores temp messages + pipeline stage from localStorage and
   * subscribes to the backend recovery SSE endpoint until completion.
   */
  async function checkPendingPipeline() {
    const rawState = getPipelineState();
    if (!rawState) {
      return;
    }

    // Resolve session ID: if empty (savePipelineState was called before
    // createSession returned), find the auto-created session by matching
    // collection_id against sessions with default "New Chat" title.
    let resolvedSessionId = rawState.sessionId;
    if (!resolvedSessionId) {
      const autoSession = sessions.value.find(
        (s) =>
          s.collection_id === rawState.collectionId &&
          (s.title === 'New Chat' || s.title === 'New Session'),
      );
      if (autoSession) {
        resolvedSessionId = autoSession.id;
        // Update localStorage so SSE recovery can use the resolved ID
        localStorage.setItem(LS_PIPELINE_SESSION_ID, resolvedSessionId);
      } else {
        // Can't recover: set visible state but skip SSE subscription
        activeSessionId.value = null;
        pipelineSessionId.value = null;
        pipelineStage.value = rawState.stage || 'connecting';
        lastCollectionId.value = rawState.collectionId;
        messages.value = [
          {
            id: `temp-${Date.now()}`,
            session_id: '',
            role: 'user',
            content: rawState.userQuery,
            created_at: new Date().toISOString(),
          },
          {
            id: `temp-assist-${Date.now()}`,
            session_id: '',
            role: 'assistant',
            content: '',
            created_at: new Date().toISOString(),
          },
        ];
        isLoading.value = true;
        return;
      }
    }

    const state = { ...rawState, sessionId: resolvedSessionId };

    activeSessionId.value = state.sessionId;
    pipelineSessionId.value = state.sessionId;
    // Show the last stage saved before reload immediately; recovery SSE will
    // continue updating it with live pipeline_stage events from the backend.
    pipelineStage.value = state.stage || 'generating';
    lastCollectionId.value = state.collectionId;

    // Restore temp session title — if sessions haven't been loaded yet
    // (fetchSessions failed or is still in-flight), insert a synthetic entry
    // so the toolbar never shows the default "New Chat" title.
    let sessIdx = sessions.value.findIndex((s) => s.id === state.sessionId);
    if (sessIdx !== -1) {
      sessions.value[sessIdx] = {
        ...sessions.value[sessIdx],
        title: state.tempTitle,
      };
    } else {
      // Entry missing — insert a placeholder so the UI shows the temp title immediately
      sessions.value.unshift({
        id: state.sessionId,
        title: state.tempTitle,
        collection_id: state.collectionId,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        message_count: 0,
      });
      sessIdx = 0;
    }

    // Restore active collection from session data (async import — do after setting visible state)
    const collectionPromise = state.sessionId
      ? (async () => {
          const session = sessions.value.find((s) => s.id === state.sessionId);
          if (session?.collection_id) {
            try {
              const { useCollectionStore } = await import('@/stores/collections');
              const collectionStore = useCollectionStore();
              collectionStore.setActiveCollection(session.collection_id);
            } catch {
              // Collection restore is best-effort during pipeline recovery.
            }
          }
        })()
      : Promise.resolve();

    // Create temp messages so the UI shows something while waiting for SSE recovery
    const tempUserMsg: Message = {
      id: `temp-${Date.now()}`,
      session_id: state.sessionId,
      role: 'user',
      content: state.userQuery,
      created_at: new Date().toISOString(),
    };
    const tempAssistMsg: Message = {
      id: `temp-assist-${Date.now()}`,
      session_id: state.sessionId,
      role: 'assistant',
      content: '',
      created_at: new Date().toISOString(),
    };
    messages.value = [tempUserMsg, tempAssistMsg];
    isLoading.value = true;

    await collectionPromise;

    const recoveryController = new AbortController();
    const timeoutId = setTimeout(() => {
      if (isLoading.value) {
        isLoading.value = false;
        pipelineStage.value = null;
        pipelineSessionId.value = null;
        error.value = 'Pipeline recovery timed out. Please try your query again.';
        clearPipelineState();
      }
      recoveryController.abort();
    }, RECOVERY_TOTAL_TIMEOUT_MS);

    try {
      const headers: Record<string, string> = {};
      const token = getAccessToken();
      if (token) {
        headers.Authorization = `Bearer ${token}`;
      }

      const response = await fetch(`/api/query/${state.sessionId}/subscribe`, {
        headers,
        signal: recoveryController.signal,
      });

      if (!response.ok) {
        throw new ApiError(response.status, response.statusText || 'Pipeline recovery failed');
      }

      if (!response.body) {
        throw new Error('Pipeline recovery stream did not include a response body');
      }

      const stream = parseNDJSON(response.body.getReader());
      const streamReader = stream.getReader();

      while (true) {
        const { done, value } = await streamReader.read();
        if (done) break;

        if (value.type === 'pipeline_stage') {
          const stageName = value.data?.stage_name || '';
          if (stageName) {
            pipelineStage.value = stageName;
            updateSavedPipelineStage(stageName);
          }
          continue;
        }

        if (value.type === 'done') {
          const result = await api.getSessionWithMessages(state.sessionId);
          const msgs = result.messages;
          const hasAssistantMessage = msgs.some(
            (msg) => msg.role === 'assistant' && msg.content.trim().length > 0,
          );

          if (!hasAssistantMessage) {
            throw new Error('Pipeline completed but assistant message was not persisted');
          }

          messages.value = msgs;
          isLoading.value = false;
          pipelineStage.value = null;
          pipelineSessionId.value = null;
          clearPipelineState();
          await fetchSessions();
          await refreshGeneratedSessionTitle(state.sessionId);
          return;
        }

        if (value.type === 'error') {
          throw new Error(value.data?.text || value.text || 'Pipeline recovery failed');
        }
      }

      throw new Error('Pipeline recovery stream ended before completion');
    } catch (err) {
      if (err instanceof ApiError && err.status === 404) {
        clearPipelineState();
        isLoading.value = false;
        pipelineStage.value = null;
        pipelineSessionId.value = null;
        return;
      }

      if (err instanceof DOMException && err.name === 'AbortError') {
        // Page navigated away — keep localStorage so next load can retry recovery.
        return;
      }

      if (pageIsUnloading) {
        // Browsers do not always surface reload-disconnects as DOMException
        // AbortError. On the second F5, fetch/reader can fail with TypeError.
        // Treat any recovery failure during unload as navigation, not terminal.
        return;
      }

      error.value = 'Response generation was interrupted. Please retry the query.';

      isLoading.value = false;
      pipelineStage.value = null;
      pipelineSessionId.value = null;
      clearPipelineState();
    } finally {
      clearTimeout(timeoutId);
    }
  }

  async function fetchSessions() {
    isLoadingSessions.value = true;
    try {
      const result = await api.get<SessionSummary[]>('/sessions');
      sessions.value = result;
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) {
        // Token expired — can happen during cold start. Just swallow.
        return;
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
      sessions.value = sessions.value.filter((s) => s.id !== sessionId);
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    }
  }

  function ensurePipelinePlaceholder(sessionId: string) {
    if (!isLoading.value || pipelineSessionId.value !== sessionId || !pipelineStage.value) {
      return;
    }

    const lastMessage = messages.value[messages.value.length - 1];
    if (lastMessage?.role === 'assistant' && !lastMessage.content) {
      return;
    }

    messages.value = [
      ...messages.value,
      {
        id: `temp-assist-${Date.now()}`,
        session_id: sessionId,
        role: 'assistant',
        content: '',
        created_at: new Date().toISOString(),
      },
    ];
  }

  async function loadSession(sessionId: string) {
    isSessionLoading.value = true;
    const requestId = ++loadSessionRequestId;
    try {
      const result = await api.get<{ session: Session; messages: Message[] }>(
        `/sessions/${sessionId}`,
      );
      // Ignore stale responses from earlier requests
      if (requestId !== loadSessionRequestId) {
        return;
      }
      messages.value = result.messages;
      activeSessionId.value = sessionId;
      ensurePipelinePlaceholder(sessionId);
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    } finally {
      isSessionLoading.value = false;
    }
  }

  async function editMessage(sessionId: string, messageId: string, content: string) {
    if (!canPersistMessageAction(messageId)) {
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

  async function editAndResend(
    sessionId: string,
    messageId: string,
    content: string,
    collectionId: string,
  ) {
    if (!canPersistMessageAction(messageId)) {
      logger.emit({
        severityNumber: SeverityNumber.DEBUG,
        severityText: 'DEBUG',
        body: 'chat.edit_and_resend.skipped_pending_message',
        attributes: {
          component: 'frontend/chat-store',
          session_id: sessionId,
          message_id: messageId,
        },
      });
      return;
    }

    if (_isResending) {
      logger.emit({
        severityNumber: SeverityNumber.DEBUG,
        severityText: 'DEBUG',
        body: 'chat.edit_and_resend.skipped_concurrent',
        attributes: {
          component: 'frontend/chat-store',
          session_id: sessionId,
          message_id: messageId,
          collection_id: collectionId,
        },
      });
      return;
    }
    _isResending = true;

    logger.emit({
      severityNumber: SeverityNumber.DEBUG,
      severityText: 'DEBUG',
      body: 'chat.edit_and_resend.started',
      attributes: {
        component: 'frontend/chat-store',
        session_id: sessionId,
        message_id: messageId,
        collection_id: collectionId,
      },
    });

    cancelStream();

    try {
      await editMessage(sessionId, messageId, content);
      const editedIdx = messages.value.findIndex((message) => message.id === messageId);
      if (editedIdx !== -1) {
        messages.value.splice(editedIdx);
      }

      logger.emit({
        severityNumber: SeverityNumber.DEBUG,
        severityText: 'DEBUG',
        body: 'chat.edit_and_resend.resending_query',
        attributes: {
          component: 'frontend/chat-store',
          session_id: sessionId,
          message_id: messageId,
          collection_id: collectionId,
          trimmed_message_count: editedIdx === -1 ? 0 : editedIdx,
        },
      });

      await sendMessage(collectionId, content);
    } catch (err) {
      logger.emit({
        severityNumber: SeverityNumber.ERROR,
        severityText: 'ERROR',
        body: 'chat.edit_and_resend.failed',
        attributes: {
          component: 'frontend/chat-store',
          session_id: sessionId,
          message_id: messageId,
          collection_id: collectionId,
          error: err instanceof Error ? err.message : String(err),
        },
      });
      throw err;
    } finally {
      _isResending = false;
    }
  }

  async function deleteMessage(sessionId: string, messageId: string) {
    if (!canPersistMessageAction(messageId)) {
      return;
    }

    const idx = messages.value.findIndex((m) => m.id === messageId);
    if (idx === -1) return;

    // Optimistic remove
    const prev = messages.value[idx];
    messages.value.splice(idx, 1);

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

  async function renameSession(sessionId: string, title: string) {
    try {
      await api.patch(`/sessions/${sessionId}`, { title });
      // Update in local sessions list
      const idx = sessions.value.findIndex((s) => s.id === sessionId);
      if (idx !== -1) {
        sessions.value[idx] = { ...sessions.value[idx], title };
      }
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    }
  }

  async function togglePinSession(sessionId: string) {
    const idx = sessions.value.findIndex((s) => s.id === sessionId);
    if (idx === -1) return;

    const newPinned = !sessions.value[idx].pinned;
    try {
      await api.patch(`/sessions/${sessionId}`, { pinned: newPinned });
      sessions.value[idx] = { ...sessions.value[idx], pinned: newPinned };
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    }
  }

  async function exportSession(sessionId: string, format: 'md' | 'json') {
    isExporting.value = true;
    try {
      const blob = await api.exportSession(sessionId, format);
      const url = URL.createObjectURL(blob);
      const extension = format === 'md' ? 'md' : 'json';
      const a = document.createElement('a');
      a.href = url;
      a.download = `session-${sessionId}.${extension}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      if (err instanceof ApiError) {
        error.value = err.message;
      }
    } finally {
      isExporting.value = false;
    }
  }

  /** Find the last user message before a given assistant message and re-send it. */
  async function regenerateMessage(assistantMessageId: string) {
    const idx = messages.value.findIndex(
      (m) => m.id === assistantMessageId && m.role === 'assistant',
    );
    if (idx < 1) {
      return;
    }

    // Find the most recent user message before this assistant message
    let userQuery = '';
    for (let i = idx - 1; i >= 0; i--) {
      if (messages.value[i].role === 'user') {
        userQuery = messages.value[i].content;
        break;
      }
    }
    if (!userQuery) {
      return;
    }

    // Remove the existing assistant message
    messages.value.splice(idx, 1);

    // Re-send the query with the stored collection
    await sendMessage(lastCollectionId.value || '', userQuery);
  }

  /** Copy a message text to clipboard. */
  async function copyMessage(messageId: string): Promise<void> {
    const msg = messages.value.find((m) => m.id === messageId);
    if (!msg) {
      return;
    }
    try {
      await navigator.clipboard.writeText(msg.content);
    } catch {
      // Clipboard access can fail when blocked by browser permissions.
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
    checkPendingPipeline,
    createSession,
    deleteSession,
    loadSession,
    editMessage,
    editAndResend,
    deleteMessage,
    exportSession,
    clearMessages,
    canPersistMessageAction,
    renameSession,
    togglePinSession,
    searchQuery,
    sidebarCollapsed,
    filteredSessions,
    filteredSessionsByPeriod,
    lastCollectionId,
    setSearchQuery,
    toggleSidebarCollapsed,
    regenerateMessage,
    copyMessage,
    pipelineStage,
    pipelineSessionId,
    pipelineStageLabel,
  };
});
