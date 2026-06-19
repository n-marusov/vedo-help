# Implementation Plan: Bulk Document Deletion from Collection

Branch: feature/bulk-document-deletion
Created: 2026-06-19

## Settings
- Testing: yes
- Logging: verbose
- Docs: yes

## Roadmap Linkage
Milestone: "v0.3 — Admin Panel & Production Polish"
Rationale: Bulk document deletion with checkboxes, toggle all, optimistic UI, and VToast feedback is the last remaining UI-facing feature in the admin panel polish milestone. 3 roadmap checklist items are covered by this plan.

## Commit Plan
- **Commit 1** (after tasks 1-8): "feat(backend): add batch document delete endpoint"
- **Commit 2** (after tasks 9-11): "feat(frontend): bulk document deletion UI with optimistic updates and toast feedback"
- **Commit 3** (after tasks 12-15): "test: add unit and E2E tests for bulk document deletion"

## Tasks

### Phase 1: Backend — Repository Layer
- [x] Task 1: **Add batch document repository methods** (`backend/src/modules/documents/repository.rs`)
  - Add `get_documents_by_ids(ids: &[Uuid]) -> Result<Vec<Document>, AppError>` — fetches multiple documents in one query (`SELECT ... WHERE id IN (...)`), needed to get their collection_ids for Chroma cleanup
  - Add `deactivate_documents_batch(ids: &[Uuid]) -> Result<u64, AppError>` — bulk soft-deactivate: `UPDATE documents SET is_active = 0 WHERE id IN (...)`; return affected row count
  - Add `deactivate_chunks_batch(document_ids: &[Uuid]) -> Result<u64, AppError>` — bulk soft-deactivate chunks: `UPDATE chunks SET is_active = 0 WHERE document_id IN (...)`
  - LOGGING: DEBUG on entry with ids count, INFO on affected rows count, WARN on zero affected
  - Unit tests in repository for all 3 new methods (using in-memory SQLite, following existing patterns)

- [x] Task 2: **Add batch delete response model** (`backend/src/modules/documents/models.rs`)
  - Add `BatchDeleteResponse { deleted_count: usize, ids: Vec<Uuid> }` — serde-serializable
  - LOGGING: N/A (pure data struct)

### Phase 2: Backend — Service Layer
- [x] Task 3: **Add batch document delete service method** (`backend/src/modules/documents/service.rs`)
  - Add `delete_documents_batch(ids: Vec<Uuid>) -> Result<BatchDeleteResponse, AppError>`
  - Logic:
    1. Fetch all documents by IDs (to get collection_ids for Chroma)
    2. Group documents by collection_id (Chroma uses collection_uuid as collection name)
    3. Deactivate chunks (bulk) → deactivate documents (bulk) in SQLite
    4. For each collection group: call `chroma.delete_where` with `{"document_id": {"$in": [id1, id2, ...]}}` — note: Chroma's `$in` operator might not work; fallback to calling `delete_where` per-document in a loop if `$in` fails
    5. Rollback semantics: if any Chroma cleanup fails, log warning but don't revert SQLite (same pattern as single delete — soft delete already succeeded)
  - Use sqlx `WHERE id IN (...)` with dynamic bindings — construct positional `?` placeholders
  - LOGGING: INFO on entry with count, DEBUG per Chroma cleanup call, WARN on Chroma failures, INFO on success with deleted_count
  - Existing `test_soft_delete_keeps_rows_but_removes_from_active_results` in the test module already validates single-document deletion patterns

- [x] Task 4: **Add batch delete service unit test** (`backend/src/modules/documents/service.rs`)
  - Add `test_batch_delete_keeps_rows_but_removes_from_active_results`:
    - Upload 3 documents to same collection
    - Confirm all 3 visible
    - Delete 2 via batch
    - Assert: remaining 1 is visible, 2 are invisible (but rows still exist with is_active=0)
  - Add `test_batch_delete_with_mixed_collections`:
    - Upload docs across 2 collections
    - Delete docs from both collections in one batch
    - Assert correct per-collection active state
  - LOGGING: N/A (test code)

### Phase 3: Backend — Handler + Route
- [x] Task 5: **Add batch delete handler** (`backend/src/modules/documents/handlers.rs`)
  - Add `BatchDeleteRequest { ids: Vec<Uuid> }` — deserializable request struct
  - Add `delete_batch` handler:
    - Extract `Json(BatchDeleteRequest)` from body
    - Validate: reject empty ids array with `AppError::BadRequest("No document IDs provided")`
    - Validate: reject if any id is not a valid UUID (already handled by serde)
    - Call `svc.delete_documents_batch(req.ids).await`
    - Return `Json(BatchDeleteResponse)`
  - LOGGING: INFO on entry with count, DEBUG on request payload, INFO on response count

- [x] Task 6: **Register batch delete route** (`backend/src/main.rs`)
  - Add `.route("/api/documents/batch", delete(documents_handlers::delete_batch))`
  - Place it **before** the `/api/documents/:id` route to prevent `:id` from swallowing `batch` as a parameter
  - LOGGING: N/A (route wiring)

- [x] Task 7: **Add API client method for batch delete** (`frontend/src/api/client.ts`)
  - Add `batchDeleteDocuments(ids: string[])` convenience method to the `api` object:
    - Calls `request<{ deleted_count: number; ids: string[] }>("/documents/batch", { method: "DELETE", body: JSON.stringify({ ids }) })`
    - No special content-type override needed since `request()` already sets `application/json`
  - LOGGING: N/A (thin wrapper)

- [x] Task 8: **Update frontend types** (`frontend/src/api/types.ts`)
  - Add `BatchDeleteResponse { deleted_count: number; ids: string[] }` interface
  - LOGGING: N/A (type definition)

<!-- Commit checkpoint: tasks 1-8 -->

### Phase 4: Frontend — Store + Optimistic UI
- [x] Task 9: **Add batch delete action to document store** (`frontend/src/stores/documents.ts`)
  - Add `deleteDocumentsBatch(ids: string[]): Promise<boolean>` action
  - **Optimistic update pattern**:
    1. Save current document list to a snapshot variable (`const snapshot = [...documents.value]`)
    2. Immediately remove selected documents from `documents.value`
    3. Call `api.batchDeleteDocuments(ids)`
    4. On success: keep the optimistic state, return `{ deleted_count, ids }`
    5. On error: **rollback** — restore `documents.value = snapshot`, throw or return error details
  - Also export a separate `getDocumentsSnapshot()` and `rollbackDocuments(snapshot)` for the component to use
  - Add `isDeleting` ref to prevent double-submits (set true before API call, false after)
  - LOGGING: N/A (frontend store)

- [x] Task 10: **Add VToast integration to DocumentList** (`frontend/src/components/DocumentList.vue`)
  - Add local toast state:
    ```ts
    const toastMessage = ref('');
    const toastType = ref<'info' | 'success' | 'error'>('info');
    const showToast = ref(false);
    ```
  - Add `<VToast :show="showToast" :message="toastMessage" :type="toastType" @close="showToast = false" />` to template
  - Use toast on:
    - **Success** (after batch delete): `"Deleted N document(s)"` (type=success)
    - **Error** (batch delete fails): `error.value || "Failed to delete documents"` (type=error)
    - **Empty selection**: `"Select documents to delete"` (type=info)
  - LOGGING: N/A (component state)

### Phase 5: Frontend — Bulk Delete UI
- [x] Task 11: **Add checkbox UI to DocumentList** (`frontend/src/components/DocumentList.vue`)
  - Add local state:
    ```ts
    const selectedIds = ref<Set<string>>(new Set());
    const showBulkDeleteDialog = ref(false);
    ```
  - **Per-document checkbox**: add `<input type="checkbox">` to each `.dl-item` row, bound to `selectedIds`
    - Use `:checked="selectedIds.has(doc.id)"` and `@change="toggleSelection(doc.id)"`
    - Style with `--color-primary` for checked state, `--color-border` for unchecked
  - **Toggle all checkbox**: add a checkbox in the `.dl-header` next to the "DOCUMENTS" label
    - Three states: unchecked (none), checked (all), indeterminate (some)
    - `@change="toggleAll()"` — if all selected → deselect all, else → select all visible
  - **Bulk delete button**: add `<VButton variant="destructive" :disabled="selectedIds.size === 0 || isDeleting">` 
    - Shows `"Delete N selected"` text
    - On click → open VDialog for confirmation
  - **Bulk delete confirmation dialog**: `<VDialog>` with `title="Delete N documents?"` and `variant="destructive"`
    - On confirm: call `documentStore.deleteDocumentsBatch(Array.from(selectedIds))`
    - On success: clear `selectedIds`, show success toast
    - On error: rollback was handled in store; show error toast
    - Disable confirm button while `isDeleting`
  - **Prevent double-click**: disable bulk delete button and dialog confirm while `isDeleting`
  - LOGGING: N/A (component state)

<!-- Commit checkpoint: tasks 9-11 -->

### Phase 6: Tests
- [x] Task 12: **Add frontend unit tests for bulk delete store action** (`frontend/src/stores/__tests__/documents.test.ts` or similar)
  - Add `test('deleteDocumentsBatch optimistically removes documents from list')`:
    - Mock `api.batchDeleteDocuments` to resolve successfully
    - Call action with 2 IDs
    - Assert documents removed from local list immediately
    - Assert API called with correct payload
  - Add `test('deleteDocumentsBatch rolls back on failure')`:
    - Mock `api.batchDeleteDocuments` to reject
    - Call action
    - Assert documents restored to original list
  - Add `test('deleteDocumentsBatch prevents double submission while deleting')`:
    - Assert `isDeleting` state

- [x] Task 13: **Add frontend unit tests for DocumentList bulk delete UI** (`frontend/src/components/__tests__/DocumentList.spec.ts` or similar)
  - Add `test('renders checkbox for each document')`
  - Add `test('toggle all checkbox selects/deselects all documents')`
  - Add `test('bulk delete button opens confirmation dialog')`
  - Add `test('bulk delete confirm calls store action and shows toast')`
  - Mock the document store and VToast

- [ ] Task 14: **Add E2E test for bulk document deletion** (`frontend/e2e/bulk-delete.spec.ts`)
  - **Precondition**: Create collection, upload 3 documents via API setup
  - **Test flow**:
    1. Navigate to admin page, select collection
    2. Verify 3 documents visible
    3. Check 2 documents' checkboxes
    4. Verify "Delete 2 selected" button is enabled
    5. Click bulk delete button
    6. Verify confirmation dialog appears with correct title
    7. Confirm deletion
    8. Verify only 1 document remains in list
    9. Verify success toast appears with "Deleted 2 document(s)"
  - Add `test('bulk delete with empty selection shows info message on attempt')`:
    - Click bulk delete without selecting anything → toast "Select documents to delete"

- [x] Task 15: **Run cleanup and validation**
  - Run `cargo fmt` + `cargo clippy` in backend/
  - Run `npx biome format` + `npx biome check` in frontend/
  - Run `npm run ai:validate` from project root (per CHECKLIST.md)
  - Run all existing tests to confirm no regressions:
    - `cargo test` in backend/
    - `npx vitest run` in frontend/
  - LOGGING: N/A

<!-- Commit checkpoint: tasks 12-15 -->
