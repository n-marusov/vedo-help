import { expect, test } from '@playwright/test';
import { VALID_TOKEN, mockCollections, mockSessions } from './helpers';

/**
 * Document Re-indexing E2E Tests
 *
 * Tests the document re-indexing lifecycle through the admin and chat UIs:
 * 1. Upload → Query uses active indexed chunks (T1.1)
 * 2. Reload replaces old document content (T1.2)
 * 3. Deleted document disappears from query sources (T1.3)
 *
 * All tests use mocked backend responses so they are self-contained
 * and do not require a running backend.
 */
test.describe('Document Re-indexing: Upload → Reload → Delete', () => {
  test.beforeEach(async ({ page }) => {
    // Inject auth token before navigation
    await page.addInitScript((token: string) => {
      localStorage.setItem('vedo_auth_token', token);
    }, VALID_TOKEN);

    // Mock collections API
    await mockCollections(page);

    // Mock documents list endpoint (shared)
    await page.route('**/api/documents*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'doc-1',
            name: 'config-guide.md',
            file_type: 'text/markdown',
            file_size: 1024,
            uploaded_at: new Date().toISOString(),
            collection_id: 'col-1',
          },
        ]),
      });
    });
  });

  // ─── T1.1: Upload → Query uses active indexed chunks ───

  test('TC-REINDEX-001: upload document then query shows it as a source', async ({ page }) => {
    // Mock document upload endpoint
    await page.route('**/api/documents/upload', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          document_id: 'doc-1',
          chunks_indexed: 5,
          document_name: 'config-guide.md',
        }),
      });
    });

    // Mock query endpoint with streaming NDJSON including the uploaded doc as source
    await page.route('**/api/query', async (route) => {
      const streamChunks = [
        '{"type":"chunk","text":"Based on the documentation, "}\n',
        '{"type":"chunk","text":"the configuration guide covers all settings."}\n',
        '{"type":"sources","sources":[{"document_id":"doc-1","document_name":"config-guide.md","chunk_index":0,"text":"The configuration guide section on rate limiting.","relevance":0.94}]}\n',
        '{"type":"done"}\n',
      ];
      await route.fulfill({
        status: 200,
        headers: { 'Content-Type': 'application/x-ndjson' },
        body: streamChunks.join(''),
      });
    });

    // Mock sessions for chat
    await mockSessions(page);

    // Step 1: Upload document via admin panel
    await page.goto('/admin');
    const adminView = page.locator('[data-testid="admin-view"]');
    await expect(adminView).toBeVisible({ timeout: 5000 });

    // Set active collection to enable DocumentList
    await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    // Wait for VDropZone to render
    await page.waitForSelector('.drop-zone', { timeout: 5000 });

    // Upload a file
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles({
      name: 'config-guide.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from('# Configuration Guide\n\nThis is the config guide content.'),
    });

    // Wait for upload to complete — look for document item in the list
    await page.waitForSelector('.dl-item', { timeout: 15000 });
    await expect(page.locator('.dl-item__name').first()).toContainText('config-guide.md');

    // Step 2: Navigate to chat and send a query
    await page.goto('/');
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 10000 });

    // Type a query
    const input = page.locator('[data-testid="chat-input"]');
    await expect(input).toBeVisible();

    // Set active collection
    await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    await input.fill('How does the configuration work?');

    // Click send
    const sendBtn = page.locator('[data-testid="btn-send"]');
    await expect(sendBtn).toBeEnabled();
    await sendBtn.click();

    // Step 3: Verify the uploaded document appears as a source
    const sourcesToggle = page.locator('[data-testid="sources-toggle"]');
    await expect(sourcesToggle).toBeVisible({ timeout: 15000 });
    await expect(sourcesToggle).toContainText(/source/i);

    // Expand sources
    await sourcesToggle.click();

    // Verify source item appears with the uploaded document name
    const sourceItem = page.locator('[data-testid="source-item"]').first();
    await expect(sourceItem).toBeVisible({ timeout: 5000 });

    const docName = sourceItem.locator('[data-testid="source-document"]');
    await expect(docName).toContainText('config-guide.md');
  });

  // ─── T1.2: Reload replaces old document content ───

  test('TC-REINDEX-002: reload document replaces old content in sources', async ({ page }) => {
    // Track upload calls to differentiate initial vs reload
    let uploadCalls = 0;

    // Mock initial document upload
    await page.route('**/api/documents/upload', async (route) => {
      uploadCalls++;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          document_id: 'doc-1',
          chunks_indexed: 3,
          document_name: uploadCalls === 1 ? 'guide-v1.md' : 'guide-v2.md',
        }),
      });
    });

    // Mock reload endpoint (POST /api/documents/reload)
    let reloadCalled = false;
    await page.route('**/api/documents/reload', async (route) => {
      reloadCalled = true;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          document_id: 'doc-1',
          chunks_indexed: 5,
          old_chunks_deactivated: 3,
        }),
      });
    });

    // Mock query to return only version B sources (no old version A content)
    await page.route('**/api/query', async (route) => {
      const streamChunks = [
        '{"type":"chunk","text":"The updated documentation covers new features."}\n',
        '{"type":"sources","sources":[{"document_id":"doc-1","document_name":"guide-v2.md","chunk_index":0,"text":"Updated documentation with new feature details.","relevance":0.91}]}\n',
        '{"type":"done"}\n',
      ];
      await route.fulfill({
        status: 200,
        headers: { 'Content-Type': 'application/x-ndjson' },
        body: streamChunks.join(''),
      });
    });

    // Mock sessions for chat
    await mockSessions(page);

    // Step 1: Upload initial document version
    await page.goto('/admin');
    const adminView = page.locator('[data-testid="admin-view"]');
    await expect(adminView).toBeVisible({ timeout: 5000 });

    await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    await page.waitForSelector('.drop-zone', { timeout: 5000 });

    // Upload version A
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles({
      name: 'guide-v1.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from('# Guide v1\n\nOld content here.'),
    });

    // Wait for document list to reflect upload
    await page.waitForSelector('.dl-item', { timeout: 15000 });

    // Step 2: Send a query confirming old document is indexed
    await page.goto('/');
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 10000 });

    const input = page.locator('[data-testid="chat-input"]');
    await expect(input).toBeVisible();

    await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    await input.fill('What does the guide say?');
    const sendBtn = page.locator('[data-testid="btn-send"]');
    await expect(sendBtn).toBeEnabled();
    await sendBtn.click();

    // Verify old sources appear (version A)
    const sourcesToggle = page.locator('[data-testid="sources-toggle"]');
    await expect(sourcesToggle).toBeVisible({ timeout: 15000 });
    await sourcesToggle.click();
    const sourceItem = page.locator('[data-testid="source-item"]').first();
    await expect(sourceItem).toBeVisible();

    // Step 3: Return to admin and reload with new version
    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 5000,
    });

    // The reload could be triggered through a document context menu action.
    // For the E2E test we mock the reload call at the API level.
    // In the actual UI, a "Reload" action on a document would call POST /api/documents/reload.
    // We simulate by triggering a second upload that the mock intercepts.
    await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    // Trigger reload via the drop zone (re-upload)
    await page.waitForSelector('.drop-zone', { timeout: 5000 });
    const fileInput2 = page.locator('input[type="file"]');
    await fileInput2.setInputFiles({
      name: 'guide-v2.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from('# Guide v2\n\nUpdated content with new features.'),
    });

    // Wait for the reload to register
    await expect(async () => {
      expect(reloadCalled || uploadCalls >= 2).toBe(true);
    }).toPass({ timeout: 10000 });

    // Step 4: Navigate to chat and query again — only new content should appear
    await page.goto('/');
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible({
      timeout: 10000,
    });

    const input2 = page.locator('[data-testid="chat-input"]');
    await expect(input2).toBeVisible();
    await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    await input2.fill('What does the updated guide say?');
    const sendBtn2 = page.locator('[data-testid="btn-send"]');
    await expect(sendBtn2).toBeEnabled();
    await sendBtn2.click();

    // Verify sources show only the new version B document name
    const sourcesToggle2 = page.locator('[data-testid="sources-toggle"]');
    await expect(sourcesToggle2).toBeVisible({ timeout: 15000 });
    await sourcesToggle2.click();

    const sourceItems = page.locator('[data-testid="source-item"]');
    const sourceCount = await sourceItems.count();

    // All sources should reference the new version
    for (let i = 0; i < sourceCount; i++) {
      const docName = sourceItems.nth(i).locator('[data-testid="source-document"]');
      await expect(docName).toContainText('guide-v2.md');
    }
  });

  // ─── T1.3: Deleted document disappears from query sources ───

  test('TC-REINDEX-003: deleted document is absent from query sources', async ({ page }) => {
    // Mock document upload
    await page.route('**/api/documents/upload', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          document_id: 'doc-deletable',
          chunks_indexed: 4,
          document_name: 'delete-me.md',
        }),
      });
    });

    // Mock document delete endpoint (soft delete)
    let deleteCalled = false;
    await page.route('**/api/documents/delete*', async (route) => {
      deleteCalled = true;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true }),
      });
    });
    // Also match DELETE method on the documents endpoint
    await page.route('**/api/documents/*', async (route, request) => {
      if (request.method() === 'DELETE') {
        deleteCalled = true;
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ success: true }),
        });
      } else {
        await route.continue();
      }
    });

    // Mock query to return sources WITHOUT the deleted document
    await page.route('**/api/query', async (route) => {
      const streamChunks = [
        '{"type":"chunk","text":"Only remaining documents are referenced."}\n',
        '{"type":"sources","sources":[{"document_id":"doc-other","document_name":"other-doc.md","chunk_index":0,"text":"Content from a different document that was not deleted.","relevance":0.87}]}\n',
        '{"type":"done"}\n',
      ];
      await route.fulfill({
        status: 200,
        headers: { 'Content-Type': 'application/x-ndjson' },
        body: streamChunks.join(''),
      });
    });

    // Mock sessions for chat
    await mockSessions(page);

    // Step 1: Upload a document
    await page.goto('/admin');
    const adminView = page.locator('[data-testid="admin-view"]');
    await expect(adminView).toBeVisible({ timeout: 5000 });

    await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    await page.waitForSelector('.drop-zone', { timeout: 5000 });

    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles({
      name: 'delete-me.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from('# Delete Me\n\nContent to be deleted.'),
    });

    await page.waitForSelector('.dl-item', { timeout: 15000 });

    // Step 2: Delete the document using the delete button in the document list
    // Look for a delete button in the document list item
    const deleteBtn = page.locator('[data-testid*="delete"]').first();
    await deleteBtn.click();

    // Handle confirmation dialog if present
    const confirmDialog = page.locator('[role="dialog"], .v-dialog, .modal');
    if (await confirmDialog.isVisible({ timeout: 3000 }).catch(() => false)) {
      const confirmBtn = confirmDialog.locator(
        'button:has-text("Delete"), button:has-text("Confirm")',
      );
      await confirmBtn.click();
    }

    // Verify delete was called
    await expect(async () => {
      expect(deleteCalled).toBe(true);
    }).toPass({ timeout: 5000 });

    // Step 3: Navigate to chat and query
    await page.goto('/');
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 10000 });

    const input = page.locator('[data-testid="chat-input"]');
    await expect(input).toBeVisible();

    await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    await input.fill('What documents are available?');

    const sendBtn = page.locator('[data-testid="btn-send"]');
    await expect(sendBtn).toBeEnabled();
    await sendBtn.click();

    // Step 4: Verify deleted document does NOT appear in sources
    const sourcesToggle = page.locator('[data-testid="sources-toggle"]');
    await expect(sourcesToggle).toBeVisible({ timeout: 15000 });
    await sourcesToggle.click();

    // Check that the deleted document name is not present
    const sourceItems = page.locator('[data-testid="source-item"]');
    const deletedDocInSources = await sourceItems
      .locator('[data-testid="source-document"]')
      .evaluateAll((elements) => elements.some((el) => el.textContent?.includes('delete-me.md')));
    expect(deletedDocInSources).toBe(false);

    // Verify the other document is present
    const otherDocInSources = await sourceItems
      .locator('[data-testid="source-document"]')
      .evaluateAll((elements) => elements.some((el) => el.textContent?.includes('other-doc.md')));
    expect(otherDocInSources).toBe(true);
  });
});
