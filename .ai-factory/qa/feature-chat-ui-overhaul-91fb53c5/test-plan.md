## Test Plan: E2E Login Tests for OAuth Provider Page

**Date:** 2026-06-17
**Branch:** feature/chat-ui-overhaul
**Environment:** local (dev) with Vite dev server + Docker Compose (KeyCloak + PostgreSQL)

---

### 1. Testing Goal

Verify that the login page renders correctly with all 5 OAuth provider buttons (VK, Yandex, Mail.ru, Google, Corporate SSO), that clicking each button triggers the correct KeyCloak redirect, that the auth guard correctly enforces authentication on protected routes, that token storage and session restoration work, and that logout clears state and redirects. Additionally, validate that the Vite proxy forwards `/auth` requests to KeyCloak in dev mode.

---

### 2. Test Scope

**In Scope** — we test:

- Login page rendering (`LoginView.vue`) — title, logo, theme toggle
- All 5 OAuth provider buttons (`LoginButtons.vue`) — visibility, labels, icons
- OAuth redirect flow — clicking a provider calls `redirectToKeycloak()` with correct `kc_idp_hint`
- Auth guard (`router.ts`) — redirects unauthenticated users to `/login`
- Auth state management (`stores/auth.ts`) — `isAuthenticated`, `userName`, `userProvider`
- Token storage (localStorage) — persistence across reloads
- Logout — clears tokens, redirects to `/login`
- Vite proxy configuration — `/auth` proxied to `localhost:8080`
- Responsive behavior at mobile viewport (375px)
- Error state on callback failure (`CallbackView.vue`)

**Out of Scope** — we don't test:

- Full OAuth2 round trip with real KeyCloak — requires KeyCloak + social providers; covered by separate integration tests
- Backend JWT validation — covered by Rust unit tests
- Other E2E specs (avatar, message-bubble, chat-window, navigation, theme-switching) — their passing state is a regression check only
- Performance or load testing

---

### 3. Test Types

| Type | Priority | Area |
|------|----------|------|
| Functional | 🔴 High | Login page rendering, provider buttons, redirect flow, auth guard, token persistence, logout |
| Regression | 🟡 Medium | Other E2E specs, router navigation for non-login routes |
| Edge cases | 🟡 Medium | Button click with `isRedirecting` guard, duplicate clicks, null/empty token |
| Negative | 🟡 Medium | Error state in callback, missing code/state params, expired token display |
| Security | 🔴 High | Auth guard on `/admin`, unauthenticated access to protected routes |
| Responsive | 🟡 Medium | Mobile viewport layout |

---

### 4. Test Data

| Category | Data | Purpose |
|----------|------|---------|
| Mock JWT | `"mock-valid-jwt-token"` | Simulate authenticated state for auth guard tests |
| Expired JWT | `"eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE1MDAwMDAwMDB9.mock"` | Simulate expired token scenario |
| Provider IDs | `vk`, `yandex`, `mailru`, `google`, `corp-sso` | Verify each button identity |
| Error param | `error=access_denied&error_description=User+cancelled` | Simulate OAuth error in callback |
| Viewport | 375x667 (iPhone SE) | Mobile responsive test |

---

### 5. Preconditions

- [ ] Docker Compose is running with `keycloak` and `keycloak-db` services healthy
- [ ] Vite dev server is running at `localhost:5173` with `/auth` proxy to `localhost:8080`
- [ ] `KEYCLOAK_HOSTNAME=localhost` is set in `.env` (or default)
- [ ] KeyCloak realm `vedo-hub` is imported and accessible at `http://localhost:8080/realms/vedo-hub/.well-known/openid-configuration`
- [ ] Social identity providers are configured and enabled in KeyCloak

---

### 6. Acceptance Criteria

- [ ] All 🔴 high-priority login page tests pass (TC-LOGIN-001 through TC-LOGIN-007 rewritten)
- [ ] All 🔴 high-priority auth guard tests pass (TC-AUTH-001 through TC-AUTH-006)
- [ ] Provider button redirect generates correct KeyCloak authorization URL with `kc_idp_hint`
- [ ] Missing token redirects unauthenticated users to `/login`
- [ ] Logout clears localStorage tokens and returns to `/login`
- [ ] No regression in other E2E specs (all previously passing tests remain green)
- [ ] Vite proxy returns valid response for `GET /auth/realms/vedo-hub/.well-known/openid-configuration`
- [ ] Negative scenarios return expected error states without crashes

---

### 7. Plan Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| KeyCloak not running in CI | High — redirect flow tests fail | Skip KeyCloak-dependent tests in CI; mock `window.location` for redirect assertion |
| Social providers not configured | Medium — `kc_idp_hint` may not resolve | Test with `password` grant or direct KeyCloak login form fallback |
| Vite proxy port mismatch | Medium — `/auth` points to wrong port | Verify `KEYCLOAK_PORT` env var in Docker Compose matches `vite.config.ts` target |

---

### 8. Checklist

| Check | Priority |
|-------|----------|
| `LoginView.vue` renders `data-testid="login-page"` | High |
| `LoginView.vue` displays "VEDO" in title and welcome/subtitle text | Medium |
| All 5 provider buttons are visible with correct labels | High |
| Each provider button has a unique SVG icon | Medium |
| Each provider button uses `data-testid="oauth-btn"` | High |
| Clicking VK/Yandex/Mail.ru calls `redirectToKeycloak(hint)` | High |
| Clicking Google/Corp-SSO calls `redirectToKeycloak()` (no hint) | High |
| Redirect URL contains `/auth/realms/vedo-hub/protocol/openid-connect/auth` | High |
| Redirect URL contains `kc_idp_hint` for mapped providers | High |
| `isRedirecting` flag disables all buttons during redirect | High |
| Auth guard redirects `/` to `/login` when no token | High |
| Auth guard allows `/login` and `/callback` without token | High |
| Mock token in localStorage allows access to `/` | Medium |
| Logout button clears `vedo_auth_token` from localStorage | High |
| Logout redirects to `/login` | High |
| Token persists after page reload | Medium |
| Expired token shows login redirect or auth error indicator | High |
| Admin page `/admin` requires auth guard | High |
| Callback page shows spinner then redirects or error | Medium |
| Callback error case shows "Authentication Failed" with link back to `/login` | Medium |
| Login page fits mobile viewport without overflow | Medium |
| Vite proxy returns 200 on `/auth/realms/vedo-hub/` | High |
| All existing passing E2E tests remain green | High |
