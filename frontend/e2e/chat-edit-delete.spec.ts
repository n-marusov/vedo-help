import { expect, test } from '@playwright/test';
import { fileInput, setActiveCollection, setupAuthAndCollection } from './helpers';

test.describe('chat message edit/delete', () => {
  test.describe.configure({ mode: 'serial' });

  // Requires full RAG pipeline (upload → index → query → response) to have
  // messages to edit/delete. The `data-testid` attributes and API endpoints
  // (PATCH/DELETE /api/sessions/:sid/messages/:mid) are verified via backend
  // integration tests and frontend unit tests.
  test.skip('hover user message → edit button visible → edit textarea → save → content updates', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request, `Edit Delete ${Date.now()}`);

    // Upload a document to the collection first
    await page.goto('/admin');
    await setActiveCollection(page, collection.id);
    await page.waitForSelector('.drop-zone', { timeout: 10000 });
    await fileInput(page).setInputFiles({
      name: 'test-doc.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from('# Test Doc\n\nThis is content for testing.'),
    });
    await expect(page.locator('.dl-item__name').first()).toContainText('test-doc.md', {
      timeout: 30000,
    });

    // Navigate to chat and send a query
    await page.goto('/');
    await setActiveCollection(page, collection.id);

    const input = page.locator('[data-testid="chat-input"]');
    await input.fill('Hello world');
    await page.locator('[data-testid="btn-send"]').click();

    // Wait for response
    await page.waitForSelector('[data-testid="message-user"]', {
      timeout: 15000,
    });
    await page.waitForSelector('[data-testid="message-assistant"]', {
      timeout: 30000,
    });

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
    const collection = await setupAuthAndCollection(page, request, `Delete ${Date.now()}`);

    // Upload a document
    await page.goto('/admin');
    await setActiveCollection(page, collection.id);
    await page.waitForSelector('.drop-zone', { timeout: 10000 });
    await fileInput(page).setInputFiles({
      name: 'test-doc.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from('# Test Doc\n\nContent for delete test.'),
    });
    await expect(page.locator('.dl-item__name').first()).toContainText('test-doc.md', {
      timeout: 30000,
    });

    // Go to chat and send a query
    await page.goto('/');
    await setActiveCollection(page, collection.id);

    const input = page.locator('[data-testid="chat-input"]');
    await input.fill('Delete me');
    await page.locator('[data-testid="btn-send"]').click();
    await page.waitForSelector('[data-testid="message-assistant"]', {
      timeout: 30000,
    });

    // Find and delete assistant message
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
