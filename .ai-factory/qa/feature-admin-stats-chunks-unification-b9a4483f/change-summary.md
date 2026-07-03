# Change Summary

> Branch: `feature/admin-stats-chunks-unification`
> Base: `main`
> Commits: 2 (`e558726`, `f0915c7`)
> Files changed: 27 (1 migration, 12 backend, 1 docs, 1 e2e, 5 frontend, 7 test/config)
> Added: 2173 lines | Removed: 272 lines

---

## Overview

This branch introduces **admin statistics and chunk browsing** features alongside a **unified chunk search module** that is reused by both the query pipeline (RAG) and the new admin chunk browser. Documents are now tagged with a `source` discriminator (`"upload"` vs `"git"`) at the database level, and the git sync subsystem was refactored to store documents in PostgreSQL alongside Chroma with proper lifecycle management.

---

## Changed Components

### 1. Shared Chunk Search Module (NEW)

**File:** `backend/src/shared/chunk_search.rs` (+242 lines)

- Extracts chunk search logic from `query/repository.rs` into a shared module
- `search_chunks_text(query, collection_id, source, limit, offset)` — PostgreSQL ILIKE with safe `QueryBuilder`
- `search_chunks_semantic(query, collection_id, source, top_k)` — Chroma vector search with metadata join
- Both accept optional `source` filter (`"upload"` / `"git"` / `None`)
- Returns `Vec<ChunkSearchResult>` with enriched payload (chunk_id, document_id, document_name, chunk_index, text, source, score, file_path)

### 2. Collections Module — Admin Endpoints

**Files:** `models.rs` (+27), `repository.rs` (+105), `service.rs` (+102), `handlers.rs` (+42)

- **`GET /api/collections/:id/stats`** — Returns document/chunk counts by source, git repo count, total file size, file type distribution
- **`GET /api/collections/:id/chunks`** — Admin chunk search with text/semantic modes, pagination, source filtering
- Both endpoints enforce admin role check (`user_ctx.roles.contains("admin")`)
- `CollectionService` now takes `embedding_service_url` as a dependency for semantic chunk search

### 3. Query Module — Refactored to Use Shared Module

**Files:** `repository.rs` (-129 lines), `service.rs` (+98 lines)

- Removed `query_chroma()` and `get_chunks_by_ids()` from repository
- Query service now calls `chunk_search::search_chunks_semantic()` directly
- `SourceRef.relevance` changed from `Option<f64>` to `f64` (SSE JSON schema change)

### 4. Documents Module — Source Field

**Files:** `models.rs` (+3), `service.rs` (+3), `repository.rs` (+287)

- Added `source: String` field to `Document` and `DocumentSummary` structs
- Upload flow explicitly sets `source: "upload"`
- New methods for git document lifecycle: `get_active_git_document_by_name`, `deactivate_git_documents_for_collection`, `deactivate_git_documents_by_names`

### 5. Git Sync Module — Dual PG + Chroma Indexing

**File:** `service.rs` (+197 lines)

- `index_chunks()` now creates real `Document` and `Chunk` rows in PostgreSQL (previously Chroma-only)
- Clone paths changed from `{clone_root}/{repo_id}` to `{clone_root}/{user_id}/{repo_id}`
- Delete flow uses soft deactivation (SET `is_active = FALSE`) in PostgreSQL
- Chunks now use real UUIDs instead of `"git-{repo_id}-{file_path}-{i}"` format

### 6. Database Migration

**File:** `backend/migrations/00000000000013_add_source_to_documents.sql`

- Adds `source VARCHAR(20) NOT NULL DEFAULT 'upload'` to `documents` table
- CHECK constraint: `source IN ('upload', 'git')`
- Composite index: `(collection_id, source)`

### 7. Frontend — Statistics Tab

**New files:** `StatsPanel.vue` (232 lines), `ChunkBrowser.vue` (454 lines), `stores/stats.ts` (63 lines)

**Modified files:** `AdminView.vue` (+57), `DocumentList.vue` (+18), `api/client.ts` (+18), `api/types.ts` (+48)

- New "Statistics" tab in AdminView with split layout: StatsPanel (left 340px) + ChunkBrowser (right)
- StatsPanel: loading skeleton, error state, empty state, 6-card stats grid with source breakdowns
- ChunkBrowser: text/semantic search toggle, source filter pills, debounced input, pagination, chunk cards with badges
- DocumentList now shows source badges (Git=green, Upload=blue) next to document names
- New Pinia store: `useStatsStore` with `fetchStats()` and `searchChunks()`
- New API client methods: `getCollectionStats()`, `searchChunks()`

### 8. E2E Tests

**File:** `frontend/e2e/admin-stats.spec.ts` (136 lines)

- 4 Playwright E2E tests: tab visibility, upload→stats display, text search→chunk results, search mode toggle

### 9. Documentation

**File:** `docs/api.md` (+89 lines)

- Documents both new API endpoints with curl examples and response schemas

---

## Risk Assessment

| Risk | Level | Description | Evidence |
|------|-------|-------------|----------|
| Stale Chroma entries on upgrade | **HIGH** | Old git documents indexed with `"git-{repo_id}-{path}"` Chroma IDs are never cleaned up by new code | `git_sync/service.rs` — new code looks up by UUID; no cleanup step for old format |
| Clone path orphaned | **HIGH** | Existing cloned repos at `{clone_root}/{repo_id}` are not found under `{clone_root}/{user_id}/{repo_id}` | `git_sync/service.rs` — clone path changed; `delete_repo_local` looks in new path |
| Migration misclassifies existing git docs | **HIGH** | Old git-synced documents (before migration) get `source = 'upload'` from DEFAULT | Migration SQL: `DEFAULT 'upload'`; app code only sets `source` on new inserts |
| Per-request HTTP clients | **MEDIUM** | `CollectionService::search_chunks` creates new ChromaClient/EmbeddingClient per request | `collections/service.rs` — clients instantiated in method body |
| Source filter asymmetry | **MEDIUM** | `search_chunks_text` treats empty string as "no filter"; `search_chunks_semantic` passes empty string as Chroma filter | `chunk_search.rs` — branching on `_source.is_empty()` only in text path |
| total_git_repos counts all repos | **LOW** | Stats endpoint counts ALL git repos, including deactivated ones | `collections/repository.rs` — no `is_active` filter on `total_git_repos` query |
| Relevance schema change | **LOW** | SSE events now send `0.0` instead of `null` for relevance | `query/service.rs` — `r.score.unwrap_or(0.0)` |
| Debounce timer module-level | **LOW** | Debounce timer is module-level in ChunkBrowser, could leak across instances | `ChunkBrowser.vue` — `let debounceTimer` outside component |
| Array guard masks schema issues | **LOW** | `Array.isArray` guard in store silently masks backend schema mismatches | `stores/stats.ts` — `chunks.value = Array.isArray(data) ... : []` |

---

## Key Evidence

| Observation | Source |
|-------------|--------|
| 2 commits, 27 files, +2173/-272 lines | `git diff --stat main...HEAD` |
| `chunk_search.rs` replaces `query/repository.rs` logic | Both files: identical filter `{"is_active": true}` |
| `embedding_service_url` now required for CollectionService | `main.rs` — `CollectionService::new()` receives URL |
| `doc_repo` shared by DocumentService and GitSyncService | `main.rs` — `.clone()` before both |
| Admin endpoints check for `"admin"` role | `collections/handlers.rs` — role check in both handlers |
| Soft deactivation (not hard delete) for git documents | `documents/repository.rs` — `SET is_active = FALSE` |
| Text search uses ILIKE with LIMIT/OFFSET; semantic uses Chroma | `chunk_search.rs` — two distinct functions |
| ChunkBrowser has 300ms debounce and pageSize=20 | `ChunkBrowser.vue` — `debounceTimer`, `pageSize = 20` |
