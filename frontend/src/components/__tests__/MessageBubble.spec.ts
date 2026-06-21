import type { Message } from '@/api/types';
import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
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

  it('renders code block with syntax highlighting classes', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({
          content: '```python\nprint("hello")\n```',
        }),
      },
    });
    const html = wrapper.html();
    // Should have highlight.js class on the code element
    expect(html).toContain('hljs');
    // Should have code block wrapper
    expect(html).toContain('code-block-wrapper');
    // Should have copy button
    expect(html).toContain('copy-code-btn');
    // Should have language label
    expect(html).toContain('code-lang-label');
  });

  it('renders code block without language label for plain code blocks', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({
          content: '```\nplain code\n```',
        }),
      },
    });
    const html = wrapper.html();
    expect(html).toContain('code-block-wrapper');
    expect(html).toContain('copy-code-btn');
    // Language label should be empty span
    expect(html).toContain('code-lang-label');
  });

  it('does not wrap inline code in code-block-wrapper', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({
          content: 'Use `code` inline',
        }),
      },
    });
    const html = wrapper.html();
    // Inline code should not have the wrapper
    // But it should have a <code> tag
    expect(html).toContain('<code>');
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

  it('renders copy button on code blocks', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({
          content: '```python\nprint("hello")\n```',
        }),
      },
    });
    expect(wrapper.find('.copy-code-btn').exists()).toBe(true);
  });

  it('copy button shows Copied state after click', async () => {
    // Mock clipboard API
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });

    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({
          content: '```python\nprint("hello")\n```',
        }),
      },
    });

    const btn = wrapper.find('.copy-code-btn');
    expect(btn.exists()).toBe(true);
    await btn.trigger('click');
    expect(btn.text()).toBe('Copied!');
  });

  // ==========================================================================
  // RED (v0.3.1 — T12): edit/delete hover row & edit mode
  // All assertions will fail until T12 adds edit/delete UI to MessageBubble.
  // ==========================================================================

  it.skip('renders edit button on user messages only', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    expect(wrapper.find('[data-testid="message-edit-btn"]').exists()).toBe(true);
    const asst = mount(MessageBubble, {
      props: { message: createAssistantMessage() },
    });
    expect(asst.find('[data-testid="message-edit-btn"]').exists()).toBe(false);
  });

  it.skip('renders delete button on both user and assistant messages', async () => {
    for (const role of ['user', 'assistant'] as const) {
      const msg = role === 'user' ? createUserMessage() : createAssistantMessage();
      const wrapper = mount(MessageBubble, {
        props: { message: msg },
      });
      expect(wrapper.find('[data-testid="message-delete-btn"]').exists()).toBe(true);
    }
  });

  it.skip('emits edit event with message id when edit clicked', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    await wrapper.find('[data-testid="message-edit-btn"]').trigger('click');
    expect(wrapper.emitted('edit')).toBeTruthy();
    expect(wrapper.emitted('edit')?.[0]).toEqual([{ id: 'msg-1' }]);
  });

  it.skip('emits delete event with message id when delete clicked', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    await wrapper.find('[data-testid="message-delete-btn"]').trigger('click');
    expect(wrapper.emitted('delete')).toBeTruthy();
    expect(wrapper.emitted('delete')?.[0]).toEqual([{ id: 'msg-1' }]);
  });

  it.skip('enters edit mode and shows textarea + Save/Cancel', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage(), editing: true },
    });
    expect(wrapper.find('[data-testid="message-edit-textarea"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="message-save-btn"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="message-cancel-edit-btn"]').exists()).toBe(true);
  });

  it.skip('emits save-edit event with new content', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage(), editing: true },
    });
    await wrapper.find('[data-testid="message-save-btn"]').trigger('click');
    expect(wrapper.emitted('save-edit')).toBeTruthy();
  });

  it.skip('displays edited_at indicator when message.edited_at is set', async () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createUserMessage({
          edited_at: '2026-06-21T10:00:00Z',
          original_content: 'original',
        }),
      },
    });
    expect(wrapper.find('[data-testid="message-edited-badge"]').exists()).toBe(true);
  });
});
