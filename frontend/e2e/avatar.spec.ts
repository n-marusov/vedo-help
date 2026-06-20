import { expect, test } from '@playwright/test';
import { setupAuth } from './helpers';

/**
 * UserAvatar component E2E checks for Task 1.2.
 *
 * Verifies the standalone preview route for the SVG-based user/assistant
 * avatar component before MessageBubble integration happens in Task 2.1.
 */
test.describe('UserAvatar component', () => {
  test.beforeEach(async ({ page }) => {
    page.on('console', (message) => {
      if (message.type() === 'debug') {
        console.debug('[FIX:avatar-e2e] browser debug', message.text());
      }
    });

    // DEBUG [e2e] avatar: adding auth setup
    await setupAuth(page);
    await page.goto('/ui-preview/avatar');
  });

  test('TC-AVATAR-001: renders user avatar with person SVG and accessible label', async ({
    page,
  }) => {
    const avatar = page.getByTestId('avatar-user-md');

    await expect(avatar).toBeVisible();
    await expect(avatar).toHaveAttribute('aria-label', 'User avatar');
    await expect(avatar.locator('[data-testid="user-avatar-icon"]')).toBeVisible();
    await expect(avatar.locator('[data-testid="assistant-avatar-icon"]')).toHaveCount(0);
  });

  test('TC-AVATAR-002: renders assistant avatar with branded V SVG', async ({ page }) => {
    const avatar = page.getByTestId('avatar-assistant-lg');

    await expect(avatar).toBeVisible();
    await expect(avatar).toHaveAttribute('aria-label', 'VEDO assistant avatar');
    await expect(avatar.locator('[data-testid="assistant-avatar-icon"]')).toBeVisible();
    await expect(avatar).toContainText('V');
  });

  test('TC-AVATAR-003: uses role-specific token colors', async ({ page }) => {
    const userAvatar = page.getByTestId('avatar-user-md');
    const assistantAvatar = page.getByTestId('avatar-assistant-lg');

    const userColor = await userAvatar.evaluate((el) => getComputedStyle(el).backgroundColor);
    const assistantColor = await assistantAvatar.evaluate(
      (el) => getComputedStyle(el).backgroundColor,
    );

    expect(userColor).not.toBe(assistantColor);
  });

  test('TC-AVATAR-004: maps size variants from the avatar token', async ({ page }) => {
    const smallWidth = await page
      .getByTestId('avatar-user-sm')
      .evaluate((el) => Number.parseInt(getComputedStyle(el).width));
    const mediumWidth = await page
      .getByTestId('avatar-user-md')
      .evaluate((el) => Number.parseInt(getComputedStyle(el).width));
    const largeWidth = await page
      .getByTestId('avatar-assistant-lg')
      .evaluate((el) => Number.parseInt(getComputedStyle(el).width));

    expect(smallWidth).toBe(24);
    expect(mediumWidth).toBe(32);
    expect(largeWidth).toBe(40);
  });

  test('TC-AVATAR-005: keeps avatar implementation emoji-free', async ({ page }) => {
    const preview = page.getByTestId('avatar-preview-grid');

    await expect(preview).not.toContainText('👤');
    await expect(preview).not.toContainText('🤖');
  });
});
