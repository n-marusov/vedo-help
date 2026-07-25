<script setup lang="ts">
import { api } from '@/api/client';
import type { ModelOption } from '@/api/types';
import VButton from '@/components/ui/VButton.vue';
import VDialog from '@/components/ui/VDialog.vue';
import VInput from '@/components/ui/VInput.vue';
import VSelect from '@/components/ui/VSelect.vue';
import VSkeleton from '@/components/ui/VSkeleton.vue';
import VToast from '@/components/ui/VToast.vue';
import { computed, onMounted, ref } from 'vue';

interface SettingsForm {
  advanced_rag_enabled: boolean;
  multi_query_enabled: boolean;
  hyde_enabled: boolean;
  bm25_enabled: boolean;
  reranking_enabled: boolean;
  chunk_method: string;
  chunk_size: number;
  chunk_overlap: number;
  hybrid_top_k: number;
  rerank_top_k: number;
  multi_query_count: number;
  llm_model: string;
  llm_rerank_model: string;
  embedding_model: string;
  llm_max_history_messages: number;
  llm_context_token_budget: number;
}

const DEFAULTS: SettingsForm = {
  advanced_rag_enabled: true,
  multi_query_enabled: true,
  hyde_enabled: true,
  bm25_enabled: true,
  reranking_enabled: true,
  chunk_method: 'paragraph',
  chunk_size: 1000,
  chunk_overlap: 200,
  hybrid_top_k: 20,
  rerank_top_k: 5,
  multi_query_count: 3,
  llm_model: 'anthropic/claude-sonnet-4.6',
  llm_rerank_model: 'cohere/rerank-4-pro',
  embedding_model: 'sentence-transformers/all-minilm-l6-v2',
  llm_max_history_messages: 20,
  llm_context_token_budget: 6000,
};

// ── Model lists fetched from backend (single source of truth) ──
const llmModels = ref<ModelOption[]>([]);
const embeddingModels = ref<ModelOption[]>([]);
const rerankModels = ref<ModelOption[]>([]);

const loading = ref(true);
const saving = ref(false);
const form = ref<SettingsForm>({ ...DEFAULTS });
const originalForm = ref<SettingsForm>({ ...DEFAULTS });
const toastMessage = ref('');
const toastType = ref<'info' | 'success' | 'error' | 'warning'>('success');
const toastShow = ref(false);
const showResetDialog = ref(false);

const changed = computed(() => {
  for (const key of Object.keys(DEFAULTS) as Array<keyof SettingsForm>) {
    if (form.value[key] !== originalForm.value[key]) {
      return true;
    }
  }
  return false;
});

// ── Model pricing display helpers ──
function getPricing(modelValue: string, options: ModelOption[]): string | null {
  const match = options.find((o) => o.value === modelValue);
  return match?.pricing ?? null;
}

const selectedLlmPricing = computed(() => getPricing(form.value.llm_model, llmModels.value));
const selectedEmbeddingPricing = computed(() =>
  getPricing(form.value.embedding_model, embeddingModels.value),
);
const selectedRerankPricing = computed(() =>
  getPricing(form.value.llm_rerank_model, rerankModels.value),
);

async function loadModels() {
  try {
    const data = await api.getModels();
    llmModels.value = data.llm_models;
    embeddingModels.value = data.embedding_models;
    rerankModels.value = data.rerank_models;
  } catch (err) {
    toastType.value = 'error';
    toastMessage.value = `Failed to load model lists: ${err instanceof Error ? err.message : String(err)}`;
    toastShow.value = true;
  }
}

async function loadSettings() {
  loading.value = true;
  try {
    const data = await api.getSettings();
    const f = form.value;
    f.advanced_rag_enabled =
      typeof data.advanced_rag_enabled === 'boolean'
        ? data.advanced_rag_enabled
        : DEFAULTS.advanced_rag_enabled;
    f.multi_query_enabled =
      typeof data.multi_query_enabled === 'boolean'
        ? data.multi_query_enabled
        : DEFAULTS.multi_query_enabled;
    f.hyde_enabled =
      typeof data.hyde_enabled === 'boolean' ? data.hyde_enabled : DEFAULTS.hyde_enabled;
    f.bm25_enabled =
      typeof data.bm25_enabled === 'boolean' ? data.bm25_enabled : DEFAULTS.bm25_enabled;
    f.reranking_enabled =
      typeof data.reranking_enabled === 'boolean'
        ? data.reranking_enabled
        : DEFAULTS.reranking_enabled;
    f.chunk_method =
      typeof data.chunk_method === 'string' ? data.chunk_method : DEFAULTS.chunk_method;
    f.chunk_size = typeof data.chunk_size === 'number' ? data.chunk_size : DEFAULTS.chunk_size;
    f.chunk_overlap =
      typeof data.chunk_overlap === 'number' ? data.chunk_overlap : DEFAULTS.chunk_overlap;
    f.hybrid_top_k =
      typeof data.hybrid_top_k === 'number' ? data.hybrid_top_k : DEFAULTS.hybrid_top_k;
    f.rerank_top_k =
      typeof data.rerank_top_k === 'number' ? data.rerank_top_k : DEFAULTS.rerank_top_k;
    f.multi_query_count =
      typeof data.multi_query_count === 'number'
        ? data.multi_query_count
        : DEFAULTS.multi_query_count;
    f.llm_model = typeof data.llm_model === 'string' ? data.llm_model : DEFAULTS.llm_model;
    f.llm_rerank_model =
      typeof data.llm_rerank_model === 'string' ? data.llm_rerank_model : DEFAULTS.llm_rerank_model;
    f.embedding_model =
      typeof data.embedding_model === 'string' ? data.embedding_model : DEFAULTS.embedding_model;
    f.llm_max_history_messages =
      typeof data.llm_max_history_messages === 'number'
        ? data.llm_max_history_messages
        : DEFAULTS.llm_max_history_messages;
    f.llm_context_token_budget =
      typeof data.llm_context_token_budget === 'number'
        ? data.llm_context_token_budget
        : DEFAULTS.llm_context_token_budget;
    originalForm.value = { ...f };
  } catch (err) {
    toastType.value = 'error';
    toastMessage.value = `Failed to load settings: ${err instanceof Error ? err.message : String(err)}`;
    toastShow.value = true;
  } finally {
    loading.value = false;
  }
}

async function saveSettings() {
  saving.value = true;
  try {
    const payload: Record<string, unknown> = {
      advanced_rag_enabled: form.value.advanced_rag_enabled,
      multi_query_enabled: form.value.multi_query_enabled,
      hyde_enabled: form.value.hyde_enabled,
      bm25_enabled: form.value.bm25_enabled,
      reranking_enabled: form.value.reranking_enabled,
      chunk_method: form.value.chunk_method,
      chunk_size: form.value.chunk_size,
      chunk_overlap: form.value.chunk_overlap,
      hybrid_top_k: form.value.hybrid_top_k,
      rerank_top_k: form.value.rerank_top_k,
      multi_query_count: form.value.multi_query_count,
      llm_model: form.value.llm_model,
      llm_rerank_model: form.value.llm_rerank_model,
      embedding_model: form.value.embedding_model,
      llm_max_history_messages: form.value.llm_max_history_messages,
      llm_context_token_budget: form.value.llm_context_token_budget,
    };
    await api.updateSettings(payload);
    originalForm.value = { ...form.value };
    toastType.value = 'success';
    toastMessage.value = 'Settings saved successfully';
    toastShow.value = true;
  } catch (err) {
    toastType.value = 'error';
    toastMessage.value = `Failed to save settings: ${err instanceof Error ? err.message : String(err)}`;
    toastShow.value = true;
  } finally {
    saving.value = false;
  }
}

function resetToDefaults() {
  form.value = { ...DEFAULTS };
  showResetDialog.value = false;
  toastType.value = 'info';
  toastMessage.value = 'Settings reset to defaults. Click Save to apply.';
  toastShow.value = true;
}

onMounted(() => {
  loadModels();
  loadSettings();
});
</script>

<template>
  <div class="settings-panel" data-testid="settings-panel">
    <h2 class="settings-title">RAG Settings</h2>

    <VSkeleton v-if="loading" :lines="8" />

    <template v-else>
      <!-- Pipeline Toggles -->
      <section class="settings-section">
        <h3 class="section-title">Pipeline</h3>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Advanced RAG</label>
            <p class="setting-description">
              Master switch. When disabled, only basic semantic search is used.
            </p>
          </div>
          <label class="toggle-switch">
            <input
              v-model="form.advanced_rag_enabled"
              type="checkbox"
              class="toggle-input"
            />
            <span class="toggle-slider" />
          </label>
        </div>

        <div
          class="pipeline-stages"
          :class="{ 'pipeline-stages--disabled': !form.advanced_rag_enabled }"
        >
          <div class="stage-row">
            <div class="setting-info">
              <label class="setting-label">Multi-Query</label>
              <p class="setting-description">
                Generate LLM-powered query variants from the original question
                to capture multiple search perspectives.
              </p>
            </div>
            <label class="toggle-switch">
              <input
                v-model="form.multi_query_enabled"
                type="checkbox"
                class="toggle-input"
                :disabled="!form.advanced_rag_enabled"
              />
              <span class="toggle-slider" />
            </label>
          </div>

          <div class="stage-row">
            <div class="setting-info">
              <label class="setting-label">HyDE</label>
              <p class="setting-description">
                Hypothetical Document Embeddings — generate a synthetic answer
                first, then search by its embedding for better retrieval.
              </p>
            </div>
            <label class="toggle-switch">
              <input
                v-model="form.hyde_enabled"
                type="checkbox"
                class="toggle-input"
                :disabled="!form.advanced_rag_enabled"
              />
              <span class="toggle-slider" />
            </label>
          </div>

          <div class="stage-row">
            <div class="setting-info">
              <label class="setting-label">BM25 Keyword Search</label>
              <p class="setting-description">
                Add keyword-based (text) search results alongside vector search
                for hybrid retrieval.
              </p>
            </div>
            <label class="toggle-switch">
              <input
                v-model="form.bm25_enabled"
                type="checkbox"
                class="toggle-input"
                :disabled="!form.advanced_rag_enabled"
              />
              <span class="toggle-slider" />
            </label>
          </div>

          <div class="stage-row">
            <div class="setting-info">
              <label class="setting-label">LLM Reranking</label>
              <p class="setting-description">
                Use an LLM to evaluate and filter retrieved chunks, keeping only
                the most relevant ones.
              </p>
            </div>
            <label class="toggle-switch">
              <input
                v-model="form.reranking_enabled"
                type="checkbox"
                class="toggle-input"
                :disabled="!form.advanced_rag_enabled"
              />
              <span class="toggle-slider" />
            </label>
          </div>
        </div>
      </section>

      <!-- Chunking Settings -->
      <section class="settings-section">
        <h3 class="section-title">Chunking</h3>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Chunk Method</label>
            <p class="setting-description">
              Strategy for splitting documents into chunks. "Paragraph" splits
              on double newlines; "Fixed" uses fixed-size character chunks with
              overlap.
            </p>
          </div>
          <div class="setting-control">
            <VSelect
              :model-value="form.chunk_method"
              :options="[
                { value: 'paragraph', label: 'Paragraph' },
                { value: 'fixed', label: 'Fixed Size' },
              ]"
              @update:model-value="
                (v: string | null) => {
                  if (v) form.chunk_method = v;
                }
              "
            />
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Chunk Size (chars)</label>
            <p class="setting-description">
              Maximum number of characters per chunk. Range: 100–5000.
            </p>
          </div>
          <div class="setting-control setting-control--narrow">
            <VInput
              :model-value="String(form.chunk_size)"
              type="number"
              @update:model-value="
                (v: string) => {
                  const n = Number(v);
                  if (!Number.isNaN(n) && n >= 100 && n <= 5000) {
                    form.chunk_size = n;
                  }
                }
              "
            />
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Chunk Overlap (chars)</label>
            <p class="setting-description">
              Overlap between consecutive chunks. Range: 0–1000.
            </p>
          </div>
          <div class="setting-control setting-control--narrow">
            <VInput
              :model-value="String(form.chunk_overlap)"
              type="number"
              @update:model-value="
                (v: string) => {
                  const n = Number(v);
                  if (!Number.isNaN(n) && n >= 0 && n <= 1000) {
                    form.chunk_overlap = n;
                  }
                }
              "
            />
          </div>
        </div>
      </section>

      <!-- Search Settings -->
      <section class="settings-section">
        <h3 class="section-title">Search</h3>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Hybrid Top K</label>
            <p class="setting-description">
              Initial chunks to retrieve per search pass (vector + keyword).
              Range: 1–100.
            </p>
          </div>
          <div class="setting-control setting-control--narrow">
            <VInput
              :model-value="String(form.hybrid_top_k)"
              type="number"
              @update:model-value="
                (v: string) => {
                  const n = Number(v);
                  if (!Number.isNaN(n) && n >= 1 && n <= 100) {
                    form.hybrid_top_k = n;
                  }
                }
              "
            />
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Rerank Top K</label>
            <p class="setting-description">
              Max chunks to keep after LLM reranking. Range: 1–50.
            </p>
          </div>
          <div class="setting-control setting-control--narrow">
            <VInput
              :model-value="String(form.rerank_top_k)"
              type="number"
              @update:model-value="
                (v: string) => {
                  const n = Number(v);
                  if (!Number.isNaN(n) && n >= 1 && n <= 50) {
                    form.rerank_top_k = n;
                  }
                }
              "
            />
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Multi-Query Count</label>
            <p class="setting-description">
              Number of query variants to generate for multi-query search.
              Range: 1–10.
            </p>
          </div>
          <div class="setting-control setting-control--narrow">
            <VInput
              :model-value="String(form.multi_query_count)"
              type="number"
              @update:model-value="
                (v: string) => {
                  const n = Number(v);
                  if (!Number.isNaN(n) && n >= 1 && n <= 10) {
                    form.multi_query_count = n;
                  }
                }
              "
            />
          </div>
        </div>
      </section>

      <!-- Model Settings -->
      <section class="settings-section">
        <h3 class="section-title">Models (Output / Embeddings / Rerank)</h3>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">LLM Model (Output)</label>
            <p class="setting-description">
              Main output model for generating chat completions. Select from
              available RouterAI inference models.
            </p>
          </div>
          <div class="setting-control">
            <VSelect
              :model-value="form.llm_model"
              :options="llmModels"
              @update:model-value="
                (v: string | null) => {
                  if (v) form.llm_model = v;
                }
              "
            />
            <p v-if="selectedLlmPricing" class="model-pricing">
              {{ selectedLlmPricing }}
            </p>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Embedding Model</label>
            <p class="setting-description">
              Model used for generating document and query embeddings for vector
              search.
            </p>
          </div>
          <div class="setting-control">
            <VSelect
              :model-value="form.embedding_model"
              :options="embeddingModels"
              @update:model-value="
                (v: string | null) => {
                  if (v) form.embedding_model = v;
                }
              "
            />
            <p v-if="selectedEmbeddingPricing" class="model-pricing">
              {{ selectedEmbeddingPricing }}
            </p>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Rerank Model</label>
            <p class="setting-description">
              Model used for reranking retrieved chunks. Dedicated rerankers
              listed first, then LLMs suitable for prompt-based reranking.
            </p>
          </div>
          <div class="setting-control">
            <VSelect
              :model-value="form.llm_rerank_model"
              :options="rerankModels"
              @update:model-value="
                (v: string | null) => {
                  if (v) form.llm_rerank_model = v;
                }
              "
            />
            <p v-if="selectedRerankPricing" class="model-pricing">
              {{ selectedRerankPricing }}
            </p>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Max History Messages</label>
            <p class="setting-description">
              Max conversation history messages to include in LLM context.
              Range: 1–100.
            </p>
          </div>
          <div class="setting-control setting-control--narrow">
            <VInput
              :model-value="String(form.llm_max_history_messages)"
              type="number"
              @update:model-value="
                (v: string) => {
                  const n = Number(v);
                  if (!Number.isNaN(n) && n >= 1 && n <= 100) {
                    form.llm_max_history_messages = n;
                  }
                }
              "
            />
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-info">
            <label class="setting-label">Context Token Budget</label>
            <p class="setting-description">
              Maximum tokens for LLM context window. Range: 1000–32000.
            </p>
          </div>
          <div class="setting-control setting-control--narrow">
            <VInput
              :model-value="String(form.llm_context_token_budget)"
              type="number"
              @update:model-value="
                (v: string) => {
                  const n = Number(v);
                  if (!Number.isNaN(n) && n >= 1000 && n <= 32000) {
                    form.llm_context_token_budget = n;
                  }
                }
              "
            />
          </div>
        </div>
      </section>

      <!-- Actions -->
      <div class="settings-actions">
        <VButton
          variant="primary"
          :disabled="!changed || saving"
          @click="saveSettings"
        >
          {{ saving ? "Saving..." : "Save Changes" }}
        </VButton>
        <VButton
          variant="ghost"
          :disabled="saving"
          @click="showResetDialog = true"
        >
          Reset to Defaults
        </VButton>
      </div>
    </template>

    <!-- Toast -->
    <VToast
      :show="toastShow"
      :message="toastMessage"
      :type="toastType"
      @close="toastShow = false"
    />

    <!-- Reset Confirmation Dialog -->
    <VDialog
      :open="showResetDialog"
      title="Reset Settings"
      description="This will reset all settings to their factory defaults. Save changes to persist."
      confirm-text="Reset"
      variant="destructive"
      @close="showResetDialog = false"
      @confirm="resetToDefaults"
    />
  </div>
</template>

<style scoped>
.settings-panel {
  flex: 1;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl, 16px);
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 24px;
  overflow-y: auto;
}

.settings-title {
  margin: 0;
  font-size: var(--font-size-xl, 18px);
  font-weight: 700;
  color: var(--color-foreground);
  font-family: var(--font-family);
}

.settings-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px;
  background: var(--color-secondary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg, 12px);
}

.section-title {
  margin: 0;
  font-size: var(--font-size-md, 14px);
  font-weight: 600;
  color: var(--color-foreground);
  font-family: var(--font-family);
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

/* ── Pipeline Stages ── */
.pipeline-stages {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px 16px;
  margin-top: 4px;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md, 8px);
  transition: opacity var(--transition-fast, 150ms);
}

.pipeline-stages--disabled {
  opacity: 0.4;
}

.stage-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.setting-info {
  flex: 1;
  min-width: 0;
}

.setting-label {
  font-size: var(--font-size-sm, 13px);
  font-weight: 600;
  color: var(--color-foreground);
  font-family: var(--font-family);
}

.setting-description {
  margin: 4px 0 0 0;
  font-size: var(--font-size-xs, 12px);
  color: var(--color-muted-foreground);
  font-family: var(--font-family);
  line-height: 1.4;
}

.setting-control {
  flex-shrink: 0;
  min-width: 200px;
}

.setting-control--narrow {
  min-width: 120px;
  max-width: 120px;
}

.model-pricing {
  color: var(--color-muted-foreground);
  font-size: var(--font-size-xs);
  margin: var(--space-1) 0 0 0;
  padding-left: var(--space-1);
}

.settings-actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
}

/* ── Toggle Switch ── */
.toggle-switch {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
  flex-shrink: 0;
}

.toggle-input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  cursor: pointer;
  inset: 0;
  background: var(--color-muted-foreground);
  border-radius: 24px;
  transition: background var(--transition-fast, 150ms);
}

.toggle-slider::before {
  content: "";
  position: absolute;
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background: white;
  border-radius: 50%;
  transition: transform var(--transition-fast, 150ms);
}

.toggle-input:checked + .toggle-slider {
  background: var(--color-primary);
}

.toggle-input:checked + .toggle-slider::before {
  transform: translateX(20px);
}

@media (max-width: 768px) {
  .setting-row {
    flex-direction: column;
    align-items: flex-start;
  }

  .setting-control,
  .setting-control--narrow {
    width: 100%;
    min-width: unset;
    max-width: unset;
  }

  .settings-panel {
    padding: 16px;
  }

  .settings-section {
    padding: 16px;
  }

  .settings-actions {
    flex-direction: column;
  }
}
</style>
