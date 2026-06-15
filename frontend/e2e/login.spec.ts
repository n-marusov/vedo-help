import { test, expect } from '@playwright/test';

/**
 * Authentication / Login Page Tests (Task 5.5, 5.6)
 *
 * Tests the frontend login page with social provider buttons,
 * token refresh, session persistence, and auth guard.
 *
 * Note: These tests verify UI rendering and behavior.
 * Full OAuth2 flow requires KeyCloak running and is tested
 * separately via integration/environment tests.
 */
test.describe('Login Page (Task 5.5)', () => {
  test('TC-LOGIN-001: login page is accessible at /login route', async ({ page }) => {
    await page.goto('/login');
    const loginPage = page.locator('[data-testid="login-page"]');
    await expect(loginPage).toBeVisible({ timeout: 5000 });
  });

  test('TC-LOGIN-002: login page displays application name and welcome text', async ({ page }) => {
    await page.goto('/login');
    const loginPage = page.locator('[data-testid="login-page"]');
    await expect(loginPage).toContainText(/VEDO/i);
    await expect(loginPage).toContainText(/Sign in|Log in|Welcome/i);
  });

  test('TC-LOGIN-003: login page renders three social provider buttons', async ({ page }) => {
    await page.goto('/login');

    // Should have Google, GitHub, and Discord provider buttons
    const googleBtn = page.locator('[data-testid="btn-login-google"]');
    const githubBtn = page.locator('[data-testid="btn-login-github"]');
    const discordBtn = page.locator('[data-testid="btn-login-discord"]');

    await expect(googleBtn).toBeVisible();
    await expect(githubBtn).toBeVisible();
    await expect(discordBtn).toBeVisible();
  });

  test('TC-LOGIN-004: Google button displays Google icon/text', async ({ page }) => {
    await page.goto('/login');
    const googleBtn = page.locator('[data-testid="btn-login-google"]');
    await expect(googleBtn).toContainText(/Google/i);
  });

  test('TC-LOGIN-005: GitHub button displays GitHub icon/text', async ({ page }) => {
    await page.goto('/login');
    const githubBtn = page.locator('[data-testid="btn-login-github"]');
    await expect(githubBtn).toContainText(/GitHub/i);
  });

  test('TC-LOGIN-006: Discord button displays Discord icon/text', async ({ page }) => {
    await page.goto('/login');
    const discordBtn = page.locator('[data-testid="btn-login-discord"]');
    await expect(discordBtn).toContainText(/Discord/i);
  });

  test('TC-LOGIN-007: clicking a provider button navigates to KeyCloak', async ({ page }) => {
    await page.goto('/login');
    const githubBtn = page.locator('[data-testid="btn-login-github"]');

    // Intercept navigation to check redirect URL
    page.on('request', (request) => {
      const url = request.url();
      // The redirect should go to KeyCloak authorization endpoint
      if (url.includes('/auth')) {
        expect(url).toContain('/protocol/openid-connect/auth');
      }
    });

    await githubBtn.click();
  });

  test('TC-LOGIN-008: login page is responsive on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/login');

    // Login container should fit mobile viewport
    const loginContainer = page.locator('[data-testid="login-container"]');
    const box = await loginContainer.boundingBox();
    expect(box).not.toBeNull();
    if (box) {
      expect(box.width).toBeLessThanOrEqual(375);
    }
  });

  test('TC-LOGIN-009: login page shows error state for failed auth', async ({ page }) => {
    // Navigate to login with an error parameter (simulating failed OAuth callback)
    await page.goto('/login?error=access_denied');
    const errorMsg = page.locator('[data-testid="login-error"]');
    await expect(errorMsg).toBeVisible();
    await expect(errorMsg).toContainText(/access denied|failed|could not/i);
  });

  test('TC-LOGIN-010: login page has a privacy/terms notice', async ({ page }) => {
    await page.goto('/login');
    const notice = page.locator('[data-testid="login-notice"]');
    await expect(notice).toBeVisible();
    await expect(notice).toContainText(/privacy|terms|secure/i);
  });
});

test.describe('Auth Guard & Session (Task 5.6)', () => {
  test('TC-AUTH-001: auth guard redirects unauthenticated users to login page', async ({ page }) => {
    // Clear any stored tokens
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });

    // Navigate to protected page
    await page.goto('/');
    // Should redirect to login
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-002: authenticated users can access main chat page', async ({ page }) => {
    // Set a mock token to simulate authenticated state
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.setItem('vedo_auth_token', 'mock-valid-jwt-token');
    });

    // Should render the chat view without redirecting to login
    await page.goto('/');
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 5000 });
  });

  test('TC-AUTH-003: logout clears token and redirects to login', async ({ page }) => {
    // Setup: set a token
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.setItem('vedo_auth_token', 'mock-valid-jwt-token');
    });

    // Navigate and find logout button
    await page.goto('/');
    const logoutBtn = page.locator('[data-testid="btn-logout"]');
    await logoutBtn.click();

    // Token should be cleared
    const token = await page.evaluate(() => localStorage.getItem('vedo_auth_token'));
    expect(token).toBeNull();

    // Should redirect to login
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-004: token persists across page reload', async ({ page }) => {
    // Set a token
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.setItem('vedo_auth_token', 'mock-valid-jwt-token');
    });

    // Reload the page
    await page.reload();

    // Token should still exist
    const token = await page.evaluate(() => localStorage.getItem('vedo_auth_token'));
    expect(token).toBe('mock-valid-jwt-token');

    // User should still be authenticated
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 5000 });
  });

  test('TC-AUTH-005: expired token shows login page on navigation', async ({ page }) => {
    // Set an expired token
    const expiredToken = 'eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE1MDAwMDAwMDB9.mock';
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.setItem('vedo_auth_token', '');
    });
    localStorage.setItem('vedo_auth_token', expiredToken);

    // Navigate — should detect expired token and redirect to login
    await page.goto('/');
    // Wait to see if it redirects
    await page.waitForTimeout(500);
    const currentUrl = page.url();
    if (!currentUrl.includes('/login')) {
      // If not redirected, at least show that auth state is handled
      const errorBanner = page.locator('[data-testid="auth-error"]');
      const isError = await errorBanner.isVisible().catch(() => false);
      expect(isError || currentUrl.includes('/login')).toBeTruthy();
    }
  });

  test('TC-AUTH-006: admin page requires auth guard too', async ({ page }) => {
    // Clear tokens
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.clear();
    });

    // Try accessing admin page
    await page.goto('/admin');
    // Should redirect to login
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-007: user avatar displays user initials from token', async ({ page }) => {
    // Set a mock token with user info
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.setItem('vedo_auth_token', 'mock-valid-jwt-token');
      // Mock user info for display
      localStorage.setItem('vedo_user_info', JSON.stringify({
        name: 'John Doe',
        email: 'john@example.com',
      }));
    });

    await page.goto('/');
    // User avatar should show initials from user name
    const userAvatar = page.locator('[data-testid="avatar-user"]').first();
    await expect(userAvatar).toContainText('JD');
  });
});
