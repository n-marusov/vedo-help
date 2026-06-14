# Architecture: Structured Modules (Technical Layers)

> Architecture guidelines for the VEDO hub RAG Assistant project.

## Overview

Structured Modules (Technical Layers) is a lightweight, domain-aware modular architecture. Each module encapsulates a feature area with its own handlers, services, repositories, and models. It enforces rich domain models and interface-based dependency inversion within a simpler folder structure, without the steep learning curve of full Explicit Architecture.

This architecture was chosen because the project is a single-developer system with medium domain complexity (RAG pipeline, document parsing, embeddings, LLM orchestration). Structured Modules provides clear organization and separation of concerns while keeping initial velocity high, with a straightforward migration path to Explicit Architecture if the project grows.

## Decision Rationale

- **Project type:** Personal RAG Q&A system with multi-service Docker deployment
- **Tech stack:** Rust (axum), Python (FastAPI), Vue 3 + TypeScript, Chroma, SQLite
- **Key factor:** Single developer needs clear organization without over-engineering; soft module boundaries with explicit dependency direction

## Folder Structure

```
backend/
├── Cargo.toml
├── Dockerfile
└── src/
    ├── main.rs                    # Entry point, server setup, router wiring
    ├── config.rs                  # App configuration (env vars, secrets)
    ├── lib.rs                     # Re-exports for integration tests
    │
    ├── modules/                   # ── FEATURE MODULES ──
    │   ├── documents/             # Document upload, parsing, chunking
    │   │   ├── handlers.rs        # HTTP handlers (axum)
    │   │   ├── service.rs         # Application service (orchestration)
    │   │   ├── repository.rs      # Data access (SQLite + Chroma)
    │   │   └── models.rs          # Domain models, DTOs
    │   │
    │   ├── collections/           # Collection CRUD
    │   │   ├── handlers.rs
    │   │   ├── service.rs
    │   │   ├── repository.rs
    │   │   └── models.rs
    │   │
    │   ├── query/                 # Question answering, RAG pipeline
    │   │   ├── handlers.rs
    │   │   ├── service.rs
    │   │   ├── repository.rs
    │   │   └── models.rs
    │   │
    │   └── conversations/         # Chat sessions, message history
    │       ├── handlers.rs
    │       ├── service.rs
    │       ├── repository.rs
    │       └── models.rs
    │
    └── shared/                    # ── SHARED (cross-cutting) ──
        ├── error.rs               # Unified error types, error responses
        ├── auth.rs                # Bearer token middleware
        ├── rate_limit.rs          # Rate limiting middleware
        ├── llm.rs                 # OpenRouter client, retry logic
        ├── chunking.rs            # Text splitter, chunk overlap
        ├── file_validation.rs     # MIME check, magic bytes
        └── types.rs               # Shared type definitions

embedding/
├── requirements.txt
├── Dockerfile
└── src/
    ├── main.py                   # FastAPI app entry
    ├── models.py                 # Request/response models
    ├── service.py                # Embedding orchestration
    └── cache.py                  # Embedding cache layer

frontend/
├── package.json
├── Dockerfile
├── vite.config.ts
├── tsconfig.json
└── src/
    ├── main.ts
    ├── App.vue
    ├── components/               # Reusable UI components
    │   ├── ChatWindow.vue
    │   ├── MessageBubble.vue
    │   ├── DocumentList.vue
    │   └── CollectionManager.vue
    ├── stores/                   # Pinia stores
    │   ├── chat.ts
    │   ├── documents.ts
    │   └── collections.ts
    ├── composables/              # Shared composition functions
    │   └── useStreamingChat.ts
    └── views/                    # Page-level views
        ├── ChatView.vue
        └── AdminView.vue
```

## Dependency Rules

- **Strict Downward Flow:** `Handlers → Service → Repository`. Inner layers must never depend on outer layers.
- **No Layer Skipping:** Handlers must not bypass the Service layer to call Repositories directly.
- **Module Isolation:** Modules depend on `shared/` but NOT on each other's internals. Cross-module dependencies use defined public APIs only.

- ✅ `handlers.rs` imports `service.rs`, `shared/error.rs`, `shared/auth.rs`
- ✅ `service.rs` imports `repository.rs`, `models.rs`, `shared/` utilities
- ✅ `repository.rs` imports `models.rs`, `shared/types.rs`
- ❌ `service.rs` importing `handlers.rs`
- ❌ `repository.rs` importing `service.rs` or `handlers.rs`
- ❌ `documents/repository.rs` directly importing `query/repository.rs`

## Layer/Module Communication

- **HTTP → Handlers:** axum router dispatches requests to module handlers
- **Handlers → Service:** Handler calls service method, receives result (Result<T, AppError>)
- **Service → Repository:** Service orchestrates use case: fetch data → call model method → save
- **Service → External APIs:** Service calls LLM client (from shared/) via async reqwest
- **Cross-service:** Backend communicates with embedding service via HTTP (Docker internal network)

## Key Principles

1. **Rich Domain Models:** Models encapsulate their own invariants and rules. Business logic lives in models, not in services. Services only orchestrate: fetch → call model method → save.
2. **Dependency Inversion (lightweight):** Services receive dependencies through function parameters or struct fields. Repository traits encourage testability.
3. **Module Boundaries:** Each module has clear public API surface. Other modules reach internals only through documented interfaces.
4. **Error Handling:** All modules use the shared `AppError` enum. Handlers convert errors to structured JSON responses with HTTP status codes.

## Code Examples

### Handler (Rust/axum)

```rust
use axum::{extract::State, Json};
use uuid::Uuid;

use crate::modules::documents::{service, models::UploadResponse};
use crate::shared::error::AppError;

pub async fn upload_document(
    State(svc): State<service::DocumentService>,
    auth: AuthToken,
    multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    // Authentication is handled by middleware — auth token is already verified
    let file = validate_and_extract_file(multipart).await?;
    let response = svc.process_upload(file).await?;
    Ok(Json(response))
}
```

### Service (Orchestration)

```rust
use crate::modules::documents::{
    repository::DocumentRepository,
    models::{Document, Chunk},
};
use crate::shared::{error::AppError, chunking::chunk_document};

pub struct DocumentService {
    repo: DocumentRepository,
    embedding_client: EmbeddingClient,
}

impl DocumentService {
    pub async fn process_upload(&self, file: UploadedFile) -> Result<UploadResponse, AppError> {
        // 1. Parse file (model method)
        let doc = Document::from_file(file)?;
        
        // 2. Chunk text (shared utility)
        let chunks = chunk_document(&doc.text);
        
        // 3. Save document metadata
        let doc_id = self.repo.save_document(&doc).await?;
        
        // 4. Send chunks to embedding service
        let embeddings = self.embedding_client.embed_chunks(&chunks).await?;
        
        // 5. Store in Chroma
        self.repo.index_chunks(&doc_id, &chunks, &embeddings).await?;
        
        Ok(UploadResponse { document_id: doc_id, chunks_indexed: chunks.len() })
    }
}
```

### Repository (Data Access)

```rust
use sqlx::SqlitePool;
use crate::modules::documents::models::Document;

pub struct DocumentRepository {
    db: SqlitePool,
    chroma_client: ChromaClient,
}

impl DocumentRepository {
    pub async fn save_document(&self, doc: &Document) -> Result<Uuid, sqlx::Error> {
        sqlx::query("INSERT INTO documents (id, name, file_type, file_size, uploaded_at) VALUES (?, ?, ?, ?, ?)")
            .bind(doc.id)
            .bind(&doc.name)
            .bind(&doc.file_type)
            .bind(doc.file_size)
            .bind(doc.uploaded_at)
            .execute(&self.db)
            .await?;
        Ok(doc.id)
    }
}
```

## Anti-Patterns

- ❌ **Anemic Domain Models:** Models that are just "data bags" (only fields, no behavior). Push validation and business rules into model methods.
- ❌ **Layer Skipping:** Handlers calling Repositories directly. Always go through the Service layer.
- ❌ **Fat Services:** Services that contain business logic instead of just orchestrating. Business rules belong in models.
- ❌ **Circular Dependencies:** Module A imports Module B, Module B imports Module A. Use shared types or restructure.
