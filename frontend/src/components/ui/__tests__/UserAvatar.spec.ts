import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import UserAvatar from '../UserAvatar.vue';

describe('UserAvatar', () => {
  it('renders user icon for user role', () => {
    const wrapper = mount(UserAvatar, {
      props: { role: 'user' },
    });
    expect(wrapper.find('[data-testid="user-avatar-icon"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="assistant-avatar-icon"]').exists()).toBe(false);
  });

  it('renders assistant icon for assistant role', () => {
    const wrapper = mount(UserAvatar, {
      props: { role: 'assistant' },
    });
    expect(wrapper.find('[data-testid="assistant-avatar-icon"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="user-avatar-icon"]').exists()).toBe(false);
  });

  it('applies default size md when not specified', () => {
    const wrapper = mount(UserAvatar, {
      props: { role: 'user' },
    });
    const el = wrapper.element as HTMLElement;
    expect(el.classList.contains('user-avatar--md')).toBe(true);
  });

  it('applies size class sm when specified', () => {
    const wrapper = mount(UserAvatar, {
      props: { role: 'user', size: 'sm' },
    });
    const el = wrapper.element as HTMLElement;
    expect(el.classList.contains('user-avatar--sm')).toBe(true);
  });

  it('applies size class lg when specified', () => {
    const wrapper = mount(UserAvatar, {
      props: { role: 'user', size: 'lg' },
    });
    const el = wrapper.element as HTMLElement;
    expect(el.classList.contains('user-avatar--lg')).toBe(true);
  });

  it('has correct aria-label for assistant', () => {
    const wrapper = mount(UserAvatar, {
      props: { role: 'assistant' },
    });
    expect(wrapper.attributes('aria-label')).toBe('VEDO assistant avatar');
  });

  it('has correct aria-label for user', () => {
    const wrapper = mount(UserAvatar, {
      props: { role: 'user' },
    });
    expect(wrapper.attributes('aria-label')).toBe('User avatar');
  });
});
