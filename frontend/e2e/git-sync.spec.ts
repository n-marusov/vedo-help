import { expect, test } from '@playwright/test';
import { API_URL, apiRequest, getTestAccessToken, setupAuthAndCollection } from './helpers';

test.describe('Git Sync: real backend repository management', () => {
  test('TC-GIT-001: register new git repo through UI → row appears', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Git UI ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await page.locator('.cm-card', { hasText: collection.name }).click();

    await page.locator('button', { hasText: 'Git Repositories' }).click();
    await expect(page.locator('[data-testid="git-repo-manager"]')).toBeVisible({
      timeout: 10000,
    });

    await page.locator('[data-testid="btn-git-repo-connect"]').click();
    await page
      .locator('[data-testid="git-repo-url-input"]')
      .fill(`https://github.com/example/e2e-${Date.now()}.git`);
    await page.locator('[data-testid="git-repo-branch-input"]').fill('main');
    await page.locator('[data-testid="btn-git-repo-register"]').click();

    const repoRow = page.locator('[data-testid="git-repo-row"]').first();
    await expect(repoRow).toBeVisible({ timeout: 10000 });
    await expect(repoRow).toContainText('github.com/example/e2e-');
    await expect(repoRow.locator('[data-testid="git-repo-status"]')).toContainText(/idle/i);
  });

  test('TC-GIT-002: backend rejects invalid repo URL', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `Git Invalid ${Date.now()}`);
    const token = await getTestAccessToken();

    const response = await request.post(`${API_URL}/api/git-sync/repos`, {
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      data: {
        url: 'ftp://example.invalid/repo.git',
        branch: 'main',
        collection_id: collection.id,
      },
    });

    expect(response.status()).toBe(400);
  });

  test('TC-GIT-003: create and delete repo through real backend API', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `Git API ${Date.now()}`);
    const repo = await apiRequest<{ id: string; url: string }>(
      request,
      'POST',
      '/api/git-sync/repos',
      {
        url: `https://github.com/example/delete-${Date.now()}.git`,
        branch: 'main',
        collection_id: collection.id,
      },
    );

    expect(repo.id).toBeTruthy();
    await apiRequest(request, 'DELETE', `/api/git-sync/repos/${repo.id}`);

    const repos = await apiRequest<Array<{ id: string }>>(request, 'GET', '/api/git-sync/repos');
    expect(repos.some((item) => item.id === repo.id)).toBe(false);
  });
});
