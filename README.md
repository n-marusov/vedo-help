# VEDO hub RAG Assistant

> AI-powered Q&A system for technical documentation with RAG.

Ingest PDF, Markdown, and DOCX documents — index them in a vector database — ask questions and get grounded answers with citations via an LLM.

## Quick Start

```bash
# 1. Set up environment
cp .env.example .env    # edit LLM_API_KEY

# 2. Start all services
docker compose up -d

# 3. Open the UI
open http://localhost:5173
```

## Key Features

- **Multi-format ingestion** — Upload PDF, Markdown, and DOCX files singly or in ZIP batches (up to 50 MB)
- **RAG pipeline** — Parse, chunk, embed (sentence-transformers), index into Chroma vector DB
- **Grounded answers** — LLM responses with source citations from retrieved document chunks
- **Collection management** — Create, delete, and switch between document collections
- **Conversation history** — Persistent chat sessions with export and deletion
- **Streaming responses** — SSE-based real-time answer generation in the Vue 3 UI

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Backend API | Rust (axum, sqlx, tokio) |
| Embedding Service | Python (FastAPI, sentence-transformers) |
| Vector Database | Chroma |
| Frontend | Vue 3 + TypeScript (Pinia, Vue Router) |
| Metadata Storage | SQLite |
| Reverse Proxy | Caddy (auto TLS) |
| CI/CD | GitHub Actions |
| LLM Gateway | RouterAI API |

---

## Documentation

| Guide | Description |
|-------|-------------|
| [Getting Started](docs/getting-started.md) | Prerequisites, installation, first run |
| [Architecture](docs/architecture.md) | Service overview, modules, data flow |
| [User Interface Guide](docs/gui.md) | Chat interface, admin panel, document management |
| [API Reference](docs/api.md) | Endpoints, authentication, examples |
| [Authentication](docs/auth.md) | KeyCloak setup, social providers, OAuth flow |
| [Configuration](docs/configuration.md) | Environment variables, Docker settings |
| [Deployment](docs/deployment.md) | VPS setup, Docker Compose, CI/CD |
| [Testing](docs/testing.md) | Manual test execution guide |
| [Technical Spec](docs/technical-specification-rag-system.md) | Full system specification (source of truth) |

## License

MIT
