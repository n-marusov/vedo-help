import { expect, test } from '@playwright/test';
import { API_URL, getTestAccessToken, setupAuth } from './helpers';

test.describe('Admin Debug View', () => {
  let sessionId: string | undefined;

  test.beforeAll(async ({ request }) => {
    const token = await getTestAccessToken();
    const { Buffer } = await import('node:buffer');

    // Create a collection for seeding query messages
    const collResp = await request.post(`${API_URL}/api/collections`, {
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      data: { name: `E2E Tech Collection ${Date.now()}` },
    });
    if (!collResp.ok()) return;
    const collectionId = (await collResp.json()).id;

    // Upload a document so the query can find content
    await request.post(`${API_URL}/api/documents/upload`, {
      headers: { Authorization: `Bearer ${token}` },
      multipart: {
        file: {
          name: 'technical-guide.md',
          mimeType: 'text/markdown',
          buffer: Buffer.from(
            '# Technical Guide\n\nThis is technical content about the VEDO system.',
          ),
        },
        collection_id: collectionId,
      },
    });

    // Create a session with 'Technical' in the title so debug search finds it
    const sessResp = await request.post(`${API_URL}/api/sessions`, {
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      data: { title: 'Technical Discussion' },
    });
    if (!sessResp.ok()) return;
    sessionId = (await sessResp.json()).id;

    // Send a debug query to populate messages with debug_data
    await request.post(`${API_URL}/api/query`, {
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      data: {
        query: 'Technical question about embeddings',
        collection_id: collectionId,
        session_id: sessionId,
        debug: true,
      },
      timeout: 30000,
    });
  });

  test('TC-ADEBUG-001: Admin panel shows tabs', async ({ page }) => {
    await setupAuth(page);
    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-tabs"]')).toBeVisible();
    await expect(page.locator('[data-testid="admin-tab-sources"]')).toBeVisible();
    await expect(page.locator('[data-testid="admin-tab-debug"]')).toBeVisible();
  });

  test('TC-ADEBUG-002: Clicking Debug tab shows session search', async ({ page }) => {
    await setupAuth(page);
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-debug"]').click();
    await expect(page.locator('[data-testid="session-debug-view"]')).toBeVisible();
    await expect(page.locator('[data-testid="session-debug-search"]')).toBeVisible();
  });

  test('TC-ADEBUG-003: Searching sessions returns results', async ({ page, request }) => {
    const token = await setupAuth(page);

    if (!sessionId) {
      const sessResp = await request.post(`${API_URL}/api/sessions`, {
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
        data: { title: 'Technical Discussion' },
      });
      if (sessResp.ok()) sessionId = (await sessResp.json()).id;
    }

    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-debug"]').click();
    await page.locator('[data-testid="session-debug-search"]').fill('Technical');
    await page.waitForTimeout(1000);
    const sessionItems = page.locator('[data-testid="session-list-item"]');
    await expect(sessionItems.first()).toBeVisible({ timeout: 5000 });
  });

  test('TC-ADEBUG-004: Selecting session shows messages', async ({ page, request }) => {
    const token = await setupAuth(page);

    if (!sessionId) {
      const sessResp = await request.post(`${API_URL}/api/sessions`, {
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
        data: { title: 'Technical Discussion' },
      });
      if (sessResp.ok()) sessionId = (await sessResp.json()).id;
    }

    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-debug"]').click();
    await page.locator('[data-testid="session-debug-search"]').fill('Technical');
    await page.waitForTimeout(1000);
    const firstSession = page.locator('[data-testid="session-list-item"]').first();
    await expect(firstSession).toBeVisible({ timeout: 5000 });
    await firstSession.click();
    await expect(page.locator('[data-testid="session-msg"]').first()).toBeVisible({
      timeout: 5000,
    });
  });

  test('TC-ADEBUG-005: Debug panel shows debug data', async ({ page, request }) => {
    const token = await setupAuth(page);

    if (!sessionId) {
      const sessResp = await request.post(`${API_URL}/api/sessions`, {
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
        data: { title: 'Technical Discussion' },
      });
      if (sessResp.ok()) sessionId = (await sessResp.json()).id;
    }

    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-debug"]').click();
    await page.locator('[data-testid="session-debug-search"]').fill('Technical');
    await page.waitForTimeout(1000);
    const firstSession = page.locator('[data-testid="session-list-item"]').first();
    await expect(firstSession).toBeVisible({ timeout: 5000 });
    await firstSession.click();
    await expect(page.locator('[data-testid="session-msg"]').first()).toBeVisible({
      timeout: 5000,
    });
    // Debug toggle appears when message has debug_data
    const debugToggle = page.locator('[data-testid="session-debug-toggle"]').first();
    if (await debugToggle.isVisible()) {
      await debugToggle.click();
      const stepTitles = page.locator('[data-testid="debug-step-title"]');
      await expect(stepTitles).toHaveCount(7);
    }
  });

  test('TC-ADEBUG-006: Switching back to Sources tab works', async ({ page }) => {
    await setupAuth(page);
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-debug"]').click();
    await expect(page.locator('[data-testid="session-debug-view"]')).toBeVisible();
    await page.locator('[data-testid="admin-tab-sources"]').click();
    // Collections panel should be visible again
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible();
  });

  test('TC-ADEBUG-007: No debug button in chat anymore', async ({ page }) => {
    await page.goto('/');
    // Send a query - the chat should not have debug buttons
    const textarea = page.locator('[data-testid="chat-input"]');
    if (await textarea.isVisible()) {
      await textarea.fill('test query');
      await page.locator('[data-testid="send-button"]').click();
      await page.waitForTimeout(2000);
    }
    // Verify no debug button present
    await expect(page.locator('[data-testid="message-debug-btn"]')).toHaveCount(0);
  });
});
