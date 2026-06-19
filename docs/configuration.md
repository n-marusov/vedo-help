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
| `GIT_CLONE_ROOT` | Root directory for cloned git repositories | `data/git-repos` |
| `GIT_SYNC_INTERVAL_SECS` | Git sync polling interval in seconds (0 = disabled) | `0` |
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
