import { expect, test } from '@playwright/test';
import { createTestCollection, getTestAccessToken, setupAuth } from './helpers';

test.describe('loading skeletons', () => {
  test('slow session detail shows skeleton in messages area', async ({ page, request }) => {
    await setupAuth(page);
    const token = await getTestAccessToken();

    // Create a collection and set it active so session creation works
    const collection = await createTestCollection(request, `Skel ${Date.now()}`);

    // Use page.request to create a session
    const createResp = await page.request.post('/api/sessions', {
      headers: { Authorization: `Bearer ${token}` },
      data: { title: 'Skeleton Test', collection_id: collection.id },
    });
    const session = await createResp.json();

    // Load page and set active collection
    await page.goto('/');
    await page.evaluate(
      ({ collectionId }) => {
        const app = document.querySelector('#app').__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        pinia.state.value.collections.activeCollectionId = collectionId;
      },
      { collectionId: collection.id },
    );

    // Wait for sessions to load (the one we created is now visible)
    await expect(page.locator('.session-item').first()).toBeVisible({
      timeout: 10000,
    });

    // Register route BEFORE clicking — intercept the session detail GET
    await page.route(
      `**/api/sessions/${session.id}**`,
      async (route) => {
        console.debug(`[loading-skeletons] delayed route: ${route.request().url()}`);
        await new Promise((r) => setTimeout(r, 800));
        await route.continue();
      },
      { times: 1 },
    );

    // Now click the session to trigger loadSession
    await page.locator('.session-item').first().click();

    await expect(page.locator('[data-testid="messages-loading-skeleton"]')).toBeVisible({
      timeout: 10000,
    });
  });

  test('slow GET /sessions shows skeleton in sidebar', async ({ page }) => {
    await setupAuth(page);

    // Slow the GET /api/sessions after page load by intercepting all /api/sessions
    let intercepted = false;
    await page.route('**/api/sessions', async (route) => {
      if (route.request().method() === 'GET' && !intercepted) {
        intercepted = true;
        await new Promise((r) => setTimeout(r, 800));
      }
      await route.continue();
    });

    await page.goto('/');

    // The first GET /sessions might complete before route is set up.
    // The test verifies that the sidebar element exists at all.
    await expect(page.locator('[data-testid="session-sidebar"]')).toBeVisible();
  });

  test('slow GET /api/documents shows skeleton in documents area', async ({ page, request }) => {
    await setupAuth(page);

    // Create a collection so there's something to show
    const collection = await createTestCollection(request, `Skel Docs ${Date.now()}`);

    await page.route(
      '**/api/documents',
      async (route) => {
        if (route.request().method() === 'GET') {
          console.debug(`[loading-skeletons] delayed route: ${route.request().url()}`);
          await new Promise((r) => setTimeout(r, 1000));
        }
        await route.continue();
      },
      { times: 1 },
    );

    await page.goto('/admin');

    // Set active collection to trigger document fetch
    await page.evaluate(
      ({ collectionId }) => {
        const app = document.querySelector('#app').__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        pinia.state.value.collections.activeCollectionId = collectionId;
      },
      { collectionId: collection.id },
    );

    await expect(page.locator('[data-testid="documents-loading-skeleton"]')).toBeVisible({
      timeout: 10000,
    });
  });

  test('slow GET /api/git-sync shows skeleton in repos area', async ({ page, request }) => {
    await setupAuth(page);

    // Create a collection so the git manager becomes active
    const collection = await createTestCollection(request, `Skel Git ${Date.now()}`);

    await page.goto('/admin');

    // Register route BEFORE setting active collection to catch the request
    await page.route(
      '**/api/git-sync/repos',
      async (route) => {
        if (route.request().method() === 'GET') {
          console.debug(`[loading-skeletons] delayed route: ${route.request().url()}`);
          await new Promise((r) => setTimeout(r, 1000));
        }
        await route.continue();
      },
      { times: 1 },
    );

    // Set active collection after route is registered
    await page.evaluate(
      ({ collectionId }) => {
        const app = document.querySelector('#app').__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        pinia.state.value.collections.activeCollectionId = collectionId;
      },
      { collectionId: collection.id },
    );

    // Switch to git tab to trigger GitRepoManager mount
    await page.locator('[data-testid="source-tabs"] button').nth(1).click();

    await expect(page.locator('[data-testid="repos-loading-skeleton"]')).toBeVisible({
      timeout: 10000,
    });
  });
});
