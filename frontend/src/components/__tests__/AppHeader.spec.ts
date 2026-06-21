import { setAccessToken } from '@/api/client';
import { setAuthToken } from '@/stores/auth';
import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
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
  return { wrapper, router };
}

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

      const adminBtn = wrapper.find('.app-header__dropdown-item');
      expect(adminBtn.exists()).toBe(true);
      expect(adminBtn.text()).toBe('Admin Panel');
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

      const adminBtn = wrapper.findAll('.app-header__dropdown-item');
      // Only "Sign Out" should be visible
      const adminPanelBtns = adminBtn.filter((b) => b.text() === 'Admin Panel');
      expect(adminPanelBtns).toHaveLength(0);
    });

    it('does not show Admin Panel when JWT has no realm_access', async () => {
      const token = makeMockJwt({
        sub: 'user-2',
        name: 'No Role User',
      });
      const { wrapper } = await mountHeader(token);

      await wrapper.find('.app-header__user-btn').trigger('click');

      const adminBtn = wrapper.findAll('.app-header__dropdown-item');
      const adminPanelBtns = adminBtn.filter((b) => b.text() === 'Admin Panel');
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
      await wrapper.find('.app-header__dropdown-item').trigger('click');

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

      const items = wrapper.findAll('.app-header__dropdown-item');
      const texts = items.map((b) => b.text());
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

      expect(wrapper.find('.app-header__dropdown-name').text()).toBe(name);
    });
  });
});
