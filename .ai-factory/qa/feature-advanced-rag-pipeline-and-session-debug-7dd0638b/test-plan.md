## Test Plan: Advanced RAG Pipeline & Session Debug

**Date:** 2026-07-04
**Branch / Version:** feature/advanced-rag-pipeline-and-session-debug
**Environment:** Docker Compose (local), production (VPS)

---

### 1. Testing Goal

Verify that the advanced 7-step RAG pipeline correctly retrieves relevant document chunks, builds an LLM context, and produces a grounded answer. Identify why the system responds "no information" despite 300+ documents being loaded. Validate that the pipeline's LLM reranking step does not incorrectly filter out all relevant chunks.

---

### 2. Test Scope

**In Scope** — we test:

- Multi-Query expansion step — generates 3 query variants
- HyDE (Hypothetical Document Embeddings) — generates a hypothetical document per variant
- Semantic search (HyDE-enhanced vector search in Chroma) — correct embedding and retrieval
- BM25 keyword search (PostgreSQL ILIKE) — correct text matching
- Merge and deduplication — correct chunk_id-based dedup, no data loss
- LLM reranking step — correct "брать"/"пропустить" verdict parsing, no over-filtering
- Final context assembly — chunks included in LLM prompt are correct
- SSE event stream — chunk → sources → done sequence
- Debug data collection — all 7 steps recorded in DebugData
- Session persistence — debug_data stored on assistant messages
- Standard pipeline fallback (`ADVANCED_RAG_ENABLED=false`) — regression check
- Full E2E: upload documents → query → verify grounded answer with citations

**Out of Scope** — we don't test:

- Frontend SessionDebug.vue rendering details (covered by existing SessionDebug.spec.ts)
- Authentication/authorization flows (unchanged)
- Conversation history load and trim logic (unchanged)
- Document upload and parsing pipeline (unchanged)
- Chroma infrastructure reliability (assumed stable)

---

### 3. Test Types

| Type              | Priority   | Area                                                             |
|-------------------|------------|------------------------------------------------------------------|
| Functional        | 🔴 High    | LLM reranking verdict parsing, chunk filtering behavior          |
| Functional        | 🔴 High    | Chunk count flow through each pipeline step (no unexpected drops)|
| Functional        | 🟡 Medium  | Multi-Query variant generation and deduplication                 |
| Functional        | 🟡 Medium  | BM25 keyword search result quality                               |
| Functional        | 🟡 Medium  | HyDE-enhanced semantic search vs direct query comparison         |
| Functional        | 🟡 Medium  | Merge & dedup with overlapping vector + keyword results          |
| Integration       | 🔴 High    | End-to-end RAG pipeline: upload → index → query → answer         |
| Regression        | 🟡 Medium  | Standard (non-advanced) query path with `ADVANCED_RAG_ENABLED=false` |
| Regression        | 🟡 Medium  | SSE event sequence and format                                    |
| Edge cases        | 🟡 Medium  | Empty Chroma results (no chunks indexed)                         |
| Edge cases        | 🟡 Medium  | LLM reranking failure (all chunks accepted or rejected)          |
| Performance       | 🟢 Low     | Reranking latency with large chunk sets                          |
| Data integrity    | 🟡 Medium  | Debug data serialization and persistence                         |

---

### 4. Test Data

| Category          | Data                                              | Purpose                              |
|-------------------|---------------------------------------------------|--------------------------------------|
| Valid documents   | 3-5 PDF/Markdown files with known content         | Happy path E2E pipeline test         |
| Large document set | 300+ documents (or mock 300 chunks in Chroma)    | Scale test for reranking behavior    |
| Query with answer | Question with explicit answer in loaded docs      | Verify correct chunk retrieval       |
| Query no answer   | Question on a topic NOT in loaded docs            | Verify "no information" response     |
| Ambiguous query   | Partial keyword match, weak semantic match        | Hybrid search + reranking edge case  |
| Empty collection  | Collection with 0 documents                       | Negative test — no chunks available  |

---

### 5. Preconditions

- [ ] Docker Compose services are running: `chroma`, `embedding`, `backend`, `frontend`, `keycloak`
- [ ] Chroma is accessible at `http://chroma:8000` (Docker) or `http://localhost:8000` (host)
- [ ] Embedding service is accessible at `http://embedding:8001` (Docker) or `http://localhost:8001` (host)
- [ ] PostgreSQL is running and accessible
- [ ] Test documents are prepared (3-5 files with factual content, one large set with 300+ docs or simulated chunks)
- [ ] LLM API key is configured (`LLM_API_KEY`) in `.env`
- [ ] `ADVANCED_RAG_ENABLED=true` (default) for primary tests
- [ ] A test collection exists with indexed documents

---

### 6. Acceptance Criteria

- [ ] All 🔴 high-priority functional tests pass
- [ ] Reranking step does not filter out >90% of relevant chunks when query has direct match
- [ ] End-to-end query returns a grounded answer with at least one source citation
- [ ] Disabling advanced RAG (`ADVANCED_RAG_ENABLED=false`) still returns valid answers
- [ ] SSE event sequence is: 1+ chunk events → sources event → done event

---

### 7. Plan Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| LLM reranking filters ALL chunks | High — system always says "no information" | Add debug logging for reranking verdicts; test with `ADVANCED_RAG_ENABLED=false` as baseline |
| Chroma query returns 0 results even with documents | High — no chunks reach LLM | Verify Chroma collection has chunks; check collection_id matches indexed data |
| LLM API rate limits during Multi-Query/HyDE/Reranking | Medium — pipeline stalls mid-query | Add timeout and retry logging; test with small chunk counts first |
| BM25 ILIKE query timeout with 300+ documents | Medium — slow response | Monitor query duration; test with indexed chunks count |
| Debug data causes large message persistence overhead | Low — storage bloat | Verify debug data size; test truncation after repeated queries |

---

### 8. Checklist

| Check | Priority |
|-------|----------|
| LLM reranking verdict parsing — verify chunks with "брать" are kept, others rejected | 🔴 High |
| Chunk count tracking through pipeline: Multi-Query → HyDE → Vector Search → BM25 → Merge → Reranking → Final | 🔴 High |
| End-to-end RAG query with document upload → indexed → grounded answer (with sources) | 🔴 High |
| End-to-end RAG query with `ADVANCED_RAG_ENABLED=false` returns valid answer | 🟡 Medium |
| SSE event stream correctness: chunk events, sources event, done event | 🟡 Medium |
| Multi-Query generates 3 distinct query variants | 🟡 Medium |
| HyDE hypothetical document is non-empty and related to query | 🟡 Medium |
| Merge & Dedup: overlapping vector/keyword chunks are deduplicated correctly | 🟡 Medium |
| Debug data serialization — all 7 steps present in JSON output | 🟡 Medium |
| Query on empty collection returns "no information" gracefully | 🟡 Medium |
| Reranking latency with 40+ chunks — measure total time | 🟢 Low |
| Debug data round-trip — stored on message, retrievable via session history | 🟢 Low |
