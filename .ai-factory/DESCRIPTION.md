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
- **Embedding Service:** RouterAI API (OpenAI-compatible `/v1/embeddings`, replaces local Python service)
- **Vector Database:** Chroma (chromadb/chroma:latest)
- **Frontend:** Vue 3 + TypeScript (streaming responses via SSE, DeepSeek-style chat UI)
- **Testing:** Vitest + @vue/test-utils + jsdom
- **Design Tokens:** CSS custom properties (chat-tokens.css) for spacing, colors, animations
- **Database:** SQLite (via sqlx) for metadata and conversation history
- **Deployment:** Docker Compose with Caddy reverse proxy (VPS)
- **CI/CD:** GitHub Actions (biome check, clippy, unit tests, integration tests)
- **LLM Gateway:** RouterAI API (configurable model)
- **Authentication:** KeyCloak 26 (OIDC/OAuth2) with PostgreSQL storage
- **Authorization:** Three-tier RBAC (guest/user/admin)
- **Advanced RAG:** Configurable pipeline with Multi-Query, HyDE, BM25 keyword search, and LLM reranking

## Configuration Variables

Key environment variables for the advanced RAG pipeline:

| Variable | Default | Description |
|----------|---------|-------------|
| Variable | Default | Description |
|----------|---------|-------------|
| `EMBEDDING_MODEL` | `sentence-transformers/all-minilm-l6-v2` | RouterAI embedding model |
| `EMBEDDING_API_KEY` | _(inherits from LLM_API_KEY)_ | RouterAI API key for embeddings |
| `EMBEDDING_BASE_URL` | `https://routerai.ru/api/v1` | RouterAI API base URL for embeddings |
| `EMBEDDING_CACHE_SIZE` | `1000` | Max entries in local embedding LRU cache |
| `ADVANCED_RAG_ENABLED` | `true` | Enable multi-query, HyDE, BM25, and reranking pipeline |
| `RERANK_TOP_K` | `5` | Max chunks to keep after LLM reranking |
| `HYBRID_TOP_K` | `20` | Initial chunks to retrieve per search pass |
| `MULTI_QUERY_COUNT` | `3` | Number of query variants to generate |
| `LLM_RERANK_MODEL` | `anthropic/claude-sonnet-4.6` | LLM model used for reranking |

## Architecture Notes

The system follows a four-service microservices architecture:

1. **backend** (Rust/axum) — REST API for upload, query, collection management, conversation history (embeddings via RouterAI API)
3. **chroma** — Vector database for semantic search
4. **frontend** (Vue 3/TypeScript) — SPA with SSE streaming for chat responses
5. **keycloak** — Authentication server (OIDC/OAuth2) with PostgreSQL storage

Backend is the orchestrator — it receives queries, retrieves chunks from Chroma, and streams LLM answers. Communication between services happens over Docker's internal network.

## Architecture

See `.ai-factory/ARCHITECTURE.md` for detailed architecture guidelines.
**Pattern:** Structured Modules (Technical Layers)

## Non-Functional Requirements

- **Security:** KeyCloak JWT token authentication, file validation (MIME + magic bytes), rate limiting, CORS
- **Logging:** Docker journald driver with structured tags
- **Reliability:** Graceful shutdown, retry logic for embeddings, health check endpoints
- **Data:** SQLite for persistent metadata, Chroma for vector storage, automated backup/restore scripts
- **Constraints:** Single-developer scope, no Kubernetes, no performance budgets, no coverage thresholds
