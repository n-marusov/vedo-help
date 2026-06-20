/**
 * Shared test helpers for E2E tests.
 *
 * Reusable utilities: mock JWT creation, auth setup, API mocking.
 */

/**
 * Build a mock JWT with the given payload claims.
 */
export function makeMockJwt(claims: Record<string, unknown>): string {
  const header = btoa(JSON.stringify({ alg: 'RS256', typ: 'JWT' }));
  const payload = btoa(JSON.stringify(claims));
  return `${header}.${payload}.mocksignature`;
}

export const VALID_TOKEN = makeMockJwt({
  sub: 'user-123',
  name: 'Test User',
  preferred_username: 'testuser',
  exp: Math.floor(Date.now() / 1000) + 7200,
  iat: Math.floor(Date.now() / 1000),
});

/**
 * Inject a valid JWT into localStorage via addInitScript.
 * Call before any page navigation to bypass the auth redirect.
 */
export async function setupAuth(page: import('@playwright/test').Page) {
  await page.addInitScript((token: string) => {
    localStorage.setItem('vedo_auth_token', token);
  }, VALID_TOKEN);
}

/**
 * Mock GET /api/collections to return a single test collection.
 */
export async function mockCollections(page: import('@playwright/test').Page) {
  await page.route('**/api/collections', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          id: 'col-1',
          name: 'Test Collection',
          description: 'A test collection',
          created_at: new Date().toISOString(),
          document_count: 2,
        },
      ]),
    });
  });
}

/**
 * Mock GET /api/sessions to return a single test session.
 */
export async function mockSessions(page: import('@playwright/test').Page) {
  await page.route('**/api/sessions', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          id: 'sess-1',
          title: 'Test Session',
          message_count: 0,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
      ]),
    });
  });
}
