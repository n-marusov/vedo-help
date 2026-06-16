/**
 * Auth API client — token management and KeyCloak token exchange.
 *
 * Handles localStorage persistence for access and refresh tokens
 * issued by KeyCloak's OIDC authorization server.
 */

const STORAGE_KEY_TOKEN = 'vedo_auth_token';
const STORAGE_KEY_REFRESH_TOKEN = 'vedo_auth_refresh_token';

// ── Access token ──

export function getStoredToken(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY_TOKEN);
  } catch {
    return null;
  }
}

export function storeAccessToken(token: string): void {
  try {
    localStorage.setItem(STORAGE_KEY_TOKEN, token);
  } catch {
    console.warn('[AuthApi] Failed to store access token in localStorage');
  }
}

export function clearStoredToken(): void {
  try {
    localStorage.removeItem(STORAGE_KEY_TOKEN);
  } catch {
    // Silently ignore
  }
}

// ── Refresh token ──

export function getStoredRefreshToken(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY_REFRESH_TOKEN);
  } catch {
    return null;
  }
}

export function storeRefreshToken(token: string): void {
  try {
    localStorage.setItem(STORAGE_KEY_REFRESH_TOKEN, token);
  } catch {
    console.warn('[AuthApi] Failed to store refresh token in localStorage');
  }
}

export function clearRefreshToken(): void {
  try {
    localStorage.removeItem(STORAGE_KEY_REFRESH_TOKEN);
  } catch {
    // Silently ignore
  }
}

// ── Bulk ──

export function clearAllTokens(): void {
  clearStoredToken();
  clearRefreshToken();
}

// ── Decode JWT payload (without validation) ──

export function decodeToken(token: string): Record<string, unknown> | null {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    const payload = parts[1];
    const decoded = atob(payload.replace(/-/g, '+').replace(/_/g, '/'));
    return JSON.parse(decoded);
  } catch {
    return null;
  }
}

/**
 * Extract user display name and provider from a JWT token's claims.
 *
 * Returns an object with optional `name` and `provider` fields.
 */
export function extractUserClaims(token: string): {
  name?: string;
  provider?: string;
} {
  const decoded = decodeToken(token);
  if (!decoded) return {};
  return {
    name: (decoded.name as string) || (decoded.preferred_username as string) || undefined,
    provider: (decoded.provider as string) || undefined,
  };
}

export function getTokenExpiry(token: string): number {
  const decoded = decodeToken(token);
  if (!decoded) return 0;
  const exp = decoded.exp as number | undefined;
  return exp ? exp * 1000 : 0;
}
