## Test Cases: Advanced RAG Pipeline Verification

---

### TC-001: End-to-End RAG Pipeline — Grounded Answer with Sources

**Priority:** High
**Type:** Positive

**Precondition:**

- Docker Compose environment is running (chroma, embedding, backend, frontend, postgres)
- A test collection exists with 3-5 indexed documents containing known factual content
- Admin user is authenticated with a valid Bearer token
- `ADVANCED_RAG_ENABLED=true` (default)
- LLM API key is configured and the LLM service is reachable

**Steps:**

1. Choose a collection with indexed documents that contain an answer to a specific question
2. Send a POST request to `/api/query` with the question, collection_id, and `debug: true`:
   ```json
   {
     "collection_id": "<known-collection-uuid>",
     "query": "What is VEDO hub?",
     "debug": true
   }
   ```
3. Read the SSE event stream until the `done` event
4. Count chunk events — verify at least one text chunk before `sources`
5. In the `sources` event, verify at least one source entry with `document_name`, `text`, and `relevance`

**Expected result:**

- At least 1 `chunk` SSE event is emitted before `sources`
- The `sources` event contains ≥1 source entry with `document_name`, `text`, and `relevance > 0.0`
- The `done` event completes the stream
- The concatenated chunk text contains a grounded answer (does NOT say "I don't have information")

**Test data:**

```
Collection: "test-docs" (UUID known from preparation)
Query: "What is VEDO hub?"
Expected context: documents about VEDO hub exist in the collection
Expected sources: ≥1 source from the indexed documents
```

---

### TC-002: LLM Reranking Verdict Parsing — No Over-Filtering

**Priority:** High
**Type:** Positive

**Precondition:**

- Docker Compose environment running
- A test collection with indexed documents
- `ADVANCED_RAG_ENABLED=true`
- LLM API key configured

**Steps:**

1. Send a POST request to `/api/query` with `debug: true`:
   ```json
   {
     "collection_id": "<test-collection-uuid>",
     "query": "What is VEDO hub?",
     "debug": true
   }
   ```
2. Capture the full SSE stream, including the `done` event payload
3. From the `done` event, extract `assistant_message_id`
4. Fetch the assistant message via the session history API (GET `/api/sessions/<session-id>/messages`)
5. Extract the `debug_data` JSON field from the assistant message
6. Parse and inspect the reranking step:
   - `reranking.input_count` — how many chunks were evaluated
   - `reranking.accepted` — how many were accepted (verdict "брать")
   - `reranking.rejected` — how many were rejected (verdict "пропустить")
   - Examine a `rerank_result` to see the LLM's `verdict` and `comment`

**Expected result:**

- `reranking.input_count` > 0 (chunks were available for reranking)
- `reranking.accepted` > 0 (at least some chunks passed reranking)
- If `reranking.rejected` = `reranking.input_count` — **this confirms the bug**: ALL chunks were rejected
- If `reranking.accepted` = 0, the response will contain "no information" — examine the verdict strings to understand why the LLM rejected all chunks
- The `final_answer.chunks_in_context` matches `reranking.accepted` (after top-k truncation)

**Test data:**

```
Expected: reranking.accepted > 0 for queries with known content
Bug symptom: reranking.accepted = 0 despite embedding_search.result_count > 0
```

---

### TC-003: Chunk Count Flow Through Pipeline Stages

**Priority:** High
**Type:** Positive

**Precondition:**

- Same as TC-002
- A debug-enabled query has been executed and the assistant message with debug_data is available

**Steps:**

1. Extract the debug_data JSON from the assistant message (same as TC-002)
2. Log the chunk count at each pipeline stage:

| Step | Debug field | Expected value |
|------|-------------|----------------|
| Multi-Query | `multi_query.variants.len()` | ≥1 (default 3) |
| HyDE | `hyde.per_query.len()` | Same as variant count |
| Vector Search | `embedding_search.result_count` | ≤ `hybrid_top_k` × variant count (default ≤ 60) |
| Keyword Search | `keyword_search.total_matches` | Any number (0 if no text matches) |
| Merge | `merge_dedup.input_chunks` | `vector_results + keyword_results` |
| After Dedup | `merge_dedup.after_dedup` | ≤ `input_chunks` |
| Reranking | `reranking.input_count` | = `merge_dedup.after_dedup` |
| Accepted | `reranking.accepted` | ≤ `reranking.input_count` |
| Final Context | `final_answer.chunks_in_context` | ≤ `rerank_top_k` (default 5) |

3. Identify the first step where the count drops to 0 or near-0

**Expected result:**

- All counts are non-zero for queries on collections with indexed data
- The first drop to 0 identifies the failing pipeline stage
- If `embedding_search.result_count = 0` → Chroma has no matching chunks (indexing issue)
- If `embedding_search.result_count > 0` but `reranking.accepted = 0` → **reranking is the root cause**

**Test data:**

```
Debug data path: assistant_message.debug_data → JSON path to each step count
Bug diagnosis:
  - embedding_search.result_count = 0 → Chroma/indexing problem
  - keyword_search.total_matches = 0 AND embedding_search.result_count = 0 → no chunks in collection
  - reranking.accepted = 0 AND reranking.input_count > 0 → LLM reranking rejects everything
```

---

### TC-004: Compare Advanced vs Standard Pipeline

**Priority:** High
**Type:** Positive

**Precondition:**

- Docker Compose environment running
- A test collection with indexed documents
- Two separate debug-enabled queries executed (one with advanced, one without)

**Steps:**

1. Query with `ADVANCED_RAG_ENABLED=true` (env var):
   ```json
   {
     "collection_id": "<test-collection-uuid>",
     "query": "What is VEDO hub?",
     "debug": true
   }
   ```
2. Set `ADVANCED_RAG_ENABLED=false` and restart the backend service:
   ```
   docker compose restart backend
   ```
3. Send the same query again
4. Compare responses:
   - How many chunks were in the final context for each mode?
   - Was an answer produced in both modes?
   - Did one mode have sources and the other not?

**Expected result:**

- Both modes return a grounded answer for the same query
- The standard mode (`advanced=false`) uses simple semantic search (`rerank_top_k` chunks)
- The advanced mode may return different (ideally better) sources
- If standard mode works but advanced mode doesn't: **the advanced pipeline is the problem**

**Test data:**

```
Environment: ADVANCED_RAG_ENABLED=true vs false
Same query, same collection, same documents
```

---

### TC-005: Query With No Matching Content

**Priority:** Medium
**Type:** Negative

**Precondition:**

- A collection with indexed documents about a specific topic (e.g., "VEDO hub setup")
- Query that asks about a completely unrelated topic not in any document

**Steps:**

1. Send a POST request to `/api/query`:
   ```json
   {
     "collection_id": "<test-collection-uuid>",
     "query": "What is the capital of France?",
     "debug": true
   }
   ```
2. Read the SSE event stream
3. Examine the response text and sources

**Expected result:**

- The LLM correctly states it does not have information on the topic
- Sources may be empty or contain irrelevant chunks
- The response is polite and informative ("no information available")
- **This is the CORRECT behavior** — the system should say it doesn't know when context is missing

**Test data:**

```
Query: "What is the capital of France?"
Documents: contain only VEDO hub documentation, no geography content
Expected: "I don't have information about that" or equivalent
```

---

### TC-006: Reranking Performance With Large Chunk Sets

**Priority:** Medium
**Type:** Edge case

**Precondition:**

- A collection with many documents (300+ documents, ideally thousands of chunks)
- OR a mock collection with many chunks (can use a small number of documents with a small chunk size to produce many chunks)
- `ADVANCED_RAG_ENABLED=true`

**Steps:**

1. Send a debug-enabled query to the collection with many chunks
2. From the debug data, extract:
   - `embedding_search.latency_ms`
   - `keyword_search.latency_ms`
   - `reranking.input_count`
   - Total reranking time (sum of individual `query_single` calls — estimated from total query latency minus search latencies)
3. Calculate the total LLM calls: 1 (Multi-Query) + N (HyDE per variant) + M (Reranking per chunk)

**Expected result:**

- Total query latency is < 30 seconds (reasonable user experience threshold)
- Reranking step processes all chunks without timeout
- If total latency > 60 seconds: the sequential reranking of many chunks is a performance bottleneck

**Test data:**

```
Collection size: 300+ documents, ~3000+ chunks
Reranking chunks: up to 40 (hybrid_top_k=20 × 2 sources)
LLM calls per query: 1 (MQ) + 3 (HyDE) + ~40 (reranking) = ~44 calls
```

---

### TC-007: SSE Event Stream Correctness

**Priority:** Medium
**Type:** Regression

**Precondition:**

- Any collection with indexed documents

**Steps:**

1. Send a POST request to `/api/query` with `debug: true`
2. Read the raw SSE event stream line by line
3. Record the `event_type` of each event

**Expected result:**

- Each line follows the SSE format: `data: {"type":"...","data":{...}}`
- Events appear in this order:
  1. Zero or more `chunk` events (LLM response tokens)
  2. Exactly one `sources` event
  3. Exactly one `done` event
- The `sources` event contains a `sources` array
- The `done` event contains `user_message_id` and `assistant_message_id`
- No events after `done`

**Test data:**

```
SSE event sequence: chunk* → sources → done
```

---

### TC-008: Debug Data Persistence Round-Trip

**Priority:** Medium
**Type:** Positive

**Precondition:**

- Session ID is provided in the query request
- Admin user authenticated

**Steps:**

1. Send a query with a `session_id` and `debug: true`:
   ```json
   {
     "collection_id": "<uuid>",
     "query": "What is VEDO hub?",
     "session_id": "<uuid>",
     "debug": true
   }
   ```
2. After the stream completes, fetch the session messages: `GET /api/sessions/<session-id>/messages`
3. Find the assistant message (role=assistant) from the response
4. Verify that `debug_data` field is a non-empty JSON string
5. Parse the `debug_data` JSON and verify these top-level keys exist:
   - `query_text`, `multi_query`, `hyde`, `embedding_search`
   - `keyword_search`, `merge_dedup`, `reranking`, `final_answer`

**Expected result:**

- `debug_data` is present and parseable on the assistant message
- All 7 pipeline step fields are present (some may be null, e.g., `multi_query`, `hyde` — they are v0.5)
- `embedding_search` and `final_answer` are non-null (they are marked as "Active")
- `reranking` is non-null (the reranking Step 2.f is always executed)

**Test data:**

```
API: GET /api/sessions/{session_id}/messages
Expected debug_data keys: query_text, multi_query, hyde, embedding_search, keyword_search, merge_dedup, reranking, final_answer
```

---

### TC-009: Multi-Query Generates Distinct Variants

**Priority:** Medium
**Type:** Positive

**Precondition:**

- Debug-enabled query with known query text

**Steps:**

1. Send a query with `debug: true`
2. Retrieve the debug data from the assistant message
3. Inspect `multi_query.variants` array

**Expected result:**

- `multi_query` is not null
- `multi_query.variants` contains ≥1 variant (default config: 3)
- Variants are non-empty strings
- Variants are not identical to each other (they should be distinct rephrasings)
- `multi_query.latency_ms` is a reasonable value (< 10 seconds)

**Test data:**

```
Query: "How do I install VEDO hub?"
Expected: 3 different rephrasings of the installation question
```

---

### TC-010: Merge & Dedup Correctness

**Priority:** Medium
**Type:** Edge case

**Precondition:**

- Debug-enabled query
- The query should have both keyword matches AND semantic matches (use a query that contains keywords present in documents)

**Steps:**

1. Send a debug-enabled query
2. From the debug data, inspect `merge_dedup`:
   - `source_breakdown.vector_chunks` — count from vector search
   - `source_breakdown.keyword_chunks` — count from BM25
   - `input_chunks` = vector + keyword
   - `after_dedup` ≤ `input_chunks`

**Expected result:**

- `after_dedup` ≤ `input_chunks` (duplicates are removed)
- The difference (`input_chunks - after_dedup`) represents the number of overlapping chunks found by both search methods
- If `keyword_chunks = 0` and the query contains actual keywords from documents, BM25 search may have an issue

**Test data:**

```
Query: phrase with both semantic meaning and keywords (e.g., "Docker Compose setup VEDO")
Expected: vector_chunks > 0 AND keyword_chunks > 0 for a well-matched query
```

---

## Test Data (based on test design techniques)

### Positive

* Collection with 3-5 indexed documents about VEDO hub (install, setup, features)
* Query: "What is VEDO hub?" — exact match expected
* Query: "How to deploy VEDO?" — semantic match expected
* Query: "vedo docker" — keyword match expected (BM25)

### Negative

* Query on empty collection (no documents indexed)
* Query with topic completely absent from documents
* `ADVANCED_RAG_ENABLED=true` with no LLM API key configured (LLM calls fail)
* Query with empty string as query text (validation should reject)
* Very long query (>1000 characters)
