/**
 * Auth Regression E2E Tests
 *
 * These tests verify that authenticated API calls correctly include the
 * Authorization header and that the app gracefully handles 401 responses
 * from the backend. They serve as regression tests for the audience-validation
 * bug that caused all /api/* endpoints to return 401.
 *
 * Key scenarios:
 * - Collection API calls include JWT in Authorization header
 * - 401 responses are properly surfaced to the user
 * - Multiple API calls are consistently auth'd
 */

import { expect, test } from '@playwright/test';
import { VALID_TOKEN } from './helpers';

test.describe('Auth Regression: Collection API calls', () => {
  test('TC-AUTH-REG-001: collection API call includes Authorization header with valid JWT', async ({
    page,
  }) => {
    // Track what headers were sent
    let sentAuthHeader: string | null = null;

    await page.addInitScript((token) => {
      localStorage.setItem('vedo_auth_token', token);
    }, VALID_TOKEN);

    // Intercept collection API calls and capture the Authorization header
    await page.route('**/api/collections', async (route, request) => {
      sentAuthHeader = request.headers().authorization ?? null;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'col-1',
            name: 'Test Collection',
            description: 'Regression test collection',
            created_at: new Date().toISOString(),
            document_count: 0,
          },
        ]),
      });
    });

    await page.goto('/');
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 5000 });

    // The store should have fetched collections — verify auth header was sent
    expect(sentAuthHeader).toBe(`Bearer ${VALID_TOKEN}`);
  });

  test('TC-AUTH-REG-002: API 401 on collections does not crash the app', async ({ page }) => {
    await page.addInitScript((token) => {
      localStorage.setItem('vedo_auth_token', token);
    }, VALID_TOKEN);

    // Mock collections to return 401 — simulating audience-validation failure
    await page.route('**/api/collections', async (route) => {
      await route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({
          error: {
            type: 'unauthorized',
            message: 'Invalid or missing token',
          },
        }),
      });
    });

    // The app should not crash — it should show the chat view gracefully
    await page.goto('/');
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 5000 });

    // There should be no visible error toast (collections is not critical for UI)
    // The chat should be accessible (no redirect)
    await expect(page).not.toHaveURL(/\/login/);
  });

  test('TC-AUTH-REG-003: multiple sequential collections calls all carry auth', async ({
    page,
  }) => {
    const authHeaders: string[] = [];

    await page.addInitScript((token) => {
      localStorage.setItem('vedo_auth_token', token);
    }, VALID_TOKEN);

    await page.route('**/api/collections', async (route, request) => {
      authHeaders.push(request.headers().authorization ?? 'MISSING');
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'col-1',
            name: 'Test Collection',
            created_at: new Date().toISOString(),
            document_count: 0,
          },
        ]),
      });
    });

    await page.goto('/');

    // Trigger multiple collection fetches by navigating between views
    // that re-trigger the collection store
    await page.goto('/admin');
    await page.waitForTimeout(500);
    await page.goto('/');
    await page.waitForTimeout(500);

    // All collection API calls should have the auth header
    for (const header of authHeaders) {
      expect(header).toBe(`Bearer ${VALID_TOKEN}`);
    }
  });

  test('TC-AUTH-REG-004: admin page collection manager handles 401 gracefully', async ({
    page,
  }) => {
    await page.addInitScript((token) => {
      localStorage.setItem('vedo_auth_token', token);
    }, VALID_TOKEN);

    // Mock collection list with 401
    await page.route('**/api/collections', async (route, request) => {
      if (request.method() === 'GET') {
        await route.fulfill({
          status: 401,
          contentType: 'application/json',
          body: JSON.stringify({
            error: {
              type: 'unauthorized',
              message: 'Invalid or missing token',
            },
          }),
        });
      } else {
        await route.fulfill({
          status: 401,
          contentType: 'application/json',
          body: JSON.stringify({
            error: {
              type: 'unauthorized',
              message: 'Invalid or missing token',
            },
          }),
        });
      }
    });

    // Mock documents to 401 too
    await page.route('**/api/documents*', async (route) => {
      await route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({
          error: { type: 'unauthorized', message: 'Invalid or missing token' },
        }),
      });
    });

    await page.goto('/admin');
    const adminView = page.locator('[data-testid="admin-view"]');
    await expect(adminView).toBeVisible({ timeout: 5000 });

    // App should still render — no crash from 401 errors
    // Check that the admin panel is still displayed
    const adminPanel = page.locator('.admin-panel');
    await expect(adminPanel).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Auth Regression: Token handling', () => {
  test('TC-AUTH-REG-010: missing token redirects to login', async ({ page }) => {
    // No token injected — restoreSession will find nothing
    await page.goto('/');
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-REG-011: expired token redirects to login', async ({ page }) => {
    const expiredToken = [
      btoa(JSON.stringify({ alg: 'RS256', typ: 'JWT' })),
      btoa(
        JSON.stringify({
          sub: 'user-123',
          name: 'Expired User',
          exp: Math.floor(Date.now() / 1000) - 3600, // expired 1 hour ago
          iat: Math.floor(Date.now() / 1000) - 7200,
        }),
      ),
      'mocksignature',
    ].join('.');

    await page.addInitScript((token) => {
      localStorage.setItem('vedo_auth_token', token);
    }, expiredToken);

    await page.goto('/');
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-REG-012: all API routes require auth header', async ({ page }) => {
    const endpoints = [
      { method: 'GET', path: '/api/collections' },
      { method: 'POST', path: '/api/collections' },
      { method: 'GET', path: '/api/documents' },
      { method: 'POST', path: '/api/documents/upload' },
      { method: 'GET', path: '/api/sessions' },
      { method: 'POST', path: '/api/sessions' },
      { method: 'GET', path: '/api/auth/me' },
    ];

    for (const ep of endpoints) {
      const response = await page.request.fetch(ep.path, {
        method: ep.method as 'GET' | 'POST',
        headers: { 'Content-Type': 'application/json' },
      });
      // Without auth token, all API endpoints should return 401
      expect(response.status(), `Expected 401 for ${ep.method} ${ep.path} without auth`).toBe(401);
    }
  });

  test('TC-AUTH-REG-013: all API routes return 401 with invalid token', async ({ page }) => {
    const endpoints = [
      { method: 'GET', path: '/api/collections' },
      { method: 'POST', path: '/api/collections' },
      { method: 'GET', path: '/api/documents' },
      { method: 'GET', path: '/api/sessions' },
      { method: 'POST', path: '/api/sessions' },
      { method: 'GET', path: '/api/auth/me' },
    ];

    for (const ep of endpoints) {
      const response = await page.request.fetch(ep.path, {
        method: ep.method as 'GET' | 'POST',
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
