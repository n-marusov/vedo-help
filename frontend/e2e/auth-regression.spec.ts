import { expect, test } from '@playwright/test';
import { API_URL, createTestCollection, getTestAccessToken, setupAuth } from './helpers';

test.describe('Auth Regression: real backend API calls', () => {
  test('TC-AUTH-REG-001: collection API accepts a real KeyCloak JWT', async ({ page, request }) => {
    const token = await setupAuth(page);
    await createTestCollection(request, `Auth Regression ${Date.now()}`);

    const response = await request.get(`${API_URL}/api/collections`, {
      headers: { Authorization: `Bearer ${token}` },
    });

    expect(response.ok()).toBe(true);
    expect(await response.json()).toEqual(expect.any(Array));
  });

  test('TC-AUTH-REG-002: app renders with real backend collections', async ({ page, request }) => {
    await setupAuth(page);
    await createTestCollection(request, `Auth UI ${Date.now()}`);

    await page.goto('/');
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible({
      timeout: 10000,
    });
    await expect(page).not.toHaveURL(/\/login/);
  });

  test('TC-AUTH-REG-003: multiple backend requests accept the same auth token', async ({
    request,
  }) => {
    const token = await getTestAccessToken();
    const headers = { Authorization: `Bearer ${token}` };

    const collections = await request.get(`${API_URL}/api/collections`, {
      headers,
    });
    const sessions = await request.get(`${API_URL}/api/sessions`, {
      headers,
    });
    const me = await request.get(`${API_URL}/api/auth/me`, {
      headers,
    });

    expect(collections.ok()).toBe(true);
    expect(sessions.ok()).toBe(true);
    expect(me.ok()).toBe(true);
  });
});

test.describe('Auth Regression: token handling', () => {
  test('TC-AUTH-REG-010: missing token redirects to login', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-REG-012: API routes require auth header', async ({ request }) => {
    const endpoints = [
      { method: 'GET', path: '/api/collections' },
      { method: 'POST', path: '/api/collections' },
      { method: 'GET', path: '/api/documents' },
      { method: 'GET', path: '/api/sessions' },
      { method: 'POST', path: '/api/sessions' },
      { method: 'GET', path: '/api/auth/me' },
    ] as const;

    for (const ep of endpoints) {
      const response = await request.fetch(`${API_URL}${ep.path}`, {
        method: ep.method,
        headers: { 'Content-Type': 'application/json' },
      });
      expect(response.status(), `Expected 401 for ${ep.method} ${ep.path} without auth`).toBe(401);
    }
  });

  test('TC-AUTH-REG-013: API routes return 401 with invalid token', async ({ request }) => {
    const endpoints = [
      { method: 'GET', path: '/api/collections' },
      { method: 'POST', path: '/api/collections' },
      { method: 'GET', path: '/api/documents' },
      { method: 'GET', path: '/api/sessions' },
      { method: 'POST', path: '/api/sessions' },
      { method: 'GET', path: '/api/auth/me' },
    ] as const;

    for (const ep of endpoints) {
      const response = await request.fetch(`${API_URL}${ep.path}`, {
        method: ep.method,
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer invalid-token-that-will-fail',
        },
      });
      expect(response.status(), `Expected 401 for ${ep.method} ${ep.path} with invalid token`).toBe(
        401,
      );
    }
  });
});
