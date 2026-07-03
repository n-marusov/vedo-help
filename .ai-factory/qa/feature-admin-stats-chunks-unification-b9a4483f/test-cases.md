# Test Cases

> Branch: `feature/admin-stats-chunks-unification`
> Based on: `test-plan.md`

---

## TC-API-STATS-001: Collection stats for uploaded documents

| Field | Value |
|-------|-------|
| **ID** | TC-API-STATS-001 |
| **Priority** | High |
| **Area** | API — Collection Statistics |
| **Type** | Functional (Happy Path) |
| **Test Data** | Collection with 3 uploaded documents (2 PDF, 1 MD). File sizes: 100KB, 200KB, 300KB. |
| **Preconditions** | Admin user is authenticated. Collection exists with the uploaded documents. |
| **Steps** | 1. `GET /api/collections/{id}/stats` with admin Bearer token |
| **Expected Result** | Response 200. `total_documents` = 3. `total_chunks` > 0 (depends on chunking). `upload_documents` = 3. `git_documents` = 0. `total_git_repos` = 0. `total_file_size_bytes` = 614400 (600KB). `document_types` = `{"application/pdf": 2, "text/markdown": 1}`. |
| **Automated?** | Rust integration test |

---

## TC-API-STATS-002: Collection stats for git-synced documents

| Field | Value |
|-------|-------|
| **ID** | TC-API-STATS-002 |
| **Priority** | High |
| **Area** | API — Collection Statistics |
| **Type** | Functional (Happy Path) |
| **Test Data** | Collection with 1 git repo synced (5 markdown files). |
| **Preconditions** | Admin user authenticated. Collection exists. Git repo has been synced and indexed. |
| **Steps** | 1. `GET /api/collections/{id}/stats` with admin Bearer token |
| **Expected Result** | Response 200. `git_documents` = 5. `upload_documents` = 0. `total_git_repos` = 1. |
| **Automated?** | Rust integration test |

---

## TC-API-STATS-003: Empty collection stats

| Field | Value |
|-------|-------|
| **ID** | TC-API-STATS-003 |
| **Priority** | High |
| **Area** | API — Collection Statistics |
| **Type** | Edge Case |
| **Preconditions** | Admin user authenticated. Collection exists with no documents. |
| **Steps** | 1. `GET /api/collections/{id}/stats` |
| **Expected Result** | Response 200. All counts are 0. `total_file_size_bytes` = 0. `document_types` = `{}`. |
| **Automated?** | Rust integration test |

---

## TC-API-STATS-004: Non-existent collection stats

| Field | Value |
|-------|-------|
| **ID** | TC-API-STATS-004 |
| **Priority** | Medium |
| **Area** | API — Collection Statistics |
| **Type** | Negative |
| **Preconditions** | Admin user authenticated. Collection UUID does not exist. |
| **Steps** | 1. `GET /api/collections/00000000-0000-0000-0000-000000000000/stats` |
| **Expected Result** | Response 404. Error body explains collection not found. |
| **Automated?** | Rust unit test |

---

## TC-API-STATS-005: Non-admin user cannot access stats

| Field | Value |
|-------|-------|
| **ID** | TC-API-STATS-005 |
| **Priority** | High |
| **Area** | API — Authorization |
| **Type** | Negative |
| **Preconditions** | Regular (non-admin) user is authenticated. Collection exists. |
| **Steps** | 1. `GET /api/collections/{id}/stats` with non-admin Bearer token |
| **Expected Result** | Response 403. Error body indicates insufficient permissions. |
| **Automated?** | Rust unit test |

---

## TC-API-CHUNKS-001: Text search returns matching chunks

| Field | Value |
|-------|-------|
| **ID** | TC-API-CHUNKS-001 |
| **Priority** | High |
| **Area** | API — Chunk Search |
| **Type** | Functional (Happy Path) |
| **Test Data** | Collection with uploaded document containing the word "deploy" in a chunk. |
| **Preconditions** | Admin user authenticated. Collection has an uploaded document. Chunks exist in PostgreSQL and Chroma. |
| **Steps** | 1. `GET /api/collections/{id}/chunks?q=deploy&search_type=text&limit=20&offset=0` |
| **Expected Result** | Response 200. Returns array of `ChunkSearchResult`. At least one result contains "deploy" in `text`. Each result has: `chunk_id` (UUID), `document_id`, `document_name`, `chunk_index`, `text`, `source: "upload"`. `score` is `null` for text search. Results are fewer or equal to `limit` (20). |
| **Automated?** | Rust integration test |

---

## TC-API-CHUNKS-002: Semantic search returns chunks with scores

| Field | Value |
|-------|-------|
| **ID** | TC-API-CHUNKS-002 |
| **Priority** | High |
| **Area** | API — Chunk Search |
| **Type** | Functional (Happy Path) |
| **Test Data** | Collection with documents containing various technical terms. |
| **Preconditions** | Admin user authenticated. Collection has documents. Embedding service is running. |
| **Steps** | 1. `GET /api/collections/{id}/chunks?q=deployment&search_type=semantic&top_k=5` |
| **Expected Result** | Response 200. Returns array of `ChunkSearchResult` with `score` values (f64, non-null). Results ordered by relevance (highest score first). Results ≤ `top_k` (5). |
| **Automated?** | Rust integration test |

---

## TC-API-CHUNKS-003: Source filter — upload only

| Field | Value |
|-------|-------|
| **ID** | TC-API-CHUNKS-003 |
| **Priority** | High |
| **Area** | API — Chunk Search |
| **Type** | Functional |
| **Test Data** | Collection with both uploaded documents and git-synced documents. |
| **Preconditions** | Admin user authenticated. Collection has mixed sources. |
| **Steps** | 1. `GET /api/collections/{id}/chunks?q=&search_type=text&source=upload&limit=50&offset=0` |
| **Expected Result** | Response 200. All returned chunks have `source = "upload"`. No chunks with `source = "git"`. |
| **Automated?** | Rust integration test |

---

## TC-API-CHUNKS-004: Source filter — git only

| Field | Value |
|-------|-------|
| **ID** | TC-API-CHUNKS-004 |
| **Priority** | High |
| **Area** | API — Chunk Search |
| **Type** | Functional |
| **Test Data** | Collection with both uploaded documents and git-synced documents. |
| **Preconditions** | Admin user authenticated. Collection has mixed sources. |
| **Steps** | 1. `GET /api/collections/{id}/chunks?q=&search_type=text&source=git&limit=50&offset=0` |
| **Expected Result** | Response 200. All returned chunks have `source = "git"`. `file_path` is present for each result. |
| **Automated?** | Rust integration test |

---

## TC-API-CHUNKS-005: Pagination boundary — limit and offset

| Field | Value |
|-------|-------|
| **ID** | TC-API-CHUNKS-005 |
| **Priority** | Medium |
| **Area** | API — Chunk Search |
| **Type** | Edge Case |
| **Test Data** | Collection with a document that produces 50+ chunks. |
| **Preconditions** | Admin user authenticated. Collection has many chunks. |
| **Steps** | 1. `GET /api/collections/{id}/chunks?q=&search_type=text&limit=10&offset=0` — get first page |
| **Expected Result** | Response 200. Page 1 has exactly 10 results (or fewer if <10 total). |
| **Steps** | 2. `GET /api/collections/{id}/chunks?q=&search_type=text&limit=10&offset=10` — get second page |
| **Expected Result** | Response 200. Page 2 has results that follow page 1 (no duplicates). |
| **Steps** | 3. `GET /api/collections/{id}/chunks?q=&search_type=text&limit=10&offset=1000` — beyond total |
| **Expected Result** | Response 200. Returns empty array `[]`. |
| **Automated?** | Rust integration test |

---

## TC-API-CHUNKS-006: Empty query returns empty results

| Field | Value |
|-------|-------|
| **ID** | TC-API-CHUNKS-006 |
| **Priority** | Medium |
| **Area** | API — Chunk Search |
| **Type** | Negative |
| **Preconditions** | Admin user authenticated. Collection with documents exists. |
| **Steps** | 1. `GET /api/collections/{id}/chunks?q=&search_type=text&limit=20&offset=0` |
| **Expected Result** | Response 200. Returns empty array `[]`. No server error. |
| **Automated?** | Rust unit test |

---

## TC-API-CHUNKS-007: Non-admin user receives 403

| Field | Value |
|-------|-------|
| **ID** | TC-API-CHUNKS-007 |
| **Priority** | High |
| **Area** | API — Authorization |
| **Type** | Negative |
| **Preconditions** | Regular (non-admin) user authenticated. Collection exists. |
| **Steps** | 1. `GET /api/collections/{id}/chunks?q=test&search_type=text` with non-admin token |
| **Expected Result** | Response 403. Error body indicates permission denied. |
| **Automated?** | Rust unit test |

---

## TC-MIGRATION-001: Forward migration adds source column

| Field | Value |
|-------|-------|
| **ID** | TC-MIGRATION-001 |
| **Priority** | High |
| **Area** | Database — Migration |
| **Type** | Migration |
| **Preconditions** | SQLite database at migration state 12 (pre-migration). |
| **Steps** | 1. Run migration `00000000000013_add_source_to_documents.sql` |
| **Expected Result** | Migration succeeds. `documents` table has `source VARCHAR(20) NOT NULL DEFAULT 'upload'`. CHECK constraint exists. Index `idx_documents_collection_id_source` exists. |
| **Steps** | 2. Query existing rows |
| **Expected Result** | All existing rows have `source = 'upload'`. |
| **Automated?** | Rust unit test (via sqlx migrate run) |

---

## TC-MIGRATION-002: Migration rollback

| Field | Value |
|-------|-------|
| **ID** | TC-MIGRATION-002 |
| **Priority** | High |
| **Area** | Database — Migration |
| **Type** | Migration |
| **Preconditions** | Migration 13 has been applied. |
| **Steps** | 1. Roll back migration 13 |
| **Expected Result** | Rollback succeeds. `source` column is removed. Other columns remain intact. |
| **Automated?** | Manual  (rollback SQL not yet written; verify rollback plan) |

---

## TC-MIGRATION-003: CHECK constraint enforcement

| Field | Value |
|-------|-------|
| **ID** | TC-MIGRATION-003 |
| **Priority** | High |
| **Area** | Database — Migration |
| **Type** | Negative |
| **Preconditions** | Migration 13 applied. |
| **Steps** | 1. Execute `INSERT INTO documents (...) VALUES (..., 'invalid_source')` |
| **Expected Result** | SQLite rejects the insert with a CHECK constraint violation error. |
| **Automated?** | Rust unit test |

---

## TC-GITSYNC-001: Git sync creates PG documents with source='git'

| Field | Value |
|-------|-------|
| **ID** | TC-GITSYNC-001 |
| **Priority** | High |
| **Area** | Backend — Git Sync |
| **Type** | Functional |
| **Test Data** | Public git repository with 3 markdown files. |
| **Preconditions** | Admin user authenticated. Collection exists. Git sync service is configured. |
| **Steps** | 1. Sync git repo to collection |
| **Expected Result** | 3 `Document` rows created in PostgreSQL. Each has `source = 'git'`. `is_active = TRUE`. Chunks are created in both PG and Chroma. |
| **Automated?** | Rust integration test |

---

## TC-GITSYNC-002: Re-sync deactivates old documents, creates new

| Field | Value |
|-------|-------|
| **ID** | TC-GITSYNC-002 |
| **Priority** | High |
| **Area** | Backend — Git Sync |
| **Type** | Functional |
| **Preconditions** | Git repo was previously synced to a collection. Documents exist with `source = 'git'`. |
| **Steps** | 1. Sync the same git repo again (pull/update) |
| **Expected Result** | Old documents in PostgreSQL have `is_active = FALSE`. New documents are created with `is_active = TRUE` and new UUIDs. Chroma contains only the new entries. |
| **Automated?** | Rust integration test |

---

## TC-GITSYNC-003: Delete repo deactivates documents

| Field | Value |
|-------|-------|
| **ID** | TC-GITSYNC-003 |
| **Priority** | High |
| **Area** | Backend — Git Sync |
| **Type** | Functional |
| **Preconditions** | Git repo was synced. Documents and local clone exist. |
| **Steps** | 1. Delete the git repository via API |
| **Expected Result** | Documents in PostgreSQL have `is_active = FALSE`. Local clone at `{clone_root}/{user_id}/{repo_id}` is removed. Chroma entries are no longer returned in searches. |
| **Automated?** | Rust integration test |

---

## TC-GITSYNC-004: Clone path includes user_id

| Field | Value |
|-------|-------|
| **ID** | TC-GITSYNC-004 |
| **Priority** | Low |
| **Area** | Backend — Git Sync |
| **Type** | Functional |
| **Preconditions** | Git sync enabled. Admin user with known `user_id`. |
| **Steps** | 1. Sync a git repo |
| **Expected Result** | Clone directory is at `{clone_root}/{user_id}/{repo_id}`. No clone directory at `{clone_root}/{repo_id}` (old format). |
| **Automated?** | Rust integration test |

---

## TC-UI-STATS-001: Statistics tab layout with selection

| Field | Value |
|-------|-------|
| **ID** | TC-UI-STATS-001 |
| **Priority** | High |
| **Area** | Frontend — Statistics Tab |
| **Type** | Functional |
| **Preconditions** | User is logged in as admin. At least one collection exists with uploaded documents. |
| **Steps** | 1. Navigate to Admin page `/admin` |
| **Steps** | 2. Click the "Statistics" tab |
| **Expected Result** | A two-panel layout is visible: StatsPanel (left) and ChunkBrowser (right). Empty state shows "Select a collection". |
| **Steps** | 3. Select a collection from the collection list |
| **Expected Result** | StatsPanel shows 6 stat cards with non-zero values matching the API response. ChunkBrowser shows search input (no results yet). |
| **Automated?** | E2E: TC-STATS-001, TC-STATS-002 |

---

## TC-UI-STATS-002: StatsPanel loading and error states

| Field | Value |
|-------|-------|
| **ID** | TC-UI-STATS-002 |
| **Priority** | Medium |
| **Area** | Frontend — StatsPanel |
| **Type** | Edge Case |
| **Preconditions** | User is admin. Collection exists. |
| **Steps** | 1. Navigate to Statistics tab and select a collection |
| **Expected Result** | Loading skeleton cards display briefly while API responds. After load, data cards replace skeleton. |
| **Steps** | 2. (Simulate) Make the stats API fail (e.g., stop backend) |
| **Steps** | 3. Trigger re-fetch by switching to another collection and back |
| **Expected Result** | Error message is displayed in StatsPanel. No crash. |
| **Automated?** | Manual (network condition simulation) |

---

## TC-UI-CHUNKS-001: Text search in ChunkBrowser

| Field | Value |
|-------|-------|
| **ID** | TC-UI-CHUNKS-001 |
| **Priority** | High |
| **Area** | Frontend — ChunkBrowser |
| **Type** | Functional |
| **Test Data** | Collection with an uploaded document containing "configuration". |
| **Preconditions** | User is admin on the Statistics tab. Collection with documents is selected. |
| **Steps** | 1. Type "configuration" into the search input |
| **Steps** | 2. Press Enter (or wait 300ms for debounce) |
| **Expected Result** | Chunk cards appear below the search input. Each card shows: document name, source badge (Upload), chunk index, truncated text containing "configuration". No score is displayed. |
| **Steps** | 3. Click the × (clear) button |
| **Expected Result** | Search input is cleared. Chunk cards are removed. |
| **Automated?** | E2E: TC-STATS-003 |

---

## TC-UI-CHUNKS-002: Semantic search in ChunkBrowser

| Field | Value |
|-------|-------|
| **ID** | TC-UI-CHUNKS-002 |
| **Priority** | High |
| **Area** | Frontend — ChunkBrowser |
| **Type** | Functional |
| **Preconditions** | User is admin on the Statistics tab. Collection with documents is selected. Embedding service is running. |
| **Steps** | 1. Click "Semantic Search" pill button |
| **Expected Result** | "Semantic Search" pill becomes active (highlighted). Text Search becomes inactive. Pagination buttons disappear (semantic uses `top_k`, not pagination). |
| **Steps** | 2. Type "deployment strategies" and press Enter |
| **Expected Result** | Chunk cards appear with score percentages (e.g., "87%"). Cards are ordered by relevance descending. No pagination controls. |
| **Automated?** | E2E: TC-STATS-004 (toggle only) |

---

## TC-UI-CHUNKS-003: Source filter pills visibility

| Field | Value |
|-------|-------|
| **ID** | TC-UI-CHUNKS-003 |
| **Priority** | Medium |
| **Area** | Frontend — ChunkBrowser |
| **Type** | Functional |
| **Preconditions** | User is admin on the Statistics tab. |
| **Steps** | 1. Select a collection that has ONLY uploaded documents |
| **Expected Result** | No source filter pills are shown. (Condition: stats.git_documents === 0) |
| **Steps** | 2. Select a collection that has git-synced documents |
| **Expected Result** | Source filter pills appear: "All" (active), "Upload", "Git". |
| **Steps** | 3. Click "Git" pill |
| **Steps** | 4. Perform a text search |
| **Expected Result** | Only chunks with `source = "git"` are shown, including `file_path` in each card. |
| **Automated?** | Manual |

---

## TC-UI-CHUNKS-004: Pagination controls

| Field | Value |
|-------|-------|
| **ID** | TC-UI-CHUNKS-004 |
| **Priority** | Medium |
| **Area** | Frontend — ChunkBrowser |
| **Type** | Functional |
| **Test Data** | Collection with a large document that produces 50+ chunks. |
| **Preconditions** | User is admin. Text search mode is active. |
| **Steps** | 1. Perform a broad text search (empty query) that returns many results |
| **Expected Result** | First 20 chunks are displayed. "Prev" button is disabled. "Next" button is enabled. Page indicator shows current page. |
| **Steps** | 2. Click "Next" |
| **Expected Result** | Next 20 chunks are displayed. "Prev" button is now enabled. |
| **Steps** | 3. Click "Next" until all pages are exhausted |
| **Expected Result** | On the last page, "Next" button becomes disabled. |
| **Automated?** | Manual |

---

## TC-UI-BADGE-001: Document source badges

| Field | Value |
|-------|-------|
| **ID** | TC-UI-BADGE-001 |
| **Priority** | High |
| **Area** | Frontend — DocumentList |
| **Type** | Functional |
| **Preconditions** | User is admin. Collection has both uploaded and git-synced documents. |
| **Steps** | 1. Navigate to Admin → "Sources" tab |
| **Expected Result** | Each document in the list shows a badge next to its name. Uploaded documents show "Upload" badge (blue/info variant). Git-synced documents show "Git" badge (green/success variant). |
| **Automated?** | E2E (visual check) |

---

## TC-SSE-001: Frontend handles relevance: 0.0

| Field | Value |
|-------|-------|
| **ID** | TC-SSE-001 |
| **Priority** | Medium |
| **Area** | Frontend — SSE/Query |
| **Type** | Regression |
| **Preconditions** | User is authenticated. A collection with documents is active. |
| **Steps** | 1. Ask a question in the chat that triggers RAG (e.g., "What is deployment?") |
| **Expected Result** | SSE stream completes without JavaScript errors. Source references display correctly. No `null` or `undefined` issues in the relevance display. The relevance shows as a number (e.g., `0.85`) instead of showing `null` or `0.0` as an obvious fallback. |
| **Automated?** | E2E (api-backend.spec.ts) |

---

## TC-REGRESSION-001: Document upload still works

| Field | Value |
|-------|-------|
| **ID** | TC-REGRESSION-001 |
| **Priority** | High |
| **Area** | Backend — Documents |
| **Type** | Regression |
| **Test Data** | A valid PDF or markdown file (< 50MB). |
| **Preconditions** | Admin user. Collection exists. |
| **Steps** | 1. Upload a document to a collection via `POST /api/collections/{id}/documents` |
| **Expected Result** | Response 200. `source` field is NOT returned in the response body (or if it is, it equals `"upload"`). Document appears in the collection listing with `source = "upload"`. |
| **Automated?** | Rust integration test |

---

## TC-REGRESSION-002: Query pipeline still returns answers

| Field | Value |
|-------|-------|
| **ID** | TC-REGRESSION-002 |
| **Priority** | High |
| **Area** | Backend — Query |
| **Type** | Regression |
| **Preconditions** | Collection has indexed documents. LLM API key configured. |
| **Steps** | 1. `POST /api/query` with `{ "collection_id": "...", "question": "test query" }` |
| **Expected Result** | Response 200 with SSE stream. Stream contains source references with document names, relevance scores (f64, not null). Answer text is coherent. |
| **Automated?** | E2E integration test |

---

## TC-API-SCHEMA-001: SSE relevance is f64 not null

| Field | Value |
|-------|-------|
| **ID** | TC-API-SCHEMA-001 |
| **Priority** | Low |
| **Area** | Backend — Query |
| **Type** | Regression |
| **Preconditions** | Collection has documents. |
| **Steps** | 1. Send a query and capture the SSE event stream |
| **Steps** | 2. Parse each source event and inspect the `relevance` field |
| **Expected Result** | `relevance` is always a number (f64), never `null`. Example: `"relevance": 0.8732`. |
| **Automated?** | Rust unit test |

---

## TC-E2E-001: Admin stats E2E tests pass

| Field | Value |
|-------|-------|
| **ID** | TC-E2E-001 |
| **Priority** | Low |
| **Area** | E2E — Admin Stats |
| **Type** | Regression |
| **Preconditions** | Docker Compose is running (frontend, backend, chroma, embedding). Playwright browsers installed. |
| **Steps** | 1. `cd frontend && npx playwright test admin-stats.spec.ts` |
| **Expected Result** | All 4 tests pass (TC-STATS-001 through TC-STATS-004). |
| **Automated?** | Playwright E2E |
