import type { Message } from '@/api/types';
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import MessageBubble from '../MessageBubble.vue';

const createUserMessage = (overrides: Partial<Message> = {}): Message => ({
  id: 'msg-1',
  session_id: 'sess-1',
  role: 'user',
  content: 'Hello!',
  created_at: '2026-06-15T12:00:00Z',
  ...overrides,
});

const createAssistantMessage = (overrides: Partial<Message> = {}): Message => ({
  id: 'msg-2',
  session_id: 'sess-1',
  role: 'assistant',
  content: 'Hi there!',
  created_at: '2026-06-15T12:00:05Z',
  ...overrides,
});

describe('MessageBubble', () => {
  it('renders user message with correct role class', () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    expect(wrapper.classes('message-user')).toBe(true);
  });

  it('renders assistant message with correct role class', () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createAssistantMessage() },
    });
    expect(wrapper.classes('message-assistant')).toBe(true);
  });

  it('renders markdown content', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({ content: '**bold** text' }),
      },
    });
    expect(wrapper.html()).toContain('<strong>bold</strong>');
  });

  it('shows source toggle when sources are present', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({
          sources: JSON.stringify([
            {
              document_id: 'doc-1',
              document_name: 'test.pdf',
              chunk_index: 0,
              text: 'some content',
              relevance: 0.95,
            },
          ]),
        }),
      },
    });
    expect(wrapper.text()).toContain('source');
  });

  it('does not show source toggle for user messages with sources', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createUserMessage({
          sources: JSON.stringify([
            {
              document_id: 'doc-1',
              document_name: 'test.pdf',
              chunk_index: 0,
              text: 'content',
              relevance: 0.9,
            },
          ]),
        }),
      },
    });
    expect(wrapper.find('.sources-section').exists()).toBe(false);
  });

  it('renders streaming bar when streaming and no content', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({ content: '' }),
        isStreaming: true,
      },
    });
    expect(wrapper.find('.streaming-bar').exists()).toBe(true);
  });

  it('renders streaming cursor when streaming with content', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage(),
        isStreaming: true,
      },
    });
    expect(wrapper.find('.streaming-cursor').exists()).toBe(true);
  });

  it('renders empty state gracefully for empty content', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({ content: '' }),
      },
    });
    expect(wrapper.find('.markdown-body').exists()).toBe(false);
  });

  it('displays formatted timestamp', () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    expect(wrapper.find('.message-time').exists()).toBe(true);
  });
});
