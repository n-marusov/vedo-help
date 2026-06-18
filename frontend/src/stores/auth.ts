import { getAccessToken, setAccessToken } from "@/api/client";
import { ref } from "vue";

/**
 * Pinia-style auth store (lightweight, no Pinia dependency needed).
 *
 * Tracks authentication state and syncs the access token with the
 * API client. The actual OIDC flow (PKCE, redirect, callback) lives
 * in composables/useOidcAuth.ts; this store only manages the reactive
 * surface consumed by components and router guards.
 */

// ── Reactive state ──

/** Whether a validated auth session exists. */
export const isAuthenticated = ref(!!getAccessToken());

/** The current user display name, derived from the JWT. */
export const userName = ref<string | null>(null);

/** The OIDC provider, currently local Keycloak password auth. */
export const userProvider = ref<string | null>(null);

// ── Actions ──

export function setAuthToken(
	token: string,
	name?: string,
	provider?: string,
): void {
	setAccessToken(token);
	isAuthenticated.value = true;
	if (name !== undefined) userName.value = name;
	if (provider !== undefined) userProvider.value = provider;
}

export function clearAuth(): void {
	setAccessToken("");
	isAuthenticated.value = false;
	userName.value = null;
	userProvider.value = null;
}
