# Test Cases: Auto-create missing Chroma collection on add_embeddings

**Branch:** `feature/document-re-indexing`
**Based on:** `change-summary.md`, `test-plan.md`
**Date:** 2026-06-19

---

## Test Case — TC-001

| Field | Value |
|-------|-------|
| **Title** | `is_missing_collection_error` detects real Chroma InvalidCollection format |
| **Priority** | High |
| **Type** | Unit |
| **Preconditions** | Rust toolchain installed |
| **Steps** | 1. Construct `AppError::ChromaError` with the exact error text from logs<br>2. Call `ChromaClient::is_missing_collection_error(&error)` |
| **Expected result** | Returns `true` |
| **Command** | `cargo test test_is_missing_collection_error_detects_chroma_invalid_collection` |

---

## Test Case — TC-002

| Field | Value |
|-------|-------|
| **Title** | `is_missing_collection_error` ignores HTTP 500 Chroma errors |
| **Priority** | High |
| **Type** | Unit |
| **Preconditions** | Rust toolchain installed |
| **Steps** | 1. Construct `AppError::ChromaError` with HTTP 500 text<br>2. Call `ChromaClient::is_missing_collection_error(&error)` |
| **Expected result** | Returns `false` |
| **Command** | `cargo test test_is_missing_collection_error_ignores_other_chroma_errors` |

---

## Test Case — TC-003

| Field | Value |
|-------|-------|
| **Title** | `is_missing_collection_error` ignores non-Chroma errors |
| **Priority** | Medium |
| **Type** | Unit |
| **Preconditions** | Rust toolchain installed |
| **Steps** | 1. Construct `AppError::BadRequest("something went wrong")`<br>2. Call `ChromaClient::is_missing_collection_error(&error)` |
| **Expected result** | Returns `false` |
| **Command** | No existing test — can be added as: `assert!(!ChromaClient::is_missing_collection_error(&AppError::BadRequest("test".into())));` |

---

## Test Case — TC-004

| Field | Value |
|-------|-------|
| **Title** | Docker: single document upload succeeds when Chroma collection is missing |
| **Priority** | High |
| **Type** | Manual / Docker |
| **Preconditions** | Docker Compose stack running; existing collection in SQLite but its Chroma collection has been deleted manually (e.g. via Chroma REST API `DELETE /api/v1/collections/<uuid>`) |
| **Steps** | 1. Open the frontend Documents tab<br>2. Select the collection with the deleted Chroma collection<br>3. Upload a `.md` or `.pdf` file<br>4. Wait for upload progress to reach 100%<br>5. Observe the document appears in the document list<br>6. Reload the page<br>7. Verify the document still appears in the list |
| **Expected result** | Document uploads successfully and persists across page reload |
| **Check backend logs** | Should contain `[FIX] Chroma collection missing during add_embeddings; creating collection and retrying` |
| **Check Chroma** | `curl http://chroma:8000/api/v1/collections/<uuid>` should return the collection metadata |

---

## Test Case — TC-005

| Field | Value |
|-------|-------|
| **Title** | Docker: ZIP batch upload succeeds when Chroma collection is missing |
| **Priority** | High |
| **Type** | Manual / Docker |
| **Preconditions** | Same as TC-004 |
| **Steps** | 1. Create a ZIP with 3–5 `.md` files<br>2. Upload via the frontend Documents tab<br>3. Wait for ZIP processing to complete |
| **Expected result** | All files appear in the document list |
| **Check backend logs** | Should contain `[FIX]` log entry for the batch `add_embeddings` call |

---

## Test Case — TC-006

| Field | Value |
|-------|-------|
| **Title** | Docker: upload succeeds when Chroma collection already exists (no regression) |
| **Priority** | Medium |
| **Type** | Manual / Docker |
| **Preconditions** | Docker Compose stack running; collection with existing healthy Chroma collection |
| **Steps** | 1. Open the frontend Documents tab for a collection that already has documents<br>2. Upload a new file<br>3. Verify the document appears in list |
| **Expected result** | Upload succeeds, document visible |
| **Check backend logs** | Should **NOT** contain `[FIX] Chroma collection missing` — the existing collection should be used directly |

---

## Test Case — TC-007

| Field | Value |
|-------|-------|
| **Title** | Error rollback: document deactivated when both create and retry fail |
| **Priority** | Medium |
| **Type** | Integration (requires mock or controlled Chroma failure) |
| **Preconditions** | Chroma service configured but the `create_collection` call is forced to fail (e.g. network unreachable, Chroma stopped) |
| **Steps** | 1. Stop the Chroma service (`docker compose stop chroma`)<br>2. Upload a document to a collection whose Chroma collection does not exist<br>3. The upload request returns an error<br>4. Query the SQLite documents table for the collection |
| **Expected result** | Document row exists but `is_active = 0`; chunks are also `is_active = 0` |
| **Recovery** | Start Chroma back: `docker compose start chroma` |

---

## Test Case — TC-008

| Field | Value |
|-------|-------|
| **Title** | Build validation: `cargo fmt` + `cargo clippy` pass |
| **Priority** | High |
| **Type** | Build |
| **Preconditions** | Rust toolchain installed |
| **Steps** | 1. `cd backend`<br>2. `cargo fmt`<br>3. `cargo clippy` |
| **Expected result** | No warnings or errors from clippy; formatting produces no diff |
