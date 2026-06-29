import { useRagDebugStore } from '@/stores/ragDebug';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it } from 'vitest';

describe('ragDebug Pinia store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('initial state is empty', () => {
    const store = useRagDebugStore();
    expect(store.stages).toEqual([]);
    expect(store.isCollecting).toBe(false);
    expect(store.sessionId).toBeNull();
  });

  it('addStage appends to stages array', () => {
    const store = useRagDebugStore();

    store.addStage({
      stage: 'expanded_questions',
      data: { original_query: 'test', variants: ['q1'], latency_ms: 100 },
      latency_ms: 100,
    });

    expect(store.stages).toHaveLength(1);
    expect(store.stages[0].stage).toBe('expanded_questions');
    expect(store.stages[0].latency_ms).toBe(100);

    // Add a second stage
    store.addStage({
      stage: 'hyde_docs',
      data: { per_query: [] },
      latency_ms: 800,
    });

    expect(store.stages).toHaveLength(2);
    expect(store.stages[1].stage).toBe('hyde_docs');
  });

  it('clear resets state to empty', () => {
    const store = useRagDebugStore();

    store.addStage({
      stage: 'expanded_questions',
      data: {},
      latency_ms: 50,
    });
    store.addStage({
      stage: 'hyde_docs',
      data: {},
      latency_ms: 200,
    });
    expect(store.stages).toHaveLength(2);

    store.clear();
    expect(store.stages).toEqual([]);
  });

  it('clear does not affect isCollecting/sessionId', () => {
    const store = useRagDebugStore();
    store.startCollection('sess-1');
    store.addStage({ stage: 'test', data: {}, latency_ms: 0 });

    store.clear();

    // clear() only resets stages, not collection state
    expect(store.stages).toEqual([]);
    expect(store.isCollecting).toBe(true);
    expect(store.sessionId).toBe('sess-1');
  });

  it('startCollection sets sessionId and isCollecting', () => {
    const store = useRagDebugStore();

    store.startCollection('sess-abc-123');

    expect(store.isCollecting).toBe(true);
    expect(store.sessionId).toBe('sess-abc-123');
  });

  it('endCollection sets isCollecting to false', () => {
    const store = useRagDebugStore();
    store.startCollection('sess-1');
    expect(store.isCollecting).toBe(true);

    store.endCollection();

    expect(store.isCollecting).toBe(false);
    // sessionId should still be preserved
    expect(store.sessionId).toBe('sess-1');
  });

  it('latestStages returns stages in reverse order (most recent first)', () => {
    const store = useRagDebugStore();

    store.addStage({ stage: 'step1', data: {}, latency_ms: 10 });
    store.addStage({ stage: 'step2', data: {}, latency_ms: 20 });
    store.addStage({ stage: 'step3', data: {}, latency_ms: 30 });

    const latest = store.latestStages;
    expect(latest).toHaveLength(3);
    // Most recent first
    expect(latest[0].stage).toBe('step3');
    expect(latest[1].stage).toBe('step2');
    expect(latest[2].stage).toBe('step1');
  });

  it('latestStages does not mutate the original stages array', () => {
    const store = useRagDebugStore();

    store.addStage({ stage: 'step1', data: {}, latency_ms: 10 });

    const before = store.stages.length;
    store.latestStages; // access computed
    expect(store.stages).toHaveLength(before);
    expect(store.stages[0].stage).toBe('step1');
  });
});
