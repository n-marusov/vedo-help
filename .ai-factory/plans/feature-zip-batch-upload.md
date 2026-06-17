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
- **Commit 1** (after tasks 1–2): "feat: add Zip file type, validation, and response models"
- **Commit 2** (after tasks 3–4): "test: add e2e and unit tests for ZIP batch upload (RED)"
- **Commit 3** (after tasks 5–6): "feat: implement backend ZIP upload endpoint with 10-file limit"
- **Commit 4** (after tasks 7–8): "feat: frontend ZIP batch upload flow with error handling"
- **Commit 5** (after task 9): "docs: update API documentation for ZIP upload"

## Tasks

> **TDD workflow per RULES.md:** Each implementation task (Phase 3–4) is preceded by its tests (Phase 2). Write the test first (RED), verify it fails, then implement until it passes (GREEN). The foundation phase (Phase 1) provides only the types and infrastructure that tests need to compile.

### Phase 1: Foundation — Types & Infrastructure

- [x] **Task 1: Add `zip` crate dependency and `FileType::Zip` variant**
  - Add `zip = "2"` to `backend/Cargo.toml` dependencies
  - Add `Zip` variant to `FileType` enum in `backend/src/shared/types.rs`
  - Implement `mime_type()` for `Zip` variant → `"application/zip"`
  - Update `FileType` display/serialization as needed
  - **Files:** `backend/Cargo.toml`, `backend/src/shared/types.rs`
  - **Logging:** DEBUG on type registration, INFO on first use

- [x] **Task 2: Add `ZipUploadResponse` models + `AppError::PayloadTooLarge` + ZIP validation**
  - Create `ZipUploadItem` struct: `{filename: String, status: String, document_id: Option<Uuid>, error: Option<String>}`
  - Create `ZipUploadResponse` struct: `{total_files: usize, processed: usize, failed: usize, items: Vec<ZipUploadItem>}`
  - Derive `Serialize` for all new types
  - Add `PayloadTooLarge(String)` variant to `AppError` in `backend/src/shared/error.rs` that returns HTTP 413
  - Add `.zip` extension → `FileType::Zip` mapping in `file_validation.rs::detect_file_type()`
  - Add `validate_zip_magic()` function checking `PK\x03\x04` header (extract shared constant with DOCX)
  - **Files:** `backend/src/modules/documents/models.rs`, `backend/src/shared/error.rs`, `backend/src/shared/file_validation.rs`
  - **Logging:** DEBUG on magic byte validation, INFO on ZIP file acceptance

### Phase 2: RED — Write Tests First

- [x] **Task 3: Write E2E tests (Playwright)**
  - Add new e2e spec `frontend/e2e/zip-upload.spec.ts` with scenarios:
    - Upload a valid ZIP file via the admin panel → verify files appear in document list
    - Upload a ZIP with >10 files → verify 413 error message is shown in UI
    - Upload a corrupted/invalid ZIP → verify error handling
    - Upload a ZIP with mixed supported/unsupported files → verify partial success
  - Run tests to verify they fail (RED) before implementation
  - **Files:** `frontend/e2e/zip-upload.spec.ts`

- [x] **Task 4: Write backend unit tests**
  - Write `#[cfg(test)]` module in `backend/src/shared/file_validation.rs`:
    - `test_validate_zip_valid()` — valid ZIP bytes → `Ok(FileType::Zip)`
    - `test_validate_zip_invalid_magic()` — wrong bytes → error
    - `test_validate_zip_empty()` — empty bytes → error
    - `test_validate_zip_extension()` — `.zip` file → `FileType::Zip`
  - Write `#[cfg(test)]` module in `backend/src/modules/documents/service.rs`:
    - Hand-crafted ZIP (via `zip::ZipWriter`) with 5 valid `.md` files → `ZipUploadResponse` with 5 processed
    - ZIP with 11 files → expect `AppError::PayloadTooLarge`
    - ZIP with mixed valid/invalid files → partial success (processed < total)
    - Empty ZIP → 0 files processed
    - Corrupted ZIP bytes → `AppError::FileError`
    - ZIP with unsupported types (.exe, .txt) → skipped with WARN
  - Run `cargo test` to verify they fail/do-not-compile (RED) before implementation
  - **Files:** `backend/src/shared/file_validation.rs`, `backend/src/modules/documents/service.rs`
  - **Follow RULES.md:** TDD — write tests first, verify they fail, then implement

### Phase 3: GREEN — Implement Backend

- [x] **Task 5: Implement `process_zip_upload` in `DocumentService`**
  - Accept `&[u8]` (raw ZIP bytes) and `collection_id: Uuid`
  - Open ZIP archive using `zip::ZipArchive`
  - Enumerate entries:
    - Skip directories
    - **Enforce 10-file limit**: if file count > 10, return `AppError::PayloadTooLarge("ZIP contains more than 10 files")`
    - For each entry: extract filename, read bytes, detect inner file type by extension, validate via `validate_file()`, parse via `parse_file_content()`, chunk via `chunk_document()`, save document + chunks via repository
    - Collect results into `ZipUploadResponse` with per-file status
    - On individual file failure: log WARN and continue (don't abort the whole batch)
  - Iterate until all unit tests pass (GREEN)
  - **Files:** `backend/src/modules/documents/service.rs`
  - **Logging (verbose):**
    - DEBUG: "ZIP opened: {count} entries found"
    - DEBUG: per-file "Extracting: {name} ({size} bytes)"
    - DEBUG: per-file "File processed: {name} -> {chunks} chunks"
    - WARN: per-file "File skipped: {name} - {reason}"
    - INFO: "ZIP upload complete: {ok}/{total} files processed"

- [x] **Task 6: Add `upload_zip` handler + wire route**
  - New handler `upload_zip` in `documents/handlers.rs`
    - Accept multipart with `file` (the ZIP) and `collection_id` fields
    - Extract raw bytes, call `svc.process_zip_upload()`
    - Return `Json<ZipUploadResponse>` on success
  - **413 Payload Too Large** is returned when:
    1. ZIP contains >10 files (enforced in service via `AppError::PayloadTooLarge`)
    2. Overall request body exceeds `RequestBodyLimitLayer` (50 MB)
  - Wire route in `main.rs`: `.route("/api/documents/upload-zip", post(documents_handlers::upload_zip))`
  - Apply `RequestBodyLimitLayer::new(50 * 1024 * 1024)` specifically to the ZIP upload route
  - Export handler from `documents/mod.rs` if needed
  - Iterate until all tests pass (GREEN — including E2E)
  - **Files:** `backend/src/modules/documents/handlers.rs`, `backend/src/main.rs`, `backend/src/modules/documents/mod.rs`
  - **Logging:**
    - INFO handler entry with collection_id
    - INFO handler completion with summary
    - WARN on 413 conditions

### Phase 4: GREEN — Implement Frontend

- [x] **Task 7: Add frontend ZIP upload types and store method**
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

- [x] **Task 8: Update `DocumentList.vue` for ZIP file handling**
  - In `handleFilesSelected()`: detect `.zip` files in the selected file list
    - If a `.zip` file is detected → call `documentStore.uploadZip()` instead of per-file `uploadDocument()`
    - Mixed .zip and non-zip files: process ZIP via batch endpoint, individual files via regular upload
  - Show enhanced upload progress during ZIP processing:
    - `"Обработка ZIP: {processed}/{total} файлов"` (progress message)
    - Show per-file results after completion (success/fail indicators)
  - Handle 413 error display:
    - Show dialog/toast: "Файлов слишком много. ZIP должен содержать не более 10 файлов."
  - **Files:** `frontend/src/components/DocumentList.vue`
  - **Logging:** console.debug on file routing decision

### Phase 5: Validation

- [x] **Task 9: Documentation and build verification**
  - Update `docs/api.md` to add the `POST /api/documents/upload-zip` endpoint documentation
  - Update `docs/gui.md` if needed to describe ZIP upload UI
  - Run `npm run ai:validate` (per RULES.md) and verify exit code 0
  - Verify Docker images build: `docker compose build backend frontend` succeeds
  - Run full test suite: `cargo test` + `npm run test:unit` + `npx playwright test`
  - **Files:** `docs/api.md`, optionally `docs/gui.md`
