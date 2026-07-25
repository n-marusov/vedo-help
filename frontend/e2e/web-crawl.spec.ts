import { expect, test } from '@playwright/test';
import { API_URL, apiRequest, getTestAccessToken, setupAuthAndCollection } from './helpers';

test.describe('Web Crawl: UI and backend integration', () => {
  test('TC-WEB-001: verify admin panel has Web Crawl source tab', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl UI ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await page.locator('.cm-card', { hasText: collection.name }).click();

    // Verify all three source tabs exist
    await expect(page.locator('button', { hasText: 'Documents' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'Git Repositories' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'Web Crawl' })).toBeVisible();

    // Click Web Crawl tab
    await page.locator('button', { hasText: 'Web Crawl' }).click();
    await expect(page.locator('[data-testid="web-crawl-manager"]')).toBeVisible({
      timeout: 5000,
    });
  });

  test('TC-WEB-002: form validation — empty URL rejected', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(
      page,
      request,
      `WebCrawl Validate ${Date.now()}`,
    );

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await page.locator('.cm-card', { hasText: collection.name }).click();

    await page.locator('button', { hasText: 'Web Crawl' }).click();
    await expect(page.locator('[data-testid="web-crawl-manager"]')).toBeVisible({
      timeout: 5000,
    });

    // Try to start crawl with empty URL
    await page.locator('[data-testid="btn-web-crawl-start"]').click();
    await expect(page.locator('[data-testid="web-crawl-url-error"]')).toBeVisible();
  });

  test('TC-WEB-003: form has all configuration inputs', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Config ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await page.locator('.cm-card', { hasText: collection.name }).click();

    await page.locator('button', { hasText: 'Web Crawl' }).click();
    await expect(page.locator('[data-testid="web-crawl-manager"]')).toBeVisible({
      timeout: 5000,
    });

    // Verify configuration form elements exist
    await expect(page.locator('[data-testid="web-crawl-url-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="web-crawl-depth-slider"]')).toBeVisible();
    await expect(page.locator('[data-testid="web-crawl-max-pages-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="web-crawl-path-prefix-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="web-crawl-delay-slider"]')).toBeVisible();
  });

  test('TC-WEB-004: create and list crawl jobs through API', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl API ${Date.now()}`);

    // Create a crawl job via API
    const job = await apiRequest<{ id: string; entry_url: string; status: string }>(
      request,
      'POST',
      '/api/web-crawl',
      {
        entry_url: 'https://example.com',
        collection_id: collection.id,
        config: { max_depth: 1, max_pages: 10, delay_ms: 1000 },
      },
    );

    expect(job.id).toBeTruthy();
    expect(job.entry_url).toBe('https://example.com');
    expect(job.status).toMatch(/idle|crawling|completed/);

    // List jobs
    const jobs = await apiRequest<Array<{ id: string }>>(request, 'GET', '/api/web-crawl');
    expect(jobs.some((item) => item.id === job.id)).toBe(true);
  });

  test('TC-WEB-005: delete crawl job through API', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Delete ${Date.now()}`);

    const job = await apiRequest<{ id: string }>(request, 'POST', '/api/web-crawl', {
      entry_url: 'https://example.com',
      collection_id: collection.id,
      config: { max_depth: 1, max_pages: 5 },
    });

    // Delete job
    await apiRequest(request, 'DELETE', `/api/web-crawl/${job.id}`);

    // Verify job is removed
    const jobs = await apiRequest<Array<{ id: string }>>(request, 'GET', '/api/web-crawl');
    expect(jobs.some((item) => item.id === job.id)).toBe(false);
  });

  test('TC-WEB-006: empty state when no crawl jobs exist', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Empty ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await page.locator('.cm-card', { hasText: collection.name }).click();

    await page.locator('button', { hasText: 'Web Crawl' }).click();
    await expect(page.locator('[data-testid="web-crawl-manager"]')).toBeVisible({
      timeout: 5000,
    });

    // Verify empty state is visible when no jobs exist
    await expect(page.locator('[data-testid="web-crawl-empty-state"]')).toBeVisible();
  });

  test('TC-WEB-007: backend rejects crawl with invalid URL', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(
      page,
      request,
      `WebCrawl Invalid ${Date.now()}`,
    );
    const token = await getTestAccessToken();

    // Invalid URL (not http/https)
    const response = await request.post(`${API_URL}/api/web-crawl`, {
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      data: {
        entry_url: 'ftp://invalid-protocol.com',
        collection_id: collection.id,
      },
    });

    expect(response.status()).toBe(400);
  });

  test('TC-WEB-008: backend rejects invalid config bounds', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Bounds ${Date.now()}`);
    const token = await getTestAccessToken();

    // max_depth out of range
    const response = await request.post(`${API_URL}/api/web-crawl`, {
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      data: {
        entry_url: 'https://example.com',
        collection_id: collection.id,
        config: { max_depth: 50, max_pages: 5 },
      },
    });

    expect(response.status()).toBe(400);
  });

  test('TC-WEB-009: cancel running crawl job', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Cancel ${Date.now()}`);

    const job = await apiRequest<{ id: string }>(request, 'POST', '/api/web-crawl', {
      entry_url: 'https://example.com',
      collection_id: collection.id,
      config: { max_depth: 1, max_pages: 5, delay_ms: 500 },
    });

    // Cancel the job
    await apiRequest(request, 'POST', `/api/web-crawl/${job.id}/cancel`);

    // Verify status changed to cancelled
    const cancelled = await apiRequest<{ status: string }>(
      request,
      'GET',
      `/api/web-crawl/${job.id}`,
    );
    expect(cancelled.status).toBe('cancelled');
  });

  test('TC-WEB-010: job detail shows discovered pages', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Detail ${Date.now()}`);

    const job = await apiRequest<{ id: string }>(request, 'POST', '/api/web-crawl', {
      entry_url: 'https://example.com',
      collection_id: collection.id,
      config: { max_depth: 1, max_pages: 5 },
    });

    // Get job detail with pages
    const detail = await apiRequest<{ pages: Array<{ url: string; status: string }> }>(
      request,
      'GET',
      `/api/web-crawl/${job.id}`,
    );

    expect(detail.pages).toBeDefined();
  });
});
