[Back to README](../README.md) · [Architecture →](architecture.md)

# Getting Started

## Prerequisites

- **Docker** & **Docker Compose** (v2.22+)
- **Make** (optional — simplifies common tasks)
- **OpenRouter API key** — sign up at [openrouter.ai](https://openrouter.ai) and create a key

## Installation

### 1. Clone the repository

```bash
git clone https://github.com/your-org/vedo-rag-assistant.git
cd vedo-rag-assistant
```

### 2. Configure environment variables

```bash
cp .env.example .env
```

Edit `.env` and set at least these two:

```env
ADMIN_API_KEY=your-secret-api-key
OPENROUTER_API_KEY=sk-or-v1-your-openrouter-key
VEDO_DB_PASSWORD=your-db-password
```

Other variables have sensible defaults — see [Configuration](configuration.md).

### 3. Start all services

```bash
docker compose up -d
```

This starts six services:

| Service | Port (dev) | Description |
|---------|-----------|-------------|
| `chroma` | — | Vector database (internal) |
| `embedding` | `8001` | Python embedding API |
| `backend` | `3000` | Rust REST API |
| `frontend` | `5173` (dev) | Vue 3 web interface |
| `keycloak` | `8080` | OIDC/OAuth2 identity provider |
| `db` | — | PostgreSQL for metadata + auth |

### 4. Verify it works

```bash
curl http://localhost:3000/health
# → OK
```

Open `http://localhost:5173` in your browser. You should see the chat interface.

## First Run

### Development mode

```bash
# Using docker compose directly (override is auto-merged)
docker compose up -d

# Or using Make targets
make dev-up
```

The override file (`docker-compose.override.yml`) is auto-merged and enables hot-reload for all three services:

- Backend auto-restarts on Rust file changes via `cargo watch`
- Embedding service reloads on Python changes via `uvicorn --reload`
- Frontend refreshes via Vite dev server on port `5173`

### Common Make targets

```bash
make test       # Run all tests (backend + frontend + embedding)
make lint       # Run all linters
make format     # Format all code
make check      # Format + lint + test (fail-fast)
```

## Next Steps

1. Create a collection in the admin panel (`/admin`)
2. Upload documents (PDF, Markdown, or DOCX)
3. Ask questions in the chat interface

## See Also

- [Architecture](architecture.md) — service overview and data flow
- [API Reference](api.md) — REST API endpoints
- [Configuration](configuration.md) — environment variables reference
