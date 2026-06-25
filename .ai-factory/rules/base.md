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

- **Backend:** `tracing` + `tracing-opentelemetry` with OTLP export to OTel Collector. JSON-formatted stdout for Docker. Use structured attributes (not string interpolation) with semantic keys: `component`, `error`, `user_id`, `document_id`, `collection_id`, etc. All calls must include `component = "module/name"`.
- **Python (Embedding):** `structlog` + `opentelemetry-sdk` with OTLP export. JSON output in production, console output in development. Use structured event names like `"cache.hit"`, `"model.loaded"` with semantic attributes.
- **Frontend (TypeScript/Vue):** `@opentelemetry/sdk-logs` with OTLP HTTP export (no more `console.*` calls). Use `logger.emit()` with `severityText`, `body`, and `attributes` including `component`.
- **All services:** Use semantic attribute keys, include `component` attribute, follow event naming conventions (e.g. `"db.connected"`, `"collection.created"`). Trace context propagation adds `trace_id`/`span_id` via OTLP.
- **Infrastructure:** Docker json-file logging driver with rotation (20m max-size, 5 max-file). OTel Collector scrapes OTLP from all services.
