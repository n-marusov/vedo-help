## Change Summary

**Commits:** 1
**Changed files:** 20
**Risk level:** 🔴 High

---

### What Changed

Implemented an advanced 7-step RAG pipeline (Multi-Query, HyDE, semantic search, BM25 keyword search, merge/dedup, LLM reranking, final answer) with session debug visualization. Replaced the simple semantic search fallback with a configurable multi-stage pipeline that uses multiple LLM calls per query for query expansion, hypothetical document generation, and chunk reranking. Added a new SessionDebug.vue frontend component to visualize each pipeline step in real time.

---

### Affected Areas

| Component     | Change type               | Description                                                                  |
|---------------|---------------------------|------------------------------------------------------------------------------|
| Query service | Changed (major)           | `backend/src/modules/query/service.rs` — 7-step advanced RAG pipeline added, replacing simple semantic search |
| BM25          | Added                     | `backend/src/shared/bm25.rs` — BM25 keyword search index builder and scorer |
| LLM client    | Changed                   | `backend/src/shared/llm.rs` — new `query_single()` method for non-streaming pipeline LLM calls |
| Config        | Changed                   | `backend/src/config.rs` — 5 new env vars: `ADVANCED_RAG_ENABLED`, `RERANK_TOP_K`, `HYBRID_TOP_K`, `MULTI_QUERY_COUNT`, `LLM_RERANK_MODEL` |
| Debug models  | Added / Changed           | `backend/src/modules/query/debug_models.rs` — 7-step debug data structures for pipeline visualization |
| Query models  | Changed                   | `backend/src/modules/query/models.rs` — `SourceRef` now includes `stage`, `rerank_score`, `rerank_verdict` |
| Session Debug | Added                     | `frontend/src/components/SessionDebug.vue` — UI component to visualize pipeline steps |
| API types     | Changed                   | `frontend/src/api/types.ts` — new debug data TypeScript types |
| Conversations | Changed (minor)           | `backend/src/modules/conversations/*` — `debug_data` field on messages, handlers pass debug flag |
| Docs           | Changed                   | `docs/api.md`, `docs/architecture.md` — updated for debug endpoint and advanced pipeline |

---

### Evidence

| Finding | Evidence |
|---------|----------|
| Multi-Query generates 3 variants via `query_single()` | `service.rs:168-183` — calls LLM with system prompt to generate alternative questions |
| HyDE generates hypothetical docs per variant | `service.rs:192-211` — calls LLM for each variant with factual-doc instruction |
| Semantic search uses HyDE-enhanced queries | `service.rs:217-232` — for each HyDE result, concatenates query + hypothetical doc, passes to `search_chunks_semantic` |
| BM25 keyword search uses PostgreSQL ILIKE | `service.rs:256-266` — calls `search_chunks_text` which does `c.text ILIKE '%query%'` |
| LLM Reranking calls LLM per chunk | `service.rs:314-318` — iterates every merged chunk and calls `query_single()` with relevance prompt |
| Reranking default "брать" on error | `service.rs:320` — `unwrap_or_else(|_| "брать".to_string())` preserves chunks on LLM error |
| Reranking rejects chunks missing "брать" in response | `service.rs:321` — `verdict.to_lowercase().contains("брать")` — if LLM responds in Russian without "брать", chunk is rejected |
| BM25 tokenizer uses alphanumeric filtering | `bm25.rs:45-55` — `split_whitespace` + `filter(|c| c.is_alphanumeric())` |
| Debug data collected at each step | `service.rs:116-352` — `debug_data` struct populated at every pipeline stage |
| Session debug frontend component | `SessionDebug.vue` — renders collapsible pipeline steps with timing and results |

---

### Risks

🔴 **Critical** (must verify):

- **LLM reranking can filter out ALL chunks**: The reranking step (`service.rs:314-337`) calls the LLM for each merged chunk. If the LLM model used for reranking (default: `anthropic/claude-sonnet-4.6`) does not consistently respond with "брать", chunks are rejected. With `unwrap_or_else` defaulting to "брать", only LLM *errors* preserve chunks — but LLM *responses* that don't contain "брать" cause rejection. This is the most likely cause of the user-reported "no information" issue with 300+ documents.
- **LLM reranking performance**: Each `query_single()` call is a full non-streaming LLM API round-trip. With 300+ documents producing potentially dozens of chunks after merge, the reranking loop could take minutes, causing SSE timeouts on the frontend.
- **No timeout or concurrency limit on reranking**: The iteration (`for chunk in merged`) is sequential, with no timeout per chunk or parallel batch processing. A single slow LLM call blocks the entire pipeline.

🟡 **Medium** (should verify):

- **Multi-Query and HyDE increase LLM costs**: The pipeline makes 1 (MQ) + N (HyDE, one per variant) + M (reranking, one per chunk) LLM calls per user query. With defaults: 1 + 3 + up to ~40 = up to 44 LLM calls per query.
- **BM25 ILIKE query is a full table scan**: `search_chunks_text` uses `ILIKE '%query%'` which cannot use a B-tree index and will scan all active chunks in the collection. With 300+ documents and thousands of chunks, this could be slow.
- **Debug data is stored in every assistant message**: `debug_data_json` (lines 386, 517) is persisted to SQLite/PostgreSQL for every message with debug=true. Over many queries, this could bloat the messages table.
- **HyDE prompt quality relies on LLM output**: If the generated hypothetical document is low quality or off-topic, the HyDE-enhanced semantic search may return less relevant results than a direct query.

🟢 **Low** (nice to verify):

- **collection_id.to_string() is safe** — consistently uses UUID string as Chroma collection name per existing pattern
- **System prompt has injection guard** — `build_messages` in `llm.rs` wraps user query in `[USER_QUERY]` tags with guard instruction

---

### Testing Recommendations

**First priority:**

- [ ] End-to-end RAG pipeline test: upload documents → query → verify chunks are retrieved and LLM response is grounded in the context
- [ ] Verify chunk count flow through each pipeline step: embedding search results count, BM25 results count, merge count, reranking acceptance rate
- [ ] Test with `ADVANCED_RAG_ENABLED=false` to compare simple semantic search behavior vs advanced pipeline

**Regression:**

- [ ] Verify conversation history + session management still works with debug data attached to messages
- [ ] Test SSE streaming events: chunk → sources → done sequence still correct
- [ ] Verify the standard (non-advanced) query path still works when advanced RAG is disabled
