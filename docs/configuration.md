[← API Reference](api.md) · [Back to README](../README.md) · [Deployment →](deployment.md)

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

## Docker Volumes

| Volume | Mount Point | Service | Purpose |
|--------|------------|---------|---------|
| `chroma_data` | `/chroma/chroma` | chroma | Vector index persistence |
| `embedding_cache` | `/data/cache` | embedding | Cached embeddings |
| `db_data` | `/data` | backend | SQLite database file |

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
