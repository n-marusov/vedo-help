import type { Message } from '@/api/types';
import MessageBubble from '@/components/MessageBubble.vue';
import { useChatStore } from '@/stores/chat';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it } from 'vitest';

function createMessage(overrides: Partial<Message> = {}) {
  return {
    id: 'test-1',
    session_id: 'session-1',
    role: 'assistant' as const,
    content: '',
    created_at: new Date().toISOString(),
    ...overrides,
  };
}

describe('MessageBubble pipeline stage indicator', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('shows stage label when streaming and pipelineStageLabel is set', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createMessage(),
        isStreaming: true,
        index: 0,
        pipelineStageLabel: 'Searching vector DB...',
      },
    });

    const streamingIndicator = wrapper.find('[data-testid="streaming-indicator"]');
    expect(streamingIndicator.exists()).toBe(true);
    expect(streamingIndicator.text()).toContain('Searching vector DB...');
  });

  it('shows dots fallback when streaming but pipelineStageLabel is null', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createMessage(),
        isStreaming: true,
        index: 0,
        pipelineStageLabel: null,
      },
    });

    const streamingIndicator = wrapper.find('[data-testid="streaming-indicator"]');
    expect(streamingIndicator.exists()).toBe(true);
    expect(streamingIndicator.text()).toContain('Assistant is typing');
  });

  it('does not show streaming indicator when not streaming', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createMessage({ content: 'Hello' }),
        isStreaming: false,
        index: 0,
      },
    });

    const streamingIndicator = wrapper.find('[data-testid="streaming-indicator"]');
    expect(streamingIndicator.exists()).toBe(false);
  });
});
