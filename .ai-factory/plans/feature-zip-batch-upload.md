# Implementation Plan: ZIP Batch Upload

Branch: `feature/zip-batch-upload`
Created: 2026-06-17

## Settings
- Testing: yes
- Logging: verbose
- Docs: yes

## Roadmap Linkage
Milestone: v0.3 — Admin Panel & Production Polish
Rationale: ZIP batch upload closes a documented gap where the frontend accepts .zip files but the backend cannot process them.

## Commit Plan
- **Commit 1** (after tasks 1–3): "feat: add Zip file type and validation infrastructure"
- **Commit 2** (after tasks 4–6): "feat: implement backend ZIP upload endpoint with 10-file limit"
- **Commit 3** (after tasks 7–8): "feat: frontend ZIP batch upload flow with error handling"
- **Commit 4** (after task 9): "test: add backend and e2e tests for ZIP batch upload"

## Tasks

### Phase 1: Backend Infrastructure

- [ ] **Task 1: Add `zip` crate dependency and `FileType::Zip` variant**
  - Add `zip = "2"` to `backend/Cargo.toml` dependencies
  - Add `Zip` variant to `FileType` enum in `backend/src/shared/types.rs`
  - Implement `mime_type()` for `Zip` variant → `"application/zip"`
  - Update `FileType` display/serialization as needed
  - **Files:** `backend/Cargo.toml`, `backend/src/shared/types.rs`
  - **Logging:** DEBUG on type registration, INFO on first use

- [ ] **Task 2: Add ZIP validation to `file_validation.rs`**
  - Add `.zip` extension → `FileType::Zip` mapping in `detect_file_type()`
  - Add `validate_zip_magic()` function checking `PK\x03\x04` header (same bytes as DOCX — extract shared constant)
  - Update `MAX_FILE_SIZE` constant reference: ZIPs inherit the 50 MB limit
  - Update doc comments to list ZIP as a supported container format
  - **Files:** `backend/src/shared/file_validation.rs`
  - **Logging:** DEBUG on magic byte validation, INFO on ZIP file acceptance
  - **Tests:** Unit tests for ZIP validation (valid zip bytes, invalid, empty, too-large)

### Phase 2: Backend ZIP Upload Endpoint

- [ ] **Task 3: Add `ZipUploadResponse` models**
  - Create `ZipUploadItem` struct: `{filename: String, status: String, document_id: Option<Uuid>, error: Option<String>}`
  - Create `ZipUploadResponse` struct: `{total_files: usize, processed: usize, failed: usize, items: Vec<ZipUploadItem>}`
  - Derive `Serialize` for all new types
  - Add `PayloadTooLarge(String)` variant to `AppError` in `backend/src/shared/error.rs` that returns HTTP 413
  - **Files:** `backend/src/modules/documents/models.rs`, `backend/src/shared/error.rs`
  - **Logging:** DEBUG on model construction, INFO on batch summary

- [ ] **Task 4: Implement `process_zip_upload` in `DocumentService`**
  - Accept `&[u8]` (raw ZIP bytes) and `collection_id: Uuid`
  - Open ZIP archive using `zip::ZipArchive`
  - Enumerate entries:
    - Skip directories
    - **Enforce 10-file limit**: if file count > 10, return `AppError::PayloadTooLarge("ZIP contains more than 10 files")` — this maps to HTTP 413
    - For each entry: extract filename, read bytes, detect actual inner file type by extension, validate via existing `validate_file()`, parse via existing `parse_file_content()`, chunk via `chunk_document()`, save document + chunks via repository
    - Collect results into `ZipUploadResponse` with per-file status
    - On individual file failure: log WARN and continue (don't abort the whole batch)
  - **Files:** `backend/src/modules/documents/service.rs`
  - **Logging (verbose):**
    - DEBUG: "ZIP opened: {count} entries found"
    - DEBUG: per-file "Extracting: {name} ({size} bytes)"
    - DEBUG: per-file "File processed: {name} -> {chunks} chunks"
    - WARN: per-file "File skipped: {name} - {reason}" (unsupported type, parse error)
    - INFO: "ZIP upload complete: {ok}/{total} files processed"
    - ERROR on ZIP parse failure
  - **Tests:** Unit tests with mock ZIP files (0, 5, 11 files, corrupted ZIP, mixed supported/unsupported files)

- [ ] **Task 5: Add `upload_zip` handler**
  - New handler `upload_zip` in `documents/handlers.rs`
  - Accept multipart with `file` (the ZIP) and `collection_id` fields
  - Extract raw bytes, call `svc.process_zip_upload()`
  - Return `Json<ZipUploadResponse>` on success
  - **413 Payload Too Large** is returned when:
    1. The ZIP contains more than 10 files (enforced in service via `AppError::PayloadTooLarge`)
    2. The overall request body exceeds the configured limit (enforced by RequestBodyLimitLayer)
  - **Files:** `backend/src/modules/documents/handlers.rs`
  - **Logging:**
    - INFO handler entry with collection_id
    - INFO handler completion with summary
    - WARN on 413 conditions
  - **Register route:** Update `backend/src/modules/documents/mod.rs` exports if needed

- [ ] **Task 6: Wire ZIP upload route + body size limit in `main.rs`**
  - Add route: `.route("/api/documents/upload-zip", post(documents_handlers::upload_zip))`
  - Apply `RequestBodyLimitLayer::new(50 * 1024 * 1024)` (50 MB) specifically to the ZIP upload route using a separate router or tower layer
  - Apply a `RequestBodyLimitLayer` to the general document upload route as well if not already present
  - **Files:** `backend/src/main.rs`
  - **Logging:** INFO on route registration, DEBUG on body limit configuration

### Phase 3: Frontend Integration

- [ ] **Task 7: Add frontend ZIP upload types and store method**
  - Add to `frontend/src/api/types.ts`:
    - `ZipUploadItem { filename: string; status: string; document_id: string | null; error: string | null }`
    - `ZipUploadResponse { total_files: number; processed: number; failed: number; items: ZipUploadItem[] }`
  - Add `uploadZip(file, collectionId, onProgress)` to `frontend/src/stores/documents.ts`
    - Use `XMLHttpRequest` for progress tracking (same pattern as existing `uploadDocument`)
    - POST to `/api/documents/upload-zip`
    - Handle 413 error with clear message: "ZIP содержит более 10 файлов. Пожалуйста, уменьшите количество файлов в архиве."
    - Return `ZipUploadResponse` on success
    - After upload completes, call `fetchDocuments` to refresh list
    - **DON'T** update `uploadDocument` — keep single-file upload independent
  - **Files:** `frontend/src/api/types.ts`, `frontend/src/stores/documents.ts`
  - **Logging (frontend):** console.debug on progress, console.warn on errors
  - **Tests:** Frontend unit test for `uploadZip` store method

- [ ] **Task 8: Update `DocumentList.vue` for ZIP file handling**
  - In `handleFilesSelected()`: detect `.zip` files in the selected file list
    - If a `.zip` file is detected → call `documentStore.uploadZip()` instead of per-file `uploadDocument()`
    - If there are mixed .zip and non-zip files, process them separately: ZIP via batch endpoint, individual files via regular upload
  - Show enhanced upload progress during ZIP processing:
    - `"Обработка ZIP: {processed}/{total} файлов"` (progress message)
    - Show per-file results after completion (success/fail indicators)
  - Handle 413 error display:
    - Show dialog/toast: "Файлов слишком много. ZIP должен содержать не более 10 файлов."
  - **Files:** `frontend/src/components/DocumentList.vue`
  - **Logging:** console.debug on file routing decision

### Phase 4: Testing & Documentation

- [ ] **Task 9: Write backend unit tests for ZIP batch upload**
  - Test `process_zip_upload` with:
    - Hand-crafted ZIP (using `zip` crate's `ZipWriter`) with 5 valid files → success
    - ZIP with 11 files → 413 error
    - ZIP with mixed valid/invalid files → partial success
    - Empty ZIP → success (0 files processed)
    - Corrupted ZIP bytes → file error
    - ZIP with unsupported file types → skipped with WARN
    - ZIP at exactly 50 MB boundary → check none or accept
  - Test `upload_zip` handler with mock multipart
  - Test `validate_file` for `.zip` extension
  - Use `#[cfg(test)]` modules and in-memory SQLite (via `tests/common/mod.rs`)
  - **Files:** `backend/src/modules/documents/service.rs` (test module), `backend/src/shared/file_validation.rs` (test module)
  - **Follow RULES.md:** TDD — write tests first, then verify they fail, then implement

- [ ] **Task 10: Write E2E tests for ZIP batch upload**
  - Add Playwright e2e test or extend existing RAG flow test:
    - Upload a valid ZIP file via the admin panel
    - Verify the files appear in the document list
    - Upload a ZIP with >10 files → verify 413 error message is shown
    - Upload a corrupted ZIP → verify error handling
  - **Files:** `frontend/e2e/` (new or extended spec file)

- [ ] **Task 11: Documentation and validation**
  - Update `docs/api.md` to add the `POST /api/documents/upload-zip` endpoint documentation
  - Update `docs/gui.md` if needed to describe ZIP upload UI
  - Run `npm run ai:validate` (per RULES.md) and verify exit code 0
  - Verify Docker images build: `docker compose build backend frontend` succeeds
  - **Files:** `docs/api.md`, optionally `docs/gui.md`
