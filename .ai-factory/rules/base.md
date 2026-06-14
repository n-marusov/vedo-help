# VEDO hub RAG Assistant — Base Rules

> Auto-detected conventions from technical specification. Edit as needed once code is scaffolded.

## Naming Conventions

- **Files:** Rust source files use `snake_case.rs`; Python uses `snake_case.py`; Vue components use `PascalCase.vue`
- **Variables:** `snake_case` (Rust), `snake_case` (Python), `camelCase` (TypeScript/Vue)
- **Functions:** `snake_case` (Rust), `snake_case` (Python), `camelCase` (TypeScript/Vue)
- **Types/Classes:** `PascalCase` (Rust structs/enums, Python classes, TypeScript interfaces)
- **API Endpoints:** `kebab-case` under `/api/v1/` (e.g., `/api/v1/documents/upload`)

## Module Structure

- **Backend (Rust):** `src/modules/<feature>/{handlers,service,repository,models}.rs` — feature modules organized by technical layer per [ARCHITECTURE.md](../ARCHITECTURE.md)
- **Embedding (Python):** Service-oriented single-module FastAPI app
- **Frontend (Vue 3 + TS):** Components, stores (Pinia), composables, views

## Error Handling

- **Backend:** Structured JSON error responses with HTTP status codes
- **Python:** HTTPException with descriptive messages
- **Frontend:** Async error handling in composables; user-facing error toasts
- **Validation:** MIME type + magic byte check before file processing; return 415 on mismatch

## Logging

- **Backend:** `tracing` crate with `tracing-subscriber` (structured, async-aware)
- **Python:** `logging` module or FastAPI logging middleware
- **Frontend:** `console` during development
- **Infrastructure:** Docker journald driver with `vedo-{{.Name}}` tag
