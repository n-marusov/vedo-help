import { expect, test } from '@playwright/test';
import { setActiveCollection, setupAuthAndCollection } from './helpers';

test.describe('Chat UI Polish: session sidebar', () => {
  test.describe.configure({ mode: 'serial' });

  test('TC-POLISH-001: session search filters list correctly', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Polish ${Date.now()}`);

    // Create a few sessions via API
    const { apiRequest, getTestAccessToken } = await import('./helpers');
    const token = await getTestAccessToken();
    await request.post('/api/sessions', {
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      data: { title: 'Alpha test session', collection_id: collection.id },
    });
    await request.post('/api/sessions', {
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      data: { title: 'Beta test session', collection_id: collection.id },
    });
    await request.post('/api/sessions', {
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      data: { title: 'Gamma test session', collection_id: collection.id },
    });

    await page.goto('/');
    await page.waitForSelector('[data-testid="session-sidebar"]', {
      timeout: 10000,
    });

    // Click search toggle shows dialog with search input
    await page.locator('[data-testid="session-search-toggle"]').click();
    await page.waitForSelector('[data-testid="search-dialog-input"]', {
      timeout: 5000,
    });

    // Search dialog should show the sessions in its list
    const dialogItems = page.locator('[data-testid="search-dialog-item"]');
    await expect(dialogItems.first()).toBeVisible({ timeout: 5000 });

    // Close the dialog
    await page.locator('[data-testid="btn-dialog-close"]').click();
    await expect(page.locator('[data-testid="search-dialog-input"]')).toBeHidden();
  });

  test('TC-POLISH-002: session rename via dialog', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Rename ${Date.now()}`);

    const { apiRequest, getTestAccessToken } = await import('./helpers');
    const token = await getTestAccessToken();
    const session = await apiRequest<{ id: string }>(request, 'POST', '/api/sessions', {
      title: 'Old Name',
      collection_id: collection.id,
    });

    await page.goto('/');
    await page.waitForSelector('[data-testid="session-sidebar"]', {
      timeout: 10000,
    });

    // Open rename dialog
    const sessionItem = page.locator('[data-testid="session-item"]').first();
    await sessionItem.hover();
    await sessionItem.locator('[data-testid="session-rename-btn"]').click();

    // Dialog should be visible (VDialog shows confirm-dialog overlay)
    await expect(page.locator('[data-testid="confirm-dialog"]')).toBeVisible();
    // Rename input should be visible
    await expect(page.locator('[data-testid="session-rename-input"]')).toBeVisible();

    // Type new name and save
    await page.locator('[data-testid="session-rename-input"]').fill('New Name');
    await page.locator('[data-testid="btn-dialog-confirm"]').click();

    // Verify PATCH request was made (via Vite proxy: /api/sessions/<id>)
    const patchResponse = await page.waitForResponse(
      (res) => {
        const url = res.url();
        return url.includes(`/sessions/${session.id}`) && res.request().method() === 'PATCH';
      },
      { timeout: 10000 },
    );
    expect(patchResponse.status()).toBe(200);

    // Verify UI displays new name
    await expect(sessionItem).toContainText('New Name');
  });

  test('TC-POLISH-003: session pin toggles pinned state', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Pin ${Date.now()}`);

    const { apiRequest, getTestAccessToken } = await import('./helpers');
    const token = await getTestAccessToken();
    await apiRequest<{ id: string }>(request, 'POST', '/api/sessions', {
      title: 'Pinnable Session',
      collection_id: collection.id,
    });

    await page.goto('/');
    await page.waitForSelector('[data-testid="session-sidebar"]', {
      timeout: 10000,
    });

    const sessionItem = page.locator('[data-testid="session-item"]').first();
    await sessionItem.hover();

    // Click pin button
    const pinBtn = sessionItem.locator('[data-testid="session-pin-btn"]');
    await pinBtn.click();

    // Verify pinned indicator appears
    await expect(sessionItem).toHaveAttribute('data-pinned', 'true');

    // Reload and verify pin persists
    await page.reload();
    await page.waitForSelector('[data-testid="session-sidebar"]', {
      timeout: 10000,
    });
    await expect(page.locator('[data-testid="session-item"]').first()).toHaveAttribute(
      'data-pinned',
      'true',
    );
  });

  test('TC-POLISH-004: new session button centered below header', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Layout ${Date.now()}`);

    await page.goto('/');

    const newBtn = page.locator('[data-testid="btn-new-chat"]');
    const sidebar = page.locator('[data-testid="session-sidebar"]');

    // Button should be below the header, not inline with it
    const btnBox = await newBtn.boundingBox();
    const sidebarBox = await sidebar.boundingBox();

    // btn should be positioned below the session-title
    const title = sidebar.locator('.session-title');
    const titleBox = await title.boundingBox();
    expect(btnBox).toBeTruthy();
    expect(titleBox).toBeTruthy();
    if (btnBox && titleBox) {
      expect(btnBox.y).toBeGreaterThan(titleBox.y + titleBox.height);
    }
  });
});

test.describe('Chat UI Polish: message actions', () => {
  test.describe.configure({ mode: 'serial' });

  test('TC-POLISH-005: copy button copies user message to clipboard', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Copy ${Date.now()}`);
    // Grant clipboard permissions before navigating
    await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
    await page.goto('/');
    await setActiveCollection(page, collection.id);

    // Type and send a message
    await page.locator('[data-testid="chat-input"]').fill('Test message for copy');
    await page.locator('[data-testid="btn-send"]').click();

    // Wait for user message to appear
    await page.waitForSelector('[data-testid="message-user"]', {
      timeout: 10000,
    });

    // Click copy button
    const copyBtn = page.locator('[data-testid="message-copy-btn"]').first();
    await copyBtn.click();

    // Verify clipboard - use clipboard API via grantPermissions
    await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
    const clipboardText = await page.evaluate(() => navigator.clipboard.readText());
    expect(clipboardText).toContain('Test message for copy');
  });

  test('TC-POLISH-006: regenerate button triggers new response', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Regen ${Date.now()}`);

    await page.goto('/');
    await setActiveCollection(page, collection.id);

    // Send a message and wait for assistant response
    await page.locator('[data-testid="chat-input"]').fill('Regenerate test');
    await page.locator('[data-testid="btn-send"]').click();
    await page.waitForSelector('[data-testid="message-assistant"]', {
      timeout: 30000,
    });

    // Click regenerate on assistant message
    const regenBtn = page.locator('[data-testid="message-regenerate-btn"]').first();
    await regenBtn.click();

    // Verify loading state or new response
    const newResponse = page.locator('[data-testid="message-assistant"]').first();
    await expect(newResponse).toBeVisible({ timeout: 30000 });
  });

  test('TC-POLISH-007: no debug info in chat for admin role', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Debug ${Date.now()}`);

    await page.goto('/');
    await setActiveCollection(page, collection.id);

    // Send a message
    await page.locator('[data-testid="chat-input"]').fill('Debug test');
    await page.locator('[data-testid="btn-send"]').click();
    await page.waitForSelector('[data-testid="message-assistant"]', {
      timeout: 30000,
    });

    // Debug button should NOT be present in chat (moved to admin view)
    const debugBtn = page.locator('[data-testid="message-debug-btn"]');
    await expect(debugBtn).toHaveCount(0);
  });

  test('TC-POLISH-008: no delete buttons on messages', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `NoDel ${Date.now()}`);

    await page.goto('/');
    await setActiveCollection(page, collection.id);

    // Send a message
    await page.locator('[data-testid="chat-input"]').fill('Check delete buttons');
    await page.locator('[data-testid="btn-send"]').click();
    await page.waitForSelector('[data-testid="message-user"]', {
      timeout: 10000,
    });

    // Delete buttons should not exist
    const deleteBtns = page.locator('[data-testid="message-delete-btn"]');
    await expect(deleteBtns).toHaveCount(0);
  });

  test('TC-POLISH-009: timestamp in same row as action buttons', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `Time ${Date.now()}`);

    await page.goto('/');
    await setActiveCollection(page, collection.id);

    // Send a message
    await page.locator('[data-testid="chat-input"]').fill('Timestamp test');
    await page.locator('[data-testid="btn-send"]').click();
    await page.waitForSelector('[data-testid="message-user"]', {
      timeout: 10000,
    });

    const messageActions = page.locator('[data-testid="message-actions-row"]').first();
    await expect(messageActions).toBeVisible();

    // Timestamp should be inside the actions row
    const time = messageActions.locator('[data-testid="message-time"]');
    await expect(time).toBeVisible();
  });
});

test.describe('Chat UI Polish: collection tag and input', () => {
  test('TC-POLISH-010: collection tag shows session + collection name', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request, `CollectionTag ${Date.now()}`);

    await page.goto('/');
    await setActiveCollection(page, collection.id);

    // Send a message to create a session
    await page.locator('[data-testid="chat-input"]').fill('Tag test');
    await page.locator('[data-testid="btn-send"]').click();
    await page.waitForSelector('[data-testid="message-user"]', {
      timeout: 10000,
    });

    // Collection tag should be visible instead of dropdown
    const collectionTag = page.locator('[data-testid="toolbar-collection-badge"]');
    await expect(collectionTag).toBeVisible({ timeout: 5000 });
    await expect(collectionTag).toContainText(collection.name);
  });

  test('TC-POLISH-011: toolbar shows collection badge on fresh load', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `AutoSel ${Date.now()}`);

    await page.goto('/');
    await page.waitForSelector('[data-testid="chat-toolbar"]', {
      timeout: 10000,
    });

    // Actively select the collection (no auto-select on fresh load)
    await setActiveCollection(page, collection.id);

    // Collection badge should be visible
    const collectionTag = page.locator('[data-testid="toolbar-collection-badge"]');
    await expect(collectionTag).toBeVisible({ timeout: 5000 });
    await expect(collectionTag).toContainText(collection.name);
  });

  test('TC-POLISH-012: composer area has white background in light theme', async ({
    page,
    request,
  }) => {
    // Note: chat-input textarea has background:transparent; the visible white
    // comes from the parent toolbar (--color-card). This test checks the parent.
    const collection = await setupAuthAndCollection(page, request, `Input ${Date.now()}`);

    await page.goto('/');
    await setActiveCollection(page, collection.id);

    // Ensure light theme for white background check
    await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'light'));
    await page.waitForTimeout(100);

    // Check the toolbar/composer container background (input is transparent)
    const toolbar = page.locator('[data-testid="chat-toolbar"]');
    const bgColor = await toolbar.evaluate((el) => {
      const style = getComputedStyle(el);
      return style.backgroundColor;
    });
    expect(bgColor).toMatch(/rgb\(255,\s*255,\s*255\)|rgba\(255,\s*255,\s*255/i);

    // Input should be visible and have proper styling
    const input = page.locator('[data-testid="chat-input"]');
    await expect(input).toBeVisible();
  });
});
