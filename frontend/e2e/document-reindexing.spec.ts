import { expect, test } from '@playwright/test';
import { apiRequest, fileInput, setActiveCollection, setupAuthAndCollection } from './helpers';

test.describe('Document lifecycle with real backend', () => {
  test('TC-REINDEX-001: upload document then query returns backend response', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request, `Docs Query ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await setActiveCollection(page, collection.id);
    await expect(page.locator('.document-list')).toBeVisible({
      timeout: 10000,
    });
    await fileInput(page).setInputFiles({
      name: 'config-guide.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from(
        '# Configuration Guide\n\nRate limiting is configured by backend middleware.',
      ),
    });
    await expect(page.locator('.dl-item__name').first()).toContainText('config-guide.md', {
      timeout: 30000,
    });

    await page.goto('/');
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible({
      timeout: 10000,
    });
    await setActiveCollection(page, collection.id);
    await page.locator('[data-testid="chat-input"]').fill('What is configured?');
    await page.locator('[data-testid="btn-send"]').click();

    await expect(page.locator('[data-testid="message-assistant"]').first()).toBeVisible({
      timeout: 30000,
    });
  });

  test('TC-REINDEX-003: deleted document disappears from real backend document list', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request, `Docs Delete ${Date.now()}`);

    await page.goto('/admin');
    await setActiveCollection(page, collection.id);
    await expect(page.locator('.document-list')).toBeVisible({
      timeout: 10000,
    });
    await fileInput(page).setInputFiles({
      name: 'delete-me.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from('# Delete Me\n\nContent to be deleted.'),
    });
    await expect(page.locator('.dl-item__name').first()).toContainText('delete-me.md', {
      timeout: 30000,
    });

    const documents = await apiRequest<Array<{ id: string; name: string }>>(
      request,
      'GET',
      `/api/documents?collection_id=${collection.id}`,
    );
    const document = documents.find((item) => item.name === 'delete-me.md');
    expect(document?.id).toBeTruthy();

    await apiRequest(request, 'DELETE', `/api/documents/${document?.id}`);
    const remaining = await apiRequest<Array<{ id: string; name: string }>>(
      request,
      'GET',
      `/api/documents?collection_id=${collection.id}`,
    );
    expect(remaining.some((item) => item.id === document?.id)).toBe(false);
  });
});
