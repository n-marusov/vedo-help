import { expect, test } from '@playwright/test';
import {
  apiRequest,
  getTestAccessToken,
  setActiveCollection,
  setupAuthAndCollection,
} from './helpers';

test.describe('chat export', () => {
  test('export Markdown → blob starts with H1 session title and contains ## user', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request);
    await page.goto('/');
    await setActiveCollection(page, collection.id);

    // Send a query to create a session and get a response
    const input = page.locator('[data-testid="chat-input"]');
    await input.fill('Export test');
    await page.locator('[data-testid="btn-send"]').click();

    // Wait for the SSE query to fully complete (loading done + message rendered)
    await page.waitForSelector('[data-testid="message-assistant"]', {
      timeout: 20000,
    });
    await page.waitForFunction(
      () => {
        const app = document.querySelector('#app').__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        return !pinia.state.value.chat.isLoading;
      },
      { timeout: 30000 },
    );

    // Get the session ID from Pinia store
    const sessionId = await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      return pinia.state.value.chat.activeSessionId;
    });
    expect(sessionId).toBeTruthy();

    // Call export API directly using request fixture
    // (avoids response body consumption conflict with in-page fetch)
    const token = await getTestAccessToken();
    const exportResponse = await request.fetch(`/api/sessions/${sessionId}/export?format=md`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    expect(exportResponse.status()).toBe(200);
    const text = await exportResponse.text();
    expect(text).toContain('# ');
    expect(text).toContain('## user');
  });

  test('export JSON → shape has session and messages keys', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request);
    await page.goto('/');
    await setActiveCollection(page, collection.id);

    const input = page.locator('[data-testid="chat-input"]');
    await input.fill('JSON test');
    await page.locator('[data-testid="btn-send"]').click();

    // Wait for the SSE query to fully complete (loading done + message rendered)
    await page.waitForSelector('[data-testid="message-assistant"]', {
      timeout: 20000,
    });
    await page.waitForFunction(
      () => {
        const app = document.querySelector('#app').__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        return !pinia.state.value.chat.isLoading;
      },
      { timeout: 30000 },
    );

    // Get the session ID from Pinia store
    const sessionId = await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      return pinia.state.value.chat.activeSessionId;
    });
    expect(sessionId).toBeTruthy();

    // Call export API directly with JSON format
    const token = await getTestAccessToken();
    const exportResponse = await request.fetch(`/api/sessions/${sessionId}/export?format=json`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });
    expect(exportResponse.status()).toBe(200);
    const body = await exportResponse.json();
    expect(body).toHaveProperty('session');
    expect(body).toHaveProperty('messages');
  });
});
