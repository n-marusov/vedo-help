## Change Summary

**Branch:** `feature/chat-ui-overhaul`
**Focus:** E2E login tests for the login page with social OAuth providers
**Risk level:** 🟡 Medium

---

### What Changed

Existing E2E tests for login (`frontend/e2e/login.spec.ts`) are entirely skipped (`test.describe.skip`) and do not correspond to the actual implementation. The real `LoginButtons.vue` uses 5 different providers (VK, Yandex, Mail.ru, Google, Corporate SSO) with `data-testid="oauth-btn"`, but the tests check for `data-testid="btn-login-google"`, `btn-login-github`, `btn-login-discord` — test IDs that don't exist. The actual login page works correctly but tests will fail if unskipped.

Additionally, the Vite dev server (`frontend/vite.config.ts`) lacks a proxy rule for `/auth`, so the OAuth redirect (`KEYCLOAK_BASE = '/auth'`) hits the SPA instead of KeyCloak, producing a Vue Router warning: `No match found for location with path "/auth/realms/vedo-hub/..."`.

---

### Affected Areas

| Component | Change type | Description |
|-----------|-------------|-------------|
| `frontend/e2e/login.spec.ts` | Outdated/skipped | 17 tests in 2 `describe.skip` blocks referencing non-existent `data-testid` values |
| `frontend/src/components/LoginButtons.vue` | Existing (real) | 5 provider buttons with `data-testid="oauth-btn"` (generic, same for all buttons) |
| `frontend/src/views/LoginView.vue` | Existing | Login page with `data-testid="login-page"` and `login-card` |
| `frontend/src/composables/useOidcAuth.ts` | Existing | PKCE OAuth flow, `redirectToKeycloak()`, `handleCallback()` |
| `frontend/src/router.ts` | Existing | Auth guard redirects unauthenticated users to `/login` |
| `frontend/src/stores/auth.ts` | Existing | `isAuthenticated`, `userName`, `userProvider` refs |
| `frontend/vite.config.ts` | Missing proxy | No `/auth` proxy rule — OAuth redirect fails in dev mode |

---

### Evidence

| Finding | Evidence |
|---------|----------|
| **Tests use wrong data-testid values** | `login.spec.ts` line 33-35: `btn-login-google`, `btn-login-github`, `btn-login-discord` — but `LoginButtons.vue` line 44: `data-testid="oauth-btn"` (same for all buttons) |
| **Tests expect 3 providers, real page has 5** | Test checks Google/GitHub/Discord, but `LoginButtons.vue` has VK, Yandex, Mail.ru, Google, Corporate SSO |
| **Tests check non-existent elements** | `login-container` (TC-LOGIN-008), `login-error` (TC-LOGIN-009), `login-notice` (TC-LOGIN-010), `btn-logout` (TC-AUTH-003), `auth-error` (TC-AUTH-005), `avatar-user` (TC-AUTH-007) |
| **Vite proxy missing `/auth`** | `vite.config.ts` only has `/api` proxy rule; KeyCloak base is `/auth` in `useOidcAuth.ts` line 24 |
| **Actual OAuth redirect fails** | Console error: `[Vue Router warn]: No match found for location with path "/auth/realms/vedo-hub/..."` |
| **Router guard works in isolation** | `router.ts` lines 50-61: `beforeEach` guard redirects `!isAuthenticated` to `/login` |
| **Auth store correctly manages state** | `stores/auth.ts`: `isAuthenticated` reactive ref, `setAuthToken()`, `clearAuth()` |

---

### Risks

🟡 **Medium** (must verify):

- **E2E tests are completely out of sync** with the actual implementation — unskipping them without rewriting will cause 17 test failures
- **Missing Vite proxy** blocks the entire OAuth redirect flow in dev mode — no social login works from `localhost:5173`
- **`data-testid="oauth-btn"` is generic** — all 5 buttons share the same test ID, making per-provider assertions impossible

🟢 **Low** (nice to verify):

- Auth guard (`router.ts`) has no unit tests
- No E2E tests exist for the callback/error flow

---

### Testing Recommendations

**First priority:**

- [ ] Rewrite `frontend/e2e/login.spec.ts` to match the actual `LoginButtons.vue` implementation
- [ ] Add `/auth` proxy rule to `frontend/vite.config.ts` for dev-mode OAuth flow
- [ ] Verify that `restoreSession()` + router guard work end-to-end

**Regression:**

- [ ] Run all existing passing E2E tests to ensure no regressions from test changes
- [ ] Verify auth guard doesn't break navigation to `/login` and `/callback`
