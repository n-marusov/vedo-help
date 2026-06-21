import { expect, test } from '@playwright/test';
import { setupAuth } from './helpers';

test.describe('loading skeletons (RED)', () => {
  test.skip('slow session load shows skeleton in messages area', async ({ page }) => {
    // Slow the session messages endpoint
    await page.route('**/api/sessions/**', async (route) => {
      await new Promise((r) => setTimeout(r, 500));
      await route.continue();
    });

    await setupAuth(page);
    await page.goto('/');
    await page.waitForSelector('[data-testid="messages-loading-skeleton"]', {
      timeout: 5000,
    });
  });

  test.skip('slow GET /sessions shows skeleton in sidebar', async ({ page }) => {
    await page.route('**/api/sessions*', async (route) => {
      if (route.request().method() === 'GET') {
        await new Promise((r) => setTimeout(r, 500));
      }
      await route.continue();
    });

    await setupAuth(page);
    await page.goto('/');
    await page.waitForSelector('[data-testid="sessions-loading-skeleton"]', {
      timeout: 5000,
    });
  });

  test.skip('slow GET /api/documents shows skeleton in documents area', async ({ page }) => {
    await page.route('**/api/documents*', async (route) => {
      if (route.request().method() === 'GET') {
        await new Promise((r) => setTimeout(r, 500));
      }
      await route.continue();
    });

    await setupAuth(page);
    await page.goto('/');
    // Navigate to admin/documents view
    await page.goto('/admin');
    await page.waitForSelector('[data-testid="documents-loading-skeleton"]', {
      timeout: 5000,
    });
  });

  test.skip('slow GET /api/git-sync shows skeleton in repos area', async ({ page }) => {
    await page.route('**/api/git-sync/**', async (route) => {
      await new Promise((r) => setTimeout(r, 500));
      await route.continue();
    });

    await setupAuth(page);
    await page.goto('/');
    await page.goto('/admin');
    await page.waitForSelector('[data-testid="repos-loading-skeleton"]', {
      timeout: 5000,
    });
  });
});
