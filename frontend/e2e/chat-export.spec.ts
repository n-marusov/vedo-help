import { expect, test } from '@playwright/test';
import { apiRequest, setActiveCollection, setupAuthAndCollection } from './helpers';

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
    await page.waitForSelector('[data-testid="message-assistant"]', {
      timeout: 20000,
    });

    // Click Export button — should be visible now because handleSend creates a session
    const exportBtn = page.locator('[data-testid="export-btn"]');
    await expect(exportBtn).toBeVisible({ timeout: 10000 });
    await exportBtn.click();

    // Wait for export response (fetch request, not XHR)
    const exportResponse = await page.waitForResponse(
      (res) => res.url().includes('/export') && res.url().includes('format=md'),
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
      timeout: 20000,
    });

    // Select JSON format using VSelect (custom button-based dropdown):
    // click the select trigger to open dropdown, then click the JSON option
    const formatSelect = page.locator('[data-testid="export-format-select"]');
    await formatSelect.click();
    // The dropdown contains buttons with class v-select__option
    // Pick the JSON option (second one: "Markdown", "JSON")
    await page.locator('.v-select__option').nth(1).click();

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
