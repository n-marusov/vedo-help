import { expect, test } from '@playwright/test';
import { setupAuthAndCollection } from './helpers';

test.describe('chat message edit/delete (RED)', () => {
  test.describe.configure({ mode: 'serial' });

  test.skip('hover user message → edit button visible → edit textarea → save → content updates', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request);
    await page.goto('/');

    // Select collection and send a query to create a session with messages
    await page.waitForSelector('[data-testid="collection-select"]');
    await page.selectOption('[data-testid="collection-select"]', collection.id);

    const input = page.locator('[data-testid="chat-input"]');
    await input.fill('Hello world');
    await page.locator('[data-testid="send-btn"]').click();

    // Wait for response (mock LLM responds)
    await page.waitForSelector('[data-testid="message-user"]', { timeout: 10000 });

    // Hover the user message → edit button visible
    const userMsg = page.locator('[data-testid="message-user"]').first();
    await userMsg.hover();
    const editBtn = userMsg.locator('[data-testid="message-edit-btn"]');
    await expect(editBtn).toBeVisible();

    // Click edit → textarea appears
    await editBtn.click();
    const textarea = page.locator('[data-testid="message-edit-textarea"]');
    await expect(textarea).toBeVisible();

    // Edit and save
    await textarea.fill('Updated content');
    await page.locator('[data-testid="message-save-btn"]').click();

    // Verify PATCH request was made
    const patchResponse = await page.waitForResponse(
      (res) => res.url().includes('/messages/') && res.request().method() === 'PATCH',
    );
    expect(patchResponse.status()).toBe(200);

    // Verify content updated in the UI
    await expect(page.locator('[data-testid="message-content"]').first()).toContainText(
      'Updated content',
    );
  });

  test.skip('hover assistant message → delete button → confirm → message removed', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request);
    await page.goto('/');
    await page.selectOption('[data-testid="collection-select"]', collection.id);

    const input = page.locator('[data-testid="chat-input"]');
    await input.fill('Delete me');
    await page.locator('[data-testid="send-btn"]').click();
    await page.waitForSelector('[data-testid="message-assistant"]', { timeout: 10000 });

    const asstMsg = page.locator('[data-testid="message-assistant"]').first();
    await asstMsg.hover();
    const deleteBtn = asstMsg.locator('[data-testid="message-delete-btn"]');
    await expect(deleteBtn).toBeVisible();

    await deleteBtn.click();

    // Verify DELETE request was made
    const deleteResponse = await page.waitForResponse(
      (res) => res.url().includes('/messages/') && res.request().method() === 'DELETE',
    );
    expect(deleteResponse.status()).toBe(204);

    // Verify assistant message no longer visible
    await expect(asstMsg).not.toBeVisible();
  });
});
