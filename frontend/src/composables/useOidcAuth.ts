/**
 * KeyCloak OIDC Auth composable with PKCE flow and silent token refresh.
 *
 * Provides OAuth 2.0 Authorization Code + PKCE for the vedo-frontend public
 * client. Stores the resulting access token in localStorage, refreshes it
 * proactively before expiry, and sets it on the API client for subsequent
 * authenticated requests.
 *
 * Flow:
 * 1. LoginView calls `redirectToKeycloak()` → user goes to KeyCloak
 * 2. KeyCloak redirects to `/callback?code=...&state=...`
 * 3. CallbackView calls `handleCallback()` → verifies state, exchanges code for tokens via PKCE
 * 4. Access + refresh tokens are stored; access token set on API client
 * 5. User is redirected to the chat view
 * 6. On app startup, `restoreSession()` loads the token and schedules a background refresh
 */

import { getAccessToken, setAccessToken } from "@/api/client";
import { setAuthToken } from "@/stores/auth";
import { ref } from "vue";

// ── Configuration ──

const KEYCLOAK_BASE =
	import.meta.env.VITE_KEYCLOAK_BASE_URL ?? "http://localhost:8080";
const REALM = "vedo-hub";
const CLIENT_ID = "vedo-frontend";
const REDIRECT_URI = `${window.location.origin}/callback`;
const STORAGE_KEY_TOKEN = "vedo_auth_token";
const STORAGE_KEY_REFRESH_TOKEN = "vedo_auth_refresh_token";
const STORAGE_KEY_VERIFIER = "vedo_pkce_verifier";
const STORAGE_KEY_STATE = "vedo_pkce_state";

/**
 * Margin in milliseconds before actual expiry to attempt a proactive refresh.
 * If the token has fewer than this many milliseconds left, refresh on restore.
 * Also used to schedule the background refresh timer.
 */
const REFRESH_MARGIN_MS = 60_000;

/**
 * Minimum interval in milliseconds between refresh attempts.
 * Prevents a tight error-retry loop if the refresh endpoint is repeatedly failing.
 */
const MIN_REFRESH_INTERVAL_MS = 30_000;

// ── Reactive auth state ──

const isAuthenticated = ref(!!getAccessToken());

// ── PKCE helpers ──

/**
 * Generate a cryptographically random code verifier (RFC 7636 §4.1).
 * 32 bytes → 43 base64url characters (without padding).
 */
function generateCodeVerifier(): string {
	const array = new Uint8Array(32);
	crypto.getRandomValues(array);
	return base64UrlEncode(array);
}

/**
 * Generate a SHA-256 code challenge from the verifier (RFC 7636 §4.2).
 */
async function generateCodeChallenge(verifier: string): Promise<string> {
	const encoder = new TextEncoder();
	const digest = await crypto.subtle.digest(
		"SHA-256",
		encoder.encode(verifier),
	);
	return base64UrlEncode(new Uint8Array(digest));
}

/**
 * Generate a random state value (16 bytes → 22 base64url chars) for CSRF protection.
 */
function generateState(): string {
	const array = new Uint8Array(16);
	crypto.getRandomValues(array);
	return base64UrlEncode(array);
}

/** Base64url-encode a byte array (no padding). */
function base64UrlEncode(bytes: Uint8Array): string {
	return btoa(String.fromCharCode(...bytes))
		.replace(/\+/g, "-")
		.replace(/\//g, "_")
		.replace(/=+$/, "");
}

// ── Verifier storage (sessionStorage — ephemeral) ──

function storeVerifier(verifier: string): void {
	try {
		sessionStorage.setItem(STORAGE_KEY_VERIFIER, verifier);
	} catch {
		console.warn("[OidcAuth] Failed to store PKCE verifier in sessionStorage");
	}
}

function getVerifier(): string | null {
	try {
		return sessionStorage.getItem(STORAGE_KEY_VERIFIER);
	} catch {
		return null;
	}
}

function clearVerifier(): void {
	try {
		sessionStorage.removeItem(STORAGE_KEY_VERIFIER);
	} catch {
		// Silently ignore
	}
}

// ── State storage (sessionStorage — ephemeral) ──

function storeState(state: string): void {
	try {
		sessionStorage.setItem(STORAGE_KEY_STATE, state);
	} catch {
		console.warn("[OidcAuth] Failed to store OAuth state in sessionStorage");
	}
}

function getState(): string | null {
	try {
		return sessionStorage.getItem(STORAGE_KEY_STATE);
	} catch {
		return null;
	}
}

function clearState(): void {
	try {
		sessionStorage.removeItem(STORAGE_KEY_STATE);
	} catch {
		// Silently ignore
	}
}

// ── Token storage (localStorage — persistent) ──

function storeAccessToken(token: string): void {
	try {
		localStorage.setItem(STORAGE_KEY_TOKEN, token);
	} catch {
		console.warn("[OidcAuth] Failed to store access token in localStorage");
	}
}

function getStoredToken(): string | null {
	try {
		return localStorage.getItem(STORAGE_KEY_TOKEN);
	} catch {
		return null;
	}
}

function clearStoredToken(): void {
	try {
		localStorage.removeItem(STORAGE_KEY_TOKEN);
	} catch {
		// Silently ignore
	}
}

function storeRefreshToken(token: string): void {
	try {
		localStorage.setItem(STORAGE_KEY_REFRESH_TOKEN, token);
	} catch {
		console.warn("[OidcAuth] Failed to store refresh token in localStorage");
	}
}

function getStoredRefreshToken(): string | null {
	try {
		return localStorage.getItem(STORAGE_KEY_REFRESH_TOKEN);
	} catch {
		return null;
	}
}

function clearRefreshToken(): void {
	try {
		localStorage.removeItem(STORAGE_KEY_REFRESH_TOKEN);
	} catch {
		// Silently ignore
	}
}

function clearAllTokens(): void {
	clearStoredToken();
	clearRefreshToken();
}

// ── Token expiry helpers ──

/**
 * Decode the JWT payload (without validation) to extract claims.
 * Validation is done by the backend against KeyCloak's JWKS endpoint.
 */
export function decodeToken(token: string): Record<string, unknown> | null {
	try {
		const parts = token.split(".");
		if (parts.length !== 3) return null;
		const payload = parts[1];
		const decoded = atob(payload.replace(/-/g, "+").replace(/_/g, "/"));
		return JSON.parse(decoded);
	} catch {
		return null;
	}
}

/** Return the `exp` claim (in ms since epoch) or 0 if unreadable. */
function getTokenExpiry(token: string): number {
	const decoded = decodeToken(token);
	if (!decoded) return 0;
	const exp = decoded.exp as number | undefined;
	return exp ? exp * 1000 : 0;
}

/**
 * Check if the token is expired or will expire within the given margin.
 * Includes a ttl check so already-expired tokens are caught too.
 */
function isTokenExpired(token: string, marginMs = 0): boolean {
	const expMs = getTokenExpiry(token);
	if (expMs === 0) return true;
	return Date.now() + marginMs >= expMs;
}

/**
 * Return how many ms until the token expires (negative = already expired).
 */
function msUntilExpiry(token: string): number {
	const expMs = getTokenExpiry(token);
	if (expMs === 0) return -1;
	return expMs - Date.now();
}

// ── Refresh state ──

/** Timestamp of the last refresh attempt; used to throttle retries. */
let lastRefreshAttempt = 0;
/** Handle for the proactive refresh timer so it can be cancelled. */
let refreshTimerHandle: ReturnType<typeof setTimeout> | null = null;

// ── Public OIDC actions ──

/**
 * Redirect the browser to KeyCloak's authorization endpoint with PKCE + state.
 */
export async function redirectToKeycloak(): Promise<void> {
	const verifier = generateCodeVerifier();
	const challenge = await generateCodeChallenge(verifier);
	const state = generateState();

	storeVerifier(verifier);
	storeState(state);

	const params = new URLSearchParams({
		client_id: CLIENT_ID,
		redirect_uri: REDIRECT_URI,
		response_type: "code",
		scope: "openid profile email",
		prompt: "login",
		code_challenge: challenge,
		code_challenge_method: "S256",
		state,
	});

	const authUrl = `${KEYCLOAK_BASE}/realms/${REALM}/protocol/openid-connect/auth?${params.toString()}`;

	console.debug("[OidcAuth] Redirecting to KeyCloak authorization endpoint");
	window.location.href = authUrl;
}

/**
 * Exchange an authorization code for tokens using PKCE.
 *
 * Called from CallbackView after KeyCloak redirects back with a `code` + `state`.
 * Verifies `state` matches the stored value for CSRF protection.
 *
 * @returns The access token on success.
 * @throws If the code exchange or state verification fails.
 */
export async function handleCallback(): Promise<string> {
	const url = new URL(window.location.href);
	const code = url.searchParams.get("code");
	const returnedState = url.searchParams.get("state");
	const errorParam = url.searchParams.get("error");
	const errorDescription = url.searchParams.get("error_description");

	if (errorParam) {
		const msg = errorDescription || `OAuth error: ${errorParam}`;
		console.error("[OidcAuth] Authorization error:", msg);
		throw new Error(msg);
	}

	if (!code) {
		throw new Error("No authorization code found in the callback URL");
	}

	// Verify state — CSRF protection (defense-in-depth alongside PKCE)
	const expectedState = getState();
	clearState();

	if (!returnedState || !expectedState || returnedState !== expectedState) {
		console.error("[OidcAuth] OAuth state mismatch — possible CSRF attack");
		throw new Error("OAuth state mismatch. Please start the login flow again.");
	}

	const verifier = getVerifier();
	if (!verifier) {
		throw new Error(
			"PKCE code verifier not found. Please start the login flow again.",
		);
	}

	clearVerifier();

	console.debug("[OidcAuth] Exchanging authorization code for tokens");

	const tokenResponse = await fetch(
		`${KEYCLOAK_BASE}/realms/${REALM}/protocol/openid-connect/token`,
		{
			method: "POST",
			headers: {
				"Content-Type": "application/x-www-form-urlencoded",
			},
			body: new URLSearchParams({
				grant_type: "authorization_code",
				code,
				redirect_uri: REDIRECT_URI,
				client_id: CLIENT_ID,
				code_verifier: verifier,
			}),
		},
	);

	if (!tokenResponse.ok) {
		const errorBody = await tokenResponse.text().catch(() => "Unknown error");
		console.error(
			"[OidcAuth] Token exchange failed:",
			tokenResponse.status,
			errorBody,
		);
		throw new Error(
			`Token exchange failed: ${tokenResponse.status} ${errorBody}`,
		);
	}

	const tokens = await tokenResponse.json();
	const accessToken = tokens.access_token as string;

	if (!accessToken) {
		throw new Error("No access token in the token response");
	}

	console.debug("[OidcAuth] Token exchange succeeded");

	// Store tokens and set on API client
	storeAccessToken(accessToken);
	if (tokens.refresh_token) {
		storeRefreshToken(tokens.refresh_token as string);
	}

	// Extract user info from the JWT for the auth store
	const decoded = decodeToken(accessToken);
	const name = (decoded?.name ?? decoded?.preferred_username ?? "") as string;
	const provider = (decoded?.identity_provider ?? "") as string;

	setAuthToken(accessToken, name || undefined, provider || undefined);
	isAuthenticated.value = true;

	// Schedule proactive token refresh
	scheduleTokenRefresh(accessToken);

	return accessToken;
}

/**
 * Attempt to refresh the access token using the stored refresh token.
 *
 * On success, updates the stored access + refresh tokens, calls `setAccessToken`
 * and schedules the next proactive refresh.
 *
 * @returns The new access token, or `null` if refresh failed.
 */
async function refreshAccessToken(): Promise<string | null> {
	const refreshToken = getStoredRefreshToken();
	if (!refreshToken) {
		console.debug("[OidcAuth] No refresh token available, skipping refresh");
		return null;
	}

	// Throttle: don't retry more often than MIN_REFRESH_INTERVAL_MS
	const now = Date.now();
	if (now - lastRefreshAttempt < MIN_REFRESH_INTERVAL_MS) {
		console.debug("[OidcAuth] Refresh throttled, skipping");
		return null;
	}
	lastRefreshAttempt = now;

	console.debug("[OidcAuth] Attempting token refresh");

	try {
		const response = await fetch(
			`${KEYCLOAK_BASE}/realms/${REALM}/protocol/openid-connect/token`,
			{
				method: "POST",
				headers: {
					"Content-Type": "application/x-www-form-urlencoded",
				},
				body: new URLSearchParams({
					grant_type: "refresh_token",
					refresh_token: refreshToken,
					client_id: CLIENT_ID,
				}),
			},
		);

		if (!response.ok) {
			const errorBody = await response.text().catch(() => "Unknown error");
			console.warn(
				"[OidcAuth] Token refresh failed:",
				response.status,
				errorBody,
			);

			// If the refresh token is invalid/expired (400), clear everything
			if (response.status === 400) {
				console.debug("[OidcAuth] Refresh token rejected, clearing session");
				clearAllTokens();
				setAccessToken("");
				isAuthenticated.value = false;
			}
			return null;
		}

		const tokens = await response.json();
		const newAccessToken = tokens.access_token as string;

		if (!newAccessToken) {
			console.warn("[OidcAuth] Refresh response missing access_token");
			return null;
		}

		console.debug("[OidcAuth] Token refresh succeeded");

		storeAccessToken(newAccessToken);
		if (tokens.refresh_token) {
			storeRefreshToken(tokens.refresh_token as string);
		}

		// Extract user info from the refreshed JWT
		const decoded = decodeToken(newAccessToken);
		const name = (decoded?.name ?? decoded?.preferred_username ?? "") as string;
		const provider = (decoded?.identity_provider ?? "") as string;
		setAuthToken(newAccessToken, name || undefined, provider || undefined);

		// Schedule the next refresh
		scheduleTokenRefresh(newAccessToken);

		return newAccessToken;
	} catch (err) {
		console.error("[OidcAuth] Token refresh network error:", err);
		return null;
	}
}

/**
 * Schedule a proactive token refresh shortly before the current token expires.
 *
 * Clears any previously scheduled refresh before setting a new one.
 */
function scheduleTokenRefresh(token: string): void {
	// Clear any existing timer
	if (refreshTimerHandle !== null) {
		clearTimeout(refreshTimerHandle);
		refreshTimerHandle = null;
	}

	const msLeft = msUntilExpiry(token);
	if (msLeft <= 0) {
		// Already expired — attempt an immediate refresh
		console.debug(
			"[OidcAuth] Token already expired, attempting immediate refresh",
		);
		refreshAccessToken();
		return;
	}

	// Schedule refresh REFRESH_MARGIN_MS before expiry, but not negative
	const delay = Math.max(0, msLeft - REFRESH_MARGIN_MS);

	if (delay > 0 && delay < 1000) {
		// Very short delay (< 1s) — just refresh immediately to avoid a tight timer
		console.debug("[OidcAuth] Token expiring soon, refreshing immediately");
		refreshAccessToken();
		return;
	}

	console.debug("[OidcAuth] Scheduling token refresh in", delay, "ms");
	refreshTimerHandle = setTimeout(() => {
		refreshAccessToken();
	}, delay);
}

/**
 * Log the user out:
 * 1. Clear local tokens and API key
 * 2. Cancel any pending refresh timer
 * 3. Redirect to KeyCloak's end_session endpoint for RP-initiated logout
 */
export function logout(): void {
	const token = getStoredToken();
	clearAllTokens();
	clearVerifier();
	clearState();
	setAccessToken("");
	isAuthenticated.value = false;

	// Cancel any scheduled refresh
	if (refreshTimerHandle !== null) {
		clearTimeout(refreshTimerHandle);
		refreshTimerHandle = null;
	}

	console.debug("[OidcAuth] Logging out");

	if (token) {
		const params = new URLSearchParams({
			client_id: CLIENT_ID,
			post_logout_redirect_uri: `${window.location.origin}/login`,
		});

		const decoded = decodeToken(token);
		const idToken = decoded?.id_token as string | undefined;
		if (idToken) {
			params.set("id_token_hint", idToken);
		}

		const logoutUrl = `${KEYCLOAK_BASE}/realms/${REALM}/protocol/openid-connect/logout?${params.toString()}`;
		window.location.href = logoutUrl;
	} else {
		window.location.href = "/login";
	}
}

/**
 * Restore a previously stored session on app startup.
 * Reads the access + refresh tokens from localStorage and sets the access
 * token on the API client. If the token is expired or expiring soon,
 * attempts a silent refresh.
 *
 * Returns `true` if a valid-looking session was restored.
 */
export function restoreSession(): boolean {
	const stored = getStoredToken();
	if (!stored) return false;

	// Check basic structure
	const decoded = decodeToken(stored);
	if (!decoded) {
		clearAllTokens();
		return false;
	}

	const expMs = getTokenExpiry(stored);
	if (expMs === 0) {
		// Can't read expiry — clear to be safe
		clearAllTokens();
		return false;
	}

	// If the token is already expired or within the refresh margin, try refreshing
	if (isTokenExpired(stored, REFRESH_MARGIN_MS)) {
		console.debug(
			"[OidcAuth] Stored token expired or expiring soon, attempting refresh",
		);
		clearAllTokens();
		setAccessToken(""); // Clear synchronously before async refresh to avoid stale accessToken

		// Fire-and-forget refresh — the caller can check isAuthenticated after a tick
		refreshAccessToken().then((newToken) => {
			if (newToken) {
				console.debug("[OidcAuth] Session restored via token refresh");
			} else {
				isAuthenticated.value = false;
				console.debug(
					"[OidcAuth] Token refresh failed after restore — session cleared",
				);
			}
		});

		// Return false synchronously; the async refresh will update state when done
		return false;
	}

	// Token is still valid — use it and schedule proactive refresh
	const name = (decoded?.name ?? decoded?.preferred_username ?? "") as string;
	const provider = (decoded?.identity_provider ?? "") as string;
	setAuthToken(stored, name || undefined, provider || undefined);
	scheduleTokenRefresh(stored);
	console.debug("[OidcAuth] Restored session from localStorage");
	return true;
}

/**
 * Reactive composable for OIDC authentication state and actions.
 */
export function useOidcAuth() {
	return {
		/** Reactive — whether a valid auth session exists. */
		isAuthenticated,
		/** Redirect to KeyCloak local login. */
		redirectToKeycloak,
		/** Handle OAuth callback (code exchange). */
		handleCallback,
		/** Log out and redirect to KeyCloak. */
		logout,
		/** Restore a previously stored session. */
		restoreSession,
		/** Decode a JWT payload without validation. */
		decodeToken,
	};
}
