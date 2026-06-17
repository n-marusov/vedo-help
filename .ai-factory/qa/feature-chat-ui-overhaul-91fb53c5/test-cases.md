## Test Cases: E2E Login Tests for OAuth Provider Page

> Based on `change-summary.md` and `test-plan.md`
> Framework: Playwright (E2E)

---

### TC-LOGIN-001: Login page is accessible at /login route

**Priority:** High
**Type:** Positive

**Steps:**
1. Navigate to `/login`
2. Wait for the page to load

**Expected result:**
The login page is visible with `data-testid="login-page"` on the screen within 5 seconds.

**Test data:**
```
URL: /login
Selector: [data-testid="login-page"]
```

---

### TC-LOGIN-002: Login page displays application name and subtitle

**Priority:** Medium
**Type:** Positive

**Steps:**
1. Navigate to `/login`
2. Check the page title

**Expected result:**
The page contains the text "VEDO" (application name) and either "Virtual Environment for Developing Ontologies" (full title) or "Build, connect, and share knowledge at scale" (subtitle).

**Test data:**
```
Selectors: [data-testid="login-title"], .subtitle
Expected texts: "VEDO", "Virtual Environment for Developing Ontologies", "Build, connect, and share knowledge"
```

---

### TC-LOGIN-003: Login page renders all 5 provider buttons

**Priority:** High
**Type:** Positive

**Precondition:**
Login page is loaded at `/login`.

**Steps:**
1. Count all elements with `data-testid="oauth-btn"`
2. Verify their text labels

**Expected result:**
Exactly 5 buttons are rendered with labels: "Continue with VK ID", "Continue with Yandex ID", "Continue with Mail.ru", "Continue with Google", "Corporate SSO (SAML/OIDC)".

**Test data:**
```
Selector: [data-testid="oauth-btn"]
Expected count: 5
Expected labels: ["Continue with VK ID", "Continue with Yandex ID", "Continue with Mail.ru", "Continue with Google", "Corporate SSO (SAML/OIDC)"]
```

---

### TC-LOGIN-004: Each provider button has an SVG icon

**Priority:** Medium
**Type:** Positive

**Precondition:**
Login page is loaded at `/login`.

**Steps:**
1. For each provider button in order, check that it contains an `svg` element

**Expected result:**
All 5 buttons contain an SVG icon inside them. The first four have provider-specific SVGs, the last (corp-sso) has a generic shield icon.

**Test data:**
```
Selectors: [data-testid="oauth-btn"] svg
Expected: each button has at least 1 svg child
```

---

### TC-LOGIN-005: Clicking VK button navigates to KeyCloak with kc_idp_hint=vk

**Priority:** High
**Type:** Positive

**Precondition:**
Login page is loaded at `/login`. Vite `/auth` proxy is configured to KeyCloak at `localhost:8080`.

**Steps:**
1. Register a page navigation listener for requests containing `/auth`
2. Click the button with text "Continue with VK ID"
3. Wait for navigation

**Expected result:**
The page redirects to a URL containing:
- `/auth/realms/vedo-hub/protocol/openid-connect/auth`
- `kc_idp_hint=vk`
- `client_id=vedo-frontend`
- `response_type=code`
- `code_challenge_method=S256`
- `redirect_uri=http://localhost:5173/callback`

**Test data:**
```
URL pattern: /auth/realms/vedo-hub/protocol/openid-connect/auth?client_id=vedo-frontend&redirect_uri=http%3A%2F%2Flocalhost%3A5173%2Fcallback&response_type=code&scope=openid+profile+email&code_challenge=...&code_challenge_method=S256&state=...&kc_idp_hint=vk
```

---

### TC-LOGIN-006: Clicking Yandex button navigates to KeyCloak with kc_idp_hint=yandex

**Priority:** High
**Type:** Positive

**Precondition:**
Login page is loaded at `/login`.

**Steps:**
1. Register a page navigation listener for requests containing `/auth`
2. Click the button with text "Continue with Yandex ID"
3. Wait for navigation

**Expected result:**
The page redirects to a URL containing `kc_idp_hint=yandex` in addition to the standard OAuth parameters.

**Test data:**
```
kc_idp_hint=yandex
```

---

### TC-LOGIN-007: Clicking Mail.ru button navigates to KeyCloak with kc_idp_hint=mailru

**Priority:** High
**Type:** Positive

**Precondition:**
Login page is loaded at `/login`.

**Steps:**
1. Register a page navigation listener for requests containing `/auth`
2. Click the button with text "Continue with Mail.ru"
3. Wait for navigation

**Expected result:**
The page redirects to a URL containing `kc_idp_hint=mailru` in addition to the standard OAuth parameters.

**Test data:**
```
kc_idp_hint=mailru
```

---

### TC-LOGIN-008: Clicking Google button navigates to KeyCloak without kc_idp_hint

**Priority:** High
**Type:** Positive

**Precondition:**
Login page is loaded at `/login`.

**Steps:**
1. Register a page navigation listener for requests containing `/auth`
2. Click the button with text "Continue with Google"
3. Wait for navigation

**Expected result:**
The page redirects to a KeyCloak authorization URL that does NOT contain `kc_idp_hint`. This tests the fallback path (no hint = KeyCloak login form).

**Test data:**
```
URL should NOT contain: kc_idp_hint
URL should contain: /auth/realms/vedo-hub/protocol/openid-connect/auth
```

---

### TC-LOGIN-009: Clicking Corporate SSO navigates to KeyCloak without kc_idp_hint

**Priority:** Medium
**Type:** Positive

**Precondition:**
Login page is loaded at `/login`.

**Steps:**
1. Register a page navigation listener for requests containing `/auth`
2. Click the button with text "Corporate SSO (SAML/OIDC)"
3. Wait for navigation

**Expected result:**
The page redirects to a KeyCloak authorization URL that does NOT contain `kc_idp_hint`.

---

### TC-LOGIN-010: Login page is responsive on mobile viewport

**Priority:** High
**Type:** Positive

**Precondition:**
Viewport is set to 375x667 (iPhone SE).

**Steps:**
1. Set viewport to 375x667
2. Navigate to `/login`
3. Check that the login card fits within the viewport width

**Expected result:**
The `data-testid="login-card"` bounding box width is ≤ 375px. No horizontal scrollbar appears. Content is stacked vertically.

**Test data:**
```
Viewport: 375x667
Selectors: [data-testid="login-card"]
Condition: .boundingBox().width <= 375
```

---

### TC-LOGIN-011: Clicking a button while redirecting does not trigger second redirect

**Priority:** Medium
**Type:** Negative

**Precondition:**
Login page is loaded at `/login`.

**Steps:**
1. Click one provider button (triggers redirect)
2. Immediately click another provider button before the page navigates away

**Expected result:**
The second click is ignored because `isRedirecting` is `true`. Only one redirect is triggered. The page navigates only once.

---

### TC-AUTH-001: Auth guard redirects unauthenticated users to /login

**Priority:** High
**Type:** Security

**Precondition:**
localStorage is cleared (no auth token).

**Steps:**
1. Clear localStorage
2. Navigate to `/`
3. Wait for the router guard to evaluate

**Expected result:**
The browser URL changes to `/login` within 2 seconds. The `data-testid="login-page"` element is visible.

**Test data:**
```
localStorage: cleared
Target URL: /login
```

---

### TC-AUTH-002: Authenticated users can access the chat page

**Priority:** High
**Type:** Positive

**Precondition:**
A mock JWT token is stored in localStorage.

**Steps:**
1. Set `vedo_auth_token` in localStorage to a mock value
2. Navigate to `/`
3. Wait for the page to load

**Expected result:**
The user is NOT redirected to `/login`. The `data-testid="app-header"` is visible (app loads).

**Test data:**
```
localStorage: vedo_auth_token = "mock-valid-jwt-token"
```

---

### TC-AUTH-003: Logout clears token and redirects to login

**Priority:** High
**Type:** Negative

**Precondition:**
A mock JWT token is stored in localStorage.

**Steps:**
1. Set `vedo_auth_token` in localStorage
2. Navigate to `/`
3. Click the user avatar/button in the header
4. Click "Sign Out" in the dropdown
5. Wait for redirect

**Expected result:**
All `vedo_*` keys are cleared from localStorage. The browser URL is `/login`. The login page is visible.

**Test data:**
```
localStorage keys to clear: vedo_auth_token, vedo_auth_refresh_token, vedo_pkce_verifier, vedo_pkce_state
```

---

### TC-AUTH-004: Token persists across page reload

**Priority:** Medium
**Type:** Positive

**Precondition:**
A mock JWT token is stored in localStorage.

**Steps:**
1. Navigate to `/`
2. Reload the page
3. Check localStorage for the token
4. Check that the user is not redirected

**Expected result:**
`vedo_auth_token` in localStorage still equals `"mock-valid-jwt-token"`. The user stays on `/` and sees the app header. The page does not redirect to `/login`.

**Test data:**
```
localStorage: vedo_auth_token = "mock-valid-jwt-token"
```

---

### TC-AUTH-005: Expired token redirects to login

**Priority:** High
**Type:** Negative

**Precondition:**
An expired JWT is stored in localStorage.

**Steps:**
1. Set `vedo_auth_token` to a JWT with an expired `exp` claim
2. Navigate to `/`
3. Wait for the router guard / session restore to evaluate

**Expected result:**
The user is either:
- Redirected to `/login`, OR
- An auth error indicator (`data-testid="auth-error"`) is visible

**Test data:**
```
localStorage: vedo_auth_token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE1MDAwMDAwMDB9.mock"
```

---

### TC-AUTH-006: Admin page requires authentication

**Priority:** High
**Type:** Security

**Precondition:**
localStorage is cleared (no token).

**Steps:**
1. Clear localStorage
2. Navigate to `/admin`
3. Wait for the router guard to evaluate

**Expected result:**
The browser URL changes to `/login`. Unauthenticated users cannot access the admin page.

**Test data:**
```
URL: /admin
localStorage: cleared
Expected redirect: /login
```

---

### TC-AUTH-007: Login and callback routes are public

**Priority:** High
**Type:** Positive

**Precondition:**
localStorage is cleared (no token).

**Steps:**
1. Navigate to `/login`
2. Verify the login page is visible
3. Navigate to `/callback`
4. Verify the callback page is visible

**Expected result:**
Both `/login` and `/callback` are accessible without authentication. The login page shows provider buttons. The callback page shows "Signing in…" spinner.

**Test data:**
```
URLs: /login, /callback
No redirect expected
```

---

### TC-AUTH-008: Callback page shows error state on OAuth failure

**Priority:** Medium
**Type:** Negative

**Precondition:**
No KeyCloak redirect is in progress (fresh page load).

**Steps:**
1. Navigate to `/callback?error=access_denied&error_description=User+cancelled`
2. Wait for the error state to render

**Expected result:**
The callback page displays:
- "Authentication Failed" error title
- An error message describing the failure
- A "Back to Login" link pointing to `/login`

**Test data:**
```
URL: /callback?error=access_denied&error_description=User+cancelled
Expected text: "Authentication Failed", "User cancelled" or "access_denied"
Expected link: href="/login"
```

---

### TC-AUTH-009: Theme toggle is visible on login page

**Priority:** Low
**Type:** Positive

**Precondition:**
Login page is loaded at `/login`.

**Steps:**
1. Navigate to `/login`
2. Find the theme toggle button in the header row

**Expected result:**
The `VThemeToggle` component is visible on the login page, next to the application title.

**Test data:**
```
Selector: [data-testid="theme-toggle"] or VThemeToggle
```

---

### TC-VITE-001: Vite proxy forwards /auth to KeyCloak

**Priority:** High
**Type:** Integration

**Precondition:**
Vite dev server is running at `localhost:5173`. Docker Compose with KeyCloak is running.

**Steps:**
1. Fetch `http://localhost:5173/auth/realms/vedo-hub/.well-known/openid-configuration`
2. Check the response status

**Expected result:**
The request returns a JSON response with status 200 containing KeyCloak realm metadata (issuer, authorization_endpoint, token_endpoint, etc.).

**Test data:**
```
URL: /auth/realms/vedo-hub/.well-known/openid-configuration
Expected: 200 OK, JSON body with "issuer": "http://localhost:8080/realms/vedo-hub"
```
