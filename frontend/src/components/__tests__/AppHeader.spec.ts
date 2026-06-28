import { setAccessToken } from '@/api/client';
import { setAuthToken } from '@/stores/auth';
import type { VueWrapper } from '@vue/test-utils';
import { mount } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { h } from 'vue';
import { createRouter, createWebHistory } from 'vue-router';
import AppHeader from '../AppHeader.vue';

/**
 * Build a mock JWT with proper base64url-encoded UTF-8 payload.
 * Header: {"alg":"RS256","typ":"JWT"}
 * Signature: mock
 */
function makeMockJwt(payload: Record<string, unknown>): string {
  const header = btoa(JSON.stringify({ alg: 'RS256', typ: 'JWT' }));
  const json = JSON.stringify(payload);
  const bytes = new TextEncoder().encode(json);
  const binary = String.fromCharCode(...bytes);
  const payloadB64 = btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  return `${header}.${payloadB64}.mocksignature`;
}

const STUB_CHAT = {
  path: '/',
  name: 'chat',
  component: { render: () => h('div', 'Chat') },
};
const STUB_ADMIN = {
  path: '/admin',
  name: 'admin',
  component: { render: () => h('div', 'Admin') },
};

function createTestRouter() {
  const router = createRouter({
    history: createWebHistory(),
    routes: [STUB_CHAT, STUB_ADMIN],
  });
  return router;
}

let activeWrapper: VueWrapper | null = null;

async function mountHeader(token?: string | null, displayName?: string) {
  if (token) {
    setAccessToken(token);
    // Set the auth store state so userName reflects decoded JWT claims
    setAuthToken(token, displayName, 'keycloak');
  } else {
    setAccessToken(null);
  }
  const router = createTestRouter();
  router.push('/');
  await router.isReady();
  const wrapper = mount(AppHeader, {
    global: {
      plugins: [router],
    },
  });
  activeWrapper = wrapper;
  return { wrapper, router };
}

function getUserDropdown(): HTMLElement {
  const dropdown = document.body.querySelector<HTMLElement>('[data-testid="user-dropdown"]');
  if (!dropdown) {
    throw new Error('Expected user dropdown to be rendered');
  }
  return dropdown;
}

function getDropdownItems(): HTMLElement[] {
  return Array.from(document.body.querySelectorAll<HTMLElement>('.app-header__dropdown-item'));
}

afterEach(() => {
  activeWrapper?.unmount();
  activeWrapper = null;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('AppHeader', () => {
  describe('branding and layout', () => {
    it('renders VEDO branding and the theme toggle', async () => {
      const { wrapper } = await mountHeader();
      expect(wrapper.get('[data-testid="app-header"]').text()).toContain('VEDO');
      expect(wrapper.find('[data-testid="theme-toggle"]').exists()).toBe(true);
    });
  });

  describe('logo navigation', () => {
    it('logo is a router-link pointing to /', async () => {
      const { wrapper } = await mountHeader();
      const brandLink = wrapper.find('.app-header__brand');
      expect(brandLink.exists()).toBe(true);
      expect(brandLink.attributes('href')).toBe('/');
    });

    it('navigates to / when clicking the VEDO logo', async () => {
      const { wrapper, router } = await mountHeader();
      const brandLink = wrapper.find('.app-header__brand');
      await brandLink.trigger('click');
      expect(router.currentRoute.value.path).toBe('/');
    });

    it('logo has accessible aria-label', async () => {
      const { wrapper } = await mountHeader();
      const brandLink = wrapper.find('.app-header__brand');
      expect(brandLink.attributes('aria-label')).toBe('VEDO home');
    });
  });

  describe('user menu - admin role', () => {
    it('shows Admin Panel button when JWT has admin role', async () => {
      const token = makeMockJwt({
        sub: 'admin-1',
        name: 'Admin User',
        realm_access: { roles: ['admin'] },
      });
      const { wrapper } = await mountHeader(token);

      // Open the user menu
      await wrapper.find('.app-header__user-btn').trigger('click');

      const adminBtn = getDropdownItems()[0];
      expect(adminBtn).toBeTruthy();
      expect(adminBtn.textContent?.trim()).toBe('Admin Panel');
    });

    it('does not show Admin Panel button when JWT lacks admin role', async () => {
      const token = makeMockJwt({
        sub: 'user-1',
        name: 'Regular User',
        realm_access: { roles: ['user'] },
      });
      const { wrapper } = await mountHeader(token);

      // Open the user menu
      await wrapper.find('.app-header__user-btn').trigger('click');

      const adminBtn = getDropdownItems();
      // Only "Sign Out" should be visible
      const adminPanelBtns = adminBtn.filter((b) => b.textContent?.trim() === 'Admin Panel');
      expect(adminPanelBtns).toHaveLength(0);
    });

    it('does not show Admin Panel when JWT has no realm_access', async () => {
      const token = makeMockJwt({
        sub: 'user-2',
        name: 'No Role User',
      });
      const { wrapper } = await mountHeader(token);

      await wrapper.find('.app-header__user-btn').trigger('click');

      const adminBtn = getDropdownItems();
      const adminPanelBtns = adminBtn.filter((b) => b.textContent?.trim() === 'Admin Panel');
      expect(adminPanelBtns).toHaveLength(0);
    });

    it('navigates to /admin when clicking Admin Panel button', async () => {
      const token = makeMockJwt({
        sub: 'admin-2',
        name: 'Admin Two',
        realm_access: { roles: ['admin'] },
      });
      const { wrapper, router } = await mountHeader(token, 'Admin Two');

      // Spy on router.push
      const pushSpy = vi.spyOn(router, 'push');

      // Open user menu and click Admin Panel
      await wrapper.find('.app-header__user-btn').trigger('click');
      getDropdownItems()[0].click();

      expect(pushSpy).toHaveBeenCalledWith('/admin');
    });

    it('shows Sign Out button alongside Admin Panel for admin users', async () => {
      const token = makeMockJwt({
        sub: 'admin-3',
        name: 'Admin Three',
        realm_access: { roles: ['admin'] },
      });
      const { wrapper } = await mountHeader(token);

      await wrapper.find('.app-header__user-btn').trigger('click');

      const items = getDropdownItems();
      const texts = items.map((b) => b.textContent?.trim());
      expect(texts).toContain('Admin Panel');
      expect(texts).toContain('Sign Out');
    });
  });

  describe('user menu - Cyrillic name display', () => {
    it('displays Cyrillic user name correctly (UTF-8 regression)', async () => {
      const name = 'Николай Марусов';
      const token = makeMockJwt({
        sub: 'user-cyr-1',
        name,
        realm_access: { roles: ['user'] },
      });
      const { wrapper } = await mountHeader(token, name);

      await wrapper.find('.app-header__user-btn').trigger('click');

      expect(getUserDropdown().querySelector('.app-header__dropdown-name')?.textContent).toBe(name);
    });
  });

  describe('user menu reactivity', () => {
    it('shows clickable user menu button when token is set after header mount (regression: non-reactive accessToken)', async () => {
      const { wrapper } = await mountHeader(null);

      // Without a token, the fallback span should be shown (not the button)
      expect(wrapper.find('.app-header__user-btn').exists()).toBe(false);
      expect(wrapper.find('.app-header__user').exists()).toBe(true);

      // Simulate restoreSession setting the token AFTER header is mounted
      // (this was the bug: getAccessToken was a plain let, not a ref,
      //  so the computed never re-evaluated after mount)
      const token = makeMockJwt({
        sub: 'user-react-1',
        name: 'Reactive User',
        realm_access: { roles: ['admin'] },
      });
      setAuthToken(token, 'Reactive User', 'keycloak');
      await wrapper.vm.$nextTick();

      // Now the clickable button should appear
      const button = wrapper.find('.app-header__user-btn');
      expect(button.exists()).toBe(true);
      expect(button.attributes('aria-label')).toBe('Open user menu');

      // Click it and verify the dropdown appears
      await button.trigger('click');
      const dropdown = getUserDropdown();
      expect(dropdown.textContent).toContain('Reactive User');
    });
  });

  describe('user menu positioning', () => {
    it('teleports the dropdown to body and positions it from the user button', async () => {
      const token = makeMockJwt({
        sub: 'user-position-1',
        name: 'Positioned User',
        realm_access: { roles: ['user'] },
      });
      const { wrapper } = await mountHeader(token);
      const button = wrapper.find('.app-header__user-btn').element as HTMLElement;
      vi.spyOn(button, 'getBoundingClientRect').mockReturnValue({
        bottom: 42,
        height: 30,
        left: 900,
        right: 930,
        top: 12,
        width: 30,
        x: 900,
        y: 12,
        toJSON: () => ({}),
      });
      vi.spyOn(window, 'innerWidth', 'get').mockReturnValue(1000);

      await wrapper.find('.app-header__user-btn').trigger('click');

      const dropdown = getUserDropdown();
      expect(wrapper.find('[data-testid="user-dropdown"]').exists()).toBe(false);
      expect(dropdown.style.top).toBe('48px');
      expect(dropdown.style.right).toBe('70px');
      expect(getComputedStyle(dropdown).position).toBe('fixed');
    });
  });
});
