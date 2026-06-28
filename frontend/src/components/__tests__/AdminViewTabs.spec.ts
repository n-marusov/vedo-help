import AdminView from '@/views/AdminView.vue';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it } from 'vitest';

function mountWithPinia() {
  const pinia = createPinia();
  setActivePinia(pinia);
  return mount(AdminView as unknown, {
    global: {
      plugins: [pinia],
      stubs: ['CollectionManager', 'DocumentList', 'GitRepoManager', 'SessionDebug'],
    },
  });
}

describe('AdminViewTabs', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('shows tab bar with two tabs', () => {
    const wrapper = mountWithPinia();

    const tabBar = wrapper.find('[data-testid="admin-tabs"]');
    expect(tabBar.exists()).toBe(true);

    const sourceTab = wrapper.find('[data-testid="admin-tab-sources"]');
    expect(sourceTab.exists()).toBe(true);
    expect(sourceTab.text()).toContain('Collections');

    const debugTab = wrapper.find('[data-testid="admin-tab-debug"]');
    expect(debugTab.exists()).toBe(true);
    expect(debugTab.text()).toContain('Session Debug');
  });

  it('clicking debug tab shows session debug view', async () => {
    const wrapper = mountWithPinia();

    await wrapper.find('[data-testid="admin-tab-debug"]').trigger('click');

    expect(wrapper.findComponent({ name: 'SessionDebug' }).exists()).toBe(true);
  });

  it('clicking sources tab returns to collections', async () => {
    const wrapper = mountWithPinia();

    // First switch to debug
    await wrapper.find('[data-testid="admin-tab-debug"]').trigger('click');
    expect(wrapper.findComponent({ name: 'SessionDebug' }).exists()).toBe(true);

    // Then switch back
    await wrapper.find('[data-testid="admin-tab-sources"]').trigger('click');

    expect(wrapper.findComponent({ name: 'SessionDebug' }).exists()).toBe(false);
  });
});
