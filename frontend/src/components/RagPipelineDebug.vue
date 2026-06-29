<script setup lang="ts">
import { api } from '@/api/client';
import type {
  DebugData,
  HydeData,
  KeywordSearchData,
  MergeDedupData,
  Message,
  MultiQueryData,
  RerankingData,
  SessionSummary,
} from '@/api/types';
import { computed, ref } from 'vue';

const sessions = ref<SessionSummary[]>([]);
const selectedSession = ref<SessionSummary | null>(null);
const selectedMessages = ref<Message[]>([]);
const searchQuery = ref('');
const isLoading = ref(false);

const pipelineSteps = [
  {
    id: 1,
    name: 'Multi-query',
    key: 'multi_query' as const,
    status: 'active' as const,
    desc: 'Question → 3 variants',
  },
  {
    id: 2,
    name: 'HyDE',
    key: 'hyde' as const,
    status: 'active' as const,
    desc: 'Hypothetical document per query',
  },
  {
    id: 3,
    name: 'Embedding search',
    key: 'embedding_search' as const,
    status: 'future' as const,
    desc: 'Chroma nearest neighbor search',
  },
  {
    id: 4,
    name: 'Hybrid keyword search',
    key: 'keyword_search' as const,
    status: 'active' as const,
    desc: 'Keywords → chunks',
  },
  {
    id: 5,
    name: 'Merge & dedup',
    key: 'merge_dedup' as const,
    status: 'active' as const,
    desc: '~15-19 chunks → unique set',
  },
  {
    id: 6,
    name: 'Reranking',
    key: 'reranking' as const,
    status: 'active' as const,
    desc: 'LLM scores each chunk',
  },
  {
    id: 7,
    name: 'Final answer',
    key: 'final_answer' as const,
    status: 'future' as const,
    desc: 'Selected chunks → response',
  },
];

async function searchSessions() {
  isLoading.value = true;
  try {
    sessions.value = await api.adminSearchSessions({
      search: searchQuery.value || undefined,
    });
  } catch (err) {
    console.error('[RagPipelineDebug] search failed', err);
  } finally {
    isLoading.value = false;
  }
}

async function loadSession(id: string) {
  selectedSession.value = sessions.value.find((s) => s.id === id) || null;
  if (!selectedSession.value) return;
  try {
    const result = await api.getSessionWithMessages(id);
    selectedMessages.value = result.messages;
  } catch (err) {
    console.error('[RagPipelineDebug] load failed', err);
  }
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
  // biome-ignore lint/suspicious/noExplicitAny: dynamic key access
): any {
  return (debug as unknown as Record<string, unknown>)[key] || null;
}

const hasActiveSession = computed(() => selectedSession.value !== null);
</script>

<template>
  <div class="rag-pipeline-debug-view" data-testid="rag-pipeline-debug-view">
    <div class="pipeline-left-panel">
      <!-- Search -->
      <div class="pipeline-search-row">
        <span class="pipeline-search-icon">🔍</span>
        <input
          v-model="searchQuery"
          type="text"
          class="pipeline-search-input"
          data-testid="rag-pipeline-search"
          placeholder="Search sessions with debug data..."
          @input="searchSessions"
        />
      </div>

      <!-- Session List -->
      <div class="pipeline-session-list">
        <div
          v-for="session in sessions"
          :key="session.id"
          class="pipeline-session-item"
          :class="{
            'pipeline-session-item--active': selectedSession?.id === session.id,
          }"
          data-testid="pipeline-session-item"
          @click="loadSession(session.id)"
        >
          <span class="pipeline-session-title">{{ session.title }}</span>
          <span class="pipeline-session-meta"
            >{{ session.message_count }} msgs</span
          >
        </div>
        <div
          v-if="!isLoading && sessions.length === 0"
          class="pipeline-session-empty"
        >
          <span>No sessions with debug data found</span>
        </div>
      </div>
    </div>

    <div class="pipeline-right-panel">
      <!-- Empty state -->
      <div v-if="!hasActiveSession" class="pipeline-empty-state">
        <span class="pipeline-empty-icon">📊</span>
        <h3 class="pipeline-empty-title">RAG Pipeline Debug</h3>
        <p class="pipeline-empty-desc">
          Search and select a session with debug data to view the 7-step RAG
          pipeline visualization.
        </p>
      </div>

      <!-- Pipeline Visualization -->
      <template v-if="hasActiveSession && selectedSession">
        <div class="pipeline-session-header">
          <h3 class="pipeline-session-header-title">
            {{ selectedSession.title }}
          </h3>
          <span class="pipeline-session-header-meta"
            >{{ selectedSession.message_count }} messages</span
          >
        </div>

        <div class="pipeline-messages-list">
          <div
            v-for="msg in selectedMessages"
            :key="msg.id"
            class="pipeline-message"
            :class="{
              'pipeline-message--user': msg.role === 'user',
              'pipeline-message--assistant': msg.role === 'assistant',
            }"
            data-testid="pipeline-msg"
          >
            <div class="pipeline-msg-role">{{ msg.role }}</div>
            <div class="pipeline-msg-content">
              {{ msg.content.slice(0, 200) }}
            </div>
            <div
              v-if="msg.role === 'assistant' && msg.debug_data"
              class="pipeline-debug-section"
            >
              <div
                v-for="step in pipelineSteps"
                :key="step.id"
                class="pipeline-step"
                :data-testid="'pipeline-step'"
                :data-status="step.status"
              >
                <details class="pipeline-step-details">
                  <summary class="pipeline-step-summary">
                    <span class="pipeline-step-number">{{ step.id }}</span>
                    <span
                      class="pipeline-step-name"
                      data-testid="pipeline-step-title"
                      >{{ step.name }}</span
                    >
                    <span
                      class="pipeline-step-badge"
                      :class="`pipeline-step-badge--${step.status}`"
                    >
                      {{ step.status === "active" ? "✓" : "○" }}
                    </span>
                  </summary>
                  <div class="pipeline-step-body">
                    <template v-if="step.id === 1">
                      <div
                        v-if="
                          parseDebugData(msg) &&
                          getStepData(parseDebugData(msg)!, step.key)
                        "
                      >
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Variants</span>
                          <span class="pipeline-meta-value">{{
                            (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as MultiQueryData
                            ).variants.length
                          }}</span>
                        </div>
                        <div
                          v-for="(variant, vi) in (
                            getStepData(
                              parseDebugData(msg)!,
                              step.key,
                            ) as MultiQueryData
                          ).variants"
                          :key="vi"
                          class="pipeline-variant-item"
                        >
                          <code>{{ variant }}</code>
                        </div>
                      </div>
                      <p v-else class="pipeline-step-placeholder">
                        No data available
                      </p>
                    </template>

                    <template v-if="step.id === 2">
                      <div
                        v-if="
                          parseDebugData(msg) &&
                          getStepData(parseDebugData(msg)!, step.key)
                        "
                      >
                        <div
                          v-for="(hr, hi) in (
                            getStepData(
                              parseDebugData(msg)!,
                              step.key,
                            ) as HydeData
                          ).per_query"
                          :key="hi"
                          class="pipeline-hyde-item"
                        >
                          <div class="pipeline-meta-row">
                            <span class="pipeline-meta-label">Query</span>
                            <span class="pipeline-meta-value">{{
                              hr.query
                            }}</span>
                          </div>
                          <div class="pipeline-meta-row">
                            <span class="pipeline-meta-label"
                              >Hypothetical doc</span
                            >
                            <span class="pipeline-meta-value"
                              >{{ hr.hypothetical_doc.slice(0, 150) }}...</span
                            >
                          </div>
                        </div>
                      </div>
                      <p v-else class="pipeline-step-placeholder">
                        No data available
                      </p>
                    </template>

                    <template v-if="step.id === 3">
                      <div
                        v-if="
                          parseDebugData(msg) &&
                          getStepData(parseDebugData(msg)!, step.key)
                        "
                      >
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Top K</span>
                          <span class="pipeline-meta-value">{{
                            getStepData(parseDebugData(msg)!, step.key).top_k
                          }}</span>
                        </div>
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Results</span>
                          <span class="pipeline-meta-value">{{
                            getStepData(parseDebugData(msg)!, step.key)
                              .result_count
                          }}</span>
                        </div>
                        <div class="pipeline-results">
                          <div
                            v-for="(result, ri) in getStepData(
                              parseDebugData(msg)!,
                              step.key,
                            ).results"
                            :key="ri"
                            class="pipeline-result-item"
                          >
                            <span class="pipeline-result-doc">{{
                              result.document_name
                            }}</span>
                            <span class="pipeline-result-score">{{
                              result.score.toFixed(4)
                            }}</span>
                          </div>
                        </div>
                      </div>
                      <p v-else class="pipeline-step-placeholder">
                        No data available
                      </p>
                    </template>

                    <template v-if="step.id === 4">
                      <div
                        v-if="
                          parseDebugData(msg) &&
                          getStepData(parseDebugData(msg)!, step.key)
                        "
                      >
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Tokens</span>
                          <span class="pipeline-meta-value">{{
                            (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as KeywordSearchData
                            ).query_tokens.join(", ")
                          }}</span>
                        </div>
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Matches</span>
                          <span class="pipeline-meta-value">{{
                            (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as KeywordSearchData
                            ).total_matches
                          }}</span>
                        </div>
                        <div class="pipeline-results">
                          <div
                            v-for="(result, ri) in (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as KeywordSearchData
                            ).results"
                            :key="ri"
                            class="pipeline-result-item"
                          >
                            <span class="pipeline-result-doc">{{
                              result.document_name
                            }}</span>
                            <span class="pipeline-result-score">{{
                              result.score.toFixed(2)
                            }}</span>
                          </div>
                        </div>
                      </div>
                      <p v-else class="pipeline-step-placeholder">
                        No data available
                      </p>
                    </template>

                    <template v-if="step.id === 5">
                      <div
                        v-if="
                          parseDebugData(msg) &&
                          getStepData(parseDebugData(msg)!, step.key)
                        "
                      >
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Input chunks</span>
                          <span class="pipeline-meta-value">{{
                            (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as MergeDedupData
                            ).input_chunks
                          }}</span>
                        </div>
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">After dedup</span>
                          <span class="pipeline-meta-value">{{
                            (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as MergeDedupData
                            ).after_dedup
                          }}</span>
                        </div>
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Vector chunks</span>
                          <span class="pipeline-meta-value">{{
                            (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as MergeDedupData
                            ).source_breakdown.vector_chunks
                          }}</span>
                        </div>
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label"
                            >Keyword chunks</span
                          >
                          <span class="pipeline-meta-value">{{
                            (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as MergeDedupData
                            ).source_breakdown.keyword_chunks
                          }}</span>
                        </div>
                      </div>
                      <p v-else class="pipeline-step-placeholder">
                        No data available
                      </p>
                    </template>

                    <template v-if="step.id === 6">
                      <div
                        v-if="
                          parseDebugData(msg) &&
                          getStepData(parseDebugData(msg)!, step.key)
                        "
                      >
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Input</span>
                          <span class="pipeline-meta-value">{{
                            (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as RerankingData
                            ).input_count
                          }}</span>
                        </div>
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Accepted</span>
                          <span class="pipeline-meta-value">{{
                            (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as RerankingData
                            ).accepted
                          }}</span>
                        </div>
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Rejected</span>
                          <span class="pipeline-meta-value">{{
                            (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as RerankingData
                            ).rejected
                          }}</span>
                        </div>
                        <div class="pipeline-results">
                          <div
                            v-for="(result, ri) in (
                              getStepData(
                                parseDebugData(msg)!,
                                step.key,
                              ) as RerankingData
                            ).results"
                            :key="ri"
                            class="pipeline-result-item"
                          >
                            <span class="pipeline-result-doc">{{
                              result.verdict
                            }}</span>
                            <span
                              class="pipeline-result-score"
                              :class="{
                                'pipeline-result-score--accepted':
                                  result.verdict === 'брать',
                                'pipeline-result-score--rejected':
                                  result.verdict === 'не брать',
                              }"
                            >
                              {{ result.score }}/10
                            </span>
                          </div>
                        </div>
                      </div>
                      <p v-else class="pipeline-step-placeholder">
                        No data available
                      </p>
                    </template>

                    <template v-if="step.id === 7">
                      <div
                        v-if="
                          parseDebugData(msg) &&
                          getStepData(parseDebugData(msg)!, step.key)
                        "
                      >
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Model</span>
                          <span class="pipeline-meta-value">{{
                            getStepData(parseDebugData(msg)!, step.key).model
                          }}</span>
                        </div>
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label"
                            >Chunks in context</span
                          >
                          <span class="pipeline-meta-value">{{
                            getStepData(parseDebugData(msg)!, step.key)
                              .chunks_in_context
                          }}</span>
                        </div>
                        <div class="pipeline-meta-row">
                          <span class="pipeline-meta-label">Latency</span>
                          <span class="pipeline-meta-value"
                            >{{
                              getStepData(parseDebugData(msg)!, step.key)
                                .latency_ms
                            }}
                            ms</span
                          >
                        </div>
                        <details class="pipeline-prompt-preview">
                          <summary>Prompt preview</summary>
                          <pre>{{
                            getStepData(parseDebugData(msg)!, step.key)
                              .prompt_preview
                          }}</pre>
                        </details>
                      </div>
                      <p v-else class="pipeline-step-placeholder">
                        No data available
                      </p>
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
.rag-pipeline-debug-view {
  display: flex;
  flex: 1;
  gap: 24px;
  overflow: hidden;
  height: 100%;
}

.pipeline-left-panel {
  width: 340px;
  min-width: 340px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.pipeline-search-row {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg, 12px);
  padding: 8px 12px;
}

.pipeline-search-icon {
  font-size: 14px;
  flex-shrink: 0;
}

.pipeline-search-input {
  flex: 1;
  border: none;
  background: none;
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  color: var(--color-foreground);
  outline: none;
}

.pipeline-search-input::placeholder {
  color: var(--color-muted-foreground);
}

.pipeline-session-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.pipeline-session-item {
  padding: 10px 12px;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg, 12px);
  cursor: pointer;
  transition: border-color var(--transition-fast);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.pipeline-session-item:hover {
  border-color: var(--color-primary);
}

.pipeline-session-item--active {
  border-color: var(--color-primary);
  background: var(--color-primary-bg, rgba(99, 102, 241, 0.06));
}

.pipeline-session-title {
  font-size: var(--font-size-sm, 13px);
  font-weight: 600;
  color: var(--color-foreground);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pipeline-session-meta {
  font-size: var(--font-size-xs, 11px);
  color: var(--color-muted-foreground);
}

.pipeline-session-empty {
  padding: 20px;
  text-align: center;
  color: var(--color-muted-foreground);
  font-size: var(--font-size-sm, 13px);
}

.pipeline-right-panel {
  flex: 1;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl, 16px);
  padding: 20px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.pipeline-empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--color-muted-foreground);
}

.pipeline-empty-icon {
  font-size: 32px;
}

.pipeline-empty-title {
  font-size: var(--font-size-lg, 16px);
  font-weight: 600;
  color: var(--color-foreground);
  margin: 0;
}

.pipeline-empty-desc {
  font-size: var(--font-size-sm, 13px);
  text-align: center;
  max-width: 360px;
  margin: 0;
}

.pipeline-session-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--color-border);
}

.pipeline-session-header-title {
  font-size: var(--font-size-md, 15px);
  font-weight: 600;
  color: var(--color-foreground);
  margin: 0;
}

.pipeline-session-header-meta {
  font-size: var(--font-size-xs, 11px);
  color: var(--color-muted-foreground);
}

.pipeline-messages-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.pipeline-message {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: var(--color-background);
  border-radius: var(--radius-lg, 12px);
}

.pipeline-msg-role {
  font-size: var(--font-size-xs, 11px);
  font-weight: 700;
  text-transform: uppercase;
  color: var(--color-muted-foreground);
}

.pipeline-msg-content {
  font-size: var(--font-size-sm, 13px);
  color: var(--color-foreground);
  line-height: 1.5;
}

.pipeline-debug-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 8px;
  border-top: 1px solid var(--color-border);
  padding-top: 8px;
}

.pipeline-step-details {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md, 8px);
  overflow: hidden;
}

.pipeline-step-summary {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  cursor: pointer;
  font-size: var(--font-size-sm, 13px);
}

.pipeline-step-number {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--color-primary);
  color: #fff;
  font-size: 11px;
  font-weight: 700;
  flex-shrink: 0;
}

.pipeline-step-name {
  flex: 1;
  font-weight: 600;
  color: var(--color-foreground);
}

.pipeline-step-badge {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  font-weight: 600;
}

.pipeline-step-badge--active {
  background: var(--color-success-bg, rgba(34, 197, 94, 0.1));
  color: var(--color-success, #22c55e);
}

.pipeline-step-badge--future {
  background: var(--color-muted-bg, rgba(156, 163, 175, 0.1));
  color: var(--color-muted-foreground);
}

.pipeline-step-body {
  padding: 12px;
  border-top: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.pipeline-step-placeholder {
  font-size: var(--font-size-xs, 11px);
  color: var(--color-muted-foreground);
  font-style: italic;
  margin: 0;
}

.pipeline-meta-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 0;
}

.pipeline-meta-label {
  font-size: var(--font-size-xs, 11px);
  color: var(--color-muted-foreground);
  font-weight: 500;
}

.pipeline-meta-value {
  font-size: var(--font-size-xs, 11px);
  color: var(--color-foreground);
  font-weight: 600;
}

.pipeline-results {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 8px;
}

.pipeline-result-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 8px;
  background: var(--color-background);
  border-radius: var(--radius-sm, 6px);
}

.pipeline-result-doc {
  font-size: var(--font-size-xs, 11px);
  color: var(--color-foreground);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pipeline-result-score {
  font-size: var(--font-size-xs, 11px);
  font-weight: 600;
  color: var(--color-muted-foreground);
  flex-shrink: 0;
}

.pipeline-result-score--accepted {
  color: var(--color-success, #22c55e);
}

.pipeline-result-score--rejected {
  color: var(--color-error, #ef4444);
}

.pipeline-variant-item {
  padding: 4px 0;
}

.pipeline-variant-item code {
  font-size: var(--font-size-xs, 11px);
  color: var(--color-primary);
  background: var(--color-primary-bg, rgba(99, 102, 241, 0.06));
  padding: 2px 6px;
  border-radius: 4px;
}

.pipeline-hyde-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px 0;
}

.pipeline-prompt-preview {
  margin-top: 8px;
}

.pipeline-prompt-preview summary {
  font-size: var(--font-size-xs, 11px);
  color: var(--color-primary);
  cursor: pointer;
  font-weight: 600;
}

.pipeline-prompt-preview pre {
  margin-top: 8px;
  padding: 12px;
  background: var(--color-background);
  border-radius: var(--radius-md, 8px);
  font-size: var(--font-size-xs, 11px);
  white-space: pre-wrap;
  overflow-x: auto;
  max-height: 200px;
}
</style>
