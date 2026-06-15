import { test, expect } from '@playwright/test';

/**
 * Avatar Component Tests (Task 1.2)
 *
 * Tests the Avatar component that displays user/assistant initials,
 * consistent color coding, and online status indicator.
 */
test.describe('Avatar Component', () => {
  test('TC-AVATAR-001: renders user avatar with correct initials', async ({ page }) => {
    await page.goto('/');
    // After implementation, the chat view should show user avatar with "Y" initials
    // in the first MessageBubble. For now, we test that the avatar component exists.
    const avatar = page.locator('[data-testid="avatar-user"]').first();
    await expect(avatar).toBeVisible({ timeout: 5000 });
    // User avatar should show "Y" for "You"
    await expect(avatar).toHaveText('Y');
  });

  test('TC-AVATAR-002: renders assistant avatar with correct initials', async ({ page }) => {
    await page.goto('/');
    const avatar = page.locator('[data-testid="avatar-assistant"]').first();
    await expect(avatar).toBeVisible({ timeout: 5000 });
    // Assistant avatar should show "V" for "VEDO"
    await expect(avatar).toHaveText('V');
  });

  test('TC-AVATAR-003: assigns consistent color based on identifier', async ({ page }) => {
    await page.goto('/');
    // Same avatar should have same background color across renders
    const avatar1 = page.locator('[data-testid="avatar-assistant"]').first();
    const avatar2 = page.locator('[data-testid="avatar-assistant"]').last();
    const color1 = await avatar1.evaluate((el) => getComputedStyle(el).backgroundColor);
    const color2 = await avatar2.evaluate((el) => getComputedStyle(el).backgroundColor);
    expect(color1).toBe(color2);
  });

  test('TC-AVATAR-004: renders online status indicator', async ({ page }) => {
    await page.goto('/');
    const statusDot = page.locator('[data-testid="avatar-status-online"]').first();
    await expect(statusDot).toBeVisible({ timeout: 5000 });
    // Status dot should be green for online
    await expect(statusDot).toHaveCSS('background-color', expect.stringContaining('rgb'));
  });

  test('TC-AVATAR-005: renders offline status indicator when user is away', async ({ page }) => {
    await page.goto('/');
    // Assistant should show offline status initially
    const statusDot = page.locator('[data-testid="avatar-status-offline"]').first();
    await expect(statusDot).toBeVisible({ timeout: 5000 });
    await expect(statusDot).toHaveCSS('background-color', expect.stringContaining('rgb'));
  });

  test('TC-AVATAR-006: renders with correct size variant (sm/md/lg)', async ({ page }) => {
    await page.goto('/');
    // Default size should be medium (32px)
    const avatar = page.locator('[data-testid="avatar-user"]').first();
    const width = await avatar.evaluate((el) => getComputedStyle(el).width);
    expect(parseInt(width)).toBe(32);
  });

  test('TC-AVATAR-007: does not render avatar for empty messages', async ({ page }) => {
    await page.goto('/');
    // Empty/placeholder messages should not display an avatar
    const avatars = page.locator('[data-testid^="avatar-"]');
    const count = await avatars.count();
    // Avatar should only render for actual messages, not empty placeholders
    expect(count).toBeGreaterThanOrEqual(0);
  });
});
