# Test Plan: Fix E2E Test Failures

**Branch:** `feature/v031-chat-rework`
**Date:** 2026-06-21
**Based on:** Change Summary

## Scope

Verify 5 fixes across backend and frontend:

1. ZIP validation early-exit on 10+ entries
2. Chroma query retry on empty results
3. Loading skeleton route ordering
4. RAG-003 Chroma propagation pause
5. Chat-export assistant timeout increase

## Test Strategy

### Automated Tests

| Area | Test | What to run |
|------|------|-------------|
| Backend unit | `cargo test --lib` | 64 tests including `test_process_zip_with_11_files_returns_413` |
| Backend integration | `cargo test --test integration` | 19 Chroma + DB integration tests |
| Frontend unit | `npm test` | 76 Vitest unit tests |
| Embedding | `uv run pytest tests/ -v` | 5 FastAPI endpoint tests |
| E2E | `docker compose ... run --rm frontend-tests` | 129 Playwright tests |

### Manual / Additional Verification

| Scenario | Expected | Priority |
|----------|----------|----------|
| 1. ZIP with 12 files → 413 | Before fix: may get 415 on malformed entries. After fix: always 413 early | High |
| 2. ZIP with 5 valid files → 200 | Early-exit doesn't affect valid ZIPs | High |
| 3. Query after upload (within 2s) | Chroma retries 3×500ms, finds results on retry | High |
| 4. Skeleton: slow git-sync repos | Route registered before `setActiveCollection`, 1s delay triggers skeleton | Medium |
| 5. Skeleton: slow session detail | 800ms route intercept shows `messages-loading-skeleton` | Medium |
| 6. Skeleton: slow /api/documents | 1s route intercept shows `documents-loading-skeleton` | Medium |
| 7. Chat export (Markdown+JSON) | Assistant message appears within 20s, export returns content | Medium |
| 8. TC-RAG-003: upload → query → sources | 2s pause allows Chroma propagation, retry may find chunks | High |

## Out of Scope

- Chat-edit-delete E2E tests (pre-existing skip, not related)
- Login page visual tests (unrelated)
- Integration test failures for foreign key constraints (pre-existing)
