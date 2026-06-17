# Implementation Plan: Collection Management in Admin Panel

Branch: feature/collection-management-admin-panel
Created: 2026-06-17

## Settings
- Testing: yes
- Logging: verbose
- Docs: yes

## Roadmap Linkage
Milestone: "v0.3 — Admin Panel & Production Polish"
Rationale: Completes the "Collection management in admin panel" item — adds rename/edit capability and full CRUD UI to the admin panel.

## Commit Plan
- **Commit 1** (task 1): "test: add failing Rust unit tests for collection rename endpoint (TDD red phase)"
- **Commit 2** (task 2): "feat: implement PATCH /api/collections/{id} endpoint (TDD green phase)"
- **Commit 3** (task 3): "test: add failing Vitest tests for CollectionManager (TDD red phase)"
- **Commit 4** (tasks 4-5): "feat: implement CollectionManager rename dialog, error toasts, and store (TDD green phase)"
- **Commit 5** (task 6): "chore: run ai:validate and finalize"

## Tasks

### 🔴 Phase 1: Backend tests first (TDD — Red)
- [x] Task 1: Write failing Rust unit tests for the collections module — create, duplicate, delete, rename operations

### 🟢 Phase 2: Backend implementation (TDD — Green)
- [ ] Task 2: Implement `PATCH /api/collections/{id}` — model, repository update, service rename, handler, route wiring

### 🔴 Phase 3: Frontend tests first (TDD — Red)
- [x] Task 3: Write failing Vitest unit tests for CollectionManager — list, create dialog, delete dialog, rename dialog, empty state, error toast

### 🟢 Phase 4: Frontend implementation (TDD — Green)
- [ ] Task 4: Add `updateCollection` to Pinia store, `patch` method to API client, rename dialog to CollectionManager
- [ ] Task 5: Add error display to CollectionManager using VToast for create/delete/rename failures

### ✅ Phase 5: Final validation
- [ ] Task 6: Run `npm run ai:validate`, `cargo clippy`, fix regressions, update CHECKLIST.md

---

## Task Details

### Task 1: Backend tests first (RED)

Create `backend/src/modules/collections/tests.rs` with tests using in-memory SQLite.
**These tests will fail** because the `rename`/`update` endpoint doesn't exist yet.

**Tests to write:**
- `test_create_collection` — create a collection, verify it appears in list
- `test_create_duplicate_collection_fails` — verify unique name constraint returns error
- `test_delete_collection` — create then delete, verify it's gone
- `test_delete_nonexistent_collection_fails` — verify 404/NotFound
- `test_update_collection_name` — rename, verify name changed in list
- `test_update_collection_description` — update description, verify
- `test_update_nonexistent_collection_fails` — update non-existent returns NotFound
- `test_list_collections` — list returns all created collections

Register as `#[cfg(test)] mod tests;` in `backend/src/modules/collections/mod.rs`.

Use in-memory SQLite via `SqlitePoolOptions::new().connect(":memory:").await`, run migrations inline, create `CollectionRepository` and test its methods directly.

**Logging requirements:**
- `tracing::debug!("Running test: {test_name}")` per test function
- `tracing::debug!("Cleaning up test data")` in test teardown
- Console output on assertion failure

**Files:**
- `backend/src/modules/collections/tests.rs` (new)
- `backend/src/modules/collections/mod.rs` (add `#[cfg(test)] mod tests;`)

---

### Task 2: Backend implementation (GREEN)

Make the failing tests pass by implementing the rename endpoint.

**Changes:**

1. **`backend/src/modules/collections/models.rs`** — add `UpdateCollectionRequest`:
   ```rust
   #[derive(Debug, Clone, Deserialize)]
   pub struct UpdateCollectionRequest {
       pub name: Option<String>,
       pub description: Option<String>,
   }
   ```

2. **`backend/src/modules/collections/repository.rs`** — add `update_collection`:
   - Build dynamic UPDATE query using COALESCE for optional fields
   - Validate name uniqueness (catch UNIQUE constraint → BadRequest)
   - Return `AppError::NotFound` if 0 rows affected
   - Return updated `Collection` after fetch

3. **`backend/src/modules/collections/service.rs`** — add `rename` method:
   - Input: `id: Uuid`, `req: UpdateCollectionRequest`
   - Validate: if `name` is provided, trim and check non-empty
   - Call `repo.update_collection(id, &name, &description)`
   - Return `CollectionSummary`

4. **`backend/src/modules/collections/handlers.rs`** — add `update` handler:
   - `PATCH /api/collections/{id}`
   - Extract `Json<UpdateCollectionRequest>` body
   - Call `svc.rename(id, req)`
   - Return `Json<CollectionSummary>`

5. **`backend/src/main.rs`** — wire route:
   Add `use axum::routing::patch;`
   ```
   .route("/api/collections/{id}", patch(collections_handlers::update))
   ```

**Logging requirements:**
- `tracing::info!("PATCH /api/collections/{id}")` in handler entry
- `tracing::info!("Renaming collection: {id} → {new_name}")` in service
- `tracing::debug!("Updating collection in SQLite: {id}")` in repository
- Log validation errors with context
- Log SQL errors with query details

**Files:**
- `backend/src/modules/collections/models.rs`
- `backend/src/modules/collections/repository.rs`
- `backend/src/modules/collections/service.rs`
- `backend/src/modules/collections/handlers.rs`
- `backend/src/main.rs`

---

### Task 3: Frontend tests first (RED)

Create `frontend/src/components/__tests__/CollectionManager.spec.ts`.

**These tests will fail** because the rename dialog and error toasts don't exist yet.

**Tests to write:**
- `renders collection list` — mount with mocked store, verify collection cards render
- `shows empty state when no collections` — verify "No collections yet" text
- `opens create dialog on +New click` — click button, verify VDialog opens with title "Create Collection"
- `submits create form` — fill name, confirm, verify store.createCollection called
- `opens delete dialog on 🗑 click` — click delete, verify confirmation dialog
- `confirms delete` — click delete, confirm, verify store.deleteCollection called
- `opens rename dialog on ✏️ click` — click edit, verify dialog with pre-filled name
- `submits rename form` — change name, confirm, verify store.updateCollection called
- `shows error toast on failure` — mock store to reject, verify toast appears

Use `@vue/test-utils` + `pinia` (`createPinia`, `setActivePinia`) for store mocking.
Stub `VDialog`, `VButton`, `VInput`, `VToast` with simple mock components if needed.

**Files:**
- `frontend/src/components/__tests__/CollectionManager.spec.ts` (new)

---

### Task 4: Frontend store + CollectionManager rename (GREEN)

Make the frontend tests pass by implementing the UI.

**Changes:**

1. **`frontend/src/api/types.ts`** — add `UpdateCollectionRequest`:
   ```typescript
   export interface UpdateCollectionRequest {
     name?: string;
     description?: string;
   }
   ```

2. **`frontend/src/api/client.ts`** — add `patch` method:
   ```typescript
   patch: <T>(path: string, body?: unknown) =>
     request<T>(path, { method: 'PATCH', body: body ? JSON.stringify(body) : undefined }),
   ```

3. **`frontend/src/stores/collections.ts`** — add `updateCollection`:
   - `async function updateCollection(id: string, req: UpdateCollectionRequest)`
   - Calls `api.patch<Collection>(`/collections/${id}`, req)`
   - Updates collection in-place in `collections.value` array
   - Returns the updated collection or null on error
   - Sets `error.value` on API failure

4. **`frontend/src/components/CollectionManager.vue`** — add rename:
   - Add `showRenameDialog` ref, `renamingCollection` ref, `editForm` ref
   - Add ✏️ edit button next to 🗑 delete on each collection card (shown on hover)
   - `openRenameDialog(col)` — pre-fills `editForm` with current name/description
   - `handleRename()` — calls `collectionStore.updateCollection(id, { name, description })`
   - Add rename `<VDialog>` with name (VInput) and description (textarea) fields
   - Track `isUpdating` state for button loading

**Logging requirements:**
- `console.debug('[CollectionStore] Updating collection:', id)` in store action
- `console.debug('[CollectionManager] Rename dialog opened for:', col.id)` in component
- `console.error('[CollectionManager] Rename failed:', err)` on error

**Files:**
- `frontend/src/api/types.ts`
- `frontend/src/api/client.ts`
- `frontend/src/stores/collections.ts`
- `frontend/src/components/CollectionManager.vue`

---

### Task 5: Error display via VToast (GREEN)

Make error toast tests pass by adding user-facing error feedback.

**Changes:**

1. **`frontend/src/components/CollectionManager.vue`** — add toast:
   - Import `VToast` component
   - Add reactive state: `toastMessage`, `toastType` ('error' | 'success'), `showToast`
   - After each store operation (create/delete/rename):
     - On success: show green success toast ("Collection created", "Collection deleted", "Collection renamed")
     - On failure: show red error toast with the error message
   - `showToast` auto-dismisses after 4s (built into VToast)

2. **`frontend/src/stores/collections.ts`** — improve error propagation:
   - Ensure `createCollection`, `deleteCollection`, `updateCollection` return enough context for the UI to show meaningful toasts
   - Clear `error.value` after successful operations

**Files:**
- `frontend/src/components/CollectionManager.vue`
- `frontend/src/stores/collections.ts`

---

### Task 6: Final validation

- Run `cargo build` and `cargo test` in backend — all tests must pass (including the new ones from Task 1)
- Run `npm run ai:validate` in frontend — must exit with code 0
- Run `cargo clippy` — fix any warnings
- Fix any type errors, lint errors, test failures
- Update `CHECKLIST.md` if needed
- Verify the branch plan is complete and all 6 tasks are [x]
