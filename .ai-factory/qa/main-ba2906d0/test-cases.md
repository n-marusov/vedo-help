# Test Cases: Document Upload (UTF-8 Chunking Fix)

> Concrete test scenarios for verifying the UTF-8 chunking fix and the document upload flow through the admin panel.

---

## TC-DOC-001: Upload Markdown file with Russian Cyrillic text

| Field | Value |
|-------|-------|
| **Priority** | High |
| **Type** | Functional |
| **Preconditions** | 1. Docker Compose services healthy (chroma, embedding, backend, frontend)<br>2. User is authenticated with a valid JWT token<br>3. At least one collection exists |
| **Test data** | A `.md` file containing Russian Cyrillic text (e.g., glossary.md from the docs repo, ~137 KB) |
| **Steps** | 1. Open the admin panel at `/admin`<br>2. Select a collection from the dropdown<br>3. Click the drop zone or Upload button<br>4. Select the Russian language `.md` file<br>5. Wait for upload progress to reach 100%<br>6. Verify the document appears in the document list |
| **Expected result** | Document is uploaded successfully. Backend logs show no panic. Document appears in the list with correct name, size, and file type. |
| **Automated check** | All 5 chunking unit tests pass (`cargo test chunking`). E2E test TC-RAG-002 passes (mocked upload). |

---

## TC-DOC-002: Upload ASCII-only text file (regression check)

| Field | Value |
|-------|-------|
| **Priority** | High |
| **Type** | Regression |
| **Preconditions** | Same as TC-DOC-001 |
| **Test data** | A plain `.txt` file with only ASCII characters |
| **Steps** | 1. Open the admin panel at `/admin`<br>2. Select a collection<br>3. Upload the ASCII text file via the drop zone<br>4. Wait for upload to complete |
| **Expected result** | Document upload succeeds. Document appears in the list. No regression from the fix. |
| **Automated check** | `test_chunk_small_text` and `test_chunk_respects_size` pass. |

---

## TC-DOC-003: Upload PDF with mixed ASCII/UTF-8 content

| Field | Value |
|-------|-------|
| **Priority** | High |
| **Type** | Functional |
| **Preconditions** | Same as TC-DOC-001 |
| **Test data** | A `.pdf` file containing a mix of English and non-ASCII characters (e.g., Chinese, accented French, Cyrillic) |
| **Steps** | 1. Navigate to `/admin`<br>2. Select a collection<br>3. Upload the PDF file<br>4. Verify completion |
| **Expected result** | PDF is processed and indexed. No crash in chunking or embedding pipeline. |

---

## TC-DOC-004: Upload unsupported file type

| Field | Value |
|-------|-------|
| **Priority** | Medium |
| **Type** | Negative |
| **Preconditions** | Same as TC-DOC-001 |
| **Test data** | A `.exe` or `.sh` file (binary, not in allowed types) |
| **Steps** | 1. Navigate to `/admin`<br>2. Select a collection<br>3. Attempt to upload the unsupported file via the drop zone<br>4. Observe the response |
| **Expected result** | The file is rejected with a validation error. Backend returns 4xx with a descriptive error message. No crash. |
| **Automated check** | MIME validation in `file_validation.rs` should reject the file. |

---

## TC-DOC-005: Upload without selecting a collection

| Field | Value |
|-------|-------|
| **Priority** | Medium |
| **Type** | Negative |
| **Preconditions** | User is authenticated. **No** collection is selected. |
| **Test data** | Any valid `.md` file |
| **Steps** | 1. Navigate to `/admin`<br>2. Ensure no collection is active (dropdown shows placeholder)<br>3. Attempt to click the Upload button or drop a file |
| **Expected result** | Upload button is disabled. Drop zone may be disabled or show a hint to select a collection first. |
| **Automated check** | UI check: `VButton` has `:disabled` prop when `!collectionStore.activeCollectionId`. |

---

## TC-DOC-006: Upload empty file

| Field | Value |
|-------|-------|
| **Priority** | Medium |
| **Type** | Edge case |
| **Preconditions** | Same as TC-DOC-001 |
| **Test data** | An empty `.md` file (0 bytes) |
| **Steps** | 1. Navigate to `/admin`<br>2. Select a collection<br>3. Upload the empty file |
| **Expected result** | Backend handles the empty file gracefully (either rejects with a validation error or processes it with 0 chunks). No crash. |

---

## TC-DOC-007: Upload with emoji and special Unicode characters

| Field | Value |
|-------|-------|
| **Priority** | Low |
| **Type** | Edge case |
| **Preconditions** | Same as TC-DOC-001 |
| **Test data** | A `.md` file containing emoji (😀🚀), mathematical symbols (∑∫√), and CJK characters (日本語) |
| **Steps** | 1. Navigate to `/admin`<br>2. Select a collection<br>3. Upload the Unicode-heavy file<br>4. Verify completion |
| **Expected result** | File is uploaded and chunked correctly. No panic on 4-byte UTF-8 sequences (emoji). |

---

## TC-DOC-008: Upload ZIP archive with 5 files

| Field | Value |
|-------|-------|
| **Priority** | Medium |
| **Type** | Functional |
| **Preconditions** | Same as TC-DOC-001 |
| **Test data** | A `.zip` archive containing 5 documents of different types (PDF, MD, TXT, HTML, JSON) |
| **Steps** | 1. Navigate to `/admin`<br>2. Select a collection<br>3. Upload the ZIP file via the drop zone<br>4. Observe batch result notification |
| **Expected result** | Zip is processed. Result shows `5 of 5 files processed`. Each file appears in the document list. |

---

## TC-DOC-009: Upload without auth token

| Field | Value |
|-------|-------|
| **Priority** | Medium |
| **Type** | Negative / Security |
| **Preconditions** | No JWT token in localStorage. User is redirected to login page. |
| **Test data** | Any valid `.md` file |
| **Steps** | 1. Clear localStorage (`vedo_auth_token`)<br>2. Navigate to `/admin`<br>3. Observe redirect to login page |
| **Expected result** | The app redirects to KeyCloak login. Direct API call to `POST /api/documents/upload` without Bearer token returns 401. |
| **Automated check** | Auth regression E2E tests (TC-AUTH-REG-010 through 012) cover this scenario. |

---

## TC-DOC-010: Upload large file (>10 MB)

| Field | Value |
|-------|-------|
| **Priority** | Low |
| **Type** | Edge case |
| **Preconditions** | Same as TC-DOC-001 |
| **Test data** | A `.txt` or `.md` file ~15 MB in size |
| **Steps** | 1. Navigate to `/admin`<br>2. Select a collection<br>3. Upload the large file<br>4. Monitor upload progress and server response |
| **Expected result** | File is either accepted and processed (with reasonable time) or rejected with a 413 Payload Too Large error. No crash. Backend rate limiting applies. |
