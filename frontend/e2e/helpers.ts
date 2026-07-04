import type { APIRequestContext, Page } from '@playwright/test';

export const API_URL = process.env.E2E_API_URL ?? 'http://localhost:13000';
const KEYCLOAK_URL = process.env.E2E_KEYCLOAK_URL ?? 'http://localhost:18080';
const KEYCLOAK_REALM = process.env.E2E_KEYCLOAK_REALM ?? 'vedo-hub';
const KEYCLOAK_CLIENT_ID = process.env.E2E_KEYCLOAK_CLIENT_ID ?? 'vedo-frontend';
const E2E_USERNAME = process.env.E2E_USERNAME ?? 'admin';
const E2E_PASSWORD = process.env.E2E_PASSWORD ?? process.env.VEDO_ADMIN_PASSWORD ?? 'admin';

let cachedToken: string | null = null;

export interface TestCollection {
  id: string;
  name: string;
  description?: string;
  created_at: string;
  document_count: number;
}

export async function getTestAccessToken(): Promise<string> {
  if (cachedToken) return cachedToken;

  const params = new URLSearchParams({
    grant_type: 'password',
    client_id: KEYCLOAK_CLIENT_ID,
    username: E2E_USERNAME,
    password: E2E_PASSWORD,
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
      `Failed to get KeyCloak test token: ${response.status} ${await response.text()}`,
    );
  }

  const body = (await response.json()) as { access_token?: string };
  if (!body.access_token) {
    throw new Error('KeyCloak token response did not include access_token');
  }

  cachedToken = body.access_token;
  return cachedToken;
}

export async function setupAuth(page: Page): Promise<string> {
  const token = await getTestAccessToken();
  await page.addInitScript((accessToken: string) => {
    localStorage.setItem('vedo_auth_token', accessToken);
  }, token);
  return token;
}

export async function apiRequest<T>(
  request: APIRequestContext,
  method: 'GET' | 'POST' | 'DELETE',
  path: string,
  body?: unknown,
  tokenOverride?: string,
): Promise<T> {
  const token = tokenOverride ?? (await getTestAccessToken());
  const response = await request.fetch(`${API_URL}${path}`, {
    method,
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    data: body,
  });

  if (!response.ok()) {
    throw new Error(`${method} ${path} failed: ${response.status()} ${await response.text()}`);
  }

  return (await response.json()) as T;
}

export async function createTestCollection(
  request: APIRequestContext,
  name = `E2E Collection ${Date.now()}`,
): Promise<TestCollection> {
  return apiRequest<TestCollection>(request, 'POST', '/api/collections', {
    name,
    description: 'Created by Playwright E2E tests',
  });
}

export async function setupAuthAndCollection(
  page: Page,
  request: APIRequestContext,
  name?: string,
): Promise<TestCollection> {
  await setupAuth(page);
  return createTestCollection(request, name);
}

export async function setActiveCollection(page: Page, collectionId: string): Promise<void> {
  await page.evaluate((id) => {
    // biome-ignore lint/suspicious/noExplicitAny: E2E helper needs access to Vue internals
    const app = (document.querySelector('#app') as any).__vue_app__;
    const pinia = app.config.globalProperties.$pinia;
    // Use the store's action instead of raw state to ensure reactivity
    const store = pinia._s.get('collections');
    if (store?.setActiveCollection) {
      store.setActiveCollection(id);
    } else {
      pinia.state.value.collections.activeCollectionId = id;
    }
  }, collectionId);
}

export function fileInput(page: Page) {
  return page.locator('input[type="file"]').first();
}
