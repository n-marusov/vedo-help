## Test Plan: v0.5 Advanced RAG — Cross-encoder, tiktoken, Multi-format, LLM Fallback

**Date:** 2026-07-11
**Branch / Version:** `feature/v0.5-advanced-rag`
**Environment:** local development (Docker Compose with Chroma, PostgreSQL, backend, frontend)

---

### 1. Testing Goal

Verify that four new v0.5 features work correctly and without regressions:

1. **Cross-encoder reranker** — `rerank.rs` correctly scores chunks and assigns verdicts.
2. **tiktoken-rs tokenizer** — `context_window.rs` counts tokens accurately and falls back gracefully.
3. **Multi-format document support** — CSV, JSON, HTML files are parsed, validated, chunked, and indexed correctly.
4. **LLM API fallback** — when RouterAI is down, the backend falls back to opencode.ai without data loss or empty responses.

No new tests exist for these features — the plan focuses on filling this gap.

---

### 2. Test Scope

**In Scope** — we test:

- Cross-encoder reranker: `rerank.rs` unit + integration with query pipeline
- Token counting: `context_window.rs` accuracy with tiktoken + fallback heuristic
- Multi-format parsing: CSV, JSON, HTML in `DocumentService`
- File validation: new MIME + magic byte checks in `file_validation.rs`
- LLM fallback: primary failure → secondary invocation in `LlmClient`
- BM25 configurable parameters + RRF fusion
- Regression: existing RAG pipeline, Chroma CRUD, document upload, git sync

**Out of Scope** — we don't test:

- Real cross-encoder ONNX inference (future task, mock scorer only for now)
- Performance / latency benchmarks for tiktoken-rs vs word-count
- Frontend UI for new formats (no new UI components were added)
- Documentation accuracy (docs/configuration.md, ROADMAP.md)

---

### 3. Test Types

| Type              | Priority   | Area                                                      |
|-------------------|------------|-----------------------------------------------------------|
| Unit (Rust)       | 🔴 High    | `rerank.rs` scoring, verdict, empty input                 |
| Unit (Rust)       | 🔴 High    | `file_validation.rs` new format validators                |
| Unit (Rust)       | 🔴 High    | `context_window.rs` tiktoken accuracy, fallback           |
| Unit (Rust)       | 🔴 High    | `documents/service.rs` CSV/JSON/HTML parsing              |
| Unit (Rust)       | 🔴 High    | `llm.rs` fallback logic, timeout, error conversion        |
| Integration       | 🔴 High    | Full upload → chunk → embed → index → query for new formats |
| Integration       | 🟡 Medium  | LLM fallback: primary down → secondary response           |
| Integration       | 🟡 Medium  | BM25 RRF fusion: ranking quality with custom parameters   |
| Regression        | 🟡 Medium  | Existing RAG pipeline tests with new config               |
| Regression        | 🟡 Medium  | Existing document upload (PDF, MD, DOCX) still works      |
| Regression        | 🟡 Medium  | Git sync still indexes documents correctly                |
| Edge cases        | 🟡 Medium  | Malformed CSV/JSON/HTML files                             |
| Edge cases        | 🟡 Medium  | Empty files, oversized files for new formats              |
| Edge cases        | 🟡 Medium  | tiktoken init failure → word-count fallback path          |
| Negative          | 🟡 Medium  | Both RouterAI and fallback down → error response          |
| Negative          | 🟡 Medium  | Unsupported file format rejected with clear error         |

---

### 4. Test Data

| Category                | Data                                                        | Purpose                                |
|-------------------------|-------------------------------------------------------------|----------------------------------------|
| Valid CSV               | `name,age\nAlice,30\nBob,25`                                | CSV parsing happy path                 |
| Valid JSON              | `[{"title":"Intro","content":"Hello"}]`                     | JSON parsing happy path                |
| Valid HTML              | `<html><body><p>Hello</p></body></html>`                    | HTML-to-text parsing                   |
| Malformed CSV           | Missing columns, binary data, excessively long lines        | Error handling                         |
| Malformed JSON          | Unclosed braces, array of non-objects, truncated            | Error handling                         |
| Malformed HTML          | Unclosed tags, nested scripts, mixed encodings              | Error handling                         |
| Empty files             | 0-byte CSV, JSON, HTML files                                | Edge case                              |
| Large files             | >50 MB CSV / JSON / HTML                                    | Edge case (file size limit)            |
| tiktoken input          | Empty string, ASCII, CJK, emoji, very long text             | Token counting edge cases              |
| Fake RouterAI error     | Sandpit API key or unreachable endpoint                     | Fallback trigger                       |
| BM25 test corpus        | Small set of documents with known term frequencies          | RRF fusion verification                |

---

### 5. Preconditions

- [ ] Docker Compose services running: `chroma`, `postgres`, `backend`, `frontend`
- [ ] PostgreSQL migrations applied
- [ ] Test LLM API key available (RouterAI)
- [ ] Fallback LLM endpoint reachable (opencode.ai or mock)
- [ ] Test files prepared: valid and malformed CSV/JSON/HTML files
- [ ] Existing test suite passes before adding new tests: `cargo test --lib`, `cargo test --test integration`
- [ ] Vitest passes: `npx vitest run`

---

### 6. Acceptance Criteria

- [ ] All 🔴 high-priority test cases pass
- [ ] Cross-encoder reranker produces deterministic scores for known inputs
- [ ] CSV, JSON, HTML files are indexable and queryable through the RAG pipeline
- [ ] Token count from tiktoken matches expected values for known strings
- [ ] LLM fallback successfully returns a response when primary is unavailable
- [ ] Existing RAG pipeline tests pass with new config fields
- [ ] Malformed files return appropriate errors (not panics)
- [ ] BM25 RRF fusion: both vector-only and hybrid search return consistent results

---

### 7. Plan Risks

| Risk                          | Impact               | Mitigation                                                       |
|-------------------------------|----------------------|------------------------------------------------------------------|
| No tests exist for 4 new features | **High** — regressions undetected before merge | Prioritise unit tests before integration; run existing regression suite first |
| tiktoken-rs may fail to download BPE vocab in CI | **Medium** — tests may be flaky | Test fallback path explicitly; cache BPE vocab in CI or test both paths |
| LLM fallback depends on external API availability | **Medium** — integration tests may fail without live API | Use mock HTTP server for fallback tests; verify live fallback separately |
| Multi-format parsing may surface encoding edge cases | **Medium** — UTF-16, BOM, mixed encodings | Validate with at least UTF-8 + BOM + Latin-1 encoded files |

---

### 8. Checklist

| Check                                                                             | Priority              |
|-----------------------------------------------------------------------------------|-----------------------|
| `rerank.rs`: `rerank_chunks()` returns correct scores for known inputs            | 🔴 High               |
| `rerank.rs`: empty chunk list returns empty result set                            | 🔴 High               |
| `rerank.rs`: `RerankVerdict::Keep` / `Discard` assigned correctly                | 🔴 High               |
| `file_validation.rs`: CSV with valid header+rows is accepted                      | 🔴 High               |
| `file_validation.rs`: JSON array of objects is accepted                           | 🔴 High               |
| `file_validation.rs`: HTML with `<body>` content is accepted                      | 🔴 High               |
| `file_validation.rs`: binary file with `.csv` extension is rejected               | 🔴 High               |
| `context_window.rs`: `count_tokens()` ASCII matches known tiktoken value          | 🔴 High               |
| `context_window.rs`: `count_tokens()` CJK text counted correctly                 | 🔴 High               |
| `context_window.rs`: tiktoken init failure falls back to `split_whitespace`       | 🔴 High               |
| `context_window.rs`: `trim_history()` respects both `max_messages` and token budget | 🔴 High             |
| `documents/service.rs`: valid CSV file → parsed → chunked → indexed                | 🔴 High               |
| `documents/service.rs`: valid JSON file → parsed → chunked → indexed               | 🔴 High               |
| `documents/service.rs`: valid HTML file → parsed → chunked → indexed               | 🔴 High               |
| `documents/service.rs`: malformed CSV/JSON/HTML returns `AppError`, not panic      | 🔴 High               |
| `llm.rs`: primary endpoint returns error → fallback invoked                        | 🔴 High               |
| `llm.rs`: both primary and fallback return error → error returned to caller       | 🟡 Medium             |
| `llm.rs`: `query_single()` returns structured response matching expected schema   | 🟡 Medium             |
| BM25 RRF: alpha=1.0 behaves like pure vector search                              | 🟡 Medium             |
| BM25 RRF: alpha=0.0 behaves like pure keyword search                             | 🟡 Medium             |
| Existing `rag_pipeline.rs` tests pass with new config fields                      | 🟡 Medium             |
| Existing document upload (PDF, MD, DOCX) still works after service refactor       | 🟡 Medium             |
| Git sync still indexes documents from repositories with new format files           | 🟡 Medium             |
| New env vars with wrong values produce clear startup error                        | 🟢 Low                |
