import ChatView from '@/views/ChatView.vue';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it } from 'vitest';

describe('ChatWindow (ChatView)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('renders welcome screen when no messages', () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="welcome-message"]').exists()).toBe(true);
  });

  it('has a send button', () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="btn-send"]').exists()).toBe(true);
  });

  it('has input textarea', () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="chat-input"]').exists()).toBe(true);
  });

  it('does not show cancel button when not loading', () => {
    const wrapper = mount(ChatView);
    expect(wrapper.find('[data-testid="btn-cancel"]').exists()).toBe(false);
  });

  it('shows cancel button when loading', async () => {
    const wrapper = mount(ChatView);
    const { useChatStore } = await import('@/stores/chat');
    const chatStore = useChatStore();
    chatStore.isLoading = true;
    await wrapper.vm.$nextTick();
    expect(wrapper.find('[data-testid="btn-cancel"]').exists()).toBe(true);
  });
});
