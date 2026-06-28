import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';

import UserAvatar from './UserAvatar.vue';

describe('UserAvatar', () => {
  it('renders the user avatar with an accessible person icon', () => {
    const wrapper = mount(UserAvatar, {
      props: {
        role: 'user',
      },
    });

    expect(wrapper.classes()).toContain('user-avatar');
    expect(wrapper.classes()).toContain('user-avatar--user');
    expect(wrapper.classes()).toContain('user-avatar--md');
    expect(wrapper.attributes('aria-label')).toBe('User avatar');
    expect(wrapper.find('[data-testid="user-avatar-icon"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="assistant-avatar-icon"]').exists()).toBe(false);
  });

  it('renders the assistant avatar with a branded V icon', () => {
    const wrapper = mount(UserAvatar, {
      props: {
        role: 'assistant',
        size: 'lg',
      },
    });

    expect(wrapper.classes()).toContain('user-avatar--assistant');
    expect(wrapper.classes()).toContain('user-avatar--lg');
    expect(wrapper.attributes('aria-label')).toBe('VEDO assistant avatar');
    expect(wrapper.find('[data-testid="assistant-avatar-icon"]').exists()).toBe(true);
    expect(wrapper.text()).toContain('V');
  });

  it('maps each size prop to a CSS variable based avatar size', () => {
    const cases = [
      ['sm', 'calc(var(--avatar-size) * 0.75)'],
      ['md', 'var(--avatar-size)'],
      ['lg', 'calc(var(--avatar-size) * 1.25)'],
    ] as const;

    for (const [size, expectedAvatarSize] of cases) {
      const wrapper = mount(UserAvatar, {
        props: {
          role: 'user',
          size,
        },
      });

      expect(wrapper.attributes('style')).toContain(`--user-avatar-size: ${expectedAvatarSize}`);
    }
  });

  it('does not log debug info after removing console.debug calls', () => {
    const debugSpy = vi.spyOn(console, 'debug').mockImplementation(() => undefined);

    mount(UserAvatar, {
      props: {
        role: 'assistant',
        size: 'sm',
      },
    });

    expect(debugSpy).not.toHaveBeenCalled();
    debugSpy.mockRestore();
  });
});
