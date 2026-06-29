import { expect, test } from '@playwright/test';

test.describe('Admin RAG Pipeline Debug', () => {
  test('TC-RAGDEBUG-001: Admin view has 3 tabs (Sources, Session Debug, RAG Pipeline Debug)', async ({
    page,
  }) => {
    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-tabs"]')).toBeVisible();
    await expect(page.locator('[data-testid="admin-tab-sources"]')).toBeVisible();
    await expect(page.locator('[data-testid="admin-tab-debug"]')).toBeVisible();
    // The new pipeline tab
    await expect(page.locator('[data-testid="admin-tab-pipeline"]')).toBeVisible();
  });

  test('TC-RAGDEBUG-002: Clicking "RAG Pipeline Debug" shows the panel with search', async ({
    page,
  }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-pipeline"]').click();
    await expect(page.locator('[data-testid="rag-pipeline-debug-view"]')).toBeVisible();
    await expect(page.locator('[data-testid="rag-pipeline-search"]')).toBeVisible();
  });

  test('TC-RAGDEBUG-003: Search sessions with debug data shows results', async ({
    page,
    request,
  }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-pipeline"]').click();
    await expect(page.locator('[data-testid="rag-pipeline-search"]')).toBeVisible();
    await page.locator('[data-testid="rag-pipeline-search"]').fill('Technical');
    // Wait for the API response and list update
    await page.waitForTimeout(1000);
    const sessionItems = page.locator('[data-testid="pipeline-session-item"]');
    await expect(sessionItems.first()).toBeVisible({ timeout: 5000 });
  });

  test('TC-RAGDEBUG-004: Clicking a session shows 7-step pipeline visualization', async ({
    page,
    request,
  }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-pipeline"]').click();
    await page.locator('[data-testid="rag-pipeline-search"]').fill('Technical');
    await page.waitForTimeout(1000);
    const firstSession = page.locator('[data-testid="pipeline-session-item"]').first();
    await expect(firstSession).toBeVisible({ timeout: 5000 });
    await firstSession.click();
    // Verify the 7 pipeline steps are shown
    const stepElements = page.locator('[data-testid="pipeline-step"]');
    await expect(stepElements).toHaveCount(7);
    // Verify each step has a title
    const stepTitles = page.locator('[data-testid="pipeline-step-title"]');
    await expect(stepTitles).toHaveCount(7);
  });

  test('TC-RAGDEBUG-005: Flipping step statuses shows correct states', async ({
    page,
    request,
  }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-pipeline"]').click();
    await page.locator('[data-testid="rag-pipeline-search"]').fill('Technical');
    await page.waitForTimeout(1000);
    const firstSession = page.locator('[data-testid="pipeline-session-item"]').first();
    await expect(firstSession).toBeVisible({ timeout: 5000 });
    await firstSession.click();
    // Steps 1-2 should now be 'active' (multi-query, HyDE)
    // Steps 4-6 should now be 'active' (keyword, merge, reranking)
    const activeSteps = page.locator('[data-testid="pipeline-step"][data-status="active"]');
    await expect(activeSteps).toHaveCount(5);
  });

  test('TC-RAGDEBUG-006: Switching back to Sources tab works', async ({ page, request }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-pipeline"]').click();
    await expect(page.locator('[data-testid="rag-pipeline-debug-view"]')).toBeVisible();
    // Switch back to Sources tab
    await page.locator('[data-testid="admin-tab-sources"]').click();
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible();
  });
});
