import { expect, test } from '@playwright/test';
import { fileInput, setActiveCollection, setupAuthAndCollection } from './helpers';

test.describe('Admin Statistics Tab', () => {
  test('TC-STATS-001: navigate to Statistics tab and see stats panel', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request, `Stats Admin ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });

    // Click Statistics tab
    await page.locator('[data-testid="admin-tab-stats"]').click();

    // Verify stats panel and chunk browser are visible
    await expect(page.locator('[data-testid="stats-panel"]')).toBeVisible({
      timeout: 5000,
    });
    await expect(page.locator('[data-testid="chunk-browser"]')).toBeVisible({
      timeout: 5000,
    });

    // With no collection selected, should show empty state
    await expect(page.locator('.stats-empty')).toBeVisible({ timeout: 5000 });
  });

  test('TC-STATS-002: stats cards render with correct counts after upload', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request, `Stats Upload ${Date.now()}`);

    // Upload a document via API
    const token = page.url(); // we already have token via setupAuthAndCollection
    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });

    // Select the collection
    await setActiveCollection(page, collection.id);
    await page.waitForSelector('.drop-zone', { timeout: 10000 });

    // Upload a document
    await fileInput(page).setInputFiles({
      name: 'stats-test.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from('# Statistics Test\n\nThis document is used for stats testing.'),
    });

    await expect(page.locator('.dl-item__name').first()).toContainText('stats-test.md', {
      timeout: 30000,
    });

    // Navigate to Statistics tab
    await page.locator('[data-testid="admin-tab-stats"]').click();

    // Wait for stats to load
    await expect(page.locator('.stats-grid')).toBeVisible({ timeout: 10000 });

    // Verify total documents is at least 1
    const totalDocs = page.locator('.stat-card').first().locator('.stat-value');
    await expect(totalDocs).not.toHaveText('0');
  });

  test('TC-STATS-003: chunk search finds uploaded document chunks', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(
      page,
      request,
      `Stats ChunkSearch ${Date.now()}`,
    );

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });

    await setActiveCollection(page, collection.id);
    await page.waitForSelector('.drop-zone', { timeout: 10000 });

    // Upload a document with known content
    await fileInput(page).setInputFiles({
      name: 'searchable-doc.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from(
        '# Searchable Content\n\nThis document contains unique search terms for testing.',
      ),
    });

    await expect(page.locator('.dl-item__name').first()).toContainText('searchable-doc.md', {
      timeout: 30000,
    });

    // Navigate to Statistics tab
    await page.locator('[data-testid="admin-tab-stats"]').click();

    // Type in the search input and search
    const searchInput = page.locator('.search-input');
    await searchInput.fill('searchable');
    await searchInput.press('Enter');

    // Wait for results
    await expect(page.locator('.chunk-card').first()).toBeVisible({ timeout: 15000 });

    // Verify the source badge shows "Upload"
    await expect(page.locator('.chunk-card').first().locator('.v-badge')).toContainText('Upload');
  });

  test('TC-STATS-004: search mode toggle works', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Stats Toggle ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });

    await page.locator('[data-testid="admin-tab-stats"]').click();

    // Verify text search and semantic search pills exist
    await expect(page.locator('.search-mode-toggle')).toBeVisible();
    const textBtn = page.locator('.pill-btn').filter({ hasText: 'Text Search' });
    const semanticBtn = page.locator('.pill-btn').filter({ hasText: 'Semantic Search' });

    await expect(textBtn).toBeVisible();
    await expect(semanticBtn).toBeVisible();

    // Click semantic search
    await semanticBtn.click();
    await expect(semanticBtn).toHaveClass(/pill-btn--active/);
    await expect(textBtn).not.toHaveClass(/pill-btn--active/);
  });
});
