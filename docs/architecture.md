[← Getting Started](getting-started.md) · [Back to README](../README.md) · [User Interface Guide →](gui.md)

# Architecture

## System Overview

The system uses a seven-service microservices architecture (including PostgreSQL for KeyCloak). The **backend** is the orchestrator — it accepts user requests, coordinates retrieval from Chroma, calls the embedding service, and streams LLM responses. All inter-service communication happens over Docker's internal bridge network.

```mermaid
graph TB
    subgraph User
        B[Browser]
    end

    subgraph "Docker Network"
        K[KeyCloak<br/>OIDC Provider]
        F[Frontend<br/>Vue 3 + TypeScript]
        BE[Backend<br/>Rust / axum]
        ES[Embedding Service<br/>Python / FastAPI]
        CDB[(Chroma<br/>Vector DB)]
        SQL[(SQLite<br/>Metadata)]
        PDB[(PostgreSQL<br/>KeyCloak DB)]
    end

    B -- "HTTP 80" --> F
    B -- "Auth redirect" --> K
    F -- "API /api/*" --> BE
    F -- "Token exchange" --> K
    BE -- "JWT validation" --> K
    BE -- "POST /embed" --> ES
    BE -- "Chroma API" --> CDB
    BE -- "sqlx" --> SQL
    K --> PDB

    classDef svc fill:#e3f2fd,stroke:#1565c0
    classDef db fill:#fff3e0,stroke:#e65100
    classDef auth fill:#f3e5f5,stroke:#7b1fa2
    class F,BE,ES svc
    class CDB,SQL,PDB db
    class K auth
```

## Service Breakdown

### 1. Backend (Rust/axum)

REST API server that handles all business logic. Follows the **Structured Modules (Technical Layers)** pattern.

```
backend/src/
├── main.rs              # Entry point, router wiring
├── config.rs            # Environment-based configuration
├── lib.rs               # Re-exports
├── modules/
│   ├── documents/       # Upload, parsing, chunking
│   ├── collections/     # Collection CRUD
│   ├── query/           # RAG pipeline, Q&A
│   ├── conversations/   # Chat sessions, messages
│   ├── auth/            # Auth endpoints (me, logout), UserContext
│   └── git_sync/        # Git repository sync (clone, pull, parse, index)
└── shared/
    ├── auth.rs          # Bearer token middleware, JWT validator
    ├── error.rs         # Unified AppError enum
    ├── llm.rs           # LLM client (RouterAI), streaming + single queries
    ├── bm25.rs          # BM25 keyword search index
    ├── chunking.rs      # Text splitting
    ├── embedding_client.rs  # Embedding service HTTP client
    ├── chroma_client.rs     # Chroma HTTP client
    ├── file_validation.rs   # MIME + magic bytes checks
    ├── rate_limit.rs        # Body size limiting
    └── types.rs             # Shared types
```

**Dependency rules:** `Handlers → Service → Repository`. Layers never skip or reverse.

### 2. Embedding Service (Python/FastAPI)

A lightweight FastAPI service that wraps `sentence-transformers` (BAAI/bge-small-en-v1.5) with disk-based caching via `diskcache`.

```
embedding/src/
├── main.py       # FastAPI app, /embed and /health endpoints
├── models.py     # Pydantic request/response schemas
├── service.py    # EmbeddingService wrapping SentenceTransformer
└── cache.py      # CachedEmbedder with diskcache
```

- **POST /embed** — accepts `{"texts": [...]}`, returns `{"embeddings": [[...]], "model": "..."}`
- Embeddings cached by exact text match using `diskcache`

### 3. Chroma (Vector Database)

Persistent ChromaDB instance storing document chunk vectors. Data persists in a Docker volume (`chroma_data`).

### 4. KeyCloak (OIDC/OAuth2 Identity Provider)

KeyCloak 26 provides authentication via the OAuth 2.0 Authorization Code flow with PKCE. It runs with a dedicated PostgreSQL 16 database (`keycloak-db`).

- **Realm:** `vedo-hub` with three-tier RBAC (`guest`, `user`, `admin`)
- **Clients:** `vedo-frontend` (public, PKCE) and `vedo-backend` (confidential, service accounts)
- **Social Identity Providers:** Yandex, VK ID, Mail.ru (optional, enabled via env vars)
- **Realm import:** `keycloak/realm-import.json.template` with env var substitution on startup (no credentials in repo)

All authentication is handled exclusively by KeyCloak JWT tokens — the legacy API key mechanism has been removed.

### 5. Frontend (Vue 3 + TypeScript)

Single-page application with five views:

- **Chat view** (`/`) — streaming Q&A interface with message history
- **Admin view** (`/admin`) — document upload and collection management
- **Login view** (`/login`) — social login via KeyCloak (Yandex, VK ID, Mail.ru)
- **Callback view** (`/callback`) — OIDC callback handler (PKCE code exchange)
- **Avatar Preview view** (`/avatar-preview`) — UI component playground

```
frontend/src/
├── main.ts
├── App.vue
├── assets/
│   ├── design-tokens.css    # Full design system CSS (from ui-kit.lib.pen)
│   └── chat-tokens.css      # Chat-specific CSS custom properties
├── api/
│   ├── client.ts            # API client with Bearer token support
│   ├── types.ts             # TypeScript interfaces (document, collection, session, git-sync, etc.)
│   └── auth.ts              # Token storage in localStorage
├── components/
│   ├── ui/                  # Atomic UI components (Pencil design system)
│   │   ├── VButton.vue      # 5 variants (primary/outline/ghost/small/destructive)
│   │   ├── VInput.vue       # Text input with design tokens
│   │   ├── VSelect.vue      # Custom dropdown select
│   │   ├── VDialog.vue      # Modal dialog (420px, 16px radius)
│   │   ├── VAvatar.vue      # User/assistant avatar (3 sizes)
│   │   ├── VBadge.vue       # Status badge (sm/xs, 4 variants)
│   │   ├── VLabel.vue       # Field label with required state
│   │   ├── VProgressBar.vue # Animated progress bar
│   │   ├── VDropZone.vue    # File drop zone (drag & drop)
│   │   ├── VThemeToggle.vue # Dark/light theme toggle
│   │   └── VToast.vue       # Toast notification (auto-dismiss)
│   ├── AppHeader.vue        # Top navigation bar
│   ├── LoginButtons.vue     # Social login provider buttons
│   ├── ChatWindow.vue       # Chat message list + input
│   ├── MessageBubble.vue    # Single message with citations
│   ├── SessionDebug.vue   # Admin session debug panel with pipeline visualization
    │   ├── DocumentList.vue     # Uploaded documents list
│   ├── CollectionManager.vue  # Collection CRUD
│   └── GitRepoManager.vue   # Git repo connect, sync, delete
├── stores/
│   ├── chat.ts              # Pinia store for chat state
│   ├── documents.ts         # Documents store
│   ├── collections.ts       # Collections store
│   └── auth.ts              # Auth/user store
├── composables/
│   ├── useOidcAuth.ts       # PKCE OIDC auth flow
│   ├── useStreamingChat.ts  # SSE streaming composable
│   └── useTheme.ts          # Theme switching
├── utils/
│   └── markdown.ts          # Markdown renderer with highlight.js & GFM
└── views/
    ├── ChatView.vue         # Main chat page
    ├── AdminView.vue        # Admin panel
    ├── LoginView.vue        # Login with social providers
    ├── CallbackView.vue     # OIDC callback handler
    └── AvatarPreviewView.vue # UI component preview
```

## Document Ingestion Flow

When an administrator uploads a document, the backend orchestrates validation, parsing, chunking, embedding, and indexing. All chunks are embedded in parallel batches and stored in Chroma alongside SQLite metadata.

```mermaid
sequenceDiagram
    participant U as Admin
    participant F as Frontend
    participant B as Backend
    participant E as Embedding
    participant C as Chroma
    participant S as SQLite

    U->>F: Uploads file (Admin view)
    F->>B: POST /api/documents/upload<br/>(multipart: file + collection_id)
    B->>B: Validate file<br/>(size, extension, magic bytes)
    B->>B: Parse file content<br/>(PDF / DOCX / Markdown)
    B->>B: Chunk text with overlap<br/>(1000 chars, 200 overlap)
    B->>S: Save document metadata
    B->>S: Save chunk records
    loop For each batch of chunks (max 32)
        B->>E: POST /embed (chunk texts)
        E-->>B: Embedding vectors
        B->>C: Add embeddings with metadata
    end
    B-->>F: 201 Created<br/>{document_id, chunks_indexed}
    F-->>U: Document added to list<br/>with chunk count
```

1. Admin uploads a file through the Admin view in the frontend
2. Frontend sends a multipart POST request to the backend
3. Backend validates the file — checks extension whitelist (PDF, MD, DOCX), size limit (50 MB), and magic bytes to confirm file type
4. Backend parses the raw text from the file using format-specific extractors
5. Backend splits the text into overlapping chunks (1000 chars, 200 char overlap) for granular retrieval
6. Backend persists document metadata and chunk records to SQLite
7. Backend sends chunk texts in batches to the embedding service, then stores the resulting vectors in Chroma with document metadata for filtered search
8. Frontend updates the document list with the uploaded file's details

## RAG Pipeline (Query Flow)

When advanced RAG is enabled (`ADVANCED_RAG_ENABLED=true`), the pipeline includes Multi-Query expansion, HyDE (Hypothetical Document Embeddings), hybrid vector + keyword search, deduplication, and LLM reranking.

```mermaid
sequenceDiagram
    participant U as User
    participant F as Frontend
    participant B as Backend
    participant E as Embedding
    participant C as Chroma
    participant L as LLM (RouterAI)

    U->>F: Asks a question
    F->>B: POST /api/query
    Note over B: Multi-Query: generate 3 query variants
    B->>L: query_single (generate variants)
    L-->>B: Variant questions
    Note over B: HyDE: generate hypothetical docs per variant
    B->>L: query_single per variant
    L-->>B: Hypothetical documents
    B->>E: POST /embed (HyDE documents)
    E-->>B: Embedding vectors
    B->>C: Query similar chunks per variant
    C-->>B: Vector chunks with scores
    Note over B: BM25 keyword search
    B->>B: Tokenize query, match inverted index
    Note over B: Merge & dedup unique chunks
    Note over B: LLM reranking
    B->>L: query_single per chunk
    L-->>B: Relevance verdicts
    B->>L: POST chat/completions (top-k chunks)
    L-->>B: Streaming response
    B-->>F: SSE stream
    F-->>U: Rendered answer with citations
```

1. **Multi-Query:** Backend generates 3 query variants via LLM (`query_single`)
2. **HyDE:** For each variant, the LLM generates a hypothetical document that would answer the question
3. **Semantic Search:** HyDE documents are embedded and used to query Chroma for similar chunks
4. **BM25 Keyword Search:** The original query is tokenized and matched against a keyword index for additional relevant chunks
5. **Merge & Dedup:** Vector and keyword results are merged, deduplicated by chunk ID
6. **LLM Reranking:** Each unique chunk is scored by the LLM (брать = keep, пропустить = skip)
7. **Final Answer:** Top-K accepted chunks are used as context for the LLM streaming response

When `ADVANCED_RAG_ENABLED=false`, the pipeline falls back to standard semantic-only search.

### Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `ADVANCED_RAG_ENABLED` | `true` | Enable multi-query, HyDE, BM25, and reranking |
| `RERANK_TOP_K` | `5` | Max chunks to keep after reranking |
| `HYBRID_TOP_K` | `20` | Initial chunks to retrieve per search pass |
| `MULTI_QUERY_COUNT` | `3` | Number of query variants to generate |
| `LLM_RERANK_MODEL` | `anthropic/claude-sonnet-4.6` | LLM model for reranking verdicts |

### Debug Data (Admin Panel)

Each pipeline stage populates the assistant message's `debug_data` JSON field:

- `multi_query` — original query, generated variants, latency
- `hyde` — per-variant hypothetical documents, latency
- `embedding_search` — vector search results, metadata, latency
- `keyword_search` — BM25 tokens, match count, latency
- `merge_dedup` — input counts, dedup stats, source breakdown
- `reranking` — per-chunk verdicts, scores, comments
- `final_answer` — model info, tokens, latency, prompt preview

## See Also

- [Getting Started](getting-started.md) — installation and first run
- [API Reference](api.md) — endpoint details
- [Deployment](deployment.md) — production setup
