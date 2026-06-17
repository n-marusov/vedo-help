## Test Cases: Git Repository Sync — Phase 1 Tests First

---

### TC-GIT-001: Register new git repo → row appears in table with idle status

**Priority:** High
**Type:** Positive

**Precondition:**

- User is authenticated (valid JWT token in localStorage)
- Admin panel is loaded
- Git Repositories section is visible

**Steps:**

1. Navigate to `/admin`
2. Verify git repository section is visible (`[data-testid="git-repo-section"]`)
3. Fill URL input `[data-testid="git-repo-url-input"]` with `https://github.com/user/test-repo.git`
4. Fill branch input `[data-testid="git-repo-branch-input"]` with `main`
5. Fill token input `[data-testid="git-repo-token-input"]` with `ghp_test123`
6. Select collection from dropdown `[data-testid="git-repo-collection-select"]` → `col-1`
7. Click "Register" button `[data-testid="btn-git-repo-register"]`
8. Wait for new row `[data-testid="git-repo-row"]` to appear

**Expected result:**

- A new table row appears showing the registered repo URL (`github.com/user/test-repo.git`)
- Status badge `[data-testid="git-repo-status"]` shows `idle`
- Token value is NOT visible anywhere in the row DOM
- ✅ **Implemented** in `frontend/e2e/git-sync.spec.ts` L88–160

**Test data:**

```
url: "https://github.com/user/test-repo.git"
branch: "main"
token: "ghp_test123"
collection: "col-1" → "Test Collection"
```

---

### TC-GIT-002: Trigger sync and observe status transitions

**Priority:** High
**Type:** Positive

**Precondition:**

- At least one repo is registered and displayed in the table
- Backend mock returns `202 Accepted` on sync POST and `200` with completed status on GET poll

**Steps:**

1. Navigate to `/admin` → Git Repositories section
2. Locate the "Sync Now" button `[data-testid="btn-git-sync-now"]` on a repo row
3. Click "Sync Now"
4. Observe the POST request is sent to `**/api/git-repos/repo-1/sync`
5. Wait for the status badge to update

**Expected result:**

- `syncRequested` flag is set to true (POST was sent)
- Status badge remains visible after sync completes
- Repo row shows `files_indexed: 12` count in the UI
- Status transitions are surfaced (POST → `syncing`, GET poll → `idle`)
- ✅ **Implemented** in `frontend/e2e/git-sync.spec.ts` L162–241

**Test data:**

```
repo_id: "repo-1"
POST response: { status: "syncing", files_indexed: 0 }
GET response: { status: "idle", files_indexed: 12, chunks_total: 48, last_commit: "abc123def" }
```

---

### TC-GIT-003: Delete repo → confirm dialog → row removed

**Priority:** High
**Type:** Positive

**Precondition:**

- At least one repo is visible in the table
- Confirmation dialog component (`VDialog`) is rendered when delete is clicked

**Steps:**

1. Navigate to Git Repositories section
2. Click "Delete" button `[data-testid="btn-git-repo-delete"]` on a repo row
3. Confirm dialog appears — verify `[data-testid="confirm-dialog"]` is visible
4. Click "Confirm" button `[data-testid="btn-confirm-delete"]` inside dialog
5. Observe DELETE request sent to `**/api/git-repos/repo-1`

**Expected result:**

- DELETE is called on the backend (verified via `deleteCalled = true`)
- After delete, GET repos list is updated to exclude the deleted repo
- Row is removed from the table
- ✅ **Implemented** in `frontend/e2e/git-sync.spec.ts` L243–296

**Test data:**

```
repo_id: "repo-1"
DELETE response: 204 No Content
```

---

### TC-GIT-004: Form validation errors — empty submit and invalid URL

**Priority:** Medium
**Type:** Negative

**Precondition:**

- Registration form is displayed in Git Repositories section

**Steps:**

1. Navigate to Git Repositories section
2. Click "Register" button without filling any fields
3. Verify inline error messages appear for required fields
4. Fill URL input with `ftp://bad-protocol/repo.git`
5. Click "Register" again

**Expected result:**

- After empty submit: URL error `[data-testid="git-repo-url-error"]` shows text matching `/required|обязатель/i`
- Collection error `[data-testid="git-repo-collection-error"]` is also visible
- After invalid URL: URL error shows text matching `/https:\/\/|git@/i` (protocol format validation)
- No network request is sent
- ✅ **Implemented** in `frontend/e2e/git-sync.spec.ts` L298–336

**Test data:**

```
Empty fields: url="", branch="", token="", collection=unselected
Invalid URL: "ftp://bad-protocol/repo.git"
```

---

### TC-GIT-005: Sync error state — broken URL shows error badge with tooltip

**Priority:** High
**Type:** Negative

**Precondition:**

- A repo is registered with an intentionally broken URL (`https://nonexistent.invalid/repo.git`)
- Backend returns `status: "error"` with an error message after sync attempt

**Steps:**

1. Navigate to Git Repositories section (mocked with error repo in list)
2. Verify error status badge `[data-testid="git-repo-status"]` contains text `error`
3. Verify error tooltip/message `[data-testid="git-repo-error"]` is visible
4. Click "Sync Now" on the broken repo row
5. Verify status badge still shows `error` after sync attempt

**Expected result:**

- Status badge shows `error` (red styling)
- Error message is displayed (e.g., "Failed to clone: repository not found")
- Error persists after re-sync attempt
- ✅ **Implemented** in `frontend/e2e/git-sync.spec.ts` L338–418

**Test data:**

```
repo_id: "repo-broken"
url: "https://nonexistent.invalid/repo.git"
GET response: { status: "error", error: "Failed to clone: repository not found" }
```

---

### TC-GIT-006: List shows multiple repos with correct collection names

**Priority:** Medium
**Type:** Positive

**Precondition:**

- Two repos are registered with different URLs, branches, and collection associations

**Steps:**

1. Navigate to Git Repositories section (mocked with 2 repos)
2. Count repo rows `[data-testid="git-repo-row"]`
3. Verify first row content
4. Verify second row content

**Expected result:**

- Exactly 2 repo rows are displayed
- First row: shows `github.com/user/docs.git` and collection `Engineering Docs`
- Second row: shows `github.com/user/api.git` and collection `API Reference`
- Different branches are correctly displayed
- ✅ **Implemented** in `frontend/e2e/git-sync.spec.ts` L420–482

**Test data:**

```
Repo 1: url="https://github.com/user/docs.git", branch="main", collection="Engineering Docs"
Repo 2: url="https://github.com/user/api.git", branch="develop", collection="API Reference"
```

---

### TC-GIT-007: Unauthenticated access returns 401 (NEW)

**Priority:** High
**Type:** Negative (Security)

**Precondition:**

- No auth token or API key in localStorage
- Backend is configured to require Bearer token or `x-api-key` header

**Steps:**

1. Clear localStorage (`vedo_auth_token` and `vedo_api_key`)
2. Navigate to `/admin` or make direct API request to `/api/git-repos`
3. Observe response

**Expected result:**

- API returns `401 Unauthorized`
- UI redirects to login page or shows authentication error
- Git repos data is NOT leaked without authentication
- ❌ **NOT YET IMPLEMENTED** — this test must be added to `frontend/e2e/git-sync.spec.ts`

**Test data:**

```
No localStorage entries
Expected: 401 status code
```

---

### TC-GIT-008: Delete dialog cancel/Escape → row stays (NEW)

**Priority:** Medium
**Type:** Negative

**Precondition:**

- At least one repo visible in table
- Confirmation dialog supports cancel and Escape dismissal

**Steps:**

1. Click "Delete" button on a repo row
2. Confirm dialog appears
3. Click "Cancel" button or press Escape key
4. Verify dialog closes
5. Verify repo row is still present (DELETE was NOT called)

**Expected result:**

- Dialog closes without sending DELETE request
- Repo row remains in the table
- No changes to the repos list
- ❌ **NOT YET IMPLEMENTED** — this test must be added to `frontend/e2e/git-sync.spec.ts`

**Test data:**

```
Dialog: cancel button click → no DELETE sent
Dialog: Escape key press → no DELETE sent
```

---

### TC-GIT-009: Empty repos list shows zero-state message (NEW)

**Priority:** Medium
**Type:** Positive

**Precondition:**

- No repos registered (empty list response from API)

**Steps:**

1. Mock GET `/api/git-repos` to return `[]`
2. Navigate to Git Repositories section
3. Observe the UI state

**Expected result:**

- No repo rows are displayed
- Zero-state message or placeholder is shown (e.g., "No git repositories registered yet")
- "Register" form is still available and functional
- ❌ **NOT YET IMPLEMENTED** — this test must be added to `frontend/e2e/git-sync.spec.ts`

**Test data:**

```
GET /api/git-repos → []
Expected: empty-state placeholder element visible
```

---

### TC-INT-001: Create and list repo — contract validation

**Priority:** High
**Type:** Positive

**Steps:**

1. Construct create request JSON with required fields
2. Verify request shape matches `CreateRepoRequest` struct
3. Verify `access_token` field is present in request but MUST NOT appear in summary JSON

**Expected result:**

- Request has: `url`, `branch`, `access_token`, `collection_id`
- Summary JSON does NOT contain: `access_token`, `webhook_secret`, `local_path`
- ✅ **Implemented** in `backend/tests/git_sync_integration.rs` L125–144

**Test data:**

```json
{
  "url": "https://github.com/user/test-docs.git",
  "branch": "main",
  "access_token": "ghp_test123",
  "collection_id": "col-test-1"
}
```

---

### TC-INT-002: Sync markdown repo from local fixture → query Chroma

**Priority:** High
**Type:** Positive

**Steps:**

1. Create unique Chroma collection
2. Create local fixture git repo with 2 `.md` files
3. Clone the fixture locally
4. Count `.md` files via `walkdir`
5. Simulate embedding + indexing into Chroma
6. Query Chroma with a known phrase from fixture files
7. Clean up collection

**Expected result:**

- At least 2 `.md` files found in fixture
- Embeddings stored successfully
- Query returns at least 1 result with `score > 0.0`
- Metadata includes `source: "git"`
- ✅ **Implemented** in `backend/tests/git_sync_integration.rs` L153–269

**Test data:**

```
Fixture files: readme.md, guide.md
Embedding dim: 8
Query: [0.0, 0.5, 0.5, 0.0, 0.1, 0.2, 0.3, 0.4]
```

---

### TC-INT-003: Incremental sync detects new file

**Priority:** High
**Type:** Positive

**Steps:**

1. Create fixture repo with 2 `.md` files
2. Clone and count initial `.md` files
3. Add new `.md` file and commit
4. Re-clone and count updated `.md` files
5. Verify count increased
6. Verify Chroma collection re-creation with same name succeeds after delete

**Expected result:**

- `updated_count > initial_count`
- New file detected after incremental commit
- ✅ **Implemented** in `backend/tests/git_sync_integration.rs` L278–378

**Test data:**

```
Initial: 2 .md files (readme.md, guide.md)
Added: new-feature.md
Expected: updated_count = 3, > initial_count (2)
```

---

### TC-INT-004: Delete repo cleans up Chroma and local clone

**Priority:** High
**Type:** Positive

**Steps:**

1. Create Chroma collection
2. Verify duplicate create fails (collection exists)
3. Delete collection
4. Verify re-create with same name succeeds
5. Verify local temp directory is removed after drop

**Expected result:**

- Chroma collection deleted successfully
- Re-create with same name works (proof of deletion)
- Local clone directory removed after `drop()`
- ✅ **Implemented** in `backend/tests/git_sync_integration.rs` L386–425

**Test data:**

```
Collection: unique_collection("git_sync_delete")
TempDir: auto-cleanup via tempfile crate
```

---

### TC-INT-005: Sync empty repo → 0 files indexed

**Priority:** Medium
**Type:** Edge case

**Steps:**

1. Create fixture repo with only non-`.md` files (`notes.txt`)
2. Clone and count `.md` files
3. Create Chroma collection and query it empty

**Expected result:**

- `md_count == 0`
- Empty collection query returns no results (`results.is_empty()`)
- ✅ **Implemented** in `backend/tests/git_sync_integration.rs` L433–518

**Test data:**

```
Repo contents: notes.txt (non-markdown)
Expected: 0 .md files found
```

---

### TC-INT-006: Sync repo with nested directories indexes all files

**Priority:** Medium
**Type:** Positive

**Steps:**

1. Create fixture repo with nested dirs: `docs/guide.md`, `docs/api/ref.md`, `readme.md`
2. Clone and collect `.md` files with relative paths
3. Verify specific nested paths are found
4. Index into Chroma with file-path metadata
5. Query for nested file content

**Expected result:**

- Exactly 3 `.md` files found across directories
- Files list includes `docs/guide.md` and `docs/api/ref.md`
- Query results reference `nested-doc-*` document IDs
- ✅ **Implemented** in `backend/tests/git_sync_integration.rs` L527–648

**Test data:**

```
Files: docs/guide.md, docs/api/ref.md, readme.md
Expected: 3 files discovered
```

---

### TC-INT-007: Sync status transitions contract

**Priority:** High
**Type:** Positive

**Steps:**

1. Validate `idle` state JSON shape
2. Validate `syncing` state JSON shape with commit hash
3. Validate `error` state JSON shape with error message
4. Verify all three statuses are in the valid set

**Expected result:**

- `idle`: `error` is null, `files_indexed` and `chunks_total` are numbers
- `syncing`: `last_commit` set to `"abc123def456"`
- `error`: `error` field is non-empty string
- Valid statuses: `["idle", "syncing", "error"]`
- ✅ **Implemented** in `backend/tests/git_sync_integration.rs` L661–716

**Test data:**

```
idle: { "status": "idle", "error": null }
syncing: { "status": "syncing", "last_commit": "abc123def456" }
error: { "status": "error", "error": "Failed to clone: SSL certificate verify failed" }
```

---

### TC-INT-008: Concurrent sync on same repo (NEW)

**Priority:** Medium
**Type:** Edge case (Concurrency)

**Precondition:**

- A repo is registered with `status: "idle"`
- Two sync requests can be issued simultaneously

**Steps:**

1. Start first sync on repo `r-1` (transition to `syncing`)
2. Before first sync completes, attempt second sync on same repo
3. Observe behavior

**Expected result:**

- Backend either queues the second request or returns `409 Conflict`
- No duplicate indexing occurs
- Database state remains consistent (no corruption)
- ❌ **NOT YET IMPLEMENTED** — this test must be added to `backend/tests/git_sync_integration.rs`

**Test data:**

```
Two parallel sync triggers on repo_id = "repo-1"
Expected: no data corruption, no duplicate Chroma entries
```

---

### TC-UNIT-001: Create repo persists all fields, summary omits sensitive data

**Priority:** High
**Type:** Positive

**Steps:**

1. Create git_repositories table in in-memory SQLite
2. Create test collection for FK constraint
3. Insert repo with all fields set (including `access_token`)
4. Retrieve row and verify all fields
5. Verify summary JSON shape excludes `access_token`, `webhook_secret`, `local_path`

**Expected result:**

- DB has `access_token` stored (verified via raw SQL query)
- Summary JSON does NOT contain `access_token` (`summary.get("access_token").is_none()`)
- Summary JSON does NOT contain `local_path` or `webhook_secret`
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L25–141

**Test data:**

```
repo_id: "repo-test-001"
url: "https://github.com/user/test-repo.git"
access_token: "ghp_secret_token_12345"
collection_id: "col-test-1"
```

---

### TC-UNIT-002: List repos returns all

**Priority:** Medium
**Type:** Positive

**Steps:**

1. Create git_repositories table
2. Insert 3 repos with different URLs
3. Query count

**Expected result:**

- `SELECT COUNT(*)` returns 3
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L145–206

**Test data:**

```
repo-1: https://github.com/user/repo-1.git
repo-2: https://github.com/user/repo-2.git
repo-3: https://github.com/user/repo-3.git
```

---

### TC-UNIT-003: Update sync status changes commit_hash and status

**Priority:** High
**Type:** Positive

**Steps:**

1. Insert repo with `status: "idle"`
2. Update to `status: "syncing"`, `last_commit_hash: "abc123def456"`
3. Query and verify
4. Update again to `status: "error"`
5. Query and verify

**Expected result:**

- After first update: `hash = "abc123def456"`, `status = "syncing"`
- After second update: `status = "error"`
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L210–304

**Test data:**

```
Commit 1: abc123def456 → status: syncing
Commit 2: status → error
```

---

### TC-UNIT-004: Delete repo removes row

**Priority:** High
**Type:** Positive

**Steps:**

1. Insert repo
2. Verify it exists (`COUNT = 1`)
3. Delete by id
4. Verify it's gone (`COUNT = 0`)

**Expected result:**

- Row is completely removed from `git_repositories` table
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L308–382

**Test data:**

```
repo_id: "repo-del"
```

---

### TC-UNIT-005: Same URL allowed with different collection_id

**Priority:** Medium
**Type:** Edge case

**Steps:**

1. Create two collections (`col-dup-a`, `col-dup-b`)
2. Insert repo 1: URL `https://github.com/user/shared-repo.git` → collection `col-dup-a`, branch `main`
3. Insert repo 2: same URL → collection `col-dup-b`, branch `develop`
4. Count repos with this URL

**Expected result:**

- Both inserts succeed (no UNIQUE constraint violation on URL)
- `COUNT(*) WHERE url = shared_url` returns 2
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L386–469

**Test data:**

```
URL: "https://github.com/user/shared-repo.git"
Repo1: collection=col-dup-a, branch=main
Repo2: collection=col-dup-b, branch=develop
```

---

### TC-UNIT-006: Token injection into HTTPS URL

**Priority:** High
**Type:** Functional

**Steps:**

1. Call `inject_token("https://github.com/user/repo.git", "ghp_secret123")`
2. Verify output

**Expected result:**

- Output: `https://ghp_secret123@github.com/user/repo.git`
- Token is injected after `https://` and before hostname
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L477–498

**Test data:**

```
Input: url="https://github.com/user/repo.git", token="ghp_secret123"
Expected: "https://ghp_secret123@github.com/user/repo.git"
```

---

### TC-UNIT-007: No-token URL passed through unchanged

**Priority:** Medium
**Type:** Edge case

**Steps:**

1. Call `inject_token` with empty token for HTTPS, SSH, file:// URLs
2. Verify all pass through unchanged

**Expected result:**

- `"https://github.com/user/pub.git"` → unchanged
- `"git@github.com:user/repo.git"` → unchanged
- `"file:///tmp/repo"` → unchanged
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L502–523

**Test data:**

```
Empty token for: https://, git@, file:// URLs
```

---

### TC-UNIT-008: Parse finds only .md files (top-level)

**Priority:** Medium
**Type:** Functional

**Steps:**

1. Create temp dir with: `readme.md`, `guide.md`, `notes.txt`, `image.png`
2. Call `find_md_files(dir)`
3. Verify results

**Expected result:**

- Returns exactly 2 files: `readme.md`, `guide.md`
- Does NOT include `notes.txt` or `image.png`
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L527–561

**Test data:**

```
Files in dir: readme.md, guide.md, notes.txt, image.png
Expected: [readme.md, guide.md]
```

---

### TC-UNIT-009: Parse skips files over 10 MB

**Priority:** Medium
**Type:** Edge case

**Steps:**

1. Check `should_skip(10 * 1024 * 1024)` → false
2. Check `should_skip(10 * 1024 * 1024 + 1)` → true
3. Check `should_skip(11 * 1024 * 1024)` → true
4. Check `should_skip(100)` → false
5. Check `should_skip(1)` → false

**Expected result:**

- Exactly 10 MB = allowed, 10 MB + 1 = skipped
- Small files always allowed
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L565–586

**Test data:**

```
MAX_FILE_SIZE: 10 * 1024 * 1024 (10,485,760 bytes)
```

---

### TC-UNIT-010: Full clone pipeline order

**Priority:** High
**Type:** Functional

**Steps:**

1. Define expected pipeline steps
2. Verify order: clone → parse → chunk → embed → index → update_status
3. Verify: embedding comes before indexing
4. Verify: clone comes before parse

**Expected result:**

- All 6 steps present in correct order
- `embed_idx < index_idx`
- `clone_idx < parse_idx`
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L595–630

**Test data:**

```
Steps: ["clone", "parse", "chunk", "embed", "index", "update_status"]
```

---

### TC-UNIT-011: Incremental sync pipeline uses git diff

**Priority:** High
**Type:** Functional

**Steps:**

1. Set `has_last_commit = true`
2. Verify sync_mode is `"incremental"`
3. Verify pipeline steps: pull → git_diff → parse_changed → embed_changed → reindex → update_status

**Expected result:**

- Incremental path activated when `last_commit_hash` is present
- Pipeline differs from full clone (uses `git_diff` and `parse_changed` instead of `clone` and `parse`)
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L634–671

**Test data:**

```
has_last_commit = true → incremental mode
Steps: ["pull", "git_diff", "parse_changed", "embed_changed", "reindex", "update_status"]
```

---

### TC-UNIT-012: Sync failure sets error status with message

**Priority:** High
**Type:** Negative

**Steps:**

1. Simulate clone failure with error message
2. Verify status after failure is `"error"`
3. Verify error response JSON shape
4. Check `files_indexed = 0`, `chunks_total = 0`

**Expected result:**

- `status == "error"`
- Error message is non-empty and informative
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L679–703

**Test data:**

```
error_message: "Failed to clone: SSL certificate verify failed"
Expected: { status: "error", error: "..." , files_indexed: 0, chunks_total: 0 }
```

---

### TC-UNIT-013: Index metadata contract

**Priority:** High
**Type:** Functional

**Steps:**

1. Define metadata JSON with required fields
2. Verify `source == "git"`, `repo_id`, `file_path`, `document_id`, `chunk_index`, `text`
3. Check all 6 required fields are present

**Expected result:**

- All required fields present: `text`, `document_id`, `chunk_index`, `source`, `repo_id`, `file_path`
- `source` must equal `"git"` (distinguishes git-sourced documents from uploads)
- ✅ **Implemented** in `backend/tests/git_sync_unit.rs` L712–745

**Test data:**

```json
{
  "text": "# Getting Started\n\nInstallation guide.",
  "document_id": "doc-git-001",
  "chunk_index": 0,
  "source": "git",
  "repo_id": "repo-test",
  "file_path": "docs/guide.md"
}
```

---

### TC-UNIT-014: Recursive `.md` file discovery in nested directories (NEW)

**Priority:** Medium
**Type:** Functional

**Precondition:**

- Temp directory with nested structure: `readme.md`, `docs/guide.md`, `docs/api/ref.md`, `notes.txt`

**Steps:**

1. Create temp dir with the nested file structure
2. Call a recursive version of `find_md_files()` using `walkdir`
3. Count results
4. Verify specific nested paths are found

**Expected result:**

- Returns exactly 3 `.md` files (including those in subdirectories)
- Files include paths relative to root
- Non-`.md` files excluded
- ❌ **NOT YET IMPLEMENTED** — this test must be added to `backend/tests/git_sync_unit.rs`

**Test data:**

```
Structure:
  readme.md
  notes.txt
  docs/guide.md
  docs/api/ref.md

Expected: 3 .md files found
Current behavior (top-level only): 1 .md file found
```

---

## Test Data (based on test design techniques)

### Positive

* `https://github.com/user/test-repo.git`, branch `main`, token `ghp_test123`, collection `col-1` → registered successfully
* Fixture repo with `readme.md` + `guide.md` → 2 files indexed into Chroma
* Nested fixture: `docs/guide.md` + `docs/api/ref.md` + `readme.md` → all 3 indexed
* Two repos with different URLs → both appear in list with correct collection names
* `inject_token("https://...", "ghp_xxx")` → `https://ghp_xxx@...`
* Pipeline order: clone → parse → chunk → embed → index → update_status

### Negative

* Empty form submit → required field errors
* `ftp://bad-protocol/repo.git` → URL format error `/https:\/\/|git@/i`
* `https://nonexistent.invalid/repo.git` → error badge with "Failed to clone" message
* No auth token → 401 Unauthorized
* Delete dialog cancel/Escape → row stays, no DELETE sent
* Clone failure → status `error`, `files_indexed = 0`
* Empty repo (no `.md` files) → `files_indexed = 0`, `status = "idle"`
* Two parallel syncs on same repo → no corruption, 409 or queued
