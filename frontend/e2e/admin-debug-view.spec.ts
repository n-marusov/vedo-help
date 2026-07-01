import { expect, test } from '@playwright/test';

test.describe('Admin Debug View', () => {
  test('TC-ADEBUG-001: Admin panel shows Sources, Debug, and Health tabs', async ({ page }) => {
    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-tabs"]')).toBeVisible();
    await expect(page.locator('[data-testid="admin-tab-sources"]')).toBeVisible();
    await expect(page.locator('[data-testid="admin-tab-debug"]')).toBeVisible();
    await expect(page.locator('[data-testid="admin-tab-health"]')).toBeVisible();
  });

  test('TC-ADEBUG-002: Clicking Debug tab shows session search', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-debug"]').click();
    await expect(page.locator('[data-testid="session-debug-view"]')).toBeVisible();
    await expect(page.locator('[data-testid="session-debug-search"]')).toBeVisible();
  });

  test('TC-ADEBUG-003: Searching sessions returns results', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-debug"]').click();
    await page.locator('[data-testid="session-debug-search"]').fill('Technical');
    // Wait for the API response and list update
    await page.waitForTimeout(1000);
    const sessionItems = page.locator('[data-testid="session-list-item"]');
    await expect(sessionItems.first()).toBeVisible({ timeout: 5000 });
  });

  test('TC-ADEBUG-004: Selecting session shows messages with pipeline debug', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-debug"]').click();
    await page.locator('[data-testid="session-debug-search"]').fill('Technical');
    await page.waitForTimeout(1000);
    const firstSession = page.locator('[data-testid="session-list-item"]').first();
    await expect(firstSession).toBeVisible({ timeout: 5000 });
    await firstSession.click();
    await expect(page.locator('[data-testid="session-msg"]').first()).toBeVisible({
      timeout: 5000,
    });
    // Assistant messages with debug data show the pipeline panel
    const debugPanels = page.locator('[data-testid="debug-panel"]');
    await expect(debugPanels.first()).toBeVisible({ timeout: 5000 });
  });

  test('TC-ADEBUG-005: Debug panel shows 7 pipeline steps', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-debug"]').click();
    await page.locator('[data-testid="session-debug-search"]').fill('Technical');
    await page.waitForTimeout(1000);
    const firstSession = page.locator('[data-testid="session-list-item"]').first();
    await expect(firstSession).toBeVisible({ timeout: 5000 });
    await firstSession.click();
    // Verify all 7 step titles
    const stepTitles = page.locator('[data-testid="debug-step-title"]');
    await expect(stepTitles).toHaveCount(7);
  });

  test('TC-ADEBUG-006: Switching back to Sources tab works', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-debug"]').click();
    await expect(page.locator('[data-testid="session-debug-view"]')).toBeVisible();
    await page.locator('[data-testid="admin-tab-sources"]').click();
    // Collections panel should be visible again
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible();
  });

  test('TC-ADEBUG-007: No separate pipeline tab exists', async ({ page }) => {
    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-tab-pipeline"]')).toHaveCount(0);
  });
});
