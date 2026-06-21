import { expect, test } from '@playwright/test';
import { fileInput, setActiveCollection, setupAuthAndCollection } from './helpers';

test.describe('RAG Flow: real backend upload → query → sources', () => {
  test('TC-RAG-001: admin page renders with real backend data', async ({ page, request }) => {
    await setupAuthAndCollection(page, request, `RAG Admin ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await expect(page.locator('[data-testid="auth-section"]')).not.toBeVisible();
    await expect(page.locator('.admin-panel')).toBeVisible();
  });

  test('TC-RAG-002: upload markdown document through UI to backend', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `RAG Upload ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await setActiveCollection(page, collection.id);
    await page.waitForSelector('.drop-zone', { timeout: 10000 });

    await fileInput(page).setInputFiles({
      name: 'test-doc.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from(
        '# Rate limiting\n\nRate limiting is configured in the backend test environment.',
      ),
    });

    await expect(page.locator('.dl-item__name').first()).toContainText('test-doc.md', {
      timeout: 30000,
    });
  });

  test('TC-RAG-003: query real backend and render streaming response', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request, `RAG Query ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await setActiveCollection(page, collection.id);
    await page.waitForSelector('.drop-zone', { timeout: 10000 });
    await fileInput(page).setInputFiles({
      name: 'config-guide.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from(
        '# Configuration Guide\n\nRate limiting uses backend middleware and environment settings.',
      ),
    });
    // Wait for document to appear in the list, then allow Chroma propagation
    await expect(page.locator('.dl-item__name').first()).toContainText('config-guide.md', {
      timeout: 30000,
    });
    // Allow Chroma propagation delay so chunks are searchable
    await page.waitForTimeout(2000);

    await page.goto('/');
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible({
      timeout: 10000,
    });
    await setActiveCollection(page, collection.id);

    const input = page.locator('[data-testid="chat-input"]');
    await input.fill('How is rate limiting configured?');
    console.debug('[rag-flow] waiting for query response');
    await page.locator('[data-testid="btn-send"]').click();

    await expect(page.locator('[data-testid="message-user"]').first()).toBeVisible({
      timeout: 15000,
    });
    const assistant = page.locator('[data-testid="message-assistant"]').first();
    await expect(assistant).toBeVisible({ timeout: 30000 });
    await expect(page.locator('[data-testid="message-content"]').last()).toContainText(
      /backend answer|Sources/i,
      {
        timeout: 30000,
      },
    );
  });
});
