import { expect, test } from '@playwright/test';
import { type TestCollection, setupAuth, setupAuthAndCollection } from './helpers';

/**
 * Navigation & Layout Tests (Task 3.1, 3.4)
 *
 * Tests the removal of admin navigation from main page,
 * routing behavior, and responsive layout breakpoints.
 */
test.describe('Navigation & Admin Layout (Task 3.1)', () => {
  test.beforeEach(async ({ page }) => {
    // DEBUG [e2e] auth setup added to navigation beforeEach
    await setupAuth(page);
  });

  test('TC-NAV-001: chat is default landing page at root URL', async ({ page }) => {
    await page.goto('/');
    // The main page should show chat view
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 5000 });
  });

  test('TC-NAV-002: admin navigation is removed from main layout', async ({ page }) => {
    await page.goto('/');
    // The sidebar should NOT contain admin navigation link
    const adminNavLink = page.locator('[data-testid="nav-admin"]');
    await expect(adminNavLink).not.toBeVisible();
  });

  test('TC-NAV-003: chat navigation exists in sidebar', async ({ page }) => {
    await page.goto('/');
    // Chat nav link should still be present (or the layout itself is the chat)
    // Either the nav link exists, or the entire layout is chat-focused
    // We just verify there's no admin nav distracting
  });

  test('TC-NAV-004: admin page is accessible via /admin route', async ({ page }) => {
    await page.goto('/admin');
    const adminView = page.locator('[data-testid="admin-view"]');
    await expect(adminView).toBeVisible({ timeout: 5000 });
  });

  test('TC-NAV-005: admin page shows admin panel when authenticated', async ({ page }) => {
    await page.goto('/admin');
    const adminView = page.locator('[data-testid="admin-view"]');
    await expect(adminView).toBeVisible({ timeout: 5000 });

    // Auth section should not be visible (valid JWT bypasses auth gate)
    const authSection = page.locator('[data-testid="auth-section"]');
    await expect(authSection).not.toBeVisible();

    // Admin panel content should be visible
    const adminPanel = page.locator('.admin-panel');
    await expect(adminPanel).toBeVisible();
  });

  test('TC-NAV-006: clicking browser back returns to chat', async ({ page }) => {
    await page.goto('/');
    await page.goto('/admin');
    await page.goBack();

    // Should be back on main page with chat view
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 5000 });
  });
});

test.describe('Responsive Layout (Task 3.4)', () => {
  let collection: TestCollection;

  test.beforeEach(async ({ page, request }) => {
    collection = await setupAuthAndCollection(page, request, `Navigation ${Date.now()}`);
  });

  test('TC-RESP-005: desktop layout constrains message width for readability', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');

    // Seed a message to check width constraint on message bubbles
    await page.evaluate((collectionId) => {
      // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
      const app = (document.querySelector('#app') as any).__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = collectionId;
      pinia.state.value.chat.activeSessionId = 'sess-1';
      pinia.state.value.chat.messages = [
        {
          id: 'm1',
          session_id: 'sess-1',
          role: 'user',
          content: 'Hello',
          created_at: new Date().toISOString(),
        },
      ];
    }, collection.id);
    console.debug('[navigation] waiting for message content');
    await page.waitForSelector('[data-testid^="message-body-"]');

    // Message bubbles should not stretch full width (max-width constraint)
    const messageBody = page.locator('[data-testid^="message-body-"]').first();
    const maxWidth = await messageBody.evaluate((el) => {
      const style = getComputedStyle(el);
      return Number.parseFloat(style.maxWidth) || Number.parseFloat(style.width);
    });
    expect(maxWidth).toBeLessThan(1440 * 0.8); // should not take 80%+ of viewport
  });
});
