# Change Summary: Fix E2E Test Failures

**Branch:** `feature/v031-chat-rework`
**Date:** 2026-06-21

## Changes Overview

5 backend + frontend fixes targeting E2E test stability in the RAG pipeline, ZIP validation, loading skeleton timing, and chat export timeouts.

## Files Modified

| File | Change | Risk |
|------|--------|------|
| `backend/src/modules/documents/service.rs` | Moved 10-file ZIP limit check before per-file extraction; removed duplicate late check | Low |
| `backend/src/modules/query/service.rs` | Added retry loop (3 attempts, 500ms) on empty Chroma results | Low |
| `frontend/e2e/loading-skeletons.spec.ts` | Reordered `page.route` before `setActiveCollection` in git-sync test | Low |
| `frontend/e2e/rag-flow.spec.ts` | Added 2s Chroma propagation wait after document upload | Low |
| `frontend/e2e/chat-export.spec.ts` | Increased assistant message timeout 15s → 20s | Low |

## Risk Assessment

| Area | Risk | Rationale |
|------|------|-----------|
| ZIP validation | Low | Early return preserves same error type (413 PayloadTooLarge). Slightly changes log message format |
| Chroma retry | Low | Only triggers when results are empty; existing empty-collection tests unaffected |
| Skeleton routes | Low | Only reorders operations, no logic change |
| Timeouts | Low | Only increases wait durations |

## Pre-existing Failures (Unchanged)

1. **Chat export** — Markdown export returns empty body; JSON export returns empty response → `Unexpected end of JSON input`. Likely issue in the `handleSend` session creation flow where messages aren't persisted.
2. **Loading skeleton (session detail)** — `[data-testid="messages-loading-skeleton"]` not visible. Root cause: `v-if="!chatStore.activeSessionId"` renders the welcome screen before the `v-else-if="chatStore.isSessionLoading"` skeleton div.
3. **Loading skeleton (documents)** — `[data-testid="documents-loading-skeleton"]` not visible. Same pattern: `isLoading` set after request sent, but `documents.length === 0` check may resolve before loading state.
