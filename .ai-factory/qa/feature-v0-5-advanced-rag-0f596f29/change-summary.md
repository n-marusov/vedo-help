## Change Summary

**Branch:** `feature/v0.5-advanced-rag`
**Commits:** 3 (ahead of `main`)
**Changed files:** 22
**Risk level:** 🔴 High

---

### What Changed

v0.5 Advanced RAG implements four major enhancements on top of the existing RAG pipeline:

1. **Cross-encoder reranker** — a new `rerank.rs` module that scores retrieved chunks by query-chunk relevance using a mock cross-encoder scorer (real ONNX inference is a future task).
2. **Accurate token counting** — `context_window.rs` rewritten to use `tiktoken-rs` (`cl100k_base`) instead of a word-count heuristic, with fallback to the old heuristic if the tokenizer fails.
3. **Multi-format document support** — `DocumentService` extended to parse CSV, JSON, and HTML files alongside the existing PDF, Markdown, DOCX formats.
4. **LLM API fallback** — `LlmClient` extended with a fallback to `opencode.ai` API when the primary RouterAI endpoint is unreachable.

Supporting changes include configurable BM25 parameters with alpha-weighted RRF fusion, extended file validation for the new formats, new environment variables, and new Cargo dependencies (`tiktoken-rs`, etc.).

---

### Affected Areas

| Component                          | Change type               | Description                                                                 |
|------------------------------------|---------------------------|-----------------------------------------------------------------------------|
| `backend/src/modules/query/rerank.rs` | **Added** (369 lines)   | Cross-encoder reranker module: types, mock scorer, orchestration           |
| `backend/src/modules/documents/service.rs` | **Extended** (+278 lines) | Multi-format parsing: CSV, JSON, HTML-to-text, re-chunking                 |
| `backend/src/shared/file_validation.rs` | **Extended** (+183 lines) | MIME + magic byte validation for CSV, JSON, HTML formats                   |
| `backend/src/modules/query/context_window.rs` | **Rewritten** | tiktoken-rs `cl100k_base` tokenizer, fallback to word-count heuristic       |
| `backend/src/shared/llm.rs` | **Extended** (+93 lines) | LLM fallback to opencode.ai, query_single for non-streaming calls           |
| `backend/src/shared/bm25.rs` | **Extended** (+28 lines) | Configurable k1/b params, alpha-weighted RRF fusion                        |
| `backend/src/config.rs` | **Extended** (+23 lines) | New env vars: `LLM_FALLBACK_BASE_URL`, `TOKEN_BUDGET`, embedding params    |
| `backend/src/modules/query/service.rs` | **Extended** (+77 lines) | Cross-encoder reranker integration into the query pipeline                 |
| `backend/src/shared/types.rs` | **Extended** (+9 lines) | New type definitions for reranker output                                   |
| `backend/Cargo.toml` | **Extended** (+3 lines) | `tiktoken-rs` dependency added                                             |
| `.env.example` | **Extended** (+18 lines) | New environment variable documentation                                     |
| `docs/configuration.md` | **Extended** (+12 lines) | Configuration docs for new settings                                        |
| `backend/tests/rag_pipeline.rs` | **Minimal** (+1 line)  | Test config updated with LLM fallback URL                                  |
| `backend/tests/common/mod.rs` | **Minimal** (+1 line)  | Test config updated with LLM fallback URL                                  |
| `frontend/src/components/DocumentList.vue` | **Adjusted** (+2 lines) | Minor UI alignment adjustment                                              |
| `frontend/src/components/ui/VDropZone.vue` | **Adjusted** (+2 lines) | Minor UI alignment adjustment                                              |

---

### Evidence

| Finding                                                                 | Evidence                                                                                  |
|-------------------------------------------------------------------------|-------------------------------------------------------------------------------------------|
| Cross-encoder reranker is a new module with mock scorer                 | `rerank.rs` L1-L369: types, `RerankVerdict` enum, `rerank_chunks` mock stub              |
| Document service handles 3 new file formats                             | `documents/service.rs`: `parse_csv()`, `parse_json()`, `html_to_text()` methods           |
| File validation extended for CSV/JSON/HTML MIME types                   | `file_validation.rs`: new `validate_csv()`, `validate_json()`, `validate_html()`          |
| tiktoken-rs replaces word-count heuristic                               | `context_window.rs`: `LazyLock<Option<CoreBPE>>`, `count_tokens()` uses BPE, fallback     |
| LLM fallback to opencode.ai                                             | `llm.rs`: `LlmClient::query_single()`, `FALLBACK_BASE_URL`, retry logic                   |
| BM25 now configurable with RRF fusion                                   | `bm25.rs`: `k1`, `b` parameters, `rrf_alpha` in `search()`                                |
| No tests for `rerank.rs` (369 lines)                                    | `git diff main..HEAD -- '**/tests/**'` shows only +2 lines total                          |
| No tests for multi-format document parsing (+278 lines)                 | Same diff evidence                                                                        |
| No tests for extended file validation (+183 lines)                      | Same diff evidence                                                                        |
| No tests for LLM fallback (+93 lines)                                   | Same diff evidence                                                                        |
| Existing `rag_pipeline.rs` test barely updated                           | Only `llm_fallback_base_url` field added to test config                                   |

---

### Risks

🔴 **Critical** (must verify):

- **New reranker module untested** — `rerank.rs` (369 lines) has zero tests. The scoring logic, verdict assignment, and pipeline integration could fail silently, degrading answer quality without obvious errors.
- **Multi-format parsing untested** — `DocumentService` (+278 lines) has no tests for CSV, JSON, or HTML parsing. Malformed input could panic, produce corrupt chunks, or silently drop data.
- **File validation extension untested** — `file_validation.rs` (+183 lines) has no tests for the new format validators. Invalid files could be accepted or valid files rejected.
- **LLM fallback untested** — `llm.rs` (+93 lines) fallback to opencode.ai has no tests. A fallback that fails or silently degrades could result in empty answers or hidden errors.
- **tiktoken-rs rewrite untested** — `context_window.rs` has no new tests. A token-counting bug could truncate context incorrectly, silently cutting relevant history.

🟡 **Medium** (should verify):

- **BM25 parameter changes** — Configurable k1/b and RRF fusion change ranking output; existing search quality may regress.
- **Existing RAG pipeline tests may be fragile** — `rag_pipeline.rs` has only 3 tests covering full flow, debug data, and advanced-disabled mode. New features are not covered.
- **Config/env var changes** — New env vars with wrong defaults could cause startup failures or unexpected behavior.
- **Docker Compose adjustments** — Minor changes may affect service discovery.

🟢 **Low** (nice to verify):

- Documentation updates (`docs/configuration.md`) are accurate.
- ROADMAP.md milestone status is consistent with actual implementation.

---

### Testing Recommendations

**First priority:**

- [ ] Unit tests for `rerank.rs`: `rerank_chunks()` scoring logic, `RerankVerdict` assignment, empty input handling
- [ ] Unit tests for `documents/service.rs` multi-format parsing: CSV, JSON, HTML valid/invalid files
- [ ] Unit tests for `file_validation.rs`: new format MIME + magic byte validation
- [ ] Integration tests for LLM fallback: RouterAI failure → opencode.ai invocation
- [ ] Unit tests for `context_window.rs` tiktoken integration: `count_tokens()` accuracy, fallback heuristic, `trim_history()` with token budget

**Regression:**

- [ ] Verify existing RAG pipeline tests pass with new config fields
- [ ] Verify BM25 search quality with default vs custom parameters
- [ ] Verify Chroma integration tests still pass
- [ ] Re-run `git_sync_integration` tests — multi-format parsing affects git-synced documents
