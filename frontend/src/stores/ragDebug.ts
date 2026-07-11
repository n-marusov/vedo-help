import type { DebugData } from '@/api/types';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

export const useRagDebugStore = defineStore('ragDebug', () => {
  /** Current pipeline stage name (from SSE pipeline_stage events). */
  const pipelineStage = ref<string | null>(null);

  /** Latest RAG debug data (from SSE debug event). */
  const debugData = ref<DebugData | null>(null);

  /** Human-readable labels for each pipeline stage. */
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

  /** Computed human-readable label for the current stage. */
  const pipelineStageLabel = computed(() => {
    if (!pipelineStage.value) return null;
    return stageLabels[pipelineStage.value] || pipelineStage.value;
  });

  /** Update the current pipeline stage (called from SSE handler). */
  function setPipelineStage(stage: string | null) {
    pipelineStage.value = stage;
  }

  /** Update the latest debug data (called from SSE handler). */
  function setDebugData(data: DebugData | null) {
    debugData.value = data;
  }

  /** Get data for a specific pipeline step key. */
  function getStepData(key: string): unknown {
    if (!debugData.value) return null;
    return (debugData.value as unknown as Record<string, unknown>)[key] || null;
  }

  /** Reset all debug state (called on new query or error). */
  function reset() {
    pipelineStage.value = null;
    debugData.value = null;
  }

  return {
    pipelineStage,
    pipelineStageLabel,
    debugData,
    stageLabels,
    setPipelineStage,
    setDebugData,
    getStepData,
    reset,
  };
});
