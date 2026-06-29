// STUB: Minimal implementation for TDD (Phase 2). Full implementation in Phase 6 (Task 6.2).
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

// Runtime type used internally by the stub — will be replaced by real types in Phase 6.
interface PipelineStageEvent {
  stage: string;
  data: Record<string, unknown>;
  latency_ms: number;
}

export const useRagDebugStore = defineStore('ragDebug', () => {
  const stages = ref<PipelineStageEvent[]>([]);
  const isCollecting = ref(false);
  const sessionId = ref<string | null>(null);

  function addStage(stage: PipelineStageEvent) {
    stages.value.push(stage);
  }

  function clear() {
    stages.value = [];
  }

  function startCollection(sid: string) {
    sessionId.value = sid;
    isCollecting.value = true;
  }

  function endCollection() {
    isCollecting.value = false;
  }

  const latestStages = computed(() => stages.value.slice().reverse());

  return {
    stages,
    isCollecting,
    sessionId,
    addStage,
    clear,
    startCollection,
    endCollection,
    latestStages,
  };
});
