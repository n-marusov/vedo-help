import { expect, test } from '@playwright/test';

test.describe('Chat session switching controls', () => {
  test('keeps sidebar controls hit-testable after selecting a session', async ({ page }) => {
    const consoleErrors: { text: string; location?: string }[] = [];
    const pageErrors: string[] = [];
    page.on('console', (message) => {
      if (message.type() === 'error') {
        consoleErrors.push({
          text: message.text(),
          location: message.location()?.url || '',
        });
      }
    });
    page.on('pageerror', (err) => pageErrors.push(`${err.message}\n${err.stack || ''}`));

    const collection = {
      id: 'collection-1',
      name: 'Technical Docs',
      created_at: '2026-06-23T00:00:00Z',
      document_count: 2,
    };
    const sessions = [
      {
        id: 'sess-1',
        title: 'Selected Session',
        collection_id: collection.id,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
        message_count: 2,
      },
      {
        id: 'sess-2',
        title: 'Other Session',
        collection_id: collection.id,
        created_at: '2026-06-23T00:00:00Z',
        updated_at: '2026-06-23T00:00:00Z',
        message_count: 1,
      },
    ];

    await page.addInitScript(() => {
      const encode = (value: unknown) =>
        btoa(JSON.stringify(value)).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
      const token = [
        encode({ alg: 'none', typ: 'JWT' }),
        encode({
          exp: Math.floor(Date.now() / 1000) + 3600,
          preferred_username: 'tester',
          realm_access: { roles: ['admin'] },
        }),
        'signature',
      ].join('.');
      localStorage.setItem('vedo_auth_token', token);
    });

    await page.route('**/api/auth/me', async (route) => {
      await route.fulfill({
        contentType: 'application/json',
        json: { id: 'user-1', username: 'tester', roles: ['admin'] },
      });
    });
    await page.route('**/api/collections', async (route) => {
      await route.fulfill({
        contentType: 'application/json',
        json: [collection],
      });
    });
    await page.route('**/api/sessions/sess-1', async (route) => {
      await route.fulfill({
        contentType: 'application/json',
        json: {
          session: sessions[0],
          messages: [
            {
              id: 'msg-1',
              session_id: 'sess-1',
              role: 'user',
              content: 'Hello',
              sources: 'null',
              created_at: '2026-06-23T00:00:00Z',
            },
            {
              id: 'msg-2',
              session_id: 'sess-1',
              role: 'assistant',
              content: 'Hi there!',
              sources: 'null',
              created_at: '2026-06-23T00:00:00Z',
            },
          ],
        },
      });
    });
    await page.route('**/api/sessions', async (route) => {
      const url = new URL(route.request().url());
      if (url.pathname !== '/api/sessions') {
        await route.fallback();
        return;
      }
      await route.fulfill({ contentType: 'application/json', json: sessions });
    });

    await page.setViewportSize({ width: 1200, height: 800 });
    await page.goto('/');

    // Wait for the app to fully load
    await expect(page.locator('[data-testid="session-sidebar"]')).toBeVisible();
    await expect(page.locator('[data-testid="session-item"]')).toHaveCount(2);
    await page.evaluate(() => {
      window.dispatchEvent(new Event('resize'));
    });

    // Print any console errors before session selection
    if (consoleErrors.length > 0) {
      console.log('CONSOLE ERRORS BEFORE SELECTION:', JSON.stringify(consoleErrors, null, 2));
    }
    if (pageErrors.length > 0) {
      console.log('PAGE ERRORS BEFORE SELECTION:', JSON.stringify(pageErrors, null, 2));
    }

    // --- Step 1: Select a session ---
    await page.locator('[data-testid="session-item"]').first().click();
    await expect(page.locator('[data-testid="toolbar-session-badge"]')).toContainText(
      'Selected Session',
    );

    await expect(page.locator('[data-testid="sidebar-overlay"]')).toHaveCount(0);

    // Log errors after session selection
    if (consoleErrors.length > 0) {
      console.log('CONSOLE ERRORS AFTER SELECTION:', JSON.stringify(consoleErrors, null, 2));
    }
    if (pageErrors.length > 0) {
      console.log('PAGE ERRORS AFTER SELECTION:', JSON.stringify(pageErrors, null, 2));
    }

    // Print full error details before asserting
    for (const err of pageErrors) {
      console.log('--- PAGE ERROR DETAIL ---');
      console.log(err);
    }

    // --- Step 2: Verify no JS errors occurred during session switching ---
    expect(pageErrors.length).toBe(0);

    // --- Step 3: Test sidebar collapse button ---
    const collapseButton = page.locator('[data-testid="sidebar-collapse-btn"]');
    await expect(collapseButton).toBeVisible();
    await expect(collapseButton).toBeEnabled();
    await collapseButton.click();
    await page.waitForTimeout(200);

    const collapsedState = await page.evaluate(() => {
      const app = document.querySelector('#app').__vue_app__;
      return app.config.globalProperties.$pinia.state.value.chat.sidebarCollapsed;
    });
    expect(
      collapsedState,
      'collapse click must update Pinia state (proves control responds to click)',
    ).toBe(true);

    // Expand sidebar back
    await page.locator('[data-testid="sidebar-expand-btn"]').click();
    await page.waitForTimeout(200);

    // --- Step 4: Test new session button ---
    const newSessionButton = page.locator('[data-testid="btn-new-chat"]');
    await expect(newSessionButton).toBeVisible();
    await expect(newSessionButton).toBeEnabled();
    await newSessionButton.click();
    // Dropdown uses Teleport to body; wait for it to appear
    await expect(page.locator('[data-testid="new-session-collection-dropdown"]')).toBeVisible({
      timeout: 3000,
    });

    // --- Step 5: Verify console is still clean ---
    expect(consoleErrors.length).toBe(0);

    // --- Step 6: Test sidebar search toggle ---
    await page.locator('[data-testid="session-search-toggle"]').click();
    await expect(page.locator('[data-testid="confirm-dialog"]')).toBeVisible({
      timeout: 2000,
    });
    await page.locator('[data-testid="btn-dialog-close"]').click();
    await expect(page.locator('[data-testid="confirm-dialog"]')).not.toBeVisible();
  });
});
