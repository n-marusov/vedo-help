[← Auth](auth.md) · [Back to README](../README.md) · [Deployment →](deployment.md)

# Configuration

## Environment Variables

Copy `.env.example` to `.env` and set the required values. All variables have sensible defaults except the two marked **required**.

### Required

| Variable | Description | Default |
|----------|-------------|---------|
| `ADMIN_API_KEY` | Bearer token for API authentication | `change-me` |
| `OPENROUTER_API_KEY` | OpenRouter API key for LLM access | _(empty — no LLM without it)_ |

### Backend

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | SQLite connection string | `sqlite:/data/vedo.db?mode=rwc` |
| `BACKEND_PORT` | Backend listen port | `3000` |
| `RUST_LOG` | Logging filter directive | `info` |
| `OPENROUTER_BASE_URL` | OpenRouter API base URL | `https://openrouter.ai/api/v1` |
| `OPENROUTER_MODEL` | LLM model identifier | `openai/gpt-4o-mini` |
| `OPENROUTER_API_KEY` | OpenRouter API key | _(required)_ |

### Embedding Service

| Variable | Description | Default |
|----------|-------------|---------|
| `EMBEDDING_MODEL` | Sentence-transformers model name | `BAAI/bge-small-en-v1.5` |
| `CACHE_DIR` | Disk cache directory for embeddings | `/data/cache` |

### Docker Compose

| Variable | Description | Default |
|----------|-------------|---------|
| `IS_PERSISTENT` | Chroma persistence mode | `TRUE` |

### KeyCloak (development only)

KeyCloak is only included in the dev stack (`docker compose up`). The production stack (`docker-compose.production.yml`) does not include KeyCloak — authentication is handled via `ADMIN_API_KEY` bearer token instead.

| Variable | Description | Default |
|----------|-------------|---------|
| `KEYCLOAK_DB_PASSWORD` | PostgreSQL password for KeyCloak database | `keycloak` |
| `KEYCLOAK_ADMIN` | KeyCloak admin console username | `admin` |
| `KEYCLOAK_ADMIN_PASSWORD` | KeyCloak admin console password | `admin` |
| `KEYCLOAK_HOSTNAME` | KeyCloak hostname | `localhost` |
| `VEDO_BACKEND_CLIENT_SECRET` | Client secret for `vedo-backend` confidential OIDC client | `changeme-vedo-backend-secret` |
| `YANDEX_CLIENT_ID` | Yandex OAuth Client ID (social IdP) | _(empty — disabled)_ |
| `YANDEX_CLIENT_SECRET` | Yandex OAuth Client Secret (social IdP) | _(empty — disabled)_ |
| `VK_CLIENT_ID` | VK ID Client ID (social IdP) | _(empty — disabled)_ |
| `VK_CLIENT_SECRET` | VK ID Client Secret (social IdP) | _(empty — disabled)_ |
| `MAILRU_CLIENT_ID` | Mail.ru OAuth Client ID (social IdP) | _(empty — disabled)_ |
| `MAILRU_CLIENT_SECRET` | Mail.ru OAuth Client Secret (social IdP) | _(empty — disabled)_ |

### Test Users (local dev)

| Username | Password | Roles |
|----------|----------|-------|
| `admin` | `KEYCLOAK_ADMIN_PASSWORD` | `admin`, `user`, `guest` |
| `alice` | `password` | `user`, `guest` |
| `guest` | `guest` | `guest` |

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

The default OpenRouter model is `openai/gpt-4o-mini`. You can change it to any model available via OpenRouter:

```env
OPENROUTER_MODEL=openai/gpt-4o
OPENROUTER_MODEL=google/gemini-pro-1.5
OPENROUTER_MODEL=anthropic/claude-3-haiku
```

## See Also

- [Getting Started](getting-started.md) — installation guide
- [Deployment](deployment.md) — production configuration
- [API Reference](api.md) — authentication details
