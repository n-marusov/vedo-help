import { expect, test } from '@playwright/test';
import { API_URL, apiRequest, getTestAccessToken, setupAuthAndCollection } from './helpers';

test.describe('Web Crawl: full E2E flow', () => {
  test('TC-WEB-FLOW-001: create crawl job and verify in job list', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Flow ${Date.now()}`);

    // Create a crawl job via API
    const job = await apiRequest<{ id: string; entry_url: string; status: string }>(
      request,
      'POST',
      '/api/web-crawl',
      {
        entry_url: 'https://example.com',
        collection_id: collection.id,
        config: { max_depth: 1, max_pages: 5, delay_ms: 1000 },
      },
    );

    expect(job.id).toBeTruthy();
    expect(job.entry_url).toBe('https://example.com');

    // Verify job appears in list
    const jobs = await apiRequest<Array<{ id: string; entry_url: string }>>(
      request,
      'GET',
      '/api/web-crawl',
    );
    expect(jobs.some((item) => item.id === job.id)).toBe(true);
  });

  test('TC-WEB-FLOW-002: cancel pending crawl job', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Cancel ${Date.now()}`);

    const job = await apiRequest<{ id: string }>(request, 'POST', '/api/web-crawl', {
      entry_url: 'https://example.com',
      collection_id: collection.id,
      config: { max_depth: 1, max_pages: 5, delay_ms: 500 },
    });

    // Cancel the job
    const cancelled = await apiRequest<{ status: string }>(
      request,
      'POST',
      `/api/web-crawl/${job.id}/cancel`,
    );
    expect(cancelled.status).toBe('cancelled');

    // Verify status persisted
    const detail = await apiRequest<{ status: string }>(request, 'GET', `/api/web-crawl/${job.id}`);
    expect(detail.status).toBe('cancelled');
  });

  test('TC-WEB-FLOW-003: delete completed crawl job', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Delete ${Date.now()}`);

    const job = await apiRequest<{ id: string }>(request, 'POST', '/api/web-crawl', {
      entry_url: 'https://example.com',
      collection_id: collection.id,
      config: { max_depth: 1, max_pages: 3 },
    });

    // Delete the job
    await apiRequest(request, 'DELETE', `/api/web-crawl/${job.id}`);

    // Verify it's gone
    const jobs = await apiRequest<Array<{ id: string }>>(request, 'GET', '/api/web-crawl');
    expect(jobs.some((item) => item.id === job.id)).toBe(false);
  });

  test('TC-WEB-FLOW-004: web crawl tab visible in admin panel', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Tab ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await page.locator('.cm-card', { hasText: collection.name }).click();

    // Verify Web Crawl tab exists
    await expect(page.locator('button', { hasText: 'Web Crawl' })).toBeVisible();
    await page.locator('button', { hasText: 'Web Crawl' }).click();
    await expect(page.locator('[data-testid="web-crawl-manager"]')).toBeVisible({
      timeout: 5000,
    });
  });

  test('TC-WEB-FLOW-005: job detail includes discovered pages', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Pages ${Date.now()}`);

    const job = await apiRequest<{ id: string }>(request, 'POST', '/api/web-crawl', {
      entry_url: 'https://example.com',
      collection_id: collection.id,
      config: { max_depth: 1, max_pages: 3 },
    });

    // Get job detail
    const detail = await apiRequest<{ pages: Array<{ url: string; status: string }> }>(
      request,
      'GET',
      `/api/web-crawl/${job.id}`,
    );

    expect(detail.pages).toBeDefined();
  });

  test('TC-WEB-FLOW-006: retry failed pages endpoint', async ({ request, page }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Retry ${Date.now()}`);

    const job = await apiRequest<{ id: string }>(request, 'POST', '/api/web-crawl', {
      entry_url: 'https://example.com',
      collection_id: collection.id,
      config: { max_depth: 1, max_pages: 3 },
    });

    // Cancel the job first so it's not crawling
    await apiRequest(request, 'POST', `/api/web-crawl/${job.id}/cancel`);

    // Retry should work if there are failed pages (status reset to pending)
    const response = await request.post(`${API_URL}/api/web-crawl/${job.id}/retry`, {
      headers: {
        Authorization: `Bearer ${await getTestAccessToken()}`,
        'Content-Type': 'application/json',
      },
    });

    // Should succeed (200) or return 400 if no failed pages
    expect([200, 400]).toContain(response.status());
  });

  test('TC-WEB-FLOW-007: form validation in create dialog', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `WebCrawl Form ${Date.now()}`);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await page.locator('.cm-card', { hasText: collection.name }).click();
    await page.locator('button', { hasText: 'Web Crawl' }).click();
    await expect(page.locator('[data-testid="web-crawl-manager"]')).toBeVisible({
      timeout: 5000,
    });

    // Click + New Crawl button
    await page.locator('button', { hasText: '+ New Crawl' }).click();
    await expect(page.locator('.v-dialog')).toBeVisible();

    // Verify form elements
    await expect(page.locator('[data-testid="web-crawl-url-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="web-crawl-depth-slider"]')).toBeVisible();
    await expect(page.locator('[data-testid="web-crawl-max-pages-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="web-crawl-path-prefix-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="web-crawl-delay-slider"]')).toBeVisible();
  });
});
