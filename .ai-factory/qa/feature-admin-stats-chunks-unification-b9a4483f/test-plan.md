# Test Plan

> Branch: `feature/admin-stats-chunks-unification`
> Based on: `change-summary.md`

---

## Scope

### In Scope

| Area | Description |
|------|-------------|
| Admin Statistics API | `GET /api/collections/:id/stats` — counts by source, file size, file types, git repos |
| Admin Chunk Search API | `GET /api/collections/:id/chunks` — text (ILIKE) and semantic (Chroma) search with source filter and pagination |
| Shared Chunk Search Module | `chunk_search.rs` — reused by both query pipeline and admin endpoints |
| Statistics Tab UI | StatsPanel component rendering, states (loading/error/empty/data), collection switch reactivity |
| Chunk Browser UI | Text/semantic search toggle, source filter pills, pagination, debounced input, chunk cards |
| Document Source Badges | Source badges (Git/Upload) in DocumentList |
| Git Sync Dual Indexing | Documents now stored in PostgreSQL alongside Chroma; clone path with user_id; soft deactivation |
| Migration Data Integrity | `source` column, CHECK constraint, DEFAULT behavior, existing row backfill |
| SSE Schema Change | `SourceRef.relevance` from `Option<f64>` to `f64` — frontend handling of `0.0` |

### Out of Scope

| Area | Reason |
|------|--------|
| Existing query pipeline accuracy | Only refactored internally (logic preserved); RAG accuracy tested elsewhere |
| Authentication/authorization framework | Existing pattern unchanged; admin role check follows same convention |
| LLM response quality | Not affected by this change |
| Chroma internal behavior | Tested via integration tests |
| Performance under concurrent load | Acceptable for admin-only endpoints; not a stated requirement |
| ZIP batch upload source tagging | Tagged in same task as single upload; no new logic |

---

## Test Types

| Type | Description |
|------|-------------|
| **Functional** | Happy-path verification of each new endpoint, component, and behavior |
| **Negative** | Invalid inputs, missing parameters, non-admin access, empty collections |
| **Edge Case** | Boundary values for pagination, empty search results, very long query strings, deactivated documents |
| **Regression** | Existing query pipeline still works, document upload still creates valid documents, git sync still indexes |
| **Migration** | Forward migration adds column with correct defaults, rollback removes column cleanly, existing data preserved |
| **UI/UX** | Component states (loading/error/empty/data), responsive layout, search interaction patterns |

---

## Verification Checklist

### High Priority

- [ ] **API-STATS-1:** `GET /api/collections/:id/stats` returns correct counts for upload-only collection
- [ ] **API-STATS-2:** `GET /api/collections/:id/stats` returns correct counts for git-only collection (after git sync)
- [ ] **API-STATS-3:** `GET /api/collections/:id/stats` returns correct counts for mixed (upload + git) collection
- [ ] **API-STATS-4:** `GET /api/collections/:id/stats` returns zero counts for empty collection
- [ ] **API-CHUNKS-1:** `GET /api/collections/:id/chunks?q=term&search_type=text` returns matching chunks with pagination
- [ ] **API-CHUNKS-2:** `GET /api/collections/:id/chunks?q=term&search_type=semantic` returns chunks with scores
- [ ] **API-CHUNKS-3:** `GET /api/collections/:id/chunks` with `source=upload` returns only upload documents
- [ ] **API-CHUNKS-4:** `GET /api/collections/:id/chunks` with `source=git` returns only git documents
- [ ] **API-AUTH-1:** Non-admin user receives 403 on both stats and chunks endpoints
- [ ] **API-AUTH-2:** Admin user can access both endpoints with correct data
- [ ] **UI-STATS-1:** Statistics tab renders StatsPanel and ChunkBrowser when no collection selected (empty state)
- [ ] **UI-STATS-2:** After selecting a collection, StatsPanel shows correct counts matching API response
- [ ] **UI-CHUNKS-1:** ChunkBrowser displays chunk cards after text search, with correct document name, source badge, and text
- [ ] **UI-CHUNKS-2:** ChunkBrowser displays chunk cards after semantic search, with score percentage shown
- [ ] **UI-CHUNKS-3:** Source filter pills appear when collection has git documents, hide when it does not
- [ ] **UI-BADGE-1:** DocumentList shows Git (green) badge for git-synced documents and Upload (blue) badge for uploaded documents
- [ ] **MIGRATION-1:** Forward migration adds `source` column with correct CHECK constraint and DEFAULT
- [ ] **MIGRATION-2:** Migration rollback removes the `source` column without data loss for other columns
- [ ] **MIGRATION-3:** Existing documents automatically get `source = 'upload'` after migration
- [ ] **GITSYNC-1:** New git sync creates `Document` rows with `source = 'git'` in PostgreSQL
- [ ] **GITSYNC-2:** Re-syncing a git repo deactivates old documents and creates new ones (soft delete)
- [ ] **GITSYNC-3:** Deleting a git repo deactivates its documents in PostgreSQL

### Medium Priority

- [ ] **API-STATS-5:** File types breakdown returns correct distribution
- [ ] **API-STATS-6:** Stats for non-existent collection returns 404
- [ ] **API-CHUNKS-5:** Empty search query returns empty results (not error)
- [ ] **API-CHUNKS-6:** Pagination: `limit=10&offset=0` returns at most 10 results
- [ ] **API-CHUNKS-7:** Pagination: `offset` beyond available results returns empty array
- [ ] **API-CHUNKS-8:** Non-existing collection ID returns 404 on chunks endpoint
- [ ] **UI-CHUNKS-4:** Pagination buttons appear for text search, Next disabled when results < pageSize
- [ ] **UI-CHUNKS-5:** Clearing search input resets results
- [ ] **UI-CHUNKS-6:** Switching collection clears previous search results and stats
- [ ] **UI-CHUNKS-7:** Search type toggle switches parameter shape (limit/offset vs top_k)
- [ ] **UI-STATS-3:** Loading skeleton appears while fetching stats
- [ ] **UI-STATS-4:** Error state displays when stats API fails
- [ ] **UI-STATS-5:** FormatBytes handles 0, 1KB, 1MB, 1GB, very large values correctly
- [ ] **REGRESSION-1:** Existing document upload still creates valid documents with auto-increment source='upload'
- [ ] **REGRESSION-2:** Existing query pipeline (chat) still returns answers with source references
- [ ] **REGRESSION-3:** SSE stream still works end-to-end (query → Chroma → LLM → streamed response)
- [ ] **REGRESSION-4:** Frontend handles SSE `relevance: 0.0` instead of `null` without errors
- [ ] **GITSYNC-4:** Clone path creates `{clone_root}/{user_id}/{repo_id}` structure
- [ ] **GITSYNC-5:** Deleting repo removes local clone from correct (new) path

### Low Priority

- [ ] **UI-DOCS-1:** API documentation for both endpoints is accurate with correct curl examples
- [ ] **UI-E2E-1:** E2E Playwright tests for admin stats pass consistently
- [ ] **UI-STATS-6:** Responsive layout stacks panels at 768px breakpoint
- [ ] **UI-CHUNKS-8:** Chunk card truncates text at 300 characters with ellipsis
- [ ] **MIGRATION-4:** Composite index `(collection_id, source)` is created and used in query plans
- [ ] **API-SCHEMA-1:** SSE relevance `0.0` is treated as neutral (no visual "relevance" emphasis)
