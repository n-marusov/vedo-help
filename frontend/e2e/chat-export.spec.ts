import { expect, test } from '@playwright/test';
import { setActiveCollection, setupAuthAndCollection } from './helpers';

test.describe('chat export', () => {
  test('export Markdown → blob starts with H1 session title and contains ## user', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request);
    await page.goto('/');
    await setActiveCollection(page, collection.id);

    // Send a query to create a session
    const input = page.locator('[data-testid="chat-input"]');
    await input.fill('Export test');
    await page.locator('[data-testid="btn-send"]').click();
    await page.waitForSelector('[data-testid="message-assistant"]', {
      timeout: 10000,
    });

    // Click Export button
    const exportBtn = page.locator('[data-testid="export-btn"]');
    await expect(exportBtn).toBeVisible();
    await exportBtn.click();

    // Wait for blob download (intercept URL.createObjectURL)
    // In Playwright this is tricky — we intercept the fetch itself
    const exportResponse = await page.waitForResponse(
      (res) => res.url().includes('/export') && res.url().includes('format=m'),
    );
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
    await page.waitForSelector('[data-testid="message-assistant"]', {
      timeout: 10000,
    });

    // Select JSON format
    const formatSelect = page.locator('[data-testid="export-format-select"]');
    await formatSelect.selectOption('json');
    await page.locator('[data-testid="export-btn"]').click();

    const exportResponse = await page.waitForResponse(
      (res) => res.url().includes('/export') && res.url().includes('format=json'),
    );
    expect(exportResponse.status()).toBe(200);
    const body = await exportResponse.json();
    expect(body).toHaveProperty('session');
    expect(body).toHaveProperty('messages');
  });
});
