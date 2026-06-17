## Test Plan: Collection Management in Admin Panel (TDD RED phase)

**Date:** 2026-06-17
**Branch / Version:** feature/collection-management-admin-panel
**Environment:** local (in-memory SQLite + jsdom for frontend)

---

### 1. Testing Goal

Verify that the planned backend rename endpoint and frontend UI work correctly according to the implementation plan. Since this is TDD RED phase, we write automated tests first — they will fail until the implementation is complete. The tests cover:

- Backend: collection CRUD with the new rename/update operation
- Frontend: CollectionManager component with create, delete, rename dialogs and error toasts

---

### 2. Test Scope

**In Scope** — we test:

- Backend: `CollectionRepository` methods (create, list, get, delete, update_collection)
- Backend: `CollectionService` validation (empty name, duplicate name, not found)
- Frontend: `CollectionManager.vue` rendering (list, empty state, dialogs)
- Frontend: `collectionStore` actions (create, delete, update, fetch)
- Frontend: Error toast display on API failures

**Out of Scope** — we don't test:

- Chroma integration (covered by existing `tests/integration.rs`)
- Authentication/authorization (existing auth middleware covers all /api/* routes)
- Other admin panel components (DocumentList remains unchanged)
- End-to-end flows (will be covered in a separate E2E test phase)

---

### 3. Test Types

| Type              | Priority   | Area                                           |
|-------------------|------------|-------------------------------------------------|
| Functional        | 🔴 High    | Collection rename/update endpoint               |
| Functional        | 🔴 High    | CollectionManager rename dialog UI              |
| Negative          | 🔴 High    | Duplicate name, empty name, nonexistent ID      |
| Regression        | 🟡 Medium  | Existing create/list/delete still work           |
| Edge cases        | 🟡 Medium  | Partial update (name only, description only)    |
| UI/UX             | 🟡 Medium  | Error toast on failure, success toast on OK     |
| Validation        | 🟢 Low     | Frontend name field validation (non-empty)      |

---

### 4. Test Data

| Category          | Data                                        | Purpose                        |
|-------------------|---------------------------------------------|--------------------------------|
| Valid data        | `{ "name": "My Collection" }`              | Happy path create/rename       |
| Valid data        | `{ "name": "New Name", "description": "desc" }` | Full update               |
| Partial update    | `{ "description": "updated desc" }`         | Update only description        |
| Empty name        | `{ "name": "" }` / `{ "name": "   " }`      | Validation failure             |
| Duplicate name    | `{ "name": "Existing" }`                    | Unique constraint violation    |
| Nonexistent ID    | `00000000-0000-0000-0000-000000000000`       | 404 Not Found                  |

---

### 5. Preconditions

- [x] In-memory SQLite available for backend tests
- [x] jsdom environment available for frontend component tests
- [x] Pinia store can be instantiated in test environment
- [x] VDialog, VButton, VInput, VToast components can be stubbed

---

### 6. Acceptance Criteria

- [ ] All 🔴 high-priority test cases pass
- [ ] Backend: create, list, get, delete, update all work with in-memory SQLite
- [ ] Frontend: CollectionManager renders list, empty state, and all three dialogs
- [ ] Negative scenarios return expected errors (400, 404)
- [ ] Duplicate name returns clear error message

---

### 7. Plan Risks

| Risk                          | Impact               | Mitigation                                    |
|-------------------------------|----------------------|-----------------------------------------------|
| No implementation yet         | Medium               | Tests will fail — this is expected TDD RED    |
| Chroma rename not supported   | Medium               | Backend `rename` only updates SQLite; Chroma collection name stays unchanged |
| UI test flakiness with dialogs| Low                  | Stub VDialog and use `findComponent` for assertions |

---

### 8. Checklist

| Check                                                  | Priority              |
|--------------------------------------------------------|-----------------------|
| Create collection inserts correct data                 | High                  |
| List returns all created collections                   | High                  |
| Get returns collection by ID                           | High                  |
| Delete removes collection and returns 404 on re-get    | High                  |
| Update collection name changes name in list            | High                  |
| Update collection description changes description      | High                  |
| Update nonexistent collection returns NotFound          | High                  |
| Create duplicate name returns error                    | High                  |
| CollectionManager renders collection cards             | High                  |
| Create dialog opens and submits                        | High                  |
| Delete dialog opens and confirms                       | High                  |
| Rename dialog opens with pre-filled data               | High                  |
| Error toast appears on API failure                     | Medium                |
| Success toast appears on create/delete/rename           | Medium                |
| Empty state shows "No collections yet"                  | Low                   |
