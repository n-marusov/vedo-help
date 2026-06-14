# Plan: VEDO hub RAG Assistant — Full Implementation

**Branch:** `feature/rag-assistant-full`
**Created:** 2026-06-14
**Type:** feature

## Settings

| Parameter | Value |
|-----------|-------|
| Testing | Yes — include tests |
| Logging | Verbose — DEBUG logs for all components |
| Documentation | Yes — mandatory docs checkpoint at completion |
| Git branch | `feature/rag-assistant-full` |

## Roadmap Linkage

| Milestone | Rationale |
|-----------|-----------|
| v0.1 — MVP (базовый RAG pipeline) | Полная реализация RAG Q&A системы с нуля: все 4 сервиса, CI, Docker Compose, документация |

---

## Architecture

**Pattern:** Structured Modules (Technical Layers)
**Services:** 4 (backend Rust/axum, embedding Python/FastAPI, chroma, frontend Vue 3)
**Communication:** REST (HTTP between backend ↔ embedding, backend ↔ Chroma), SSE (backend → frontend)
**Database:** SQLite (metadata, conversations), Chroma (vector search)
**Deployment:** Docker Compose with Caddy reverse proxy

## Phases & Tasks

---

### Phase 1: Project Scaffolding & Shared Infrastructure

Creates the project skeleton for all 4 services, CI, configuration, and shared traits/types.

- [x] Task 1.1 — Backend Rust project skeleton

**Files to create:**
- `backend/Cargo.toml` — all dependencies per spec (axum, tokio, serde, serde_json, reqwest, uuid, chrono, tracing, tracing-subscriber, tower-http, tower-governor, sqlx, text-splitter, pdf-extract, docx-rs, axum-extra) plus dev-dependencies: `mockall`, `rstest`, `reqwest` with `mock` feature, `tempfile`
- `backend/Dockerfile` — multi-stage build (builder + runtime distroless)
  - Use `rust:<local-version>-slim-bookworm` that matches the `Cargo.lock` version 4 format (requires Rust ≥ 1.78); match local toolchain via `rustc --version`
- `backend/.dockerignore`
- `backend/src/main.rs` — axum server startup, graceful shutdown, router wiring placeholder, health check (`GET /health` → 200 OK)
  - Prefix unused variables with `_` to avoid clippy warnings in release builds (e.g. `_chroma_client`, `_query_repo`)
- `backend/src/config.rs` — `AppConfig` struct loaded from env vars: `ADMIN_API_KEY`, `DATABASE_URL`, `EMBEDDING_SERVICE_URL`, `CHROMA_URL`, `OPENROUTER_API_KEY`, `OPENROUTER_MODEL`, `HOST`, `PORT`, `RUST_LOG`
- `backend/src/lib.rs` — re-exports for integration tests
- `backend/tests/common/mod.rs` — `fn setup_test_db() -> SqlitePool` (in-memory SQLite), `fn setup_test_config() -> AppConfig` helper functions for unit/integration tests

**Logging requirements:**
- App startup logs config (except secrets) at INFO level
- Graceful shutdown logged at INFO
- `main.rs`: `tracing::info!("Starting server on {host}:{port}")`

**Blocked by:** Nothing
**Dependencies:** None

---

- [x] Task 1.2 — Backend shared module — error, auth, types

**Files to create:**
- `backend/src/shared/mod.rs` — module exports
- `backend/src/shared/error.rs` — `AppError` enum (typed errors: NotFound, Unauthorized, BadRequest, InternalError, EmbeddingError, ChromaError, LlmError, FileError, RateLimited) with `IntoResponse` impl returning structured JSON `{ "error": { "type": "...", "message": "..." } }`
- `backend/src/shared/auth.rs` — `AuthToken` extractor that validates `Authorization: Bearer <ADMIN_API_KEY>` header, returns 401 on failure
- `backend/src/shared/rate_limit.rs` — rate limiter setup using `tower-governor` (`GovernorLayer`), 30 requests/min per IP
- `backend/src/shared/types.rs` — shared types: `ChunkData { text: String, index: usize }`, `Embedding { vector: Vec<f32> }`
- `backend/src/shared/mod.rs` — public re-exports

**Logging requirements:**
- Auth failures logged at WARN level: `warn!("Unauthorized request from {remote_addr}")`
- Rate limit exceeded logged at WARN: `warn!("Rate limit exceeded for {ip}")`

**Blocked by:** Task 1.1
**Dependencies:** Task 1.1 must exist (Cargo.toml with dependencies)

---

- [x] Task 1.3 — Backend shared: LLM client, Chroma client, chunking, file validation

**Files to create:**
- `backend/src/shared/llm.rs` — `OpenRouterClient` struct with:
  - `async fn query_stream(prompt, chunks, conversation_history) -> impl Stream<Item = Result<String>>` — POST to OpenRouter API with streaming response
  - Retry logic: 3 retries with 1s delay on 5xx/429
  - `const PRIMARY_MODEL`, `const MAX_RETRIES`, `const RETRY_DELAY_MS`, `const SYSTEM_PROMPT` per spec
- `backend/src/shared/chroma_client.rs` — `ChromaClient` struct (HTTP-клиент для Chroma REST API):
  - `async fn add_embeddings(collection, ids, embeddings, metadatas)` — POST `/api/v1/collections/{name}/add`
  - `async fn query(collection, embedding, top_k) -> Vec<ChromaResult>` — POST `/api/v1/collections/{name}/query`
  - `async fn create_collection(name)` — POST `/api/v1/collections`
  - `async fn delete_collection(name)` — DELETE `/api/v1/collections/{name}`
  - `async fn delete_document(collection, ids)` — POST `/api/v1/collections/{name}/delete`
  - URL base из `AppConfig.chroma_url`, HTTP-клиент через `reqwest::Client`
  - Retry-логика: 3 retries, 500ms delay на 5xx/connection errors
- `backend/src/shared/chunking.rs` — `fn chunk_document(text: &str) -> Vec<ChunkData>`:
  - Uses `text-splitter` crate for semantic chunking
  - `const CHUNK_SIZE: usize = 1000`, `const CHUNK_OVERLAP: usize = 200`
  - Returns `Vec<ChunkData>` with index preservation
- `backend/src/shared/file_validation.rs` — `fn validate_file(content: &[u8], filename: &str) -> Result<FileType, AppError>`:
  - Supported types: PDF (`.pdf`), Markdown (`.md`), DOCX (`.docx`)
  - MIME check + magic bytes validation
  - Max file size: 50 MB

**Tests:**
- Unit test for `chunk_document`: verify chunk sizes, overlap, no data loss
- Unit test for `validate_file`: valid PDF header, valid DOCX header, invalid file rejection
- Unit test for LLM retry logic (mock HTTP server via `reqwest::mock`)
- Unit test for `ChromaClient.add_embeddings`: verify correct request body (mock HTTP)
- Unit test for `ChromaClient.query`: verify correct parsing of response
- Unit test for Chroma retry logic

**Logging requirements:**
- LLM: `debug!("LLM request: {n_chunks} chunks, model={model}")`, `error!("LLM request failed after {retries} retries: {err}")`
- Chroma: `debug!("Chroma query: collection={collection}, top_k={k}")`, `error!("Chroma request failed after {retries} retries: {err}")`
- Chunking: `debug!("Document chunked into {n_chunks} chunks (size={chunk_size}, overlap={overlap})")`
- File validation: `info!("Validated file: {filename} ({file_type}, {size} bytes)")`, `warn!("File rejected: {filename} - {reason}")`

**Blocked by:** Task 1.2 (uses AppError, AuthToken, types)
**Dependencies:** Task 1.2

---

- [x] Task 1.4 — Python embedding service skeleton

**Files to create:**
- `embedding/requirements.txt` — fastapi, uvicorn, pydantic, sentence-transformers, torch, numpy, typing-extensions, diskcache + dev: pytest, pytest-asyncio, httpx, ruff, coverage
- `embedding/pyproject.toml` — project metadata with ruff config (line-length 100, select ALL, ignore E501), pytest config (testpaths = ["tests"]), coverage config
- `embedding/Dockerfile` — Python 3.11-slim, installs requirements, runs uvicorn
- `embedding/.dockerignore`
- `embedding/src/__init__.py`
- `embedding/src/main.py` — FastAPI app with `POST /embed` endpoint, startup model loading, health `GET /health`
- `embedding/src/models.py` — `EmbedRequest(texts: List[str])`, `EmbedResponse(embeddings: List[List[float]], model: str)`
- `embedding/src/service.py` — `EmbeddingService` class wrapping `sentence-transformers` model, `def embed(texts: list[str]) -> list[list[float]]`
- `embedding/src/cache.py` — disk-based embedding cache using `diskcache`, `CachedEmbedder` class with `get(key)`, `set(key, value)`, cache hit/miss tracking
- `embedding/tests/__init__.py`
- `embedding/tests/conftest.py` — pytest fixtures: `test_client` (FastAPI TestClient wrapping the app), `sample_texts`

**Утилиты качества (закладываются сразу):**
- `ruff check embedding/src/` — линтер (в pyproject.toml)
- `ruff format embedding/src/` — форматтер
- `pytest embedding/tests/ -v --cov=src` — тесты с coverage

**Tests:**
- Unit test for cache: verify cache hit returns stored value, cache miss delegates to model
- Unit test for service: verify embedding shape matches input count
- Smoke test via TestClient: `POST /embed` returns 200 with correct shape
- Health check test: `GET /health` returns 200

**Blocked by:** Nothing
**Dependencies:** None

---

- [x] Task 1.5 — Vue 3 frontend skeleton

**Files to create:**
- `frontend/package.json` — Vue 3, TypeScript, Vite, Pinia, marked (markdown rendering), highlight.js + devDependencies: @biomejs/biome, vitest, @vue/test-utils, jsdom
- `frontend/biome.json` — singleQuote, semi, trailingComma all, lineWidth 100, recommended linter rules
- (ESLint and Prettier removed — replaced by Biome)
- `frontend/vitest.config.ts` — jsdom environment, setup file
- `frontend/Dockerfile` — multi-stage (node:20-alpine build + nginx:alpine runtime)
  - Must `COPY package.json package-lock.json ./` before `RUN npm ci` (Docker requires lockfile for `npm ci`)
- `frontend/.dockerignore`
- `frontend/vite.config.ts` — proxy `/api` to backend during dev, SPA fallback
- `frontend/tsconfig.json`
- `frontend/index.html`
- `frontend/src/main.ts` — Vue 3 app creation, Pinia setup
- `frontend/src/App.vue` — root layout shell with router-view, sidebar placeholder

**Scripts in `package.json`:**
- `"lint": "biome check ."`
- `"lint:fix": "biome check --write ."`
- `"format": "biome format --write ."`
- `"format:check": "biome format ."`
- `"lint:ci": "biome ci ."`
- (ESLint and Prettier replaced by Biome)
- `"test": "vitest run"`
- `"test:watch": "vitest"`
- `"dev": "vite"`
- `"build": "vue-tsc --noEmit && vite build"` (type-check + build)

**Logging requirements:** None (frontend)

**Blocked by:** Nothing
**Dependencies:** None

---

- [x] Task 1.6 — CI pipeline (GitHub Actions) — multi-service

**Files to create:**
- `.github/workflows/ci.yml` — multi-job pipeline covering all 3 services:
  - Trigger: push/pull_request to main
  - Jobs:
    1. **backend** (runs-on: ubuntu-latest):
       - actions/checkout@v4
       - Cache cargo registry
       - Setup Rust stable with rustfmt, clippy
       - `cargo fmt --check`
       - `cargo clippy -- -D warnings`
       - `cargo test --lib`
       - `cargo test --test integration -- --ignored`
    2. **embedding** (runs-on: ubuntu-latest):
       - Setup Python 3.11
       - `pip install -r embedding/requirements.txt`
       - `ruff check embedding/src/`
       - `ruff format embedding/src/ --check`
       - `pytest embedding/tests/ -v --cov=src --cov-report=term`
    3. **frontend** (runs-on: ubuntu-latest):
       - Setup Node 20
       - `cd frontend && npm ci`
       - `npm run lint`
       - `npm run test -- --run`
       - `npm run build` (type-check + production build)
  - Optional: `coverage` job that aggregates coverage reports (informational only)

**Blocked by:** Task 1.1 (Cargo.toml), Task 1.4 (Python tools), Task 1.5 (Frontend tools)
**Dependencies:** All Phase 1 project skeletons

---

- [x] Task 1.7 — Developer tooling: Makefile, rust-toolchain, .gitignore, .editorconfig

**Files to create:**
- `Makefile` — targets:
  - `test` — `cargo test --lib && cd frontend && npm test && cd ../embedding && pytest tests/ -v`
  - `lint` — `cargo clippy -- -D warnings && cd frontend && npm run lint && cd ../embedding && ruff check src/`
  - `format` — `cargo fmt && cd frontend && npm run format && cd ../embedding && ruff format src/`
  - `check` — format + lint + test в последовательности (fail-fast)
  - `coverage` — `cargo tarpaulin --out Xml --target-dir target/coverage 2>/dev/null || echo "[WARN] tarpaulin not installed"`
  - `ci-backend`, `ci-embedding`, `ci-frontend` — отдельные цели для каждого сервиса (такие же команды как в github CI)
  - `.PHONY` для всех целей
- `rust-toolchain.toml`:
  ```toml
  [toolchain]
  channel = "stable"
  components = ["rustfmt", "clippy"]
  ```
- `.gitignore`:
  ```
  target/
  node_modules/
  __pycache__/
  *.pyc
  .env
  *.db
  backups/
  *.egg-info
  dist/
  .ruff_cache/
  coverage/
  .coverage
  ```
- `.editorconfig`:
  ```ini
  root = true
  [*]
  end_of_line = lf
  insert_final_newline = true
  [{*.rs,*.py}]
  indent_style = space
  indent_size = 4
  [{*.ts,*.vue,*.js,*.json,*.yaml,*.yml,*.css,*.html}]
  indent_style = space
  indent_size = 2
  ```

**Blocked by:** Task 1.1, 1.4, 1.5
**Dependencies:** All skeletons exist (knows what dirs/tooling to reference)

---

### Phase 2: Backend Core — Document Management & Embedding

- [x] Task 2.1 — Documents module: data layer

**Files to create/modify:**
- `backend/src/modules/mod.rs` — module declarations
- `backend/src/modules/documents/mod.rs` — module exports
- `backend/src/modules/documents/models.rs` — `Document { id: Uuid, name: String, file_type: String, file_size: i64, uploaded_at: DateTime<Utc>, collection_id: Uuid }`, `UploadResponse { document_id, chunks_indexed, document_name }`, `DocumentSummary { id, name, file_type, file_size, uploaded_at, collection_id }`
- `backend/src/modules/documents/repository.rs` — `DocumentRepository` with:
  - `save_document(doc)` — INSERT into SQLite
  - `get_document(id)` — SELECT by id
  - `list_documents(collection_id)` — SELECT by collection
  - `delete_document(id)` — DELETE from SQLite + call `ChromaClient::delete_document()` (из shared/chroma_client.rs) для удаления векторов
  - Index chunks: после сохранения в SQLite — вызов `ChromaClient::add_embeddings()` для пакетной записи векторов

**SQLite migration** (inline or via sqlx migrations):
```sql
CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    file_type TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    uploaded_at TEXT NOT NULL,
    collection_id TEXT NOT NULL,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);
CREATE TABLE chunks (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    index INTEGER NOT NULL,
    text TEXT NOT NULL,
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);
```

**Tests:**
- Unit test: save and retrieve document from SQLite (in-memory pool)
- Unit test: delete document and verify cascade

**Logging requirements:**
- `info!("Document saved: {id} ({name}, {size} bytes)")`
- `debug!("Document {id}: {n} chunks stored in Chroma")`
- `warn!("Chroma index operation failed: {err}")`

**Blocked by:** Task 1.2 (error types), Task 1.3 (chunking, file validation)
**Dependencies:** Phase 1 shared modules

---

- [x] Task 2.2 — Documents module: parsing and chunking pipeline

**Files to create/modify:**
- `backend/src/modules/documents/service.rs` — `DocumentService` with `process_upload`:
  1. `File::from_multipart(multipart)` — extract file bytes + filename
  2. `validate_file()` — MIME + magic bytes
  3. Parse file using `pdf-extract` for PDF, custom markdown parser, `docx-rs` for DOCX
  4. `chunk_document()` — split text into chunks
  5. `save_document()` in SQLite
  6. Send chunks to embedding service via HTTP
  7. Store embeddings in Chroma via HTTP API
  8. Return `UploadResponse`
- `backend/src/modules/documents/handlers.rs`:
  - `POST /api/documents/upload` — multipart form data, auth required
  - `GET /api/documents` — list documents (optional `?collection_id=`)
  - `DELETE /api/documents/:id` — delete document

**Tests:**
- Unit test: parse a sample markdown file and verify chunk output
- Unit test: reject invalid file type with appropriate error
- Integration test: upload flow (mock embedding + Chroma)

**Logging requirements:**
- `info!("Document uploaded: {filename} ({file_type}, {size} bytes)")`
- `info!("Document parsed: {id} -> {n_chunks} chunks")`
- `debug!("Chunk {idx}: {text_preview}...")` — first 80 chars
- `error!("Upload failed: {err}")`
- `debug!("Embedding request: {n_chunks} chunks")`
- `debug!("Embedding response received: {n_vectors} vectors")`

**Blocked by:** Task 2.1 (models, repository)
**Dependencies:** Task 2.1, Task 1.4 (embedding service must be running)

---

- [x] Task 2.3 — Embedding service: wire up with backend

**Files to modify:**
- `embedding/src/main.py` — add `POST /embed` route, configure CORS, add request logging middleware
- `backend/src/shared/embedding_client.rs` — `EmbeddingClient`:
  - `async fn embed(texts: Vec<String>) -> Result<Vec<Vec<f32>>>`
  - HTTP POST to `{EMBEDDING_SERVICE_URL}/embed`
  - Retry on 5xx/connection errors (3 retries, 500ms delay)
  - URL base из `AppConfig.embedding_service_url`, HTTP-клиент через `reqwest::Client`

**Tests:**
- Python: test `POST /embed` returns correct shape
- Python: test health endpoint
- Rust: unit test EmbeddingClient retry behavior (mock HTTP)

**Logging requirements:**
- `debug!("Embedding request: {n_texts} texts")`
- `info!("Embedding successful: {n_vectors} vectors, model={model}")`
- `warn!("Embedding service unavailable, attempt {attempt}/{max}")`
- `error!("Embedding failed after {attempt} retries: {err}")`

**Blocked by:** Task 1.4, Task 2.2
**Dependencies:** Embedding service + documents module

---

### Phase 3: Query & RAG Pipeline

#### [x] Task 3.1: Query module — RAG pipeline

**Files to create:**
- `backend/src/modules/query/mod.rs`
- `backend/src/modules/query/models.rs` — `QueryRequest { collection_id: Uuid, query: String, session_id: Option<Uuid> }`, `QueryResponse { answer: Stream, sources: Vec<SourceRef>, confidence: f64 }`, `SourceRef { document_id: Uuid, document_name: String, chunk_index: usize, text: String, relevance: f64 }`, `StreamEvent { type: EventType, data: String }` (for SSE)
- `backend/src/modules/query/repository.rs` — `QueryRepository`:
  - `query_chroma(collection_name, embedding, top_k: 5) -> Vec<ChromaResult>` — search Chroma by vector similarity
  - `get_chunks_by_ids(ids) -> Vec<ChunkData>` — fetch full chunk text
- `backend/src/modules/query/service.rs` — `QueryService`:
  1. Receive query + collection_id
  2. Embed query via embedding service
  3. Search Chroma for top-5 similar chunks
  4. Format context: retrieved chunks + system prompt
  5. Stream LLM response via `OpenRouterClient`
  6. Yield SSE events: `type: chunk` (text), `type: sources` (citations), `type: done` (final)
  7. Return structured sources with each chunk
- `backend/src/modules/query/handlers.rs`:
  - `POST /api/query` — accepts `QueryRequest`, returns SSE stream
  - `GET /api/collections/:id/query` — (alt) query within collection context

**Tests:**
- Unit test: format context from chunks (verify prompt assembly)
- Unit test: parse Chroma response into typed results
- Integration test: mock embedding → mock Chroma → mock LLM, verify SSE output

**Logging requirements:**
- `info!("Query: {query_preview}... (collection={collection_id})")`
- `debug!("Query embedded: {query_preview}... -> {dim} dims")`
- `debug!("Chroma returned {n_results} results for query: {query_preview}...")`
- `info!("LLM context: {n_chunks} chunks, {total_chars} chars")`
- `debug!("LLM stream: received {n_tokens} tokens")`
- `warn!("Chroma query returned 0 results for collection {collection_id}")`
- `error!("Query pipeline failed: {err}")`

**Blocked by:** Task 2.3 (embedding client), Task 1.3 (LLM client)
**Dependencies:** Phase 2 embedding + shared LLM

---

#### [x] Task 3.2: SSE streaming implementation

**Files to modify:**
- `backend/src/modules/query/handlers.rs` — implement SSE response with `axum::response::Sse`:
  - Event format: `data: {"type":"chunk","text":"..."}\n\n`
  - Event format: `data: {"type":"sources","sources":[...]}\n\n`
  - Event format: `data: {"type":"error","text":"..."}\n\n`
  - Event format: `data: {"type":"done"}\n\n`
  - `Cache-Control: no-cache` header
  - `Connection: keep-alive` header
  - Client disconnect detection (cancellation token)
- `backend/Cargo.toml` — add `futures` dependency for stream combinators

**Tests:**
- Unit test: verify SSE event serialization format
- Integration test: send query, parse SSE stream, verify at least one chunk + sources event

**Logging requirements:**
- `debug!("SSE stream started: session={session_id}, query_id={query_id}")`
- `info!("SSE stream completed: {n_events} events sent in {elapsed:.2}s")`
- `warn!("SSE client disconnected prematurely")`
- `error!("SSE stream error: {err}")`

**Blocked by:** Task 3.1
**Dependencies:** Query service

---

### Phase 4: Collections & Conversation Management

#### [x] Task 4.1: Collections module

**Files to create:**
- `backend/src/modules/collections/mod.rs`
- `backend/src/modules/collections/models.rs` — `Collection { id: Uuid, name: String, description: Option<String>, created_at: DateTime<Utc>, document_count: i64 }`, `CreateCollectionRequest { name, description? }`, `CollectionSummary { id, name, document_count, created_at }`
- `backend/src/modules/collections/repository.rs` — `CollectionRepository`:
  - `create_collection(collection)` — INSERT, also create Chroma collection
  - `list_collections()` — SELECT all
  - `get_collection(id)` — SELECT by id
  - `delete_collection(id)` — DELETE cascade + drop Chroma collection
  - `get_document_count(id)` — COUNT documents in collection
- `backend/src/modules/collections/service.rs` — `CollectionService`:
  - `create` — validate name uniqueness, create in SQLite + Chroma
  - `delete` — remove from SQLite + drop Chroma collection
  - `list` — fetch collections with document counts
- `backend/src/modules/collections/handlers.rs`:
  - `POST /api/collections` — create
  - `GET /api/collections` — list
  - `GET /api/collections/:id` — get
  - `DELETE /api/collections/:id` — delete

**SQLite migration:**
```sql
CREATE TABLE collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT NOT NULL
);
```

**Tests:**
- Unit test: create and retrieve collection
- Unit test: delete collection (verify SQLite + Chroma drop)
- Unit test: duplicate collection name rejection

**Logging requirements:**
- `info!("Collection created: {name} (id={id})")`
- `info!("Collection deleted: {name} (id={id})")`
- `warn!("Collection not found: {id}")`
- `debug!("Listing collections: {n} found")`

**Blocked by:** Task 1.1 (db pool)
**Dependencies:** Phase 1 shared

---

#### [x] Task 4.2: Conversations module

**Files to create:**
- `backend/src/modules/conversations/mod.rs`
- `backend/src/modules/conversations/models.rs` — `Session { id: Uuid, title: String, collection_id: Option<Uuid>, created_at: DateTime<Utc>, updated_at: DateTime<Utc>, message_count: i64 }`, `Message { id: Uuid, session_id: Uuid, role: String (user|assistant), content: String, sources: Option<Vec<SourceRef>>, created_at: DateTime<Utc> }`, `SessionSummary { id, title, message_count, created_at, updated_at }`
- `backend/src/modules/conversations/repository.rs` — `ConversationRepository`:
  - `create_session(session)` — INSERT
  - `list_sessions()` — SELECT all ordered by updated_at DESC
  - `get_session(id)` — SELECT by id
  - `delete_session(id)` — DELETE cascade
  - `add_message(message)` — INSERT
  - `get_messages(session_id)` — SELECT ordered by created_at
- `backend/src/modules/conversations/service.rs` — `ConversationService`:
  - `create_session` — new session with auto-generated title
  - `list_sessions` — paginated session list
  - `get_session_history` — full message history for session
  - `delete_session` — cleanup session + messages
  - `export_session` — export as JSON
  - `add_message` — store message with optional sources
- `backend/src/modules/conversations/handlers.rs`:
  - `GET /api/sessions` — list sessions
  - `POST /api/sessions` — create session
  - `GET /api/sessions/:id` — get session with messages
  - `DELETE /api/sessions/:id` — delete session
  - `GET /api/sessions/:id/export` — export session history (JSON file)
  - `DELETE /api/sessions` — delete ALL sessions (bulk)

**SQLite migration:**
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL DEFAULT 'New Chat',
    collection_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE SET NULL
);
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    sources TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);
```

**Tests:**
- Unit test: create session, add messages, retrieve history
- Unit test: delete session cascade
- Unit test: export session as JSON

**Logging requirements:**
- `info!("Session created: {id}")`
- `info!("Session deleted: {id} ({n_messages} messages)")`
- `debug!("Session {id}: {n} messages in history")`
- `info!("Session exported: {id} ({n_messages} messages, {size} bytes)")`

**Blocked by:** Task 1.1 (db pool)
**Dependencies:** Collections module (optional FK)

---

### Phase 5: Frontend — Complete SPA

#### [x] Task 5.1: Frontend API layer and stores

**Files to create:**
- `frontend/src/api/client.ts` — fetch wrapper with base URL, auth header, error handling:
  - `setApiKey(key)` — set Bearer token
  - `get<T>(path)`, `post<T>(path, body)`, `del(path)`
  - Error interceptor → `ApiError` type
- `frontend/src/api/types.ts` — TypeScript interfaces mirroring backend DTOs:
  - `Document`, `UploadResponse`, `Collection`, `Session`, `Message`, `SourceRef`, `StreamEvent`, `QueryRequest`, `QueryResponse`
- `frontend/src/stores/chat.ts` — Pinia store:
  - `sendMessage(collectionId, query, sessionId?)` → SSE stream
  - SSE parser: accumulate chunks, update sources, set loading state
  - `messages: Message[]`, `isLoading: boolean`, `activeSessionId: string | null`
  - Persist sessions locally (or fetch from API)
- `frontend/src/stores/documents.ts` — Pinia store:
  - `documents: Document[]`
  - `fetchDocuments(collectionId)`, `uploadDocument(file, collectionId)`, `deleteDocument(id)`
- `frontend/src/stores/collections.ts` — Pinia store:
  - `collections: Collection[]`
  - `fetchCollections()`, `createCollection(name)`, `deleteCollection(id)`

**Tests:** Optional (per spec)
**Blocked by:** Task 1.5 (Vue skeleton)
**Dependencies:** Phase 3 API endpoints exist

---

#### [x] Task 5.2: Frontend — Chat interface

**Files to create:**
- `frontend/src/composables/useStreamingChat.ts` — composable wrapping SSE logic:
  - `streamQuery(query, collectionId, sessionId)` → returns async generator of `StreamEvent`
  - Handles reconnection, error recovery
  - Uses `EventSource` or fetch + ReadableStream
- `frontend/src/components/MessageBubble.vue` — message component:
  - Props: `message: Message`, `isStreaming: boolean`
  - Renders markdown content via `marked`
  - Collapsible source citations with document name + text snippet
  - Typing indicator animation when streaming
- `frontend/src/components/ChatWindow.vue` — main chat area:
  - Virtual-scrolled message list (auto-scroll to bottom)
  - Input box with send button (Enter to send)
  - Collection selector dropdown
  - Session management (new chat, switch session)
  - Loading indicator during streaming
  - Disabled input during streaming
- `frontend/src/views/ChatView.vue` — page-level chat view:
  - Layout: sidebar (session list) + main (ChatWindow)
  - Mobile responsive: sidebar as overlay/drawer

**Tests:** Optional (per spec)
**Blocked by:** Task 5.1 (stores, types)
**Dependencies:** API layer, stores

---

#### [x] Task 5.3: Frontend — Document & collection management

**Files to create:**
- `frontend/src/components/DocumentList.vue`:
  - File list with icons (PDF/MD/DOCX), file size, upload date
  - Upload button → file picker (single and ZIP batch)
  - Per-file delete button with confirmation
  - Drag-and-drop upload area
  - Upload progress indicator
- `frontend/src/components/CollectionManager.vue`:
  - Collection list with document count
  - Create collection dialog (name + description)
  - Delete collection with confirmation (warns about document loss)
  - Active collection highlight
- `frontend/src/views/AdminView.vue`:
  - Layout: collection sidebar + document list
  - Auth-key input for admin operations
  - Toggle between Chat and Admin modes
- `frontend/src/App.vue` — update with navigation between Chat and Admin views

**Tests:** Optional (per spec)
**Blocked by:** Task 5.1 (stores), Task 5.2 (component patterns)
**Dependencies:** Stores, Chat components

---

### Phase 6: Integration, Deployment & Testing

#### [x] Task 6.1: Backend router wiring

**File to modify:**
- `backend/src/main.rs` — wire all modules into axum router:
  ```
  GET  /health
  POST /api/documents/upload        → documents::handlers::upload
  GET  /api/documents               → documents::handlers::list
  DEL  /api/documents/:id           → documents::handlers::delete
  POST /api/collections             → collections::handlers::create
  GET  /api/collections             → collections::handlers::list
  GET  /api/collections/:id         → collections::handlers::get
  DEL  /api/collections/:id         → collections::handlers::delete
  POST /api/query                   → query::handlers::query
  GET  /api/sessions                → conversations::handlers::list_sessions
  POST /api/sessions                → conversations::handlers::create_session
  GET  /api/sessions/:id            → conversations::handlers::get_session
  DEL  /api/sessions/:id            → conversations::handlers::delete_session
  GET  /api/sessions/:id/export     → conversations::handlers::export_session
  DEL  /api/sessions                → conversations::handlers::delete_all_sessions
  ```
- Apply auth middleware (via `AuthToken` extractor) to all `/api/*` routes except health
- Apply rate limiter middleware
- Add CORS layer (allow frontend origin from env var)
- Initialize SQLite pool via sqlx + migrations (auto-run on startup)
- Initialize Chroma HTTP client
- Initialize embedding service HTTP client

**Logging requirements:**
- `info!("Registered {n} routes")` with route list at DEBUG
- `info!("Database connected: {db_url}")`
- `info!("Chroma client connected: {chroma_url}")`
- `info!("Embedding service configured: {embedding_url}")`

**Blocked by:** All Phase 2-4 handlers
**Dependencies:** All module handlers exist

---

#### Task 6.2: Integration tests

**File to create:**
- `backend/tests/integration/mod.rs` — test helpers (spawn app with test DB)
- `backend/tests/integration/documents_test.rs` — test upload flow end-to-end
- `backend/tests/integration/query_test.rs` — test query pipeline end-to-end
- `backend/tests/integration/conversations_test.rs` — test session lifecycle
- `backend/tests/integration/collections_test.rs` — test collection CRUD

**Key integration test** (per spec):
```rust
#[tokio::test]
#[ignore]
async fn test_upload_and_query() {
    // 1. Upload a markdown document
    // 2. Verify upload response
    // 3. Query the collection
    // 4. Verify response contains answer + sources
}
```

**Blocked by:** Task 6.1 (router wiring)
**Dependencies:** All modules wired

---

#### [x] Task 6.3: Docker Compose and deployment config

**Files to create:**
- `docker-compose.yml` — 4 services per spec:
  - `frontend`: build ./frontend, port 80, depends_on backend
  - `backend`: build ./backend, env vars (from .env), volumes (chroma client, db), depends_on embedding, chroma
  - `embedding`: build ./embedding, env vars (model name), volumes (cache dir)
  - `chroma`: image chromadb/chroma:latest, env (IS_PERSISTENT=TRUE), volumes (chroma_data), port 8000
  - Networks: `internal` (bridge)
  - Volumes: `chroma_data`, `embedding_cache`, `db_data`
- `docker-compose.override.yml` (dev) — volume mounts for hot-reload
- `docker-compose.production.yml` (prod) — Caddy reverse proxy
- `.env.example` — all env vars with placeholders:
  ```
  ADMIN_API_KEY=change-me
  DATABASE_URL=sqlite:///data/vedo.db
  OPENROUTER_API_KEY=sk-...
  OPENROUTER_MODEL=anthropic/claude-sonnet-20241022
  EMBEDDING_MODEL=BAAI/bge-small-en-v1.5
  HOST=0.0.0.0
  PORT=3000
  RUST_LOG=vedo_backend=debug
  FRONTEND_URL=http://localhost:5173
  CHROMA_URL=http://chroma:8000
  ```
- `Caddyfile` — reverse proxy config:
  - Route `/api/*` to backend:3000
  - Route `/` to frontend:80
  - Auto TLS via Let's Encrypt (production)
  - Rate limiting headers

**Logging requirements:** Docker compose logging config: `driver: journald`, `options: { tag: "vedo/{{.Name}}" }`

**Blocked by:** All Dockerfiles (Task 1.1, 1.4, 1.5)
**Dependencies:** Dockerfiles exist

---

#### [x] Task 6.4: Backup & restore scripts

**Files to create:**
- `scripts/backup.sh` — per spec:
  - Stop containers
  - Backup SQLite DB (`backups/vedo-$(date +%F).db`)
  - Backup Chroma data directory (`backups/chroma-$(date +%F).tar.gz`)
  - Restart containers
  - Prune backups older than 30 days
- `scripts/restore.sh` — per spec:
  - Arguments: `<db_file>` `<chroma_archive>` (optional)
  - Stop containers
  - Restore SQLite DB
  - Restore Chroma data
  - Restart containers
- `scripts/` directory with executable permissions

**Logging requirements:**
- `echo "[INFO] Backing up SQLite database..."` style output
- Error handling with `set -e` and meaningful error messages

**Blocked by:** Task 6.3 (Docker Compose, path conventions)
**Dependencies:** Docker Compose setup

---

#### Task 6.5: Documentation checkpoint

**Triggered by:** Post-implementation docs checkpoint
**Run:** `/aif-docs` to:
- Update `README.md` with complete setup instructions
- Document all API endpoints
- Document environment variables
- Document deployment steps
- Document backup/restore procedures
- Update `AGENTS.md` with final project structure

**Blocked by:** All implementation tasks
**Dependencies:** Everything implemented

---

## Commit Plan

| # | Tasks | Commit Message | Description |
|---|-------|---------------|-------------|
| 1 | 1.1, 1.2, 1.3, 1.7 | `feat: add backend scaffold with shared infrastructure` | Cargo.toml, main.rs, config, error/auth/types, LLM client, Chroma client, chunking, file validation, Makefile, rust-toolchain, .gitignore, .editorconfig |
| 2 | 1.4 | `feat: add python embedding service with quality tooling` | FastAPI app, sentence-transformers, disk cache, pyproject.toml, pytest, ruff |
| 3 | 1.5 | `feat: add vue 3 frontend skeleton with quality tooling` | Vite config, Pinia, Biome, Vitest, App.vue |
| 4 | 1.6 | `ci: add multi-service CI pipeline` | GitHub Actions with backend, embedding, frontend jobs |
| 5 | 2.1, 2.2 | `feat: implement document upload and indexing pipeline` | Documents module (models, repo, service, handlers), file parsing, chunking, embedding integration |
| 6 | 2.3 | `feat: wire embedding service with backend` | EmbeddingClient, /embed route wiring |
| 7 | 3.1, 3.2 | `feat: implement RAG query pipeline with SSE streaming` | Query module, OpenRouter integration, SSE streaming |
| 8 | 4.1 | `feat: add collection management` | Collections module CRUD |
| 9 | 4.2 | `feat: add conversation history management` | Sessions/messages CRUD, export/delete |
| 10 | 5.1, 5.2, 5.3 | `feat: build complete frontend SPA` | Stores, types, ChatWindow, MessageBubble, DocumentList, CollectionManager, views |
| 11 | 6.1, 6.2 | `test: wire routes and add integration tests` | Router setup, integration test suite |
| 12 | 6.3, 6.4 | `feat: add docker compose deployment and backup scripts` | Compose files, Caddyfile, scripts |
| 13 | 6.5 | `docs: update project documentation` | README, AGENTS.md, API docs |

---

## Dependencies Graph

```
Task 1.1 (Rust skeleton)
├── Task 1.2 (shared: error, auth, types)
│   ├── Task 1.3 (shared: LLM, chunking, validation)
│   │   ├── Task 2.1 (documents: data layer)
│   │   │   └── Task 2.2 (documents: parsing pipeline)
│   │   │       └── Task 2.3 (embedding wiring)
│   │   │           ├── Task 3.1 (query: RAG pipeline)
│   │   │           │   └── Task 3.2 (SSE streaming)
│   │   │           └── Task 6.1 (router wiring)
│   └── Task 4.1 (collections module)
│       └── Task 4.2 (conversations module)
│           └── Task 6.1
├── Task 1.7 (dev tooling) ─── after Task 1.1, 1.4, 1.5
├── Task 1.6 (CI) ─── blocked by Task 1.1, 1.4, 1.5
├── Task 1.4 (embedding service) ─── parallel to backend
│   └── Task 2.3
└── Task 1.5 (frontend skeleton) ─── parallel to backend
    ├── Task 5.1 (API layer & stores)
    │   ├── Task 5.2 (chat interface)
    │   └── Task 5.3 (documents & collection UI)
    └── Task 6.3 (Docker Compose)

Task 6.1 ── Task 6.2 (integration tests) ─── blocked by Task 1.1 (dev-deps)
Task 6.3 ── Task 6.4 (backup scripts)
Task 6.5 (docs checkpoint) ── everything else
```
