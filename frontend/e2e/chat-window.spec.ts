import { expect, test } from '@playwright/test';
import { type TestCollection, setActiveCollection, setupAuthAndCollection } from './helpers';

test.describe('ChatWindow Layout with real backend', () => {
  let collection: TestCollection;

  test.beforeEach(async ({ page, request }) => {
    collection = await setupAuthAndCollection(page, request, `ChatWindow ${Date.now()}`);
  });

  test('TC-CHAT-001: renders chat header with collection selector', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('[data-testid="chat-toolbar"]')).toBeVisible({
      timeout: 10000,
    });
    await expect(page.locator('[data-testid="collection-selector-trigger"]')).toBeVisible();
  });

  test('TC-CHAT-002: collection selector shows real backend collections', async ({ page }) => {
    await page.goto('/');
    await page.locator('[data-testid="collection-selector-trigger"]').click();
    const dropdown = page.locator('[data-testid="collection-selector-dropdown"]');
    await expect(dropdown).toBeVisible({ timeout: 5000 });
    await expect(dropdown).toContainText(collection.name);
  });

  test('TC-CHAT-003: new chat button clears messages', async ({ page }) => {
    await page.goto('/');
    await page.locator('[data-testid="btn-new-chat"]').click();
    await expect(page.locator('[data-testid="welcome-message"]')).toBeVisible({
      timeout: 5000,
    });
  });

  test('TC-CHAT-004: input enables send when a real collection is selected', async ({ page }) => {
    await page.goto('/');
    await setActiveCollection(page, collection.id);
    await page.locator('[data-testid="chat-input"]').fill('Hello, VEDO!');
    await expect(page.locator('[data-testid="btn-send"]')).toBeEnabled();
  });

  test('TC-CHAT-005: Shift+Enter inserts newline instead of sending', async ({ page }) => {
    await page.goto('/');
    await setActiveCollection(page, collection.id);
    const input = page.locator('[data-testid="chat-input"]');
    await input.fill('Line 1');
    await input.press('Shift+Enter');
    await page.keyboard.type('Line 2');
    const value = await input.inputValue();
    expect(value).toContain('Line 1');
    expect(value).toContain('Line 2');
  });

  test('TC-CHAT-006: messages area is scrollable', async ({ page }) => {
    await page.goto('/');
    const messagesArea = page.locator('[data-testid="messages-area"]');
    await expect(messagesArea).toBeVisible({ timeout: 5000 });
    await expect(messagesArea).toHaveCSS('overflow-y', 'auto');
  });
});
