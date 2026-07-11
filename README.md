# VEDO hub RAG Assistant

[![CI](https://img.shields.io/github/actions/workflow/status/vedo-hub/vedo-rag-assistant/ci.yml?branch=main&label=CI)](https://github.com/vedo-hub/vedo-rag-assistant/actions)
![Production](https://img.shields.io/badge/status-production-green)

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

- **Multi-format ingestion** — Upload PDF, Markdown, DOCX, text, HTML, JSON, CSV, and ZIP batches (up to 50 MB)
- **Multi-source indexing** — Ingest manual uploads, Git repositories, and web crawls into Chroma
- **Advanced RAG pipeline** — Multi-Query, HyDE, hybrid vector + BM25 keyword search, reranking, and citations
- **Grounded answers** — Streaming SSE responses with source references from retrieved document chunks
- **Admin operations** — Collections, documents, Git sync, web crawl jobs, health, statistics, and runtime settings
- **Secure access** — KeyCloak OIDC login with guest/user/admin roles and audited API activity

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Backend API | Rust (axum, sqlx, tokio) |
| Embeddings | RouterAI-compatible `/v1/embeddings` API with local LRU cache |
| Vector Database | Chroma |
| Frontend | Vue 3 + TypeScript (Pinia, Vue Router) |
| Metadata Storage | PostgreSQL 16 |
| Authentication | KeyCloak 26 (OIDC/OAuth2, RBAC) |
| Reverse Proxy | Caddy (auto TLS) |
| Observability | OpenTelemetry, Prometheus, Grafana, cAdvisor |
| CI/CD | GitHub Actions |
| LLM Gateway | RouterAI API with fallback base URL |

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
| [Web Crawler](docs/web-crawler.md) | Website ingestion workflow and crawl job API |
| [Monitoring](docs/monitoring.md) | Prometheus, Grafana, cAdvisor, OTel Collector |
| [Runbook](docs/runbook.md) | Production operations and incident response |
| [C4 Architecture](docs/c4-architecture.md) | C4 context, container, component, deployment diagrams |
| [Technical Spec](docs/technical-specification-rag-system.md) | Full system specification and requirements |

## License

MIT
