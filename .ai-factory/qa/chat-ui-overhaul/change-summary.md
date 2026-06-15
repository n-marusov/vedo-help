# Change Summary: Chat UI Overhaul + Authentication

> Generated from plan `feature-chat-ui-overhaul.md` and codebase analysis.
> Branch: `feature/chat-ui-overhaul` (planned, not yet implemented)

## Overview

Complete redesign of the chat interface with a minimalistic design system, avatar component, smooth animations, responsive layout, and addition of KeyCloak-based authentication with social identity providers.

## Scope

- **Frontend (Vue 3 + TypeScript):** Design tokens, MessageBubble rewrite, ChatWindow/layout redesign, avatar component, animations, responsive behavior, login page
- **Backend (Rust/axum):** JWT validation middleware, auth endpoints, user context
- **Infrastructure (Docker Compose):** KeyCloak + PostgreSQL services added
- **Configuration:** KeyCloak realm setup, social identity providers configuration

## Changed Components

### Frontend

| Component | Current State | Planned Change | Risk Level |
|-----------|--------------|----------------|------------|
| `App.vue` | Has sidebar with admin navigation | Remove admin nav from main layout, streamline to pure chat-focused layout | **High** — structural layout change |
| `ChatWindow.vue` | Current chat with session sidebar + messages + input | Redesign layout, improve collection selector, streamline header | **High** — core UX change |
| `MessageBubble.vue` | Current with emoji avatars, sources toggle, markdown | Minimalistic design, refined streaming/typing indicator | **Medium** — visual only |
| New `Avatar.vue` | — | New component with user initials, color assignment, online status | **Medium** — new component |
| New `LoginView.vue` + `auth store` | — | Login page with social provider buttons (Google, GitHub, Discord) | **High** — new auth flow |
| `stores/chat.ts` | Current Pinia store | Minimal changes (layout integration only) | **Low** |
| `router.ts` | Current `/` and `/admin` | Add auth guard, login route | **High** — auth integration |
| `style tokens` (new) | Hardcoded colors | CSS custom properties (design tokens) | **Medium** — refactoring |

### Backend

| Component | Current State | Planned Change | Risk Level |
|-----------|--------------|----------------|------------|
| `shared/auth.rs` | Bearer token (API key) middleware | KeyCloak JWT validation middleware | **High** — auth replacement |
| New auth module | — | KeyCloak endpoints, user context extraction | **High** — new module |
| Configuration | `ADMIN_API_KEY` env | KeyCloak client/issuer configuration | **High** — config change |

### Infrastructure

| Component | Current State | Planned Change | Risk Level |
|-----------|--------------|----------------|------------|
| `docker-compose.yml` | 4 services | Add KeyCloak + PostgreSQL | **Medium** — service addition |
| `Caddyfile` | Current reverse proxy | Add KeyCloak routes, auth callback | **Medium** |

## Evidence

1. **Current `App.vue`** renders a sidebar with Chat/Admin navigation links — Task 3.1 removes this
2. **Current `MessageBubble.vue`** uses emoji avatars (`👤`/`🤖`) and has a `fadeIn` animation — Task 2.1 rewrites to minimalistic design with proper avatar component
3. **Current `ChatWindow.vue`** has session sidebar, collection selector, messages, and input in a single layout — Task 3.2 redesigns this
4. **Current `auth.rs`** uses simple bearer token middleware with `ADMIN_API_KEY` — Task 5.3 replaces this with KeyCloak JWT validation
5. **No existing `Avatar.vue`** component — Task 1.2 creates it from scratch
6. **No existing `LoginView.vue`** — Task 5.5 creates login page with social provider buttons
7. **No existing design tokens** — colors are hardcoded in component scoped styles — Task 1.1 introduces CSS custom properties

## Assumptions

- KeyCloak will be configured with realm `vedo-hub` and social providers (Google, GitHub, Discord)
- Frontend will use OAuth2 authorization code flow with PKCE
- Token refresh will use refresh tokens from KeyCloak
- Existing API key authentication remains as a fallback during migration (dual auth mode)
