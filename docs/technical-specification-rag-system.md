[← C4 Architecture](c4-architecture.md) · [Back to README](../README.md)

# Technical Specification: VEDO hub RAG Assistant

> Current implementation requirements and design decisions for the production Docker Compose stack.

## 1. Purpose

VEDO hub RAG Assistant is a personal/team knowledge assistant for technical documentation. It ingests content from manual uploads, Git repositories, and websites; indexes chunks in Chroma; and answers questions with grounded citations through an LLM gateway.

## 2. Current System Scope

| Area | Requirement | Implementation |
|------|-------------|----------------|
| Authentication | All application routes require login except `/login` and `/callback` | KeyCloak 26 OIDC/OAuth2 with PKCE |
| Authorization | Admin workspace requires admin role | `realm_access.roles` checked in frontend and backend RBAC middleware |
| Metadata storage | Persistent relational metadata | PostgreSQL 16 via `sqlx` migrations |
| Vector storage | Semantic retrieval over document chunks | Chroma `0.6.2` |
| Embeddings | OpenAI-compatible embedding API | RouterAI-compatible `/v1/embeddings` client with LRU cache |
| Answer generation | Streaming grounded answer generation | RouterAI-compatible chat completions via backend SSE |
| Deployment | Single-node deployment | Docker Compose + Caddy reverse proxy |
| Observability | Health, metrics, traces, dashboards | `/health`, `/api/health/deep`, OTel Collector, Prometheus, Grafana, cAdvisor |

## 3. Services

| Service | Image/build | Responsibility |
|---------|-------------|----------------|
| `frontend` | `frontend/Dockerfile` | Vue 3 SPA, KeyCloak login, chat and admin UI |
| `backend` | `backend/Dockerfile` | Rust/axum API, auth middleware, ingestion, RAG pipeline, SSE |
| `db` | `postgres:16-alpine` | PostgreSQL databases for app metadata and KeyCloak |
| `chroma` | `chromadb/chroma:0.6.2` | Vector index storage and semantic search |
| `keycloak-init` | `alpine:3.19` | Realm template validation/substitution |
| `keycloak` | `quay.io/keycloak/keycloak:26.1` | OIDC provider and RBAC roles |
| `otel-collector` | configured in Compose | Receives backend telemetry |
| `caddy` | production compose only | TLS termination, reverse proxy, security headers |
| `prometheus`, `grafana`, `cadvisor` | production compose only | Monitoring stack |

## 4. Functional Requirements

### 4.1 Authentication and Roles

- Users authenticate through KeyCloak Authorization Code Flow with PKCE.
- Backend validates Bearer JWTs for `/api/*` routes except explicitly public health/webhook endpoints.
- Supported realm roles:
  - `guest` — limited read/chat access according to backend RBAC rules.
  - `user` — owns collections, documents, sessions, and sync jobs.
  - `admin` — can access admin routes, global session debug, settings, audit log, and collection management.

### 4.2 Collections and Documents

- Users can create, list, inspect, and delete collections.
- Documents are associated with a collection and owner.
- Manual upload supports PDF, Markdown, DOCX, TXT, CSV, JSON, HTML, and ZIP batches.
- Upload size limit is 50 MB; ZIP archives are limited to 10 files.
- Documents can be soft-deleted and reloaded/re-indexed.
- Chroma collection names must use internal UUIDs, not user display names.

### 4.3 Git Repository Sync

- Users can register Git repositories by URL, branch, optional token, and collection.
- Sync clones or pulls repositories into `GIT_CLONE_ROOT`.
- Supported repository content is parsed, chunked, embedded, and indexed as document source `git`.
- Sync can be triggered manually and queried for status.
- Webhook endpoint is public but authenticated by HMAC signature or token.

### 4.4 Web Crawling

- Users can create web crawl jobs for a collection.
- Crawling uses breadth-first search (BFS) from an entry URL.
- Safeguards:
  - Same-domain enforcement.
  - Optional path-prefix scope.
  - `max_depth`, `max_pages`, and `delay_ms` limits.
  - `robots.txt` checks.
- Crawl jobs expose list/detail/cancel/retry/delete endpoints and SSE progress subscription.
- Crawled pages are indexed as document source `web`.

### 4.5 RAG Query Pipeline

When `ADVANCED_RAG_ENABLED=true`, the query pipeline performs:

1. Conversation context window assembly using `LLM_MAX_HISTORY_MESSAGES` and `LLM_CONTEXT_TOKEN_BUDGET`.
2. Multi-Query expansion (`MULTI_QUERY_COUNT`).
3. HyDE hypothetical-document generation.
4. Embedding search against Chroma.
5. BM25 keyword search with `BM25_K1` and `BM25_B`.
6. Hybrid result fusion with `HYBRID_SEARCH_ALPHA`.
7. Deduplication and reranking.
8. Final answer generation with citations.
9. SSE progress events and final message persistence.

The frontend can recover from page reloads during generation via `GET /api/query/{session_id}/subscribe`.

### 4.6 Conversations

- Sessions store title, owner, collection, pin state, timestamps, and messages.
- Messages support edit and soft-delete behavior.
- Editing a user message starts a corrected query flow and preserves original content for auditability.
- Sessions can be exported as JSON or Markdown.
- Admins can search sessions globally and inspect recorded debug stages.

### 4.7 Admin Operations

Admin-only routes support:

- Global collection listing/deletion.
- Audit log browsing.
- Session search and user filters.
- Runtime settings read/update.
- Model catalog lookup.
- Health, statistics, and chunk browsing through the frontend admin UI.

## 5. Non-Functional Requirements

### 5.1 Security

- No static API-key auth path is used for application routes.
- Secrets are supplied through `.env`/Compose variables and must not be committed.
- KeyCloak realm import is generated from `keycloak/realm-import.json.template` at startup.
- Backend CORS must be restricted to configured frontend/public origins.
- File validation uses extension/MIME/magic-byte checks where applicable.
- Admin APIs require backend role enforcement, not only frontend route guards.

### 5.2 Reliability

- Backend retries PostgreSQL and Chroma startup connections.
- Chroma/LLM/embedding failures are surfaced through structured `AppError` responses and deep healthcheck status.
- Query rate limiting is applied to `/api/query` with role-aware limits.
- Production Compose uses restart policies and healthchecks.
- Backup/restore scripts cover PostgreSQL app data, KeyCloak data, and Chroma volume data.

### 5.3 Observability

- `/health` is a lightweight liveness endpoint.
- `/api/health/deep` checks PostgreSQL, Chroma, embedding API, and LLM connectivity.
- Backend emits tracing/telemetry to OTel Collector.
- Production monitoring includes cAdvisor, Prometheus, and Grafana.
- Audit middleware records API activity for admin review.

### 5.4 Performance and Limits

| Limit | Current value |
|-------|---------------|
| Single upload size | 50 MB |
| ZIP file count | 10 files |
| Chunk size | 1000 characters |
| Chunk overlap | 200 characters |
| Query cache TTL | `QUERY_CACHE_TTL_SECS` default `300` |
| Query cache entries | `QUERY_CACHE_MAX_ENTRIES` default `100` |
| Default query rate window | `QUERY_RATE_LIMIT_WINDOW_SECS` default `60` |

No Kubernetes, autoscaling, or multi-region deployment is required for the current scope.

## 6. API Surface

Primary API groups:

- `/api/auth/*` — current user and logout acknowledgement.
- `/api/collections/*` — collection CRUD, stats, chunk search.
- `/api/documents/*` — upload, ZIP upload, reload, list, delete, batch delete.
- `/api/git-sync/*` — repository registration, sync, status, webhook.
- `/api/web-crawl/*` — crawl jobs and progress SSE.
- `/api/query` and `/api/query/{session_id}/subscribe` — RAG query and recovery SSE.
- `/api/sessions/*` — conversation sessions, export, message edit/delete.
- `/api/admin/*` — admin collection, audit, session, model, and settings routes.
- `/health` and `/api/health/deep` — public health endpoints.

See [API Reference](api.md) for endpoint details.

## 7. Data Model

PostgreSQL migrations define the authoritative schema. Current schema areas include:

| Area | Migration coverage |
|------|--------------------|
| Collections | ownership and metadata |
| Documents | source, file metadata, active/deleted state |
| Chunks | chunk text, index, source metadata |
| Sessions/messages | history, pinning, edit/soft-delete, debug data |
| Git sync | repository registration and status |
| Web crawl | crawl jobs and crawl pages |
| Settings | persisted runtime settings |
| Audit | API audit events |

Chroma stores vector embeddings and chunk metadata. Backend code must use collection UUIDs for Chroma collection names.

## 8. Configuration Requirements

Required production inputs:

- `LLM_API_KEY`
- `POSTGRES_PASSWORD`
- `VEDO_DB_PASSWORD`
- `KEYCLOAK_DB_PASSWORD`
- `KEYCLOAK_ADMIN_PASSWORD`
- `VEDO_BACKEND_CLIENT_SECRET`
- Public URLs/hosts for frontend, backend, and KeyCloak.

Key runtime knobs are documented in [Configuration](configuration.md).

## 9. Testing and Validation

Expected validation layers:

- Rust formatting and linting: `cargo fmt`, `cargo clippy`.
- Rust unit/integration tests under `backend/`.
- Frontend formatting/linting/tests/build: Biome, Vitest, Vue type-check, Vite build.
- Playwright E2E tests for auth, navigation, chat, admin, theme, and RAG flows.
- Docker Compose validation scripts for ports, URLs, migrations, and KeyCloak template.
- Smoke tests for development and production Compose stacks.

See [Testing](testing.md) for manual execution guidance.

## 10. Deployment Requirements

- Development starts with `docker compose up -d` and automatically merges `docker-compose.override.yml`.
- Production uses `docker-compose.yml` + `docker-compose.production.yml`.
- Caddy terminates TLS and proxies browser/API traffic.
- CI/CD builds backend/frontend images and deploys to VPS on `main` according to GitHub Actions workflows.
- Backups should be scheduled with the provided backup timer script.

## 11. Current Design Decisions

| Decision | Status | Rationale |
|----------|--------|-----------|
| PostgreSQL metadata store | Implemented | Multi-user metadata, KeyCloak integration, migrations |
| Managed RouterAI-compatible embeddings | Implemented | Simpler Compose stack and shared API model configuration |
| KeyCloak-only auth | Implemented | Removes static API-key auth and enables social login/RBAC |
| Docker Compose instead of Kubernetes | Intentional | Single-node VPS target |
| Advanced RAG enabled by default | Implemented | Better recall and explainable debug pipeline |
| Monitoring stack in production Compose | Implemented | Operational visibility without Kubernetes |

## See Also

- [Architecture](architecture.md) — current service/module architecture
- [Configuration](configuration.md) — environment variables
- [Deployment](deployment.md) — production setup
