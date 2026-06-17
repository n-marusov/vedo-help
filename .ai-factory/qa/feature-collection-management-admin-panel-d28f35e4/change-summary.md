## Change Summary: Collection Management in Admin Panel

**Commits:** 0 (plan stage — TDD RED, no implementation yet)
**Changed files:** 0 planned (5 backend + 4 frontend files to be changed)
**Risk level:** 🟡 Medium

---

### What Changed

This is a TDD RED phase — writing tests before implementation. No code has been changed yet. The planned feature adds a rename/update endpoint for collections (PATCH /api/collections/{id}) and a corresponding rename dialog in the admin panel's CollectionManager UI component. Full CRUD (create, read, update, delete) for collections will be available from the admin panel.

---

### Affected Areas (planned)

| Component     | Change type               | Description                                              |
|---------------|---------------------------|----------------------------------------------------------|
| Backend API   | Added (planned)           | `PATCH /api/collections/{id}` — rename/update collection |
| Backend models| Added (planned)           | `UpdateCollectionRequest` — optional name/description    |
| Backend repo  | Added (planned)           | `update_collection()` — dynamic SQL UPDATE with COALESCE  |
| Backend service| Added (planned)          | `rename()` — validation + orchestration                   |
| Frontend types| Added (planned)           | `UpdateCollectionRequest` — TypeScript interface           |
| Frontend API  | Added (planned)           | `api.patch()` — new HTTP PATCH method                      |
| Frontend store| Added (planned)           | `updateCollection()` — Pinia store action                  |
| Frontend UI   | Changed (planned)         | `CollectionManager.vue` — rename dialog, error toasts      |

---

### Evidence

| Finding                               | Evidence                                          |
|---------------------------------------|----------------------------------------------------|
| No `PATCH` endpoint for collections   | `backend/src/main.rs` — no `patch` route wired     |
| No rename dialog in collection UI     | `frontend/src/components/CollectionManager.vue` — no rename button/dialog |
| No `updateCollection` in store        | `frontend/src/stores/collections.ts` — no `updateCollection` function |
| No `patch` method in API client       | `frontend/src/api/client.ts` — no `patch` on `api` object |
| Existing CRUD (create/list/get/delete)| `backend/src/modules/collections/` — handlers, service, repository |
| No unit tests for collections module  | `backend/src/modules/collections/` — no `tests.rs` |
| No component tests for CollectionManager | `frontend/src/components/__tests__/` — no `CollectionManager.spec.ts` |

---

### Risks

🔴 **Critical** (must verify):

- **Data integrity**: rename operation must validate name uniqueness; duplicate names should produce a clear error and not corrupt existing data
- **Chroma sync**: the existing service already syncs collection creation/deletion with Chroma — rename must either sync the Chroma collection name or not require it (since Chroma may not support renaming collections)

🟡 **Medium** (should verify):

- **Partial update semantics**: `UpdateCollectionRequest` uses optional fields — sending only `description` must preserve the existing name and vice versa
- **Error consistency**: error responses from the new endpoint must match the existing `AppError` JSON format
- **UI validation**: empty name should be rejected both on backend (service validation) and frontend (dialog validation)

🟢 **Low** (nice to verify):

- **Toast display**: success/error toasts should auto-dismiss and not stack incorrectly
- **Hover UX**: rename button visibility on hover should match existing delete button behavior

---

### Testing Recommendations

**First priority:**

- [ ] Unit test collection rename — happy path, duplicate name, empty name, not found
- [ ] Unit test CollectionManager — rename dialog opens, submits, shows toast
- [ ] Verify partial update preserves unchanging fields

**Regression:**

- [ ] Existing create/list/delete still work after adding PATCH route
- [ ] Existing collection tests in integration.rs still pass
