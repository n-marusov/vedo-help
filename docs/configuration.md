[← Auth](auth.md) · [Back to README](../README.md) · [Deployment →](deployment.md)

# Configuration

## Environment Variables

Copy `.env.example` to `.env` and set the required values. All variables have sensible defaults except the two marked **required**.

### Required

| Variable | Description | Default |
|----------|-------------|---------|
| `ADMIN_API_KEY` | Bearer token for API authentication | `change-me` |
| `LLM_API_KEY` | RouterAI API key for LLM access | _(empty — no LLM without it)_ |

### Backend

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | SQLite connection string | `sqlite:/data/vedo.db?mode=rwc` |
| `BACKEND_PORT` | Backend listen port | `3000` |
| `RUST_LOG` | Logging filter directive | `info` |
| `LLM_BASE_URL` | RouterAI API base URL | `https://routerai.ru/api/v1` |
| `LLM_MODEL` | LLM model identifier | `anthropic/claude-sonnet-4.6` |
| `GIT_CLONE_ROOT` | Root directory for cloned git repositories | `data/git-repos` |
| `GIT_SYNC_INTERVAL_SECS` | Git sync polling interval in seconds (0 = disabled) | `0` |
| `LLM_MAX_HISTORY_MESSAGES` | Max conversation history messages to include in LLM context | `20` |
| `LLM_CONTEXT_TOKEN_BUDGET` | Token budget for LLM context window (word-count heuristic) | `6000` |
| `LLM_API_KEY` | RouterAI API key | _(required)_ |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OpenTelemetry OTLP gRPC endpoint | `http://otel-collector:4317` |
| `OTEL_SERVICE_NAME` | Service name for OTel resource attributes | `vedo-backend` |
| `ENVIRONMENT` | Deployment environment (development, production) | `development` |
| `CHROMA_CONNECT_RETRIES` | Chroma startup retry count (30 = ~30s wait, 0 = skip) | `30` |
| `EMBEDDING_CONNECT_RETRIES` | Embedding service startup retry count (30 = ~30s wait, 0 = skip) | `30` |
| `ADVANCED_RAG_ENABLED` | Enable advanced RAG pipeline (multi-query, HyDE, hybrid search, reranking) | `true` |
| `RERANK_TOP_K` | Top-K chunks to keep after LLM reranking | `5` |
| `HYBRID_TOP_K` | Top-K results from BM25 keyword search for hybrid merge | `3` |
| `MULTI_QUERY_COUNT` | Number of query variants to generate in multi-query expansion | `3` |
| `LLM_RERANK_MODEL` | Separate model for LLM reranking (can be cheaper/faster than main model) | _(same as `LLM_MODEL`)_ |

### Embedding Service

| Variable | Description | Default |
|----------|-------------|---------|
| `EMBEDDING_MODEL` | Sentence-transformers model name | `BAAI/bge-small-en-v1.5` |
| `CACHE_DIR` | Disk cache directory for embeddings | `/data/cache` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OpenTelemetry OTLP gRPC endpoint | `http://otel-collector:4317` |
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
| `embedding_cache` | `/data/cache` | embedding | Cached embeddings |
| `db_data` | `/data` | backend | SQLite database file |
| `keycloak_db_data` | `/var/lib/postgresql/data` | keycloak-db | KeyCloak PostgreSQL data |

## File Upload Limits

| Limit | Value |
|-------|-------|
| Single file max size | 50 MB |
| Request body max size | 10 MB |
| Supported formats | PDF, Markdown, DOCX, ZIP |
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

The backend uses a lightweight word-count heuristic for token estimation (no `tiktoken-rs` dependency). The sliding window policy:

1. Drops oldest user+assistant message pairs until both `max_messages` and `token_budget` are satisfied
2. Always preserves at least the 2 most recent messages (1 turn)
3. Configurable via `LLM_MAX_HISTORY_MESSAGES` and `LLM_CONTEXT_TOKEN_BUDGET`

This is a v0.3.1 limitation — revisit with accurate tokenization in v0.5 Advanced RAG.

## See Also

- [Getting Started](getting-started.md) — installation guide
- [Deployment](deployment.md) — production configuration
- [API Reference](api.md) — authentication details
