<script setup lang="ts">
import { api } from '@/api/client';
import type { DebugData, Message, SessionSummary } from '@/api/types';
import VDatePicker from '@/components/ui/VDatePicker.vue';
import { computed, onMounted, ref } from 'vue';

const sessions = ref<SessionSummary[]>([]);
const selectedSession = ref<SessionSummary | null>(null);
const selectedMessages = ref<Message[]>([]);
const searchQuery = ref('');
const userNameFilter = ref('');
const dateFrom = ref('');
const dateTo = ref('');
const isLoading = ref(false);
const expandedDebug = ref<Record<string, boolean>>({});
const users = ref<string[]>([]);

const steps = [
  {
    id: 1,
    name: 'Multi-query',
    key: 'multi_query',
    status: 'disabled',
    desc: 'Question → 3 variants',
  },
  {
    id: 2,
    name: 'HyDE',
    key: 'hyde',
    status: 'disabled',
    desc: 'Hypothetical document per query',
  },
  {
    id: 3,
    name: 'Embedding search',
    key: 'embedding_search',
    status: 'active',
    desc: 'Chroma nearest neighbor search',
  },
  {
    id: 4,
    name: 'Hybrid keyword search',
    key: 'keyword_search',
    status: 'disabled',
    desc: 'Keywords → chunks',
  },
  {
    id: 5,
    name: 'Merge & dedup',
    key: 'merge_dedup',
    status: 'disabled',
    desc: '~15-19 chunks → unique set',
  },
  {
    id: 6,
    name: 'Reranking',
    key: 'reranking',
    status: 'disabled',
    desc: 'LLM scores each chunk',
  },
  {
    id: 7,
    name: 'Final answer',
    key: 'final_answer',
    status: 'active',
    desc: 'Selected chunks → response',
  },
];

async function searchSessions() {
  isLoading.value = true;
  try {
    // Convert date input values (YYYY-MM-DD) to RFC3339 for the API
    const fromVal = dateFrom.value ? `${dateFrom.value}T00:00:00Z` : undefined;
    const toVal = dateTo.value ? `${dateTo.value}T23:59:59Z` : undefined;

    sessions.value = await api.adminSearchSessions({
      search: searchQuery.value || undefined,
      user_name: userNameFilter.value || undefined,
      from: fromVal,
      to: toVal,
    });
  } catch (err) {
    console.error('[SessionDebug] search failed', err);
  } finally {
    isLoading.value = false;
  }
}

async function loadUsers() {
  try {
    users.value = await api.adminGetSessionUsers();
  } catch (err) {
    console.error('[SessionDebug] load users failed', err);
  }
}

async function loadSession(id: string) {
  selectedSession.value = sessions.value.find((s) => s.id === id) || null;
  if (!selectedSession.value) return;
  try {
    const result = await api.getSessionWithMessages(id);
    selectedMessages.value = result.messages;
  } catch (err) {
    console.error('[SessionDebug] load failed', err);
  }
}

function toggleDebug(msgId: string) {
  expandedDebug.value[msgId] = !expandedDebug.value[msgId];
}

function parseDebugData(msg: Message): DebugData | null {
  if (!msg.debug_data) return null;
  try {
    return JSON.parse(msg.debug_data) as DebugData;
  } catch {
    return null;
  }
}

function getStepData(
  debug: DebugData,
  key: string,
  // biome-ignore lint/suspicious/noExplicitAny: needs dynamic key access
): any {
  return (debug as unknown as Record<string, unknown>)[key] || null;
}

/** Cache parsed debug data per message to avoid repeated JSON.parse calls */
function getDebugForMsg(msg: Message): DebugData {
  return parseDebugData(msg) || ({} as DebugData);
}

const hasActiveSession = computed(() => selectedSession.value !== null);

// Load all sessions and user list on mount
onMounted(async () => {
  await Promise.all([searchSessions(), loadUsers()]);
});
</script>

<template>
  <div class="session-debug-view" data-testid="session-debug-view">
    <div class="debug-left-panel">
      <!-- Search -->
      <div class="debug-search-row">
        <span class="debug-search-icon">🔍</span>
        <input
          v-model="searchQuery"
          class="debug-search-input"
          data-testid="session-debug-search"
          placeholder="Search sessions by title..."
          type="text"
          @input="searchSessions"
        />
      </div>
      <!-- User name filter -->
      <div class="debug-search-row">
        <span class="debug-search-icon">👤</span>
        <select
          v-model="userNameFilter"
          class="debug-search-input debug-user-select"
          data-testid="session-debug-user-search"
          @change="searchSessions"
        >
          <option value="">All users</option>
          <option v-for="u in users" :key="u" :value="u">
            {{ u }}
          </option>
        </select>
      </div>

      <!-- Date filters -->
      <div class="debug-date-row">
        <label class="debug-date-label">From:</label>
        <VDatePicker v-model="dateFrom" @change="searchSessions" />
        <label class="debug-date-label">To:</label>
        <VDatePicker v-model="dateTo" @change="searchSessions" />
      </div>

      <!-- Session list -->
      <div class="debug-session-list">
        <div
          v-for="s in sessions"
          :key="s.id"
          class="debug-session-item"
          :class="{
            'debug-session-item--active': selectedSession?.id === s.id,
          }"
          data-testid="session-list-item"
          @click="loadSession(s.id)"
        >
          <span class="debug-session-title">{{ s.title }}</span>
          <span class="debug-session-meta"
            >{{ new Date(s.created_at).toLocaleDateString() }} ·
            {{ s.message_count }} msgs</span
          >
          <span class="debug-session-user"
            >👤 {{ s.user_name || "Unknown user" }}</span
          >
        </div>
        <div
          v-if="sessions.length === 0 && !isLoading"
          class="debug-session-empty"
        >
          <span>No sessions found</span>
        </div>
      </div>
    </div>

    <div class="debug-right-panel">
      <!-- Empty state -->
      <div
        v-if="!hasActiveSession"
        class="debug-empty-state"
        data-testid="session-debug-empty"
      >
        <span class="debug-empty-icon">🔍</span>
        <h3 class="debug-empty-title">Select a session</h3>
        <p class="debug-empty-desc">
          Choose a session from the list to view messages and debug data.
        </p>
      </div>

      <!-- Session detail -->
      <template v-if="hasActiveSession && selectedSession">
        <div class="debug-session-header">
          <h3 class="debug-session-header-title">
            {{ selectedSession.title }}
          </h3>
          <span class="debug-session-header-meta">
            {{ new Date(selectedSession.created_at).toLocaleDateString() }} ·
            {{ selectedSession.message_count }} messages
          </span>
        </div>

        <div class="debug-messages-list">
          <div
            v-for="(msg, _idx) in selectedMessages"
            :key="msg.id"
            class="debug-message"
            :class="{
              'debug-message--user': msg.role === 'user',
              'debug-message--assistant': msg.role === 'assistant',
            }"
            data-testid="session-msg"
          >
            <div class="debug-msg-role">
              {{ msg.role === "user" ? "User" : "Assistant" }}
            </div>
            <div class="debug-msg-content">{{ msg.content }}</div>

            <!-- Debug toggle (assistant only) -->
            <button
              v-if="msg.role === 'assistant' && msg.debug_data"
              class="debug-toggle-btn"
              data-testid="session-debug-toggle"
              @click="toggleDebug(msg.id)"
            >
              {{ expandedDebug[msg.id] ? "▼" : "▶" }} Debug — Generation
              Pipeline
            </button>

            <!-- Debug panel -->
            <div
              v-if="expandedDebug[msg.id] && msg.debug_data"
              class="debug-panel-inner"
              data-testid="debug-panel"
            >
              <div
                v-for="step in steps.filter(
                  (s) =>
                    getStepData(getDebugForMsg(msg), s.key) ||
                    s.status === 'active',
                )"
                :key="step.id"
                class="debug-step"
                data-testid="debug-step"
              >
                <details class="debug-step-details">
                  <summary class="debug-step-summary">
                    <span class="debug-step-number">{{ step.id }}.</span>
                    <span
                      class="debug-step-name"
                      data-testid="debug-step-title"
                      >{{ step.name }}</span
                    >
                  </summary>
                  <div class="debug-step-body" data-testid="debug-step-data">
                    <template
                      v-if="
                        step.id === 1 &&
                        getStepData(getDebugForMsg(msg), 'multi_query')
                      "
                    >
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Original Query</span>
                        <span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "multi_query")
                            ?.original_query
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Latency</span>
                        <span class="debug-meta-value"
                          >{{
                            getStepData(getDebugForMsg(msg), "multi_query")
                              ?.latency_ms
                          }}ms</span
                        >
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Variants</span>
                        <div class="debug-meta-value">
                          <ul style="margin: 0; padding-left: 1.2em">
                            <li
                              v-for="(v, idx) in getStepData(
                                getDebugForMsg(msg),
                                'multi_query',
                              )?.variants"
                              :key="idx"
                            >
                              {{ v }}
                            </li>
                          </ul>
                        </div>
                      </div>
                    </template>
                    <template
                      v-if="
                        step.id === 2 &&
                        getStepData(getDebugForMsg(msg), 'hyde')
                      "
                    >
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Results</span>
                        <div class="debug-meta-value">
                          <ul style="margin: 0; padding-left: 1.2em">
                            <li
                              v-for="(v, idx) in getStepData(
                                getDebugForMsg(msg),
                                'hyde',
                              )?.per_query"
                              :key="idx"
                            >
                              <strong>{{ v.query }}</strong> ({{
                                v.latency_ms
                              }}ms)<br />
                              <em>{{ v.hypothetical_doc }}</em>
                            </li>
                          </ul>
                        </div>
                      </div>
                    </template>
                    <template v-if="step.id === 3">
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Query snippet</span
                        ><span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "embedding_search")
                            ?.query_snippet
                        }}</span>
                      </div>
                      <div
                        class="debug-meta-row"
                        v-if="
                          getStepData(getDebugForMsg(msg), 'embedding_search')
                            ?.embedding_dimension
                        "
                      >
                        <span class="debug-meta-label">Dimension</span
                        ><span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "embedding_search")
                            ?.embedding_dimension
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Latency</span
                        ><span class="debug-meta-value"
                          >{{
                            getStepData(getDebugForMsg(msg), "embedding_search")
                              ?.latency_ms
                          }}ms</span
                        >
                      </div>
                      <div
                        v-if="
                          getStepData(getDebugForMsg(msg), 'embedding_search')
                            ?.results?.length
                        "
                        class="debug-results"
                      >
                        <div
                          v-for="(r, i) in getStepData(
                            getDebugForMsg(msg),
                            'embedding_search',
                          )?.results"
                          :key="i"
                          class="debug-result-item"
                        >
                          <span class="debug-result-doc">{{
                            r.document_name
                          }}</span>
                          <span class="debug-result-score"
                            >{{ Math.round(r.score * 100) }}%</span
                          >
                          <details>
                            <summary>Chunk #{{ r.chunk_index }}</summary>
                            <pre>{{ r.text_snippet }}</pre>
                          </details>
                        </div>
                      </div>
                    </template>
                    <template
                      v-if="
                        step.id === 4 &&
                        getStepData(getDebugForMsg(msg), 'keyword_search')
                      "
                    >
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Tokens</span>
                        <span class="debug-meta-value">{{
                          getStepData(
                            getDebugForMsg(msg),
                            "keyword_search",
                          )?.query_tokens.join(", ")
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Matches</span>
                        <span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "keyword_search")
                            ?.total_matches
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Latency</span>
                        <span class="debug-meta-value"
                          >{{
                            getStepData(getDebugForMsg(msg), "keyword_search")
                              ?.latency_ms
                          }}ms</span
                        >
                      </div>
                      <div
                        v-if="
                          getStepData(getDebugForMsg(msg), 'keyword_search')
                            ?.results?.length
                        "
                        class="debug-results"
                      >
                        <div
                          v-for="(r, i) in getStepData(
                            getDebugForMsg(msg),
                            'keyword_search',
                          )?.results"
                          :key="i"
                          class="debug-result-item"
                        >
                          <span class="debug-result-doc">{{
                            r.document_name
                          }}</span>
                          <span class="debug-result-score">{{
                            r.score.toFixed(2)
                          }}</span>
                        </div>
                      </div>
                    </template>
                    <template
                      v-if="
                        step.id === 5 &&
                        getStepData(getDebugForMsg(msg), 'merge_dedup')
                      "
                    >
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Input Chunks</span>
                        <span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "merge_dedup")
                            ?.input_chunks
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">After Dedup</span>
                        <span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "merge_dedup")
                            ?.after_dedup
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Breakdown</span>
                        <span class="debug-meta-value"
                          >Vector:
                          {{
                            getStepData(getDebugForMsg(msg), "merge_dedup")
                              ?.source_breakdown.vector_chunks
                          }}, BM25:
                          {{
                            getStepData(getDebugForMsg(msg), "merge_dedup")
                              ?.source_breakdown.keyword_chunks
                          }}</span
                        >
                        >
                      </div>
                      <div
                        v-if="
                          getStepData(getDebugForMsg(msg), 'merge_dedup')
                            ?.deduped_ids?.length
                        "
                        class="debug-meta-row"
                      >
                        <span class="debug-meta-label">Both (dedup)</span>
                        <span class="debug-meta-value"
                          >{{
                            getStepData(getDebugForMsg(msg), "merge_dedup")
                              ?.deduped_ids?.length
                          }}
                          chunks found by vector + BM25</span
                        >
                      </div>
                      <div
                        v-if="
                          getStepData(getDebugForMsg(msg), 'merge_dedup')
                            ?.results?.length
                        "
                        class="debug-results"
                      >
                        <div
                          v-for="(r, i) in getStepData(
                            getDebugForMsg(msg),
                            'merge_dedup',
                          )?.results"
                          :key="i"
                          class="debug-result-item"
                          style="
                            flex-direction: column;
                            align-items: flex-start;
                            gap: 4px;
                          "
                        >
                          <div
                            style="
                              display: flex;
                              justify-content: space-between;
                              width: 100%;
                            "
                          >
                            <span class="debug-result-doc">{{
                              r.document_name
                            }}</span>
                            <span class="debug-result-score">{{
                              r.score.toFixed(2)
                            }}</span>
                          </div>
                          <details>
                            <summary>Chunk #{{ r.chunk_index }}</summary>
                            <pre>{{ r.text_snippet }}</pre>
                          </details>
                        </div>
                      </div>
                    </template>
                    <template
                      v-if="
                        step.id === 6 &&
                        getStepData(getDebugForMsg(msg), 'reranking')
                      "
                    >
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Input Count</span>
                        <span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "reranking")
                            ?.input_count
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Accepted</span>
                        <span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "reranking")
                            ?.accepted
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Rejected</span>
                        <span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "reranking")
                            ?.rejected
                        }}</span>
                      </div>
                      <div
                        v-if="
                          getStepData(getDebugForMsg(msg), 'reranking')?.results
                            ?.length
                        "
                        class="debug-results"
                      >
                        <div
                          v-for="(r, i) in getStepData(
                            getDebugForMsg(msg),
                            'reranking',
                          )?.results"
                          :key="i"
                          class="debug-result-item"
                          style="
                            flex-direction: column;
                            align-items: flex-start;
                            gap: 4px;
                          "
                        >
                          <div
                            style="
                              display: flex;
                              justify-content: space-between;
                              width: 100%;
                            "
                          >
                            <span class="debug-result-doc"
                              >{{ r.document_name || r.chunk_id }} (chunk
                              {{ r.chunk_index }})</span
                            >
                            <span
                              class="debug-result-score"
                              :style="{
                                color:
                                  r.verdict === 'брать'
                                    ? 'var(--color-primary)'
                                    : 'var(--color-destructive)',
                              }"
                              >{{ r.verdict }} ({{ r.score }})</span
                            >
                          </div>
                          <div
                            style="
                              font-size: 10px;
                              color: var(--color-muted-foreground);
                            "
                          >
                            {{ r.comment }}
                          </div>
                          <details>
                            <summary>Chunk text</summary>
                            <pre>{{ r.text_snippet }}</pre>
                          </details>
                        </div>
                      </div>
                    </template>
                    <template v-if="step.id === 7">
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Model</span
                        ><span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "final_answer")
                            ?.model
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Max Retries</span
                        ><span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "final_answer")
                            ?.max_retries
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Chunks in context</span
                        ><span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "final_answer")
                            ?.chunks_in_context
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">History Messages</span
                        ><span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "final_answer")
                            ?.history_message_count
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Latency</span
                        ><span class="debug-meta-value"
                          >{{
                            getStepData(getDebugForMsg(msg), "final_answer")
                              ?.latency_ms
                          }}ms</span
                        >
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label">Token Budget</span
                        ><span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "final_answer")
                            ?.token_budget
                        }}</span>
                      </div>
                      <div class="debug-meta-row">
                        <span class="debug-meta-label"
                          >Total Tokens Estimate</span
                        ><span class="debug-meta-value">{{
                          getStepData(getDebugForMsg(msg), "final_answer")
                            ?.total_tokens_estimate
                        }}</span>
                      </div>
                      <details class="debug-prompt-preview">
                        <summary>Prompt preview</summary>
                        <pre>{{
                          getStepData(getDebugForMsg(msg), "final_answer")
                            ?.prompt_preview
                        }}</pre>
                      </details>
                    </template>
                  </div>
                </details>
              </div>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.session-debug-view {
  display: flex;
  flex: 1;
  gap: 24px;
  height: 100%;
  overflow: hidden;
}

.debug-left-panel {
  width: 380px;
  min-width: 380px;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl, 16px);
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow: hidden;
}

.debug-search-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.debug-search-icon {
  font-size: 13px;
  color: var(--color-muted-foreground);
}

.debug-search-input {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--color-foreground);
  font-family: var(--font-family);
  font-size: 12px;
  outline: none;
}

.debug-search-input::placeholder {
  color: var(--color-muted-foreground);
}

.debug-user-select {
  cursor: pointer;
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  padding-right: 4px;
}

.debug-user-select option {
  background: var(--color-background);
  color: var(--color-foreground);
}

.debug-date-row {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.debug-date-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--color-muted-foreground);
  font-family: var(--font-family);
}

.debug-session-list {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow-y: auto;
}

.debug-session-item {
  padding: 8px 12px;
  border-radius: 8px;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 2px;
  transition: background var(--transition-fast);
}

.debug-session-item:hover {
  background: var(--color-muted);
}

.debug-session-item--active {
  background: var(--color-primary);
  color: var(--color-primary-foreground);
}

.debug-session-title {
  font-size: 11px;
  font-weight: 600;
  color: inherit;
}

.debug-session-meta {
  font-size: 9px;
  color: var(--color-muted-foreground);
}

.debug-session-user {
  font-size: 9px;
  color: var(--color-muted-foreground);
  opacity: 0.7;
}

.debug-session-item--active .debug-session-meta {
  color: var(--color-primary-foreground);
  opacity: 0.8;
}

.debug-session-item--active .debug-session-user {
  color: var(--color-primary-foreground);
  opacity: 0.7;
}

.debug-session-empty {
  padding: 20px;
  text-align: center;
  color: var(--color-muted-foreground);
  font-size: 12px;
}

.debug-right-panel {
  flex: 1;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl, 16px);
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow: hidden;
}

.debug-empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
}

.debug-empty-icon {
  font-size: 28px;
  color: var(--color-muted-foreground);
}

.debug-empty-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-foreground);
  margin: 0;
}

.debug-empty-desc {
  font-size: 11px;
  color: var(--color-muted-foreground);
  margin: 0;
}

.debug-session-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex-shrink: 0;
}

.debug-session-header-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-foreground);
  margin: 0;
}

.debug-session-header-meta {
  font-size: 11px;
  color: var(--color-muted-foreground);
}

.debug-messages-list {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
}

.debug-message {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 12px;
  border-radius: 8px;
  background: var(--color-background);
  border: 1px solid var(--color-border);
}

.debug-message--user {
  border-left: 3px solid var(--color-primary);
}

.debug-message--assistant {
  border-left: 3px solid var(--color-muted-foreground);
}

.debug-msg-role {
  font-size: 10px;
  font-weight: 600;
  color: var(--color-muted-foreground);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.debug-msg-content {
  font-size: 12px;
  color: var(--color-foreground);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}

.debug-toggle-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: none;
  border: none;
  color: var(--color-primary);
  font-family: var(--font-family);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  padding: 4px 0;
}

.debug-toggle-btn:hover {
  opacity: 0.8;
}

.debug-panel-inner {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px;
  margin-top: 4px;
  background: var(--color-muted);
  border-radius: 8px;
}

.debug-step {
  border-radius: 6px;
  background: var(--color-background);
  border: 1px solid rgba(128, 128, 128, 0.1);
}

.debug-step-details {
  padding: 6px 8px;
}

.debug-step-summary {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 11px;
}

.debug-step-number {
  color: var(--color-muted-foreground);
  font-weight: 600;
}

.debug-step-name {
  flex: 1;
  font-weight: 600;
  color: var(--color-foreground);
}

.debug-step-badge {
  font-size: 9px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 4px;
}

.debug-step-badge--active {
  background: #10b98126;
  color: #10b981;
  border: 1px solid #10b9814d;
}

.debug-step-badge--future {
  background: #f59e0b26;
  color: #f59e0b;
  border: 1px solid #f59e0b4d;
}

.debug-step-body {
  padding: 6px 0 0 16px;
}

.debug-meta-row {
  display: flex;
  justify-content: space-between;
  padding: 2px 0;
  font-size: 10px;
}

.debug-meta-label {
  color: var(--color-muted-foreground);
}

.debug-meta-value {
  color: var(--color-foreground);
  font-weight: 500;
}

.debug-results {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: 4px;
}

.debug-result-item {
  display: flex;
  justify-content: space-between;
  padding: 2px 0;
  font-size: 10px;
}

.debug-result-doc {
  color: var(--color-foreground);
  font-weight: 500;
}

.debug-result-score {
  color: var(--color-primary);
}

.debug-prompt-preview {
  margin-top: 4px;
  font-size: 10px;
}

.debug-prompt-preview summary {
  cursor: pointer;
  color: var(--color-primary);
  font-weight: 600;
}

.debug-prompt-preview pre {
  background: var(--color-muted);
  padding: 6px 8px;
  border-radius: 4px;
  font-size: 9px;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--color-foreground);
  margin: 4px 0 0;
}
</style>
