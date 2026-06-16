import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import AppHeader from '../AppHeader.vue';

describe('AppHeader', () => {
  it('renders VEDO branding and the theme toggle', () => {
    const wrapper = mount(AppHeader);

    expect(wrapper.get('[data-testid="app-header"]').text()).toContain('VEDO');
    expect(wrapper.find('[data-testid="theme-toggle"]').exists()).toBe(true);
  });
});
