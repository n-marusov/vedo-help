# Test Cases: Fix E2E Test Failures

**Branch:** `feature/v031-chat-rework`
**Date:** 2026-06-21
**Based on:** Test Plan

---

## TC-001: ZIP with 11+ files returns 413 early

| Field | Value |
|-------|-------|
| **Priority** | High |
| **Type** | Regression |
| **Precondition** | Running backend service |

**Steps:**
1. Create a ZIP archive containing 12 small Markdown files
2. Upload through `/api/documents/upload-zip` endpoint
3. Observe response status code

**Expected result:**
- Status 413 (Payload Too Large)
- Error message: "ZIP contains more than 10 files (found 12)"
- Backend log: `"ZIP has 12 entries — exceeds limit of 10, rejecting early"`
- No per-file validation performed on individual entries

**Automated check:** `test_process_zip_with_11_files_returns_413` unit test passes

---

## TC-002: ZIP with 5 valid files returns 200

| Field | Value |
|-------|-------|
| **Priority** | High |
| **Type** | Regression |
| **Precondition** | Running backend service |

**Steps:**
1. Create a ZIP archive containing 5 valid Markdown files
2. Upload through `/api/documents/upload-zip` endpoint
3. Observe response

**Expected result:**
- Status 200
- Response contains `total_files: 5`, `processed >= 0`
- Each file is individually validated and indexed

**Automated check:** `test_process_zip_with_5_md_files` unit test passes

---

## TC-003: Query returns results with Chroma propagation retry

| Field | Value |
|-------|-------|
| **Priority** | High |
| **Type** | New behavior |
| **Precondition** | Running backend + Chroma + embedding services |

**Steps:**
1. Upload and index a document into a collection
2. Immediately query the collection (within <500ms)
3. Observe backend logs

**Expected result:**
- First query attempt returns 0 results
- Retry loop activates: 3 attempts, 500ms apart
- Results appear on retry 1, 2, or 3
- Log: `"Chroma results found after retry {attempt}"`

**Automated check:** E2E `TC-REINDEX-001` (document-reindexing.spec.ts) passes

---

## TC-004: Loading skeleton shows on slow git-sync repos request

| Field | Value |
|-------|-------|
| **Priority** | Medium |
| **Type** | Regression |
| **Precondition** | Running test environment |

**Steps:**
1. Navigate to `/admin`
2. Route is registered before `setActiveCollection`
3. `GET /api/git-sync/repos` is intercepted with 1s delay
4. Switch to the git tab

**Expected result:**
- `[data-testid="repos-loading-skeleton"]` becomes visible during the delay

**Automated check:** `loading-skeletons.spec.ts` test 4 passes

---

## TC-005: Chat export with 20s timeout for assistant message

| Field | Value |
|-------|-------|
| **Priority** | Medium |
| **Type** | Regression |
| **Precondition** | Running test environment, OpenRouter mock available |

**Steps:**
1. Send a query in chat UI
2. Wait for assistant streaming response
3. Click Export button

**Expected result:**
- Assistant message `<div data-testid="message-assistant">` appears within 20 seconds
- Export button is visible
- Markdown export returns non-empty body with `# ` and `## user` content
- JSON export returns body with `session` and `messages` properties

**Automated check:** `chat-export.spec.ts` both tests pass

---

## TC-006: TC-RAG-003 upload → query → sources with Chroma pause

| Field | Value |
|-------|-------|
| **Priority** | High |
| **Type** | Regression |
| **Precondition** | Running test environment |

**Steps:**
1. Navigate to `/admin`
2. Set active collection
3. Upload `config-guide.md` via drop zone
4. Wait for document to appear in list
5. Wait additional 2s for Chroma propagation
6. Navigate to `/`
7. Send query: "How is rate limiting configured?"

**Expected result:**
- User message visible
- Assistant streaming response visible within 30s
- Response content contains "backend answer" or "Sources"

**Note:** The 2s pause combined with the 3×500ms Chroma retry loop provides ~3.5s total propagation window, significantly increasing the chance of finding indexed chunks.

---

## TC-007: Backend unit test suite

| Field | Value |
|-------|-------|
| **Priority** | High |
| **Type** | Regression |
| **Precondition** | Running test DB |

**Steps:**
1. `export DATABASE_URL=postgres://vedo:test-vedo-password@localhost:15432/vedo`
2. `cd backend && cargo test --lib`

**Expected result:** 64 tests pass, 0 fail

---

## TC-008: Frontend unit test suite

| Field | Value |
|-------|-------|
| **Priority** | High |
| **Type** | Regression |

**Steps:**
1. `cd frontend && npm test`

**Expected result:** 76+ tests pass (Vitest)

---

## Pre-existing Issues (Not Fixed)

| Issue | Root Cause |
|-------|-----------|
| Chat export returns empty response | Session may not have persisted messages in time for export, or export path has an auth/data issue |
| `messages-loading-skeleton` not visible | Vue template ordering: `v-if="!chatStore.activeSessionId"` catches the state before `v-else-if="chatStore.isSessionLoading"` |
| `documents-loading-skeleton` not visible | The loading state may resolve before the template evaluates, or the `documents.length === 0` check short-circuits |
