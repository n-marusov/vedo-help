# Test Plan: Auto-create missing Chroma collection on add_embeddings

**Branch:** `feature/document-re-indexing`
**Based on:** `change-summary.md`

---

## Test Scope

### In Scope

| Area | Description |
|------|-------------|
| Chroma collection auto-creation | Verifying that `add_embeddings()` creates a missing collection and succeeds on retry |
| Single document upload | Verifying that a file upload that hits `InvalidCollection` recovers transparently |
| ZIP batch upload | Verifying that ZIP processing also recovers via `add_embeddings()` |
| Error detection helper | Verifying that `is_missing_collection_error()` correctly identifies the Chroma error format |
| Error recovery rollback | Verifying that if both create and retry fail, the document is still deactivated properly |
| Existing healthy collections | Verifying that uploads with existing Chroma collections continue to work unchanged |

### Out of Scope

| Area | Reason |
|------|--------|
| Chroma collection creation via `CollectionService` | Unchanged — collection creation is already tested elsewhere |
| Chroma `delete_collection` | Unchanged by this fix |
| Frontend display changes | Not affected by backend-only fix |
| Document parsing / chunking | Unchanged |
| Authentication | Unchanged |

---

## Test Types

| Type | Description |
|------|-------------|
| **Unit tests** | Verify `is_missing_collection_error` logic, verify retry/create-add flow behavior |
| **Integration tests** | Verify end-to-end upload → embed → index flow with a real or mock Chroma |
| **Manual / Docker** | Verify the fix in a running Docker Compose stack where Chroma collections may be stale |

---

## Verification Checklist

| # | Check | Priority | Type |
|---|-------|----------|------|
| 1 | `is_missing_collection_error` returns `true` for exact Chroma `InvalidCollection` message | High | Unit |
| 2 | `is_missing_collection_error` returns `false` for HTTP 500 error | High | Unit |
| 3 | `is_missing_collection_error` returns `false` for non-Chroma errors (e.g. `AppError::BadRequest`) | Medium | Unit |
| 4 | Single document upload succeeds when Chroma collection does not exist before upload | High | Integration |
| 5 | ZIP batch upload succeeds when Chroma collection does not exist before upload | High | Integration |
| 6 | Single document upload creates the missing Chroma collection (verify via Chroma API) | High | Integration |
| 7 | Existing collection uploads complete without extra collection-creation attempts (no `[FIX]` log) | Medium | Integration |
| 8 | Error rollback: document/chunks deactivated if both create-collection and retry-add fail | Medium | Integration |
| 9 | Race condition: concurrent uploads to same missing collection both succeed | Low | Integration |
| 10 | Backend compiles and passes `cargo fmt` + `cargo clippy` | High | Build |

---

## Dependencies & Environment

- Rust toolchain (stable) for unit test execution
- For integration tests: running Chroma service (Docker Compose or standalone)
- For manual Docker verification: full Docker Compose stack with Chroma, backend, embedding service
