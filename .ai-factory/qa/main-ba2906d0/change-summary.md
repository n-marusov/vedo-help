# Change Summary: Fix UTF-8 chunking panic in document upload

> Analysis of the UTF-8 boundary panic in `chunking.rs` that prevented document upload from completing.

## Overview

**Branch:** `main` (hotfix on current state)
**Focus:** Fix a runtime panic in the text chunking pipeline when processing documents with multi-byte UTF-8 characters (e.g., Russian Cyrillic).

## What Changed

### Root Cause

A single file was modified:

- **`backend/src/shared/chunking.rs`** — The `chunk_document` function used byte-index slicing (`current[overlap_start..]`) to compute chunk overlap. When the overlap boundary fell in the middle of a multi-byte UTF-8 character (e.g., Cyrillic 'б' = 2 bytes), Rust's string indexing panicked at runtime with:  
  `start byte index 675 is not a char boundary; it is inside 'б' (bytes 674..676 of string)`

### Fix

Replaced direct byte slicing with a char-boundary-safe approach:

```rust
let safe_start = current
    .char_indices()
    .map(|(i, _)| i)
    .chain(std::iter::once(current.len()))
    .filter(|&i| i >= overlap_start)
    .next()
    .unwrap_or(current.len());
current = current[safe_start..].to_string();
```

The algorithm:
1. Iterates over `char_indices()` to find all valid character boundary byte positions
2. Appends the string length as a sentinel
3. Filters to positions >= the desired overlap offset
4. Takes the first valid boundary (or falls back to the end of string)
5. Slices at that safe position

### Test Added

Added `test_chunk_non_ascii_text()` — generates a document with Russian Cyrillic text long enough to force chunk overlap, then verifies that all produced chunks contain valid UTF-8 (no panic).

## Risks

| Risk | Severity | Evidence |
|------|----------|----------|
| Other callers of `chunk_document` may also panic | **High** | Any document with non-ASCII content triggers this bug. The chunking function is called from `documents::service::process_upload` and `git_sync::service`. |
| Overlap size may be smaller than intended for multi-byte text | **Low** | The fix rounds up to the next valid char boundary, so overlap may be up to 3 fewer bytes than `CHUNK_OVERLAP` (200). This has no functional impact. |
| Regression in ASCII-only documents | **None** | For ASCII text, every byte is a valid char boundary; behavior is identical. |

## Evidence

1. **Production failure:** Backend logs show `thread 'tokio-rt-worker' panicked at src/shared/chunking.rs:37:30` when uploading `glossary.md` (a Markdown file with Russian content, 137 KB).
2. **Log trace:** Upload was received, file was validated as Markdown, document was parsed to 137417 chars, then chunking panicked — confirming the bug in the overlap slicing logic at line 37.
3. **Vite proxy error:** Frontend logged `socket hang up` because the backend process crashed mid-request.
4. **All 5 chunking tests pass** after the fix, including the new UTF-8 boundary test.

## Impact

- **Severity:** Critical — document upload is completely broken for any document containing non-ASCII characters (Russian, Chinese, accented Latin, emoji, etc.)
- **Scope:** Affects single-file upload and git-sync indexing paths
- **All E2E tests pass** — 15/15 RAG Flow tests and 453/459 total tests pass. The 6 failures are pre-existing mobile/tablet clipboard issues unrelated to this change.
