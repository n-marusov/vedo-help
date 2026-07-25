# VEDO hub RAG Assistant

> A personal Q&A system for technical documentation with RAG (Retrieval-Augmented Generation).

## Overview

VEDO hub RAG Assistant ingests documents (PDF, Markdown, DOCX), indexes them in a vector database, and answers user questions using an LLM via RouterAI. Every answer includes citations. The system is designed for single-developer VPS deployment with Docker Compose.

## Core Features

- **Document Upload:** Upload PDF, Markdown, and DOCX files (single or ZIP batch, up to 50 MB)
- **Indexing Pipeline:** Parse, chunk, embed (via RouterAI API), and index into Chroma vector DB
- **Question-Answer Interface:** Streaming LLM responses with grounded citations and source references
- **Collection Management:** Create, delete, and switch between document collections
- **Conversation History:** Persistent storage with session management and history export/deletion
- **Developer Ergonomics:** GitHub Actions CI with formatting, linting, and test automation

## Tech Stack

- **Backend:** Rust (axum framework, sqlx, tokio, serde, tracing, jsonwebtoken, git2, hmac, sha2)
- **Embeddings:** RouterAI API (OpenAI-compatible `/v1/embeddings`, no local Python embedding container)
- **Vector Database:** Chroma (chromadb/chroma:latest)
- **Frontend:** Vue 3 + TypeScript (streaming responses via SSE, DeepSeek-style chat UI)
- **Testing:** Vitest + @vue/test-utils + jsdom
- **Design Tokens:** CSS custom properties (chat-tokens.css) for spacing, colors, animations
- **Database:** PostgreSQL 16 (via sqlx) for metadata and conversation history
- **Deployment:** Docker Compose with Caddy reverse proxy (VPS), systemd backup timer
- **CI/CD:** GitHub Actions (biome check, clippy, unit tests, integration tests, E2E with Docker stack, auto-deploy on main)
- **LLM Gateway:** RouterAI API (configurable model)
- **Authentication:** KeyCloak 26 (OIDC/OAuth2) with PostgreSQL storage
- **Monitoring:** Prometheus + Grafana (cAdvisor container metrics), OTel Collector
- **Load Testing:** k6 (smoke, load, stress, soak scenarios)
- **Health Checks:** Deep healthcheck endpoint with per-service probes and notifications
- **Authorization:** Three-tier RBAC (guest/user/admin)
- **Advanced RAG:** Configurable pipeline with Multi-Query, HyDE, BM25 keyword search, and LLM reranking

## Configuration Variables

Key environment variables for the advanced RAG pipeline:

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://vedo:...@localhost:5432/vedo` | PostgreSQL metadata database URL |
| `CHROMA_URL` | `http://localhost:8000` | Chroma API URL (use `http://chroma:8000` inside Docker) |
| `LLM_BASE_URL` | `https://routerai.ru/api/v1` | Primary OpenAI-compatible LLM API base URL |
| `LLM_FALLBACK_BASE_URL` | `https://opencode.ai/api/v1` | Fallback LLM API base URL |
| `LLM_MODEL` | `anthropic/claude-sonnet-4.6` | Main answer-generation model |
| `EMBEDDING_MODEL` | `sentence-transformers/all-minilm-l6-v2` | RouterAI embedding model |
| `EMBEDDING_API_KEY` | _(inherits from LLM_API_KEY)_ | RouterAI API key for embeddings |
| `EMBEDDING_BASE_URL` | _(inherits from LLM_BASE_URL)_ | RouterAI API base URL for embeddings |
| `EMBEDDING_CACHE_SIZE` | `1000` | Max entries in local embedding LRU cache |
| `ADVANCED_RAG_ENABLED` | `true` | Enable multi-query, HyDE, BM25, and reranking pipeline |
| `RERANK_TOP_K` | `5` | Max chunks to keep after reranking |
| `HYBRID_TOP_K` | `20` | Initial chunks to retrieve per search pass |
| `MULTI_QUERY_COUNT` | `3` | Number of query variants to generate |
| `LLM_RERANK_MODEL` | `anthropic/claude-sonnet-4.6` | LLM model used for reranking |
| `BM25_K1` | `1.2` | BM25 term frequency saturation |
| `BM25_B` | `0.75` | BM25 document-length normalization |
| `HYBRID_SEARCH_ALPHA` | `0.5` | Vector-vs-keyword fusion weight |
| `QUERY_CACHE_TTL_SECS` | `300` | Query response cache TTL |
| `QUERY_CACHE_MAX_ENTRIES` | `100` | Max cached query responses |
| `QUERY_RATE_LIMIT_REQUESTS` | `10` | Default per-window query request limit |
| `QUERY_RATE_LIMIT_WINDOW_SECS` | `60` | Query rate-limit window in seconds |

## Architecture Notes

The system follows a multi-service microservices architecture:

1. **backend** (Rust/axum) — REST API for upload, Git sync, web crawl, query, admin, settings, audit, collection management, and conversation history
2. **db** — PostgreSQL 16 database for application metadata and KeyCloak data
3. **chroma** — Vector database for semantic search
4. **frontend** (Vue 3/TypeScript) — SPA with KeyCloak login and SSE streaming for chat responses
5. **keycloak** — Authentication server (OIDC/OAuth2) with PostgreSQL storage
6. **otel-collector** — OpenTelemetry collector for backend traces/metrics export

Backend is the orchestrator — it receives queries, retrieves chunks from Chroma, and streams LLM answers. Communication between services happens over Docker's internal network.

## Architecture

See `.ai-factory/ARCHITECTURE.md` for detailed architecture guidelines.
**Pattern:** Structured Modules (Technical Layers)

## Non-Functional Requirements

- **Security:** KeyCloak JWT token authentication, file validation (MIME + magic bytes), rate limiting, CORS
- **Logging:** Docker journald driver with structured tags
- **Reliability:** Graceful shutdown, retry logic for embeddings, health check endpoints
- **Data:** PostgreSQL for persistent metadata, Chroma for vector storage, automated backup/restore scripts (pg_dump + tarball)
- **Constraints:** Single-developer scope, no Kubernetes, no performance budgets, no coverage thresholds
