[← Auth](auth.md) · [Back to README](../README.md) · [Deployment →](deployment.md)

# Configuration

## Environment Variables

Copy `.env.example` to `.env` and set the required values. Most variables have development-safe defaults; replace all `CHANGEME-*` secrets before production use.

### Required

| Variable | Description | Default |
|----------|-------------|---------|
| `LLM_API_KEY` | RouterAI/OpenAI-compatible API key for LLM access | _(empty — no LLM without it)_ |
| `VEDO_BACKEND_CLIENT_SECRET` | KeyCloak confidential client secret | `changeme-vedo-backend-secret` |
| `POSTGRES_PASSWORD` | PostgreSQL superuser password | `CHANGEME-postgres-password` |
| `VEDO_DB_PASSWORD` | Application database password | `CHANGEME-vedo-password` |
| `KEYCLOAK_DB_PASSWORD` | KeyCloak database password | `CHANGEME-keycloak-password` |

### Backend

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://vedo:CHANGEME-db-password@localhost:5432/vedo` (`db:5432` in Compose) |
| `HOST` | Backend bind address | `0.0.0.0` |
| `PORT` / `BACKEND_PORT` | Container listen port / host-published port | `3000` |
| `RUST_LOG` | Logging filter directive | `vedo_backend=debug,tower_http=debug` |
| `LLM_BASE_URL` | Primary OpenAI-compatible API base URL | `https://routerai.ru/api/v1` |
| `LLM_FALLBACK_BASE_URL` | Fallback API base URL used when primary fails | `https://opencode.ai/api/v1` |
| `LLM_MODEL` | Main LLM model identifier | `anthropic/claude-sonnet-4.6` |
| `LLM_API_KEY` | RouterAI/OpenAI-compatible API key | _(required)_ |
| `GIT_CLONE_ROOT` | Root directory for cloned git repositories | `data/git-repos` (`/app/data/git-repos` in Compose) |
| `GIT_SYNC_INTERVAL_SECS` | Git sync polling interval in seconds (0 = disabled) | `0` |
| `LLM_MAX_HISTORY_MESSAGES` | Max conversation history messages to include in LLM context | `20` |
| `LLM_CONTEXT_TOKEN_BUDGET` | Token budget for LLM context window (v0.5: tiktoken-rs BPE tokenizer) | `6000` |
| `DB_CONNECT_RETRIES` | PostgreSQL startup retry count | `30` |
| `CHROMA_CONNECT_RETRIES` | Chroma startup retry count (0 = skip) | `30` |
| `EMBEDDING_API_KEY` | RouterAI API key for embeddings (defaults to `LLM_API_KEY`) | _(inherits from LLM_API_KEY)_ |
| `EMBEDDING_BASE_URL` | RouterAI API base URL for embeddings (defaults to `LLM_BASE_URL`) | `https://routerai.ru/api/v1` |
| `EMBEDDING_MODEL` | RouterAI embedding model identifier | `sentence-transformers/all-minilm-l6-v2` |
| `EMBEDDING_CACHE_SIZE` | Max entries in local embedding LRU cache | `1000` |
| `ADVANCED_RAG_ENABLED` | Enable Multi-Query, HyDE, hybrid search, and reranking | `true` |
| `RERANK_TOP_K` | Chunks kept after reranking | `5` |
| `HYBRID_TOP_K` | Initial chunks to retrieve per search pass | `20` |
| `MULTI_QUERY_COUNT` | Number of query variants to generate | `3` |
| `LLM_RERANK_MODEL` | Model used for reranking | `anthropic/claude-sonnet-4.6` |
| `BM25_K1` | BM25 term frequency saturation parameter | `1.2` |
| `BM25_B` | BM25 length normalization parameter | `0.75` |
| `HYBRID_SEARCH_ALPHA` | Weighted fusion alpha (0.0=pure BM25, 1.0=pure vector) | `0.5` |
| `QUERY_CACHE_TTL_SECS` | Query response cache TTL | `300` |
| `QUERY_CACHE_MAX_ENTRIES` | Max cached query responses | `100` |
| `QUERY_RATE_LIMIT_REQUESTS` | Default query requests per window | `10` |
| `QUERY_RATE_LIMIT_WINDOW_SECS` | Query rate-limit window in seconds | `60` |

| `NOTIFICATION_TELEGRAM_BOT_TOKEN` | Telegram bot token for failure notifications (empty = disabled) | _(empty)_ |
| `NOTIFICATION_TELEGRAM_CHAT_ID` | Telegram chat/channel ID for notifications | _(empty)_ |
| `NOTIFICATION_WEBHOOK_URL` | Generic webhook URL for notifications (empty = disabled) | _(empty)_ |
| `NOTIFICATION_MIN_SEVERITY` | Minimum notification severity (`error`, `warn`, `info`) | `error` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OpenTelemetry OTLP gRPC endpoint | `http://otel-collector:4317` |
| `OTEL_SERVICE_NAME` | Service name for OTel resource attributes | `vedo-backend` |
| `ENVIRONMENT` | Deployment environment (development, production) | `development` |

### Docker Compose

| Variable | Description | Default |
|----------|-------------|---------|
| `IS_PERSISTENT` | Chroma persistence mode | `TRUE` |

### KeyCloak (development only)

KeyCloak is included in the Docker Compose stack. The backend uses two URLs: a public issuer URL that must match the `iss` claim in browser-issued tokens, and an internal JWKS URL used to fetch signing keys from Docker's internal network.

| Variable | Description | Default |
|----------|-------------|---------|
| `KEYCLOAK_DB_PASSWORD` | PostgreSQL password for KeyCloak database | `keycloak` |
| `KEYCLOAK_ADMIN` | KeyCloak admin console username | `admin` |
| `KEYCLOAK_ADMIN_PASSWORD` | KeyCloak admin console password (master realm) | `admin` |
| `KEYCLOAK_HOSTNAME` | KeyCloak hostname | `localhost` |
| `KEYCLOAK_PUBLIC_URL` | Public issuer URL used for JWT `iss` validation | `http://localhost:8080` |
| `KEYCLOAK_JWKS_URL` | Internal URL used by backend to fetch JWKS | `http://keycloak:8080` in Docker Compose |
| `KEYCLOAK_URL` | Backward-compatible fallback for public issuer URL | `http://localhost:8080` |
| `VEDO_BACKEND_CLIENT_SECRET` | Client secret for `vedo-backend` confidential OIDC client | `changeme-vedo-backend-secret` |
| `VEDO_ADMIN_PASSWORD` | vedo-hub realm: admin user password | `admin` |
| `VEDO_ALICE_PASSWORD` | vedo-hub realm: alice user password | `password` |
| `VEDO_GUEST_PASSWORD` | vedo-hub realm: guest user password | `guest` |
| `YANDEX_CLIENT_ID` | Yandex OAuth Client ID (social IdP) | _(empty — disabled)_ |
| `YANDEX_CLIENT_SECRET` | Yandex OAuth Client Secret (social IdP) | _(empty — disabled)_ |
| `VK_CLIENT_ID` | VK ID Client ID (social IdP) | _(empty — disabled)_ |
| `VK_CLIENT_SECRET` | VK ID Client Secret (social IdP) | _(empty — disabled)_ |
| `MAILRU_CLIENT_ID` | Mail.ru OAuth Client ID (social IdP) | _(empty — disabled)_ |
| `MAILRU_CLIENT_SECRET` | Mail.ru OAuth Client Secret (social IdP) | _(empty — disabled)_ |

### Test Users (local dev)

| Username | Password | Roles |
|----------|----------|-------|
| `admin` | `VEDO_ADMIN_PASSWORD` | `admin`, `user`, `guest` |
| `alice` | `VEDO_ALICE_PASSWORD` | `user`, `guest` |
| `guest` | `VEDO_GUEST_PASSWORD` | `guest` |

## Docker Volumes

| Volume | Mount Point | Service | Purpose |
|--------|------------|---------|---------|
| `chroma_data` | `/chroma/chroma` | chroma | Vector index persistence |
| `db_data` | `/var/lib/postgresql/data` | db | PostgreSQL data for application and KeyCloak databases |
| `keycloak_import` | `/opt/keycloak/data/import` | keycloak-init | Generated realm import file |
| `otel_data` | Collector storage directory | otel-collector | OpenTelemetry collector state |
| `git_repos` | `/app/data/git-repos` | backend | Cloned Git repositories for sync jobs |

## File Upload Limits

| Limit | Value |
|-------|-------|
| Single file max size | 50 MB |
| ZIP archive max size | 50 MB |
| ZIP entries limit | 10 files |
| Supported formats | PDF, Markdown, DOCX, TXT, CSV, JSON, HTML, ZIP |
| Chunk size | 1000 characters |
| Chunk overlap | 200 characters |

## Model Selection

The default model is `anthropic/claude-sonnet-4.6` via RouterAI. You can change it to any model available via the RouterAI catalog:

```env
LLM_MODEL=anthropic/claude-sonnet-4.6
LLM_MODEL=openai/gpt-5.2
LLM_MODEL=deepseek/deepseek-v3.2
```

## Context Window

The backend uses `tiktoken-rs` (`cl100k_base` encoding) for accurate BPE token counting in conversation context. Falls back to word-count heuristic only if the tokenizer fails to initialise. The sliding window policy:

1. Drops oldest user+assistant message pairs until both `max_messages` and `token_budget` are satisfied
2. Always preserves at least the 2 most recent messages (1 turn)
3. Configurable via `LLM_MAX_HISTORY_MESSAGES` and `LLM_CONTEXT_TOKEN_BUDGET`

## See Also

- [Getting Started](getting-started.md) — installation guide
- [Deployment](deployment.md) — production configuration
- [API Reference](api.md) — authentication details
