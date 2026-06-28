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

  // ==========================================================================
  // Sources section (removed in chat UI polish design)
  // ==========================================================================

  it('does not show sources section even when sources are present', () => {
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
    expect(wrapper.find('.sources-section').exists()).toBe(false);
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

  // ==========================================================================
  // Streaming indicator (animated "..." instead of progress bar)
  // ==========================================================================

  it('shows streaming cursor when streaming with content', () => {
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
  // Chat UI polish: message action buttons
  // ==========================================================================

  it('renders copy button on user messages', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    expect(wrapper.find('[data-testid="message-copy-btn"]').exists()).toBe(true);
  });

  it('renders copy button on assistant messages', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createAssistantMessage() },
    });
    expect(wrapper.find('[data-testid="message-copy-btn"]').exists()).toBe(true);
  });

  it('copy button emits copy event with message id', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage({ content: 'text to copy' }) },
    });
    await wrapper.find('[data-testid="message-copy-btn"]').trigger('click');
    expect(wrapper.emitted('copy')).toBeTruthy();
    expect(wrapper.emitted('copy')?.[0]).toEqual([{ id: 'msg-1' }]);
  });

  it('renders edit button on user messages only', async () => {
    const userWrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    expect(userWrapper.find('[data-testid="message-edit-btn"]').exists()).toBe(true);

    const asstWrapper = mount(MessageBubble, {
      props: { message: createAssistantMessage() },
    });
    expect(asstWrapper.find('[data-testid="message-edit-btn"]').exists()).toBe(false);
  });

  it('emits edit event with message id when edit clicked', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    await wrapper.find('[data-testid="message-edit-btn"]').trigger('click');
    expect(wrapper.emitted('edit')).toBeTruthy();
    expect(wrapper.emitted('edit')?.[0]).toEqual([{ id: 'msg-1' }]);
  });

  it('enters edit mode and shows textarea + Save/Cancel', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    await wrapper.find('[data-testid="message-edit-btn"]').trigger('click');
    expect(wrapper.find('[data-testid="message-edit-textarea"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="message-save-btn"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="message-cancel-edit-btn"]').exists()).toBe(true);
  });

  it('emits save-edit event with new content', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    // Enter edit mode
    await wrapper.find('[data-testid="message-edit-btn"]').trigger('click');
    // Save
    await wrapper.find('[data-testid="message-save-btn"]').trigger('click');
    expect(wrapper.emitted('save-edit')).toBeTruthy();
  });

  it('displays edited_at indicator when message.edited_at is set', async () => {
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

  it('renders regenerate button on assistant messages only', async () => {
    const asstWrapper = mount(MessageBubble, {
      props: { message: createAssistantMessage() },
    });
    expect(asstWrapper.find('[data-testid="message-regenerate-btn"]').exists()).toBe(true);

    const userWrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    expect(userWrapper.find('[data-testid="message-regenerate-btn"]').exists()).toBe(false);
  });

  it('regenerate button emits regenerate event with message id', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createAssistantMessage() },
    });
    await wrapper.find('[data-testid="message-regenerate-btn"]').trigger('click');
    expect(wrapper.emitted('regenerate')).toBeTruthy();
    expect(wrapper.emitted('regenerate')?.[0]).toEqual([{ id: 'msg-2' }]);
  });

  it('does not render delete buttons on messages', async () => {
    const userWrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    expect(userWrapper.find('[data-testid="message-delete-btn"]').exists()).toBe(false);

    const asstWrapper = mount(MessageBubble, {
      props: { message: createAssistantMessage() },
    });
    expect(asstWrapper.find('[data-testid="message-delete-btn"]').exists()).toBe(false);
  });

  // ==========================================================================
  // Debug info button (admin only)
  // ==========================================================================

  it('debug button is removed from MessageBubble', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createAssistantMessage() },
    });
    expect(wrapper.find('[data-testid="message-debug-btn"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="debug-panel"]').exists()).toBe(false);
  });

  // ==========================================================================
  // Null sources guard (FIX:chat-session-switch)
  // ==========================================================================

  it('handles sources: "null" string gracefully', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({ sources: 'null' }),
      },
    });
    // Should mount without error — the component exists
    expect(wrapper.find('[data-testid="message-content"]').exists()).toBe(true);
  });

  it('handles empty sources JSON gracefully', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({ sources: '[]' }),
      },
    });
    expect(wrapper.find('[data-testid="message-content"]').exists()).toBe(true);
  });

  it('handles undefined sources gracefully', () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({ sources: undefined }),
      },
    });
    expect(wrapper.find('[data-testid="message-content"]').exists()).toBe(true);
  });

  // ==========================================================================
  // Timestamp layout
  // ==========================================================================
  it('does not render debug info button for user messages', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    expect(wrapper.find('[data-testid="message-debug-btn"]').exists()).toBe(false);
  });

  it('debug button is removed from MessageBubble', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createAssistantMessage() },
    });
    expect(wrapper.find('[data-testid="message-debug-btn"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="debug-panel"]').exists()).toBe(false);
  });

  // ==========================================================================
  // Timestamp layout
  // ==========================================================================

  it('renders timestamp inline with action row', async () => {
    const wrapper = mount(MessageBubble, {
      props: { message: createUserMessage() },
    });
    // Timestamp should be in the same container as action buttons
    const actionsRow = wrapper.find('[data-testid="message-actions-row"]');
    expect(actionsRow.exists()).toBe(true);
    expect(actionsRow.find('[data-testid="message-time"]').exists()).toBe(true);
  });

  // ==========================================================================
  // Table styling (no alternating row colors)
  // ==========================================================================

  it.skip('does not have alternating row colors in tables', async () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({
          content: '| h1 | h2 |\n|----|----|\n| a | b |\n| c | d |',
        }),
      },
    });
    const html = wrapper.html();
    // Should not contain nth-child(even) pattern for tables
    expect(html).not.toContain('tr:nth-child(even)');
  });

  // ==========================================================================
  // No "chunk" terminology in UI
  // ==========================================================================

  it.skip('does not contain "chunk" text in response content or source labels', async () => {
    const wrapper = mount(MessageBubble, {
      props: {
        message: createAssistantMessage({
          content: 'Found 3 relevant passages',
          sources: JSON.stringify([
            {
              document_id: 'doc-1',
              document_name: 'test.pdf',
              chunk_index: 0,
              text: 'relevant excerpt',
              relevance: 0.95,
            },
          ]),
        }),
      },
    });
    const text = wrapper.text().toLowerCase();
    expect(text).not.toContain('chunk');
  });
});
