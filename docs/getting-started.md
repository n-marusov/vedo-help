[Back to README](../README.md) · [Architecture →](architecture.md)

# Getting Started

## Prerequisites

- **Docker** & **Docker Compose** (v2.22+)
- **Make** (optional — simplifies common tasks)
- **RouterAI API key** — sign up at [routerai.ru](https://routerai.ru) and create a key

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

Edit `.env` and set at least this:

```env
LLM_API_KEY=sk-or-v1-your-routerai-key
```

Other variables have sensible defaults — see [Configuration](configuration.md).

### 3. Start all services

```bash
docker compose up -d
```

This starts the core development stack:

| Service | Port (dev) | Description |
|---------|-----------|-------------|
| `chroma` | internal | Vector database |
| `backend` | `3000` | Rust REST API and RAG orchestrator |
| `frontend` | `5173` (dev override) | Vue 3 web interface |
| `db` | internal | PostgreSQL 16 for app metadata and KeyCloak |
| `keycloak-init` | — | Realm template validation/substitution |
| `keycloak` | `8080` | OIDC/OAuth2 identity provider |
| `otel-collector` | internal | OpenTelemetry collector |

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

The override file (`docker-compose.override.yml`) is auto-merged and enables hot-reload for application services:

- Backend auto-restarts on Rust file changes via `cargo watch`
- Frontend refreshes via Vite dev server on port `5173`

### Common Make targets

```bash
make test       # Run backend/frontend tests configured by the project
make lint       # Run Rust clippy and frontend Biome checks
make format     # Format Rust and frontend code
make check      # Format + lint + test (fail-fast)
```

## Next Steps

1. Create a collection in the admin panel (`/admin`)
2. Add sources: upload documents, connect a Git repository, or start a web crawl
3. Ask questions in the chat interface

## See Also

- [Architecture](architecture.md) — service overview and data flow
- [API Reference](api.md) — REST API endpoints
- [Configuration](configuration.md) — environment variables reference
