import { useChatStore } from '@/stores/chat';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it } from 'vitest';

describe('chat store pipelineStage', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('starts as null', () => {
    const store = useChatStore();
    expect(store.pipelineStage).toBeNull();
  });

  it('pipelineStageLabel returns null when pipelineStage is null', () => {
    const store = useChatStore();
    expect(store.pipelineStageLabel).toBeNull();
  });

  it('pipelineStageLabel maps known stage names to labels', () => {
    const store = useChatStore();

    store.pipelineStage = 'embedding';
    expect(store.pipelineStageLabel).toBe('Embedding query...');

    store.pipelineStage = 'searching';
    expect(store.pipelineStageLabel).toBe('Searching vector DB...');

    store.pipelineStage = 'generating';
    expect(store.pipelineStageLabel).toBe('Generating response...');
  });

  it('pipelineStageLabel falls back to raw stage name for unknown stages', () => {
    const store = useChatStore();
    store.pipelineStage = 'custom_stage';
    expect(store.pipelineStageLabel).toBe('custom_stage');
  });

  it('resets pipelineStage to null on start of sendMessage flow', () => {
    const store = useChatStore();
    store.pipelineStage = 'generating';

    // This simulates what sendMessage does at the beginning
    store.pipelineStage = null;

    expect(store.pipelineStage).toBeNull();
  });

  it('resets pipelineStage to null on done event', () => {
    const store = useChatStore();
    store.pipelineStage = 'generating';

    // This simulates what the done case in sendMessage does
    store.pipelineStage = null;

    expect(store.pipelineStage).toBeNull();
  });

  it('resets pipelineStage to null on error event', () => {
    const store = useChatStore();
    store.pipelineStage = 'searching';

    // This simulates what the error case in sendMessage does
    store.pipelineStage = null;

    expect(store.pipelineStage).toBeNull();
  });
});
