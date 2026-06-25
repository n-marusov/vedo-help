import { expect, test } from '@playwright/test';
import { API_URL, apiRequest, getTestAccessToken } from './helpers';

/**
 * Multi-tenancy E2E Tests
 *
 * These tests verify data isolation between users. They use KeyCloak's
 * password grant to obtain tokens for different test users:
 *   - admin: full access, can see all collections
 *   - alice: regular user, can only see her own collections
 *   - guest: restricted user, can only see her own collections
 *
 * Prerequisites:
 *   - KeyCloak running with vedo-hub realm containing users: admin, alice, guest
 *   - Backend running at E2E_API_URL (default: http://localhost:13000)
 *   - Backend has multi-tenancy migrations applied (user_id columns)
 */

test.describe('Multi-tenancy: data isolation between users', () => {
  test.describe.configure({ mode: 'serial' });

  let adminToken: string;
  let aliceToken: string;
  let guestToken: string;
  const ts = Date.now();

  test.beforeAll(async ({ request }) => {
    // Obtain tokens for all test users
    adminToken = await getTestAccessToken();
    // For alice and guest we need to obtain tokens with different credentials.
    // We call KeyCloak directly (same as getTestAccessToken but with different username/password).
    aliceToken = await getUserToken('alice', process.env.VEDO_ALICE_PASSWORD ?? 'password');
    guestToken = await getUserToken('guest', process.env.VEDO_GUEST_PASSWORD ?? 'guest');
  });

  test('TC-MT-001: alice creates a collection and it is visible to alice only', async ({
    request,
  }) => {
    // Alice creates a collection
    const aliceColl = await apiRequest<{ id: string; name: string }>(
      request,
      'POST',
      '/api/collections',
      { name: `Alice Collection ${ts}` },
      aliceToken,
    );
    expect(aliceColl.id).toBeTruthy();

    // Alice can see her collection
    const aliceCollections = await apiRequest<Array<{ id: string; name: string }>>(
      request,
      'GET',
      '/api/collections',
      undefined,
      aliceToken,
    );
    const foundAlice = aliceCollections.find((c) => c.id === aliceColl.id);
    expect(foundAlice).toBeTruthy();

    // Guest cannot see Alice's collection
    const guestCollections = await apiRequest<Array<{ id: string; name: string }>>(
      request,
      'GET',
      '/api/collections',
      undefined,
      guestToken,
    );
    const foundByGuest = guestCollections.find((c) => c.id === aliceColl.id);
    expect(foundByGuest).toBeFalsy();

    // Cleanup: delete Alice's collection
    const delResp = await request.delete(`${API_URL}/api/collections/${aliceColl.id}`, {
      headers: { Authorization: `Bearer ${aliceToken}` },
    });
    expect(delResp.ok()).toBe(true);
  });

  test('TC-MT-002: admin can see collections created by any user', async ({ request }) => {
    // Alice creates a collection
    const aliceColl = await apiRequest<{ id: string; name: string }>(
      request,
      'POST',
      '/api/collections',
      { name: `Admin Visibility ${ts}` },
      aliceToken,
    );
    expect(aliceColl.id).toBeTruthy();

    // Admin can see Alice's collection
    const adminCollections = await apiRequest<Array<{ id: string; name: string }>>(
      request,
      'GET',
      '/api/collections',
      undefined,
      adminToken,
    );
    const foundByAdmin = adminCollections.find((c) => c.id === aliceColl.id);
    expect(foundByAdmin).toBeTruthy();

    // Cleanup: delete Alice's collection
    const delResp = await request.delete(`${API_URL}/api/collections/${aliceColl.id}`, {
      headers: { Authorization: `Bearer ${aliceToken}` },
    });
    expect(delResp.ok()).toBe(true);
  });

  test("TC-MT-003: user cannot delete another user's collection", async ({ request }) => {
    // Alice creates a collection
    const aliceColl = await apiRequest<{ id: string; name: string }>(
      request,
      'POST',
      '/api/collections',
      { name: `Ownership Test ${ts}` },
      aliceToken,
    );
    expect(aliceColl.id).toBeTruthy();

    // Guest tries to delete Alice's collection — should fail with 404 (info leak prevention)
    const delResp = await request.delete(`${API_URL}/api/collections/${aliceColl.id}`, {
      headers: { Authorization: `Bearer ${guestToken}` },
    });
    expect(delResp.status()).toBe(404);

    // Alice can still delete her own collection
    const aliceDelResp = await request.delete(`${API_URL}/api/collections/${aliceColl.id}`, {
      headers: { Authorization: `Bearer ${aliceToken}` },
    });
    expect(aliceDelResp.ok()).toBe(true);
  });

  test("TC-MT-004: admin can delete any user's collection", async ({ request }) => {
    // Alice creates a collection
    const aliceColl = await apiRequest<{ id: string; name: string }>(
      request,
      'POST',
      '/api/collections',
      { name: `Admin Delete ${ts}` },
      aliceToken,
    );
    expect(aliceColl.id).toBeTruthy();

    // Admin can delete Alice's collection
    const delResp = await request.delete(`${API_URL}/api/collections/${aliceColl.id}`, {
      headers: { Authorization: `Bearer ${adminToken}` },
    });
    expect(delResp.ok()).toBe(true);
  });

  test('TC-MT-005: sessions are isolated per user', async ({ request }) => {
    // Alice creates a session
    const aliceSession = await apiRequest<{ id: string; title: string }>(
      request,
      'POST',
      '/api/sessions',
      { title: `Alice Session ${ts}` },
      aliceToken,
    );
    expect(aliceSession.id).toBeTruthy();

    // Guest cannot see Alice's session
    const guestSessions = await apiRequest<Array<{ id: string; title: string }>>(
      request,
      'GET',
      '/api/sessions',
      undefined,
      guestToken,
    );
    const foundByGuest = guestSessions.find((s) => s.id === aliceSession.id);
    expect(foundByGuest).toBeFalsy();

    // Cleanup: Alice deletes her session
    const aliceDelResp = await request.delete(`${API_URL}/api/sessions/${aliceSession.id}`, {
      headers: { Authorization: `Bearer ${aliceToken}` },
    });
    expect(aliceDelResp.ok()).toBe(true);
  });

  test('TC-MT-006: frontend login page renders correctly for unauthenticated users', async ({
    page,
  }) => {
    await page.goto('/login');
    const loginPage = page.locator('[data-testid="login-page"]');
    await expect(loginPage).toBeVisible({ timeout: 5000 });
  });

  test('TC-MT-007: admin can access admin panel after authenticating with admin role', async ({
    page,
  }) => {
    // Inject admin token
    await page.addInitScript((token: string) => {
      localStorage.setItem('vedo_auth_token', token);
      // Simulate admin role extraction from JWT
      localStorage.setItem('vedo_auth_roles', JSON.stringify(['admin']));
    }, adminToken);

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
  });
});

/**
 * Obtain a KeyCloak access token for a specific user.
 */
async function getUserToken(username: string, password: string): Promise<string> {
  const KEYCLOAK_URL = process.env.E2E_KEYCLOAK_URL ?? 'http://localhost:18080';
  const KEYCLOAK_REALM = process.env.E2E_KEYCLOAK_REALM ?? 'vedo-hub';
  const KEYCLOAK_CLIENT_ID = process.env.E2E_KEYCLOAK_CLIENT_ID ?? 'vedo-frontend';

  const params = new URLSearchParams({
    grant_type: 'password',
    client_id: KEYCLOAK_CLIENT_ID,
    username,
    password,
  });

  const response = await fetch(
    `${KEYCLOAK_URL}/realms/${KEYCLOAK_REALM}/protocol/openid-connect/token`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: params,
    },
  );

  if (!response.ok) {
    throw new Error(
      `Failed to get KeyCloak token for ${username}: ${response.status} ${await response.text()}`,
    );
  }

  const body = (await response.json()) as { access_token?: string };
  if (!body.access_token) {
    throw new Error(`KeyCloak token response did not include access_token for ${username}`);
  }

  return body.access_token;
}
