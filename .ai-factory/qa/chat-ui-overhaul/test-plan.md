# Test Plan: Chat UI Overhaul + Authentication

> Based on `change-summary.md` and plan `feature-chat-ui-overhaul.md`

## 1. Test Scope

### In Scope

| Area | Description | Priority |
|------|-------------|----------|
| **Design Tokens** | CSS custom properties definition, variable naming, theme consistency | Medium |
| **Avatar Component** | Rendering, initials extraction, color assignment, size variants, online status | High |
| **MessageBubble Redesign** | User/assistant messages, markdown rendering, source citations, streaming state | High |
| **Typing Indicator** | Animation, timing, visibility during streaming, cleanup on completion/error | Medium |
| **ChatWindow Layout** | Streamlined header, collection selector, messages area, input area | High |
| **Message Animations** | Smooth enter/exit animations, scroll behavior, performance | Medium |
| **Responsive Layout** | Breakpoints (768px), mobile/tablet/desktop views, input accessibility | High |
| **Admin Nav Removal** | App.vue layout change, routing behavior, no regressions for admin page | High |
| **Auth Middleware (Backend)** | JWT validation, token expiry, signature verification, audience/issuer checks | High |
| **Auth Endpoints (Backend)** | Login, token refresh, logout, user info, session management | High |
| **Login Page (Frontend)** | Social provider buttons (Google, GitHub, Discord), redirect flow, error states | High |
| **Token Refresh** | Automatic refresh on 401, silent refresh, expired session handling | High |
| **Docker Compose** | KeyCloak startup, PostgreSQL init, health checks, network connectivity | Medium |

### Out of Scope

- Unit tests for data layer (repository tests in Rust) — covered by backend test suite
- Embedding service changes — none planned
- Document upload flow — unchanged
- Collection management UX — unchanged
- Performance/load testing — single-developer scope

## 2. Test Types

| Type | Description | Coverage |
|------|-------------|----------|
| **Unit Tests (Vitest)** | Component rendering, computed properties, helper functions, Pinia store logic | Frontend components & stores |
| **Component Tests (@vue/test-utils)** | Avatar, MessageBubble, ChatWindow, LoginView rendering and interaction | All new/redesigned components |
| **Integration Tests (Playwright)** | Full-page rendering, responsive breakpoints, navigation flows, auth flow | Phases 2-5 |
| **Visual Regression** | Design token application, avatar appearance, source citation styling | CSS custom properties, visual changes |
| **Accessibility** | Keyboard navigation, ARIA labels, focus management, screen reader support | All new/redesigned components |
| **Negative Tests** | Error handling, missing data, invalid tokens, network failures | Auth, streaming, layout |
| **Responsive Tests** | Mobile (375px), tablet (768px), desktop (1440px) viewports | Layout, chat window |

## 3. Verification Checklist

### Phase 1-4: Chat UI Overhaul

| # | Check | Priority | Type |
|---|-------|----------|------|
| 1 | Design tokens are correctly applied across all components | High | Visual |
| 2 | Avatar component renders with correct initials for user/assistant | High | Unit |
| 3 | Avatar component provides consistent color assignment per user | Medium | Unit |
| 4 | Avatar displays online/offline status indicator | Medium | Unit |
| 5 | MessageBubble renders user messages right-aligned with correct styling | High | Component |
| 6 | MessageBubble renders assistant messages left-aligned with correct styling | High | Component |
| 7 | Markdown content (code blocks, lists, links) renders correctly in messages | High | Component |
| 8 | Source citations are collapsible, display document name and relevance | Medium | Component |
| 9 | Typing indicator appears during streaming and disappears on completion | High | Component |
| 10 | Typing indicator animation runs smoothly without layout shift | Low | Visual |
| 11 | ChatWindow header shows collection selector and new chat button | High | Component |
| 12 | Messages area auto-scrolls to bottom on new message | High | Component |
| 13 | Welcome message shows when no messages exist | Medium | Component |
| 14 | Cancel button appears during streaming, stops the request | High | Component |
| 15 | Input textarea is disabled when no collection is selected | Medium | Component |
| 16 | Send button is disabled for empty input | Medium | Component |
| 17 | Message fade-in animation plays smoothly | Low | Visual |
| 18 | Admin navigation is removed from main App.vue layout | High | Integration |
| 19 | Admin page is still accessible via `/admin` route | High | Integration |
| 20 | Chat view is the default landing page (`/`) | High | Integration |
| 21 | Layout is responsive at 375px (mobile) — stacked, full-width inputs | High | Responsive |
| 22 | Layout is responsive at 768px (tablet) — readable, no overflow | High | Responsive |
| 23 | Layout is responsive at 1440px (desktop) — comfortable, max-width constrained | Medium | Responsive |
| 24 | Keyboard navigation works (Tab, Enter, Escape) | Medium | A11y |

### Phase 5: Authentication

| # | Check | Priority | Type |
|---|-------|----------|------|
| 25 | KeyCloak service starts and is healthy in Docker Compose | High | Integration |
| 26 | PostgreSQL for KeyCloak initializes correctly | High | Integration |
| 27 | JWT validation middleware rejects requests with missing token | High | Security |
| 28 | JWT validation middleware rejects requests with expired token | High | Security |
| 29 | JWT validation middleware rejects requests with invalid signature | High | Security |
| 30 | JWT validation middleware accepts requests with valid token | High | Security |
| 31 | User context is correctly extracted from validated JWT | High | Unit |
| 32 | Login page renders three social provider buttons (Google, GitHub, Discord) | High | Component |
| 33 | Clicking a social provider button redirects to KeyCloak login | High | Integration |
| 34 | Auth callback endpoint handles the OAuth2 redirect correctly | High | Integration |
| 35 | Token refresh works silently in the background | High | Integration |
| 36 | Expired session redirects user to login page | High | Integration |
| 37 | Logout clears local tokens and redirects to login | High | Integration |
| 38 | Auth guard redirects unauthenticated users to login page | High | Integration |
| 39 | Token is persisted across page reloads (localStorage/sessionStorage) | Medium | Integration |

## 4. Testing Environment

- **Unit/Component tests:** Vitest + jsdom + @vue/test-utils
- **Integration/E2E tests:** Playwright with Chrome/Chromium
- **Responsive tests:** Playwright device emulation (iPhone SE, iPad, Desktop)
- **Backend auth tests:** Rust `cargo test` with mock JWT tokens
- **Docker Compose tests:** `docker-compose up` health check assertions
