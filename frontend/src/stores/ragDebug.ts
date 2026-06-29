import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import type { PipelineStageEvent } from '@/api/types';

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
