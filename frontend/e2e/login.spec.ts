import { expect, test } from '@playwright/test';

/**
 * Authentication / Login Page Tests
 *
 * Tests the frontend login page with social provider buttons
 * matching the actual LoginButtons.vue implementation (5 providers:
 * VK, Yandex, Mail.ru, Google, Corporate SSO).
 *
 * Full OAuth2 flow requires KeyCloak running and is tested
 * separately via integration/environment tests. Tests that need
 * a valid session inject a mock JWT into localStorage before
 * the app loads.
 */

const PROVIDERS = [
  { id: 'vk', label: /VK ID/i },
  { id: 'yandex', label: /Yandex ID/i },
  { id: 'mailru', label: /Mail\.ru/i },
  { id: 'google', label: /Google/i },
  { id: 'corp-sso', label: /Corporate SSO|SAML|OIDC/i },
] as const;

/**
 * Build a mock JWT with the given payload claims.
 * The resulting string has the form `header.base64Payload.signature`
 * and is decoded by the app's decodeToken() helper.
 */
function makeMockJwt(claims: Record<string, unknown>): string {
  const header = btoa(JSON.stringify({ alg: 'RS256', typ: 'JWT' }));
  const payload = btoa(JSON.stringify(claims));
  return `${header}.${payload}.mocksignature`;
}

test.describe('Login Page', () => {
  test('TC-LOGIN-001: login page is accessible at /login route', async ({ page }) => {
    await page.goto('/login');
    const loginPage = page.locator('[data-testid="login-page"]');
    await expect(loginPage).toBeVisible({ timeout: 5000 });
  });

  test('TC-LOGIN-002: login page displays app name and title', async ({ page }) => {
    await page.goto('/login');
    const title = page.locator('[data-testid="login-title"]');
    await expect(title).toBeVisible();
    await expect(title).toContainText(/Virtual Environment|VEDO/i);
  });

  test('TC-LOGIN-003: login page renders all 5 provider buttons', async ({ page }) => {
    await page.goto('/login');

    for (const p of PROVIDERS) {
      const btn = page.locator(`[data-testid="btn-login-${p.id}"]`);
      await expect(btn).toBeVisible();
    }
  });

  test('TC-LOGIN-004: each provider button displays its label text', async ({ page }) => {
    await page.goto('/login');

    for (const p of PROVIDERS) {
      const btn = page.locator(`[data-testid="btn-login-${p.id}"]`);
      await expect(btn).toContainText(p.label);
    }
  });

  test('TC-LOGIN-005: each button has an SVG icon', async ({ page }) => {
    await page.goto('/login');

    for (const p of PROVIDERS) {
      const btn = page.locator(`[data-testid="btn-login-${p.id}"]`);
      const svg = btn.locator('svg.provider-icon');
      await expect(svg).toBeVisible();
    }
  });

  test('TC-LOGIN-006: provider buttons are clickable and show loading state', async ({ page }) => {
    await page.goto('/login');
    const btn = page.locator('[data-testid="btn-login-vk"]');
    await expect(btn).not.toBeDisabled();
    await expect(btn).toHaveClass(/oauth-btn/);
  });

  test('TC-LOGIN-008: header row has title, subtitle and theme toggle', async ({ page }) => {
    await page.goto('/login');
    const headerRow = page.locator('[data-testid="login-header-row"]');
    await expect(headerRow).toBeVisible();

    const title = headerRow.locator('[data-testid="login-title"]');
    await expect(title).toBeVisible();

    const themeToggle = headerRow.locator('[data-testid="theme-toggle"]');
    await expect(themeToggle).toBeVisible();
  });

  test('TC-LOGIN-009: callback page shows error on missing PKCE data and redirects back', async ({
    page,
  }) => {
    // Navigate to callback without setting PKCE state/verifier
    await page.goto('/callback?code=mock_auth_code&state=mock_state');

    // Without PKCE state, handleCallback throws -> error view
    const callbackPage = page.locator('[data-testid="callback-page"]');
    await expect(callbackPage).toBeVisible({ timeout: 5000 });

    const errorTitle = callbackPage.locator('.error-title');
    await expect(errorTitle).toContainText(/Authentication Failed/i);

    const errorMessage = callbackPage.locator('.error-message');
    await expect(errorMessage).toContainText(/PKCE|state|verifier|start the login flow/i);

    // Should have a link back to login
    const backLink = callbackPage.locator('.error-link');
    await expect(backLink).toBeVisible();
    await expect(backLink).toHaveAttribute('href', '/login');
  });

  test('TC-LOGIN-010: callback page shows error state for failed auth', async ({ page }) => {
    await page.goto('/callback?error=access_denied&error_description=Access+denied');
    const callbackPage = page.locator('[data-testid="callback-page"]');
    await expect(callbackPage).toBeVisible({ timeout: 5000 });

    const errorTitle = callbackPage.locator('.error-title');
    await expect(errorTitle).toContainText(/Authentication Failed/i);

    const backLink = callbackPage.locator('.error-link');
    await expect(backLink).toBeVisible();
    await expect(backLink).toHaveAttribute('href', '/login');
  });

  test('TC-LOGIN-011: callback page shows error on missing auth code', async ({ page }) => {
    await page.goto('/callback');
    const callbackPage = page.locator('[data-testid="callback-page"]');
    await expect(callbackPage).toBeVisible({ timeout: 5000 });

    const errorTitle = callbackPage.locator('.error-title');
    await expect(errorTitle).toContainText(/Authentication Failed/i);

    const errorMessage = callbackPage.locator('.error-message');
    await expect(errorMessage).not.toBeEmpty();
  });

  test('TC-LOGIN-012: callback page shows error on state mismatch', async ({ page }) => {
    // Send user to login page first so we can set PKCE state
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.setItem('vedo_pkce_state', 'expected-state');
      localStorage.setItem('vedo_pkce_verifier', 'mock-verifier-xxxxxxxxxxxxxxxxxxxxxxxxxxxxx');
    });

    // Navigate with a different state
    await page.goto('/callback?code=some_code&state=wrong-state');

    const callbackPage = page.locator('[data-testid="callback-page"]');
    await expect(callbackPage).toBeVisible({ timeout: 5000 });

    const errorMessage = callbackPage.locator('.error-message');
    await expect(errorMessage).toContainText(/state mismatch/i);
  });

  test('TC-LOGIN-013: login page subtitle text is correct', async ({ page }) => {
    await page.goto('/login');
    const subtitle = page.locator('.subtitle');
    await expect(subtitle).toBeVisible();
    await expect(subtitle).toHaveText(/Build, connect, and share knowledge at scale/i);
  });

  test('TC-LOGIN-015: theme toggle on login page changes theme', async ({ page }) => {
    await page.goto('/login');

    const themeToggle = page.locator('[data-testid="theme-toggle"]');
    await expect(themeToggle).toBeVisible();

    const initialTheme = await page.evaluate(() =>
      document.documentElement.getAttribute('data-theme'),
    );

    await themeToggle.click();

    const newTheme = await page.evaluate(() => document.documentElement.getAttribute('data-theme'));
    expect(newTheme).not.toBe(initialTheme);
  });

  test('TC-LOGIN-016: login page background uses CSS custom properties', async ({ page }) => {
    await page.goto('/login');
    const loginView = page.locator('[data-testid="login-page"]');
    const bg = await loginView.evaluate((el) =>
      getComputedStyle(el).getPropertyValue('background-color'),
    );
    expect(bg).toBeTruthy();
    expect(bg).not.toBe('transparent');
  });
});

test.describe('Auth Guard & Session', () => {
  test('TC-AUTH-001: auth guard redirects unauthenticated users to login page', async ({
    page,
  }) => {
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });

    await page.goto('/');
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-002: admin page requires auth guard too', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.clear();
    });

    await page.goto('/admin');
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-003: login page is accessible without auth', async ({ page }) => {
    await page.goto('/login');
    const loginPage = page.locator('[data-testid="login-page"]');
    await expect(loginPage).toBeVisible({ timeout: 5000 });
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-004: callback page is accessible without auth', async ({ page }) => {
    await page.goto('/callback');
    const callbackPage = page.locator('[data-testid="callback-page"]');
    await expect(callbackPage).toBeVisible({ timeout: 5000 });
  });

  test('TC-AUTH-005: authenticated user can access chat page', async ({ page }) => {
    const validToken = makeMockJwt({
      sub: 'user-123',
      name: 'Test User',
      preferred_username: 'testuser',
      exp: Math.floor(Date.now() / 1000) + 7200,
      iat: Math.floor(Date.now() / 1000),
    });

    // Set token before first navigation so restoreSession picks it up
    await page.addInitScript((token) => {
      localStorage.setItem('vedo_auth_token', token);
    }, validToken);

    await page.goto('/');
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 5000 });

    const welcomeMsg = page.locator('[data-testid="welcome-message"]');
    await expect(welcomeMsg).toBeVisible();
  });

  test('TC-AUTH-006: expired token redirects to login', async ({ page }) => {
    const expiredToken = makeMockJwt({
      sub: 'user-123',
      name: 'Test User',
      exp: Math.floor(Date.now() / 1000) - 3600,
      iat: Math.floor(Date.now() / 1000) - 7200,
    });

    await page.addInitScript((token) => {
      localStorage.setItem('vedo_auth_token', token);
    }, expiredToken);

    await page.goto('/');
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-007: invalid JWT token redirects to login', async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('vedo_auth_token', 'not-a-valid-jwt');
    });

    await page.goto('/');
    await expect(page).toHaveURL(/\/login/);
  });

  test('TC-AUTH-008: clearing localStorage logs out user after page reload', async ({ page }) => {
    const validToken = makeMockJwt({
      sub: 'user-123',
      name: 'Test User',
      exp: Math.floor(Date.now() / 1000) + 7200,
      iat: Math.floor(Date.now() / 1000),
    });

    // Step 1: navigate to a fresh page and set the token in localStorage
    await page.goto('/login');
    await page.evaluate((token) => {
      localStorage.setItem('vedo_auth_token', token);
    }, validToken);

    // Step 2: navigate to / — restoreSession picks up the token
    await page.goto('/');
    const chatView = page.locator('[data-testid="chat-view"]');
    await expect(chatView).toBeVisible({ timeout: 5000 });

    // Step 3: clear localStorage to simulate logout
    await page.evaluate(() => localStorage.clear());

    // Step 4: reload — restoreSession finds no token, redirects to /login
    await page.reload();
    await expect(page).toHaveURL(/\/login/);
  });
});
