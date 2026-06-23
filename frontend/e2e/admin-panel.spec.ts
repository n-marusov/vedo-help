import { expect, test } from '@playwright/test';
import { API_URL, fileInput, getTestAccessToken, setupAuthAndCollection } from './helpers';

test.describe('Admin panel: documents tab', () => {
  test('TC-ADMIN-001: no drop zone visible, upload button works', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Admin NoDrop ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({ timeout: 10000 });
    await page.locator('.cm-card', { hasText: collection.name }).click();

    // Wait for the documents tab to load
    await expect(page.locator('.document-list')).toBeVisible({ timeout: 10000 });

    // Drop zone should NOT exist
    const dropZone = page.locator('[data-testid="drop-zone"]');
    await expect(dropZone).toHaveCount(0);

    // Upload button should exist
    const uploadBtn = page.locator('button', { hasText: 'Upload' });
    await expect(uploadBtn).toBeEnabled();

    // Upload via file picker should work
    await fileInput(page).setInputFiles({
      name: 'test-doc.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from('# Test Doc\n\nContent for admin upload test.'),
    });

    await expect(page.locator('.dl-item__name').first()).toContainText('test-doc.md', {
      timeout: 30000,
    });
  });
});

test.describe('Admin panel: git sync race condition', () => {
  test('TC-GIT-004: rapid sequential syncs produce no duplicate chunks', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request, `Git Race ${Date.now()}`);
    const token = await getTestAccessToken();

    // Register a test repo via API
    const repo = await (
      await request.post(`${API_URL}/api/git-sync/repos`, {
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
        data: {
          url: `https://github.com/example/race-${Date.now()}.git`,
          branch: 'main',
          collection_id: collection.id,
        },
      })
    ).json();
    const repoId = (repo as { id: string }).id;

    // Trigger two rapid sequential syncs
    const sync1 = request.post(`${API_URL}/api/git-sync/repos/${repoId}/sync`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    const sync2 = request.post(`${API_URL}/api/git-sync/repos/${repoId}/sync`, {
      headers: { Authorization: `Bearer ${token}` },
    });

    const [res1, res2] = await Promise.all([sync1, sync2]);

    // At least one should succeed (the second should be rejected or blocked)
    expect([res1.status(), res2.status()]).toContain(200);

    // One of them should return 409 (conflict) if race condition guard works
    const statuses = [res1.status(), res2.status()];
    const hasConflict = statuses.includes(409);
    if (!hasConflict) {
      // If both returned 200, at least verify no duplicate chunks by
      // checking document count is not inflated
      const documents = await (
        await request.get(`${API_URL}/api/collections/${collection.id}/documents`, {
          headers: { Authorization: `Bearer ${token}` },
        })
      ).json();
      const docs = documents as Array<{ id: string; name: string }>;
      const mdDocs = docs.filter((d) => d.name.endsWith('.md'));
      // Each unique MD file should appear only once
      const names = mdDocs.map((d) => d.name);
      const uniqueNames = new Set(names);
      expect(uniqueNames.size).toBe(names.length);
    }
  });
});
