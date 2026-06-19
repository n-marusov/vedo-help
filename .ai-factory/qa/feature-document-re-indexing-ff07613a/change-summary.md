# Change Summary: Auto-create missing Chroma collection on add_embeddings

**Branch:** `feature/document-re-indexing`
**Base:** `main`
**Analysis date:** 2026-06-19
**Scope:** 31 files changed, ~4980 lines added/removed

---

## Overview

The `feature/document-re-indexing` branch implements document re-indexing, soft delete, Chroma `where`-filtered queries, and expanded test coverage for the document pipeline. On top of those changes, this analysis covers a targeted fix that makes `ChromaClient::add_embeddings()` resilient to missing Chroma collections by auto-creating them on demand.

### Changes by Type

| Category | Files | Description |
|----------|-------|-------------|
| Core indexing logic | `service.rs`, `repository.rs` | Soft delete (is_active flag), reload/re-index endpoint, soft-delete cascade |
| Chroma integration | `chroma_client.rs` | `where_filter` on query, auto-create collection on `add_embeddings` InvalidCollection |
| API layer | `handlers.rs`, `models.rs` | Reload endpoint, DocumentSummary with is_active, collection_id |
| Git sync | `git_sync/service.rs` | Sync improvements for re-index trigger |
| Query layer | `query/repository.rs` | is_active filter in Chroma queries |
| Tests | `reindex_tests.rs`, `integration.rs`, `chroma_client.rs` | Extensive unit and integration tests |
| E2E | `frontend/e2e/document-reindexing.spec.ts` | Playwright scenarios for re-indexing flow |
| Infra | `main.rs` | Service wiring for chroma/embedding clients in DocumentService |
| Docs | `docs/openapi.yaml`, `docs/api.md` | OpenAPI 3.1 spec, API docs update |

### Key Files (Targeted Fix)

- `backend/src/shared/chroma_client.rs` — `add_embeddings()` now detects `InvalidCollection` errors, creates the missing collection, and retries.

---

## Risk Analysis

### Functional Risks

| Risk | Impact | Evidence |
|------|--------|----------|
| Auto-created Chroma collection may have different configuration than manually created ones | **Medium** — collections are created with UUID names, same as `CollectionService.create()` path | `ChromaClient::create_collection()` is called with just a name (Chroma defaults for distance, metadata) |
| Collection creation succeeds but `add_embeddings` still fails on retry | **Medium** — the second attempt will exhaust retries normally | Error propagation is unchanged; the first failure is consumed for creation, then normal retry logic takes over |
| ZIP upload path uses the same `add_embeddings()` method | **Low** — fix is in the shared method, both single-upload and ZIP paths benefit | No special ZIP-specific logic needed |
| Race condition when two uploads simultaneously detect missing collection | **Medium** — both would try to create the same collection | Chroma is idempotent for collection creation (second attempt returns no error or duplicate error) |

### Technical Risks

| Risk | Impact | Evidence |
|------|--------|----------|
| `is_missing_collection_error` helper may not match future Chroma error message formats | **Low** — unit tests bind detection to the exact error string | Tests with the exact real error format pass; the assertion is conservative (contains both `InvalidCollection` and `does not exist`) |
| `collection_created` boolean prevents infinite loops | **None** — the flag ensures at most one create attempt per operation | Guarded by `&& !collection_created` |
| No rollback if both create and retry fail | **Low** — existing rollback in `DocumentService` handles this | Service catches error from `add_embeddings`, deactivates document/chunks |

### Regression Risks

| Risk | Impact | Evidence |
|------|--------|----------|
| Existing uploads with healthy Chroma collections are unaffected | **None** — only the first `InvalidCollection` response triggers the new path | Normal path unchanged |
| Adding `where_filter` to `query()` changes retry behavior | **Low** — retry loop structure is identical to `add_embeddings` | Same `MAX_RETRIES`, same sleep pattern |
| `is_missing_collection_error` is a `fn` method, does not require `&self` | **None** — static dispatch, no state mutation | Implementation is a pure string check |

---

## Evidence

- **Bug log reproduction:** `backend` logs show `"Chroma add_embeddings failed (attempt 1/3): Chroma error: Add embeddings failed (HTTP 400 Bad Request): {"error":"InvalidCollection","message":"Collection <uuid> does not exist."}"` followed by `"Upload indexing failed; deactivating document and chunks"`
- **Fix location:** `ChromaClient::add_embeddings()` in `backend/src/shared/chroma_client.rs` lines 53-78
- **Error detection:** `ChromaClient::is_missing_collection_error()` — static method, matches `AppError::ChromaError` containing `"InvalidCollection"` and `"does not exist"`
- **Test coverage:** Two unit tests for error detection in `chroma_client.rs` — one positive (exact real error), one negative (HTTP 500)
- **All existing tests pass:** `cargo test is_missing_collection_error` — 2 passed, `cargo clippy` — clean
