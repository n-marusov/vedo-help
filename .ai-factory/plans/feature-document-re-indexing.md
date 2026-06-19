# Document Re-indexing — деактивация старых чанков при перезагрузке

**Branch:** `feature/document-re-indexing`
**Created:** 2026-06-19

## Settings

| Setting | Value |
|---------|-------|
| Testing | Yes — strict TDD: all tests first, implementation only after tests |
| Logging | Verbose (DEBUG-level logging throughout) |
| Docs | Yes — mandatory docs checkpoint |
| Document language | English |

## Roadmap Linkage

| | |
|---|---|
| **Milestone** | `v0.3 — Admin Panel & Production Polish` |
| **Roadmap item** | `Document re-indexing` — деактивация старых чанков при перезагрузке |
| **Rationale** | Re-indexing completes the document lifecycle: upload → index → re-upload → old chunks deactivated. Closes a correctness gap where stale chunks remain in Chroma forever. |

## Agent Instructions

Implementation agents must follow `.ai-factory/RULES.md` and `AGENTS.md`:

1. **All tests come first.** Complete Phase 1, Phase 2, and Phase 3 before touching production implementation code.
2. **Tests are executable specification.** Read the new e2e, integration, and unit tests before implementing. The behavior encoded in tests is the source of truth for implementation details.
3. **Do not reorder tasks.** Schema, backend, frontend, docs, and validation implementation tasks start only after all test-writing phases are complete.
4. **Expected interim state:** after Phase 1–3, tests may fail because production code is not implemented yet. Do not weaken or delete tests to make them pass prematurely.
5. **Implementation goal:** make the tests pass without changing their intent.

## Overview

### Problem

1. **Document upload** (`process_upload`, `process_zip_upload`) saves chunks to SQLite but **never sends them to Chroma**. Uploaded documents are invisible to RAG queries.
2. **Git sync** (`index_chunks`) adds chunks to Chroma but **never deactivates old ones** on re-sync — old and new chunks coexist.
3. **No `is_active` tracking** exists anywhere: no SQL column, no Rust model field, no Chroma metadata attribute.
4. **`ChromaClient::query()`** does not support `where` filters, so querying by `is_active=true` is impossible.
5. **Document deletion** is hard delete (removes from SQLite but not from Chroma).

### Solution

- Add `is_active` column to SQLite `documents` and `chunks` tables (default `1`)
- Add `is_active` to Chroma metadata on every `add_embeddings` call
- Support `where` filter in `ChromaClient::query()`
- On document re-upload: deactivate old chunks (SQLite + Chroma cleanup), then index new ones
- On document deletion: soft delete (`is_active=false`) in SQLite + Chroma cleanup
- On git sync re-index: delete old chunks by `document_id` before re-adding
- Query only active chunks (`where: {"is_active": true}`)

## Tasks

### Phase 1 — E2E tests first: user-visible re-indexing behavior

Files:
- `frontend/e2e/rag-flow.spec.ts` or new `frontend/e2e/document-reindexing.spec.ts`
- `frontend/e2e/helpers.ts` if route helpers need expansion

- [x] **T1.1 — E2E spec: upload → query uses active indexed chunks**
  - Add an e2e test that uploads a document through the admin UI and then queries it through chat.
  - Mock backend responses so the test asserts the UI consumes sources from the newly indexed active chunks.
  - Expected behavior: uploaded document appears as a source after query.
  - Logging requirements for implementation: backend should emit INFO when upload indexing completes and DEBUG for chunk metadata including `is_active=true`.
  - Dependency notes: this test defines the end-to-end contract; implementation must satisfy it later.

- [x] **T1.2 — E2E spec: reload replaces old document content**
  - Add an e2e test for reloading/re-uploading a document version.
  - Flow: first version has source text A; reload with version B; query should show only source text B.
  - Expected behavior: old source text A is not displayed after reload.
  - Logging requirements for implementation: reload should log old chunk deactivation count and new chunk count.
  - Dependency notes: drives `/api/documents/reload` or equivalent UI/API contract.

- [x] **T1.3 — E2E spec: deleted document disappears from query sources**
  - Add an e2e test that deletes a document and then queries the collection.
  - Expected behavior: deleted document is absent from sources and no stale chunks are displayed.
  - Logging requirements for implementation: soft delete logs document id and collection id at INFO.
  - Dependency notes: verifies Chroma + SQLite active filtering from the user perspective.

### Phase 2 — Integration tests second: Chroma and backend contracts

File:
- `backend/tests/integration.rs`

- [x] **T2.1 — Integration spec: Chroma query supports `where: {"is_active": true}`**
  - Add embeddings with mixed metadata: active and inactive chunks.
  - Query active-only using the new Chroma client API.
  - Expected behavior: only active chunks are returned.
  - Logging requirements for implementation: `ChromaClient::query` logs collection, top_k, and filter when present.
  - Dependency notes: implementation must add query filter support without breaking existing query callers.

- [x] **T2.2 — Integration spec: Chroma `delete_where` removes stale document chunks**
  - Add embeddings for two `document_id` values.
  - Call `delete_where({"document_id": "test-doc-1"})`.
  - Query after deletion.
  - Expected behavior: chunks for deleted document are gone, other document chunks remain.
  - Logging requirements for implementation: `delete_where` logs filter at DEBUG and retry failures at WARN.
  - Dependency notes: validates cleanup primitive used by reload, delete, and git sync.

- [x] **T2.3 — Integration spec: query repository applies active-only filter**
  - Add/adjust an integration test around `QueryRepository::query_chroma` or Chroma request behavior.
  - Expected behavior: query path always passes `where: {"is_active": true}`.
  - Logging requirements for implementation: repository logs that active-only filter is applied.
  - Dependency notes: prevents regressions where Chroma returns inactive chunks.

### Phase 3 — Unit tests third: repository, service, and git sync specifications

Files:
- `backend/src/modules/documents/reindex_tests.rs` or existing `backend/src/modules/documents/service.rs` test module
- `backend/src/modules/documents/repository.rs` tests if a colocated test module is preferred
- `backend/src/modules/git_sync/service.rs` test module
- `backend/src/shared/chroma_client.rs` test module
- `backend/tests/common/mod.rs`
- `backend/src/modules/documents/service.rs` (the `make_service()` test helper also needs schema updates)

- [x] **T3.1 — Unit spec: test DB schemas include `is_active`**
  - Update only test schema helpers first.
  - **Normalize the chunk index column name** across all schema locations:
    Production uses `"index"` (quoted SQL keyword), but `tests/common/mod.rs` and the integration test inserts use `chunk_index`. Align all three locations to `"index"` (consistent with production).
  - The affected files are:
    - `backend/tests/common/mod.rs` — rename `chunk_index` to `"index"`
    - `backend/src/modules/documents/service.rs` — the `make_service()` helper (`"index"`)
    - `backend/tests/integration.rs` — INSERT statements that reference `chunk_index`
  - Add assertions that `documents.is_active` and `chunks.is_active` exist and default to `1`.
  - Expected behavior: new tests describe the required schema before production migrations are changed.
  - Logging requirements for implementation: production migrations should log successful active-state migration.
  - Dependency notes: all repository tests depend on this schema. The `make_service()` helper in `service.rs` must be updated to include `is_active` so that T3.4 and T3.5 can use it.

- [x] **T3.2 — Unit spec: `DocumentRepository` deactivates chunks and documents**
  - Test `deactivate_chunks(document_id)` sets all matching chunks to inactive.
  - Test `deactivate_document(document_id)` sets document inactive without removing the row.
  - Test non-matching documents/chunks remain active.
  - Expected behavior: soft delete is state change, not hard delete.
  - Logging requirements for implementation: DEBUG for affected row counts.
  - Dependency notes: implementation must add repository methods.

- [x] **T3.3 — Unit spec: active chunk lookup filters inactive chunks**
  - Test `get_active_chunks(document_id)` returns only `is_active=1` chunks ordered by index.
  - Expected behavior: inactive chunks are never returned to indexing/query assembly code.
  - Logging requirements for implementation: TRACE per fetched active chunk or DEBUG summary count.
  - Dependency notes: query double-filter depends on active lookup behavior.

- [x] **T3.4 — Unit spec: document reload deactivates old chunks then saves new active chunks**
  - Write a service-level test with fake/mock embedding and Chroma dependencies if needed.
  - Expected behavior: reload keeps document identity, marks old chunks inactive, adds new active chunks.
  - Logging requirements for implementation: INFO with `document_id`, `old_chunks`, `new_chunks`.
  - Dependency notes: defines `DocumentService::reload_document` behavior.

- [x] **T3.5 — Unit spec: soft delete keeps rows but removes them from active results**
  - Upload/save a document, call service delete.
  - Expected behavior: document/chunks remain in SQLite with `is_active=0`, active queries return none.
  - Logging requirements for implementation: INFO for soft-delete completion.
  - Dependency notes: changes current `delete_document` behavior from hard delete to soft delete.

- [x] **T3.6 — Unit spec: `ChromaClient::query` request body includes optional `where`**
  - Add test around query request construction or a local mock server.
  - Expected behavior: `where` omitted when `None`, included unchanged when `Some(filter)`.
  - Logging requirements for implementation: DEBUG includes filter only when present.
  - Dependency notes: existing callers must be updated to new signature.

- [x] **T3.7 — Unit spec: git sync deletes old file chunks before adding new ones**
  - Test `GitSyncService::index_chunks` behavior with fake/mock Chroma where practical.
  - Expected behavior: for each changed file document id, cleanup happens before add, and new metadata includes `is_active=true`.
  - Logging requirements for implementation: DEBUG per file cleanup and INFO final indexed counts.
  - Dependency notes: prevents incremental sync from appending stale chunks.

### Phase 4 — Production implementation: schema and models

Files:
- `backend/src/main.rs`
- `backend/src/modules/documents/models.rs`
- `backend/tests/common/mod.rs` (must be updated to match production schema)
- `backend/src/modules/documents/service.rs` (the `make_service()` test helper)

- [ ] **T4.1 — Add active-state migrations to SQLite**
  - Add `is_active INTEGER NOT NULL DEFAULT 1` to `documents` and `chunks` creation SQL in `backend/src/main.rs`.
  - Add idempotent `ALTER TABLE` migration path for existing databases.
  - **Ensure `backend/tests/common/mod.rs` and `backend/src/modules/documents/service.rs` (`make_service()`) are both updated** to match the production schema exactly: same column names (`"index"`, not `chunk_index`) and the new `is_active` column.
  - Expected behavior: fresh and existing DBs get active-state columns safely. All schema definitions remain consistent.
  - Logging requirements: INFO after migration completes; DEBUG when a column already exists.
  - Dependency notes: must preserve existing data with default active state.

- [ ] **T4.2 — Update document and chunk models**
  - Add `is_active: bool` to `Document` and `Chunk`.
  - Update construction sites to set `true` for new records.
  - Expected behavior: active state is explicit in domain models.
  - Logging requirements: no new logging required beyond construction callers.
  - Dependency notes: repository row mapping depends on model changes.

### Phase 5 — Production implementation: repository active-state operations

File:
- `backend/src/modules/documents/repository.rs`

- [ ] **T5.1 — Update save/read methods for `is_active`**
  - Update `save_document`, `save_chunk`, `get_document`, and `list_documents`.
  - Keep list behavior focused on active documents unless a test/spec requires all documents.
  - Expected behavior: repository persists and reads active-state correctly.
  - Logging requirements: DEBUG summary counts for list/get operations.
  - Dependency notes: service reload/delete depends on these operations.

- [ ] **T5.2 — Add repository deactivation and active lookup methods**
  - Implement `deactivate_chunks`, `deactivate_document`, `get_active_chunks`, and any helper needed by service tests.
  - Expected behavior: old chunks can be retained for audit but excluded from active paths.
  - Logging requirements: DEBUG affected row counts; WARN if deactivation target does not exist.
  - Dependency notes: satisfies Phase 3 repository tests.

### Phase 6 — Production implementation: Chroma client query filters

File:
- `backend/src/shared/chroma_client.rs`

- [ ] **T6.1 — Add optional `where` filter to Chroma queries**
  - Update `query(collection, embedding, top_k, where_filter)` signature.
  - Include `"where": filter` only when provided.
  - Update all existing call sites and tests.
  - Expected behavior: old unfiltered callers can pass `None`; query path passes active-only filter.
  - Logging requirements: DEBUG logs filter when present.
  - Dependency notes: integration tests in Phase 2 define expected request behavior.

### Phase 7 — Production implementation: document service re-indexing and soft delete

Files:
- `backend/src/modules/documents/service.rs`
- `backend/src/modules/documents/handlers.rs`
- `backend/src/main.rs` or router wiring file

- [ ] **T7.1 — Inject embedding and Chroma dependencies into `DocumentService`**
  - Add `EmbeddingClient` and `ChromaClient` to the service.
  - Update constructor and all call sites/test helpers.
  - Expected behavior: upload and reload can index into Chroma.
  - Logging requirements: DEBUG when service is initialized with external clients.
  - Dependency notes: may need test fakes or constructor variants for unit tests.

- [ ] **T7.2 — Index uploaded documents into Chroma**
  - Update `process_upload` and `process_zip_upload` to embed chunks and call `add_embeddings`.
  - Metadata must include `document_id`, `document_name`, `chunk_id`, `chunk_index`, `is_active: true`, and `source`.
  - Expected behavior: uploaded documents become queryable.
  - Logging requirements: INFO per successful document indexing; ERROR and rollback strategy on indexing failure.
  - Dependency notes: related roadmap item `Embedding submission in upload pipeline` overlaps; keep scope focused but avoid duplicate future work.

- [ ] **T7.3 — Implement soft delete for documents**
  - Replace hard delete behavior with soft delete unless tests require a separate hard-delete helper.
  - Delete/deactivate Chroma entries by `document_id` and mark SQLite rows inactive.
  - Expected behavior: deleted documents no longer appear in queries but remain in SQLite as inactive rows.
  - Logging requirements: INFO with document id, collection id, affected chunk count.
  - Dependency notes: must fetch document first to know collection id / Chroma collection name.

- [ ] **T7.4 — Implement document reload/re-index service and endpoint**
  - Add `DocumentService::reload_document`.
  - Add handler `POST /api/documents/reload` accepting multipart `file` and `document_id`.
  - Reuse the existing document id for the new version; deactivate old chunks and save new active chunks.
  - Expected behavior: same document identity, new chunks only active.
  - Logging requirements: INFO start/success, DEBUG old/new chunk counts, ERROR on partial failure.
  - Dependency notes: e2e tests define exact API/UI contract.

### Phase 8 — Production implementation: git sync and query active filtering

Files:
- `backend/src/modules/git_sync/service.rs`
- `backend/src/modules/query/repository.rs`
- `backend/src/modules/query/service.rs` if needed

- [ ] **T8.1 — Update git sync indexing to replace old file chunks**
  - In `GitSyncService::index_chunks`, call `delete_where` for each file document id before adding new chunks.
  - Include `is_active: true` in all new git sync Chroma metadata.
  - Expected behavior: incremental sync does not leave stale chunks behind.
  - Logging requirements: DEBUG cleanup per file, INFO final indexed counts.
  - Dependency notes: satisfies Phase 3 git sync test.

- [ ] **T8.2 — Apply active-only filtering in query path**
  - `QueryRepository::query_chroma` passes `where: {"is_active": true}`.
  - `get_chunks_by_ids` filters `c.is_active = 1` to protect against stale Chroma data.
  - Expected behavior: inactive chunks never reach the LLM context or source refs.
  - Logging requirements: DEBUG active-only filter; WARN when Chroma returned stale/inactive chunk ids.
  - Dependency notes: satisfies Phase 2 and Phase 3 query specs.

### Phase 9 — Documentation and validation

Files:
- `docs/api.md`
- `docs/technical-specification-rag-system.md` if implementation deviates from current spec
- `CHECKLIST.md` must be read before final completion

- [ ] **T9.1 — Documentation checkpoint**
  - Document `/api/documents/reload` if added.
  - Document soft-delete/re-index semantics.
  - Expected behavior: docs match implemented API and data lifecycle.
  - Logging requirements: none.
  - Dependency notes: required by plan settings and project checklist.

- [ ] **T9.2 — Rust validation**
  - Run `cargo fmt`.
  - Run `cargo clippy`.
  - Run `cargo test`.
  - Run `cargo test --test integration` when Chroma is available.
  - Expected behavior: all pass or any pre-existing failures are documented with evidence.

- [ ] **T9.3 — Frontend/e2e validation**
  - Run `npx biome format` and `npx biome check` in `frontend/` if frontend test files changed.
  - Run Playwright e2e tests relevant to re-indexing.
  - Expected behavior: all relevant e2e tests pass.

- [ ] **T9.4 — Project checklist validation**
  - Read `CHECKLIST.md`.
  - Run `npm run ai:validate` from project root and verify exit code 0, or document known pre-existing failures.
  - Expected behavior: final implementation satisfies project gates.

## Commit Plan

| # | Commit | Tasks | Message |
|---|--------|-------|---------|
| 1 | Test specifications | T1.1–T3.7 | `test: specify document re-indexing behavior` |
| 2 | Schema + repository + Chroma client | T4.1–T6.1 | `feat(backend): track active document chunks` |
| 3 | Document service re-indexing | T7.1–T7.4 | `feat(backend): implement document reload and soft delete` |
| 4 | Git sync + query filtering | T8.1–T8.2 | `feat(backend): filter inactive chunks from retrieval` |
| 5 | Docs + validation | T9.1–T9.4 | `docs: document document re-indexing lifecycle` |
