# Chat UI Overhaul

> DeepSeek-style minimalistic chat interface: cleaner messages, avatars, animations, responsive layout.

**Branch:** `feature/chat-ui-overhaul`
**Created:** 2026-06-15
**Plan ID:** `feature-chat-ui-overhaul`

---

## Settings

| Setting | Value |
|---------|-------|
| Testing | ✅ Yes — include tests |
| Logging | ✅ Verbose (DEBUG-level logs for development) |
| Docs | ✅ Yes — mandatory docs checkpoint after implementation |

---

## Roadmap Linkage

| Field | Value |
|-------|-------|
| Milestone | v0.2 — GUI Redesign |
| Rationale | This is the first phase of the GUI Redesign milestone — the chat interface is the primary user-facing surface and must match DeepSeek-level quality before proceeding to theme, sidebar, and Markdown rendering phases. |

---

## Tasks

### Phase 0: Design Exploration (Pencil)

#### [x] Task 0.1 — Sync Pencil design library and page/dialog design files

Before implementation, keep the Pencil design reference aligned with the actual `design/` directory. The design system lives in `design/ui-kit.lib.pen`, while screen-level layouts are split into separate page files instead of a single `ui-design.pen` exploration file.

**Files to change/maintain:**
- `design/ui-kit.lib.pen` — reusable design library with tokens, atoms, molecules, and organisms for chat, admin, auth, upload, dialogs, and source previews
- `design/chat.pen` — Chat page layouts, FullHD `1920×1080`, dark theme on the left and light theme on the right, horizontally separated by `150px`
- `design/admin.pen` — Admin page layouts, FullHD `1920×1080`, dark theme on the left and light theme on the right, horizontally separated by `150px`
- `design/dialogs.pen` — dialog window layouts, stacked vertically; each row contains dark and light variants aligned by top edge with `150px` horizontal and vertical gaps
- `design/login.pen` — existing login page reference using the same UI-kit import and visual language

**Design library components currently reused from `ui-kit.lib.pen`:**

*Atoms:*
- `Component/Button`, `Component/Button/Outline`, `Component/Button/Ghost`, `Component/Button/XS`
- `Component/Input`, `Component/Select`, `Component/AlertBox`, `Component/ProgressBar`
- `Component/Avatar`, `Component/SendButton`, `Component/StreamingBar`, `Component/StreamingIndicator`

*Molecules:*
- `Component/SourceItem` — source reference row/card with document name and relevance score
- `Component/ChatInput` — prompt input with inline send button
- `Component/FileListItem`, `Component/DropZone`, `Component/FormField`, `Component/FormDialog`

*Organisms / screen-level reusable components:*
- `Component/Message/User` — right-aligned user message
- `Component/Message/Bot` — assistant message
- `Component/Message/BotWithSources` — assistant message with source citations
- `Component/LoginCard` — centered auth card with provider button area
- `Component/Dialog` and `Component/ConfirmDialog` — modal dialog references

**Page design requirements:**
- Keep global Chat/Admin navigation sidebar out of page designs; only page-local sidebars remain (for example, chat sessions or admin collections).
- Maintain separate page files for Chat and Admin rather than a combined `ui-design.pen`.
- Every page artboard must be FullHD (`1920×1080`).
- Dark theme variant must be placed on the left; light theme variant must be placed to the right with a `150px` gap and aligned to the same top edge.
- Dialog variants must be arranged vertically, with each light variant mirrored to the right of its dark variant and aligned by top edge.
- All screen files must import the design library via `"imports": { "B": "./ui-kit.lib.pen" }` and reuse `ref` components where available.

**Design coverage:**
- Chat page: session sidebar, collection selector, welcome state, user/assistant messages, citations, streaming/typing state, and prompt input.
- Admin page: collection list, document list, upload action, drag-and-drop upload area, upload progress, and API key clear action.
- Dialogs: Admin Access, Create Collection, Delete Collection, Delete Document, Upload Progress, Source Preview, and Upload Error.
- Login reference: OAuth/social-provider login card remains in `design/login.pen` for auth phase continuity.

**Dependencies:** None

---

### Phase 1: Design Tokens & Shared Variables

#### [x] Task 1.1 — Define chat UI design tokens

Create CSS custom properties for the chat design system. These tokens will be the single source of truth for spacing, radii, animation timing, and message colors.

**Files to create/change:**
- `frontend/src/assets/chat-tokens.css` — new CSS file with `:root` custom properties

**Scope:**
- Message spacing: `--msg-gap`, `--msg-padding-y`, `--msg-padding-x`
- Border radius: `--msg-radius-user`, `--msg-radius-assistant`, `--avatar-radius`
- Animation: `--anim-msg-enter-duration`, `--anim-msg-enter-ease`, `--anim-stream-duration`
- Colors: `--msg-user-bg`, `--msg-assistant-bg`, `--msg-user-text`, `--msg-assistant-text`, `--msg-time-color`, `--avatar-user-bg`, `--avatar-assistant-bg`
- Sizing: `--avatar-size`, `--max-msg-width`, `--input-min-height`

**Logging:** Console-log the resolved token values once on mount at DEBUG level for verification.

**Dependencies:** None

---

#### [x] Task 1.2 — Design avatar component

Create a lightweight `UserAvatar` component that renders either an SVG icon (user) or a branded "V" icon (assistant) instead of emoji.

**Files to create/change:**
- `frontend/src/components/ui/UserAvatar.vue` — new component
- `frontend/src/components/ui/` — ensure directory exists

**Component API:**
```ts
defineProps<{
  role: 'user' | 'assistant';
  size?: 'sm' | 'md' | 'lg';  // maps to --avatar-size * factor
}>();
```

**Design:**
- User: solid circle with a person silhouette SVG, neutral color
- Assistant: solid circle with a "V" letter SVG (VEDO brand), accent color
- No emoji, no images — pure SVG inline for zero network cost

**Logging:** Component mount log at DEBUG with role and size.

**Dependencies:** Task 1.1 (uses CSS tokens)

---

### Phase 2: MessageBubble Redesign

#### [x] Task 2.1 — Rewrite MessageBubble with minimalistic design

Replace the current `MessageBubble.vue` with a cleaner, DeepSeek-inspired layout. Preserve existing functionality (Markdown rendering via `marked`, sources toggle) but simplify the visual hierarchy.

**Files to change:**
- `frontend/src/components/MessageBubble.vue`

**Changes:**
- Remove the `.message-body` colored background blocks for assistant messages (clean, no-card look)
- User messages stay right-aligned with a subtle background
- Use `UserAvatar` component instead of emoji `👤`/`🤖`
- Remove the "You" / "VEDO Assistant" label header (role is implied by position + avatar)
- Keep the timestamp but style it smaller and more subtle
- Sources remain collapsible with a cleaner chevron icon
- Add a `.message-enter` CSS class for smooth entry animation (opacity + translateY, staggered by index)

**Logging:**
- DEBUG: message render with role, content length, sources count
- WARN: if marked.parse fails

**Dependencies:** Task 1.1, Task 1.2

---

#### [x] Task 2.2 — Refine streaming / typing indicator

Replace the three-dot typing bounce animation with a smoother streaming glow animation — a subtle pulsing bar that appears at the bottom of the assistant message while content is being received.

**Files to change:**
- `frontend/src/components/MessageBubble.vue`

**Scope:**
- When `isStreaming && !message.content` → show a thin animated gradient bar (skeleton-like)
- When `isStreaming && message.content` → show a vertical cursor bar at the end of the text that blinks
- Remove the old `.typing-indicator` with bouncing dots
- Ensure the streaming state transitions smoothly (no layout shift)

**Logging:** DEBUG log when streaming starts/ends with message ID.

**Dependencies:** Task 2.1 (same file, implement together or as follow-up)

---

### Phase 3: Layout & ChatWindow Redesign

#### [x] Task 3.1 — Remove admin navigation from main page

The current `App.vue` renders a persistent sidebar with links to `/` (Chat) and `/admin` (Admin). In the DeepSeek-style chat, the main page should show only the chat — no navigation sidebar, no link to admin. The admin panel remains accessible exclusively via direct URL `/admin`.

**Files to change:**
- `frontend/src/App.vue`

**Scope:**
- Remove the `<aside class="sidebar">` with nav links (Chat, Admin) from `App.vue`
- The app layout becomes full-height `<router-view />` without a persistent nav sidebar
- Keep a minimal branding element if desired (e.g., small "VEDO" logo in the corner), but no clickable navigation
- Do NOT remove or modify the `/admin` route in `router.ts` — it stays accessible by direct URL
- The session sidebar in `ChatView.vue` is NOT affected — it remains as part of the chat interface

**Non-goals:**
- Do NOT build an admin login gate or auth — that's a separate task
- Do NOT remove the AdminView component or route

**Logging:** DEBUG: app mount, route change

**Dependencies:** None

---

#### [x] Task 3.2 — Redesign ChatWindow layout

Rewrite `ChatWindow.vue` with a cleaner, more focused layout now that there's no global nav sidebar. Remove the heavy header bar with collection selector — integrate collection selection into a minimal dropdown or keep in the session sidebar.

**Files to change:**
- `frontend/src/components/ChatWindow.vue`

**Scope:**
- **Header:** Remove or greatly simplify — no collection selector in the header. If needed, move to a small inline indicator + dropdown near the top.
- **Welcome screen:** Redesign with a clean centered layout, brief tagline, and a subtle background gradient or pattern (no emoji 3rem icon).
- **Messages area:** Full-height flex layout with max-width centering (like DeepSeek — messages don't span full width on wide screens).
- **Input area:** 
  - Clean input with rounded corners, no heavy border
  - Send button as an icon-only button inside the input (right side)
  - Auto-resize textarea with min/max height
  - Disabled state styling

**Logging:**
- DEBUG: component mount, session change
- INFO: message sent, message received
- WARN: if active collection is missing on send

**Dependencies:** Task 3.1 (App.vue restructuring), Task 2.1 (MessageBubble redesign)

---

#### [x] Task 3.3 — Implement smooth message animations

Add staggered entrance animations for messages. When a new message appears (optimistic user message or streaming assistant reply), it should animate in smoothly.

**Files to change:**
- `frontend/src/components/ChatWindow.vue`
- `frontend/src/components/MessageBubble.vue` (if needed)

**Animation spec:**
- Entrance: `opacity: 0 → 1`, `transform: translateY(8px) → translateY(0)`
- Duration: 250ms, ease-out
- Stagger: 50ms delay between consecutive messages
- Use CSS `@keyframes` with `animation-delay` set via inline style `--i` index variable
- Reduce motion: respect `prefers-reduced-motion` — disable animations entirely

**Logging:** DEBUG log per animation trigger.

**Dependencies:** Task 3.2

---

#### [x] Task 3.4 — Responsive chat layout

Make the chat window fully responsive for mobile and tablet viewports.

**Files to change:**
- `frontend/src/components/ChatWindow.vue`
- `frontend/src/views/ChatView.vue` (if sidebar adjustments needed)

**Breakpoints:**
- `< 480px` (mobile): full-screen chat, no sidebar, input at bottom with safe-area padding
- `480px – 768px` (tablet): collapsible sidebar, chat takes full width when sidebar closed
- `> 768px` (desktop): current split layout with sidebar

**Scope:**
- Media queries in ChatWindow for message max-width (100% on mobile, 75% on desktop)
- Input area sticks to bottom on mobile (like native chat apps)
- Font-size adjustment for readability on small screens
- Touch-friendly input height (min 44px)

**Logging:** WARN if viewport changes are detected but layout fails to adjust.

**Dependencies:** Task 3.2

---

### Phase 4: Integration & Polish

#### [x] Task 4.1 — Add unit tests for new components

Write Vitest tests for the redesigned components.

**Files to create:**
- `frontend/src/components/__tests__/MessageBubble.spec.ts`
- `frontend/src/components/__tests__/ChatWindow.spec.ts`
- `frontend/src/components/__tests__/UserAvatar.spec.ts`
- `frontend/src/components/ui/__tests__/UserAvatar.spec.ts` (if separate)

**Test coverage:**
- `UserAvatar`: renders correct icon per role, applies size classes, handles missing props
- `MessageBubble`: renders user vs assistant messages, renders Markdown content, shows sources toggle, handles streaming state, handles empty content
- `ChatWindow`: renders welcome screen when no messages, renders message list, sends message on Enter, auto-scrolls on new message, shows loading state, cancel streaming button works

**Logging:** N/A (tests don't log)

**Dependencies:** Task 2.1, Task 3.1, Task 1.2

---

#### [x] Task 4.2 — Documentation checkpoint

Update project documentation to reflect the redesigned chat UI.

**Files to change:**
- `docs/gui.md` — update screenshots and descriptions of chat interface
- `frontend/README.md` (if exists) — note the design token approach
- `.ai-factory/ROADMAP.md` — mark Chat UI overhaul as complete

**Scope:**
- Document the new design tokens (`chat-tokens.css`) for maintainers
- Document component API for `UserAvatar`
- Update any screenshots or usage examples

**Dependencies:** All previous tasks

---

#### [x] Task 4.3 — Update ROADMAP.md

Mark the Chat UI overhaul phase as complete and move to the next phase in the v0.2 milestone.

**Files to change:**
- `.ai-factory/ROADMAP.md` — check off "Chat UI overhaul"

**Dependencies:** Task 4.2 (docs checkpoint)

---

### Phase 5: Authentication — KeyCloak + Social Providers

> Add KeyCloak as a local identity provider and integrate Yandex, VK, and Mail.ru for social login. This phase enables user authentication for the admin panel without exposing the admin login page on the public-facing chat.

**Architecture:**
- KeyCloak runs as a Docker Compose service with PostgreSQL
- Social providers (Yandex, VK, Mail.ru) are configured as Identity Providers in KeyCloak realm
- Backend validates JWT tokens issued by KeyCloak using JWKS endpoint
- Frontend login page initiates OAuth flow via KeyCloak
- Admin panel (`/admin`) is protected by auth guard; main chat page stays public
- API routes remain protected but accept both legacy `ADMIN_API_KEY` and KeyCloak JWT

---

- [x] Task 5.1 — Add KeyCloak + PostgreSQL to Docker Compose

Add KeyCloak and its PostgreSQL database as new services in the Docker Compose stack for local development.

**Files to change:**
- `docker-compose.yml` — add `keycloak` and `keycloak-db` services
- `docs/configuration.md` — document new env vars
- `.env.example` — add KeyCloak-related variables

**Scope:**
- **`keycloak-db`:** PostgreSQL 16, persistent volume, healthcheck
- **`keycloak`:** `quay.io/keycloak/keycloak:26.1` (latest stable), starts in production mode, connects to PostgreSQL
  - Port mapping: `8080:8080` (dev only, not exposed in production compose)
  - Env vars: `KC_DB=postgres`, `KC_DB_URL`, `KC_DB_USERNAME`, `KC_DB_PASSWORD`, `KC_HOSTNAME`, `KEYCLOAK_ADMIN`, `KEYCLOAK_ADMIN_PASSWORD`
  - Healthcheck: `curl -f http://localhost:8080/realms/master`
- **Network:** both services join the `internal` Docker network
- **Caddy:** add reverse proxy rule for `/auth/*` → `keycloak:8080` in `Caddyfile`
- **Dev only:** KeyCloak port `8080` is available on the host for realm configuration via Admin Console

**Logging:**
- INFO: KeyCloak service startup, realm import status
- WARN: if KeyCloak healthcheck fails (graceful degradation — fall back to API key auth)

**Dependencies:** None

---

- [x] Task 5.2 — Create KeyCloak realm and configure social identity providers

Create a reproducible KeyCloak realm configuration with Yandex, VK, and Mail.ru as Identity Providers. The realm must be exportable as JSON for CI/CD.

**Files to create:**
- `keycloak/ realm-export.json` — exported realm configuration (or import script)
- `keycloak/Dockerfile` — custom KeyCloak image with realm auto-import
- `docs/auth.md` — new doc: authentication setup guide

**Scope:**
- Create realm `vedo-hub` with:
  - Client `vedo-frontend` (public, redirect URIs: `http://localhost:5173/*`, `https://<production>/*`)
  - Client `vedo-backend` (confidential, client secret, service account roles)
  - Custom JWT token mapper: add `provider` claim (yandex/vk/mailru/password)
- Configure Identity Providers:
  - **Yandex:** https://oauth.yandex.com/ — requires Client ID + Client Secret (set as env vars)
  - **VK ID:** https://id.vk.com/ — requires Client ID + Client Secret + Service Token
  - **Mail.ru:** https://account.mail.ru/ — requires Client ID + Client Secret
- For local development, configure all three with `kc_`-prefixed env vars
- Default user: `admin` (password from `KEYCLOAK_ADMIN_PASSWORD`) for local login
- Document where to register apps with each provider (link to developer consoles)

**Logging:** DEBUG: realm import details; WARN: if an IdP is misconfigured (missing client secret)

**Dependencies:** Task 5.1 (KeyCloak service running)

---

#### [x] Task 5.3 — Add KeyCloak JWT validation middleware to backend

Extend the backend auth middleware to validate both the legacy `ADMIN_API_KEY` header and KeyCloak-issued JWT tokens using JWKS.

**Files to change:**
- `backend/Cargo.toml` — add `jsonwebtoken` crate for JWT validation
- `backend/src/config.rs` — add `keycloak_url`, `keycloak_realm`, `keycloak_client_id` fields
- `backend/src/shared/auth.rs` — rewrite auth logic to accept both token types
- `backend/src/main.rs` — update auth middleware wiring

**Auth flow:**
1. Extract `Bearer` token from `Authorization` header
2. Try legacy validation: token === `ADMIN_API_KEY` → immediate pass (backward compat)
3. Try JWT validation: fetch JWKS from `{keycloak_url}/realms/{realm}/protocol/openid-connect/certs`
4. Verify JWT signature, expiry (`exp`), issuer (`iss`), audience (`aud`)
5. Extract user claims: `sub` (user ID), `name`, `email`, `preferred_username`, custom `provider` claim
6. Attach `AuthUser` struct to request extensions (instead of just `AuthToken`)

**Logging:**
- DEBUG: JWT validation result per request
- WARN: JWKS fetch failure, expired token, invalid signature
- INFO: auth type used (legacy API key vs JWT)
- ERROR: JWKS endpoint unreachable (still allow API key fallback)

**Dependencies:** Task 5.2 (realm configuration)

---

#### [x] Task 5.4 — Add auth endpoints and user context to backend

Create new `/api/auth/*` endpoints for session management and user info retrieval.

**Files to create/change:**
- `backend/src/modules/auth/` — new module (handlers, service, models)
- `backend/src/modules/auth/handlers.rs` — `/api/auth/me`, `/api/auth/logout`
- `backend/src/modules/auth/service.rs` — user info resolution
- `backend/src/modules/auth/models.rs` — `AuthUser`, `UserInfo` types
- `backend/src/lib.rs` — add `pub mod auth`
- `backend/src/main.rs` — wire auth routes

**Endpoints:**
- `GET /api/auth/me` — returns current user info (name, email, provider, avatar URL) from JWT claims; 401 if not authenticated
- `POST /api/auth/logout` — invalidates session (KeyCloak admin API or client-side token removal)

**UserContext:**
- Create `UserContext` struct with fields: `user_id`, `name`, `provider`, `email`
- Attach to request extensions in auth middleware
- Expose via axum extractor for handlers that need user context

**Logging:** DEBUG: user info request; WARN: unauthenticated access to protected endpoints

**Dependencies:** Task 5.3 (JWT validation middleware)

---

#### [x] Task 5.5 — Create frontend login page with social provider buttons

Create a login page at `/login` with branded buttons for Yandex, VK, and Mail.ru authentication via KeyCloak. Implement the OAuth authorization code flow (PKCE for public client).

**Files to create/change:**
- `frontend/src/views/LoginView.vue` — new login page
- `frontend/src/components/LoginButtons.vue` — social provider button group
- `frontend/src/stores/auth.ts` — new Pinia store for auth state
- `frontend/src/api/auth.ts` — auth API client (login URL, token refresh, user info)
- `frontend/src/router.ts` — add `/login` route
- `frontend/src/App.vue` — mount auth guard (redirect to `/login` for protected routes)

**Login flow:**
1. User clicks Yandex/VK/Mail.ru button
2. Redirect to `{keycloak_url}/realms/vedo-hub/protocol/openid-connect/auth` with:
   - `client_id=vedo-frontend`
   - `redirect_uri={origin}/auth/callback`
   - `response_type=code`
   - `scope=openid profile email`
   - `kc_idp_hint=yandex` (or `vk`, `mailru`) — skips KeyCloak login form
3. After provider auth, KeyCloak redirects to `/auth/callback` with `code`
4. Frontend exchanges `code` for tokens via `POST {keycloak_url}/realms/vedo-hub/protocol/openid-connect/token`
5. Store `access_token`, `refresh_token`, `id_token` in `localStorage`
6. Decode `id_token` (JWT) to get user info, store in Pinia auth store
7. Redirect to `/` (or to the originally requested page)

**UI:**
- LoginView: clean centered card with provider logo buttons, no password form (all auth goes through social providers)
- Yandex: white button with red "Ya" logo
- VK: blue button with white VK logo
- Mail.ru: blue button with white Mail.ru logo
- DeepSeek-style dark theme consistent with chat UI
- Loading spinner during OAuth redirect
- Error state if auth fails (retry button)

**Auth guard:**
- `router.ts`: add `meta: { requiresAuth: true }` to `/admin` route
- `App.vue` or router `beforeEach`: if route requires auth and no token in store → redirect to `/login?redirect=/admin`
- Main chat page (`/`) stays public — no login required to chat

**Logging:** DEBUG: login flow steps (redirect, callback, token exchange); WARN: token exchange failure, expired token, auth guard redirect

**Dependencies:** Task 5.2 (realm config with clients)

---

#### [x] Task 5.6 — Token refresh and session persistence

Implement silent token refresh using KeyCloak's refresh token and persist the auth session across page reloads.

**Files to change:**
- `frontend/src/stores/auth.ts` — add `initAuth()` method, refresh logic
- `frontend/src/api/auth.ts` — add `refreshToken()` function
- `frontend/src/App.vue` — call `initAuth()` on mount
- `frontend/src/api/client.ts` — automatically attach Bearer token from auth store (fallback to `apiKey`)

**Scope:**
- On app mount: check `localStorage` for `refresh_token`, try to get a new `access_token`
- If refresh succeeds: decode JWT, populate auth store, redirect to original page
- If refresh fails: clear storage, stay on login page
- Refresh token rotation: KeyCloak returns a new `refresh_token` on each refresh call
- Silent refresh: use `postMessage` iframe or `navigator.cookies` (depending on KeyCloak version), fallback to full redirect
- Auto-refresh: monitor `access_token` expiry (`exp` claim), refresh 30s before expiration
- Update `api/client.ts`: Auth header picks from Pinia auth store first, falls back to `apiKey`

**Logging:** DEBUG: token refresh attempt and result; WARN: refresh failure (expected if user hasn't logged in); ERROR: persistent auth failure

**Dependencies:** Task 5.5 (login page and token storage)

---

### Phase 6: Documentation & Cleanup

#### [x] Task 6.1 — Auth documentation and configuration guide

Document the complete authentication setup: KeyCloak realm configuration, social provider registration, environment variables, and troubleshooting.

**Files to create/change:**
- `docs/auth.md` — comprehensive auth guide (created in Task 5.2, update with operational details)
- `docs/configuration.md` — add all new KC_ env vars with descriptions
- `README.md` — add auth section in the overview
- `.env.example` — add complete set of KeyCloak + social provider env vars

**Scope:**
- Prerequisites: Docker with Compose plugin
- Step-by-step KeyCloak setup with realm import
- Registering applications at Yandex, VK, and Mail.ru developer consoles
- Mapping environment variables
- Local development vs production configuration
- Troubleshooting common issues (redirect URI mismatch, JWKS errors, token expiry)

**Dependencies:** All Phase 5 tasks

---

#### [x] Task 6.2 — Update ROADMAP.md

Mark the Chat UI overhaul (Phase 5 auth tasks backfilled into v0.2 scope) as complete.

**Files to change:**
- `.ai-factory/ROADMAP.md` — check off "Chat UI overhaul"

**Dependencies:** Task 6.1 (docs checkpoint)

---

## Commit Plan

| # | Commits | Scope | Message |
|---|---------|-------|---------|
| 1 | Task 0.1 | Pencil design exploration | `design(ui): add chat UI and auth components to design library + ui-design.pen` |
| 2 | Tasks 1.1, 1.2 | Design tokens + avatar | `feat(ui): add chat design tokens and UserAvatar component` |
| 3 | Tasks 2.1, 2.2 | MessageBubble rewrite | `feat(ui): redesign MessageBubble with minimalistic layout and streaming indicator` |
| 4 | Task 3.1 | Remove admin nav | `feat(ui): remove admin navigation from main chat page` |
| 5 | Tasks 3.2, 3.3 | ChatWindow redesign | `feat(ui): redesign ChatWindow with smooth animations and clean layout` |
| 6 | Task 3.4 | Responsive | `feat(ui): add responsive layout for mobile and tablet viewports` |
| 7 | Task 4.1 | Tests | `test(ui): add unit tests for chat components` |
| 8 | Tasks 4.2, 4.3 | Docs + roadmap | `docs(ui): update documentation for chat UI overhaul` |
| 9 | Tasks 5.1, 5.2 | KeyCloak infra | `feat(auth): add KeyCloak + PostgreSQL to Docker Compose and configure realm` |
| 10 | Tasks 5.3, 5.4 | Backend auth | `feat(auth): add JWT validation middleware and auth endpoints` |
| 11 | Tasks 5.5, 5.6 | Frontend auth | `feat(auth): add login page with social providers and token refresh` | ✅
| 12 | Tasks 6.1, 6.2 | Auth docs | `docs(auth): document authentication setup and configuration` | ✅

**Total tasks:** 18 (across 7 phases)
**Estimated commits:** 12
**Completed:** 18/18 ✅
