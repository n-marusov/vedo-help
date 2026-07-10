## Test Cases: v0.5 Advanced RAG

**Branch:** `feature/v0.5-advanced-rag`
**Date:** 2026-07-11

---

### Group 1: Cross-encoder Reranker (`rerank.rs`)

---

### TC-001: Rerank chunks returns scored results in priority order

**Priority:** High
**Type:** Positive

**Precondition:**
- `rerank::rerank_chunks()` function is imported and available
- Mock scorer is active (real ONNX model not required)

**Steps:**
1. Create a list of `RerankItem` with 3 chunks of varying relevance to "What is Rust ownership?"
2. Call `rerank_chunks(items, query, top_k = 2)`
3. Inspect the returned `Vec<RerankResult>`

**Expected result:**
- Returns 2 `RerankResult` items (respects `top_k = 2`)
- Each result has a `rerank_score` between 0.0 and 1.0
- Results are sorted by `rerank_score` descending
- Each result contains the original `item` data (id, text, document_name preserved)
- Each result has a `RerankVerdict` (Keep or Discard)

**Test data:**
```
items = [
    RerankItem { id: "c1", text: "Ownership is Rust's core memory management system", document_name: "rust-book.md", original_index: 0, original_score: 0.85 },
    RerankItem { id: "c2", text: "The weather today is sunny", document_name: "weather.md", original_index: 1, original_score: 0.45 },
    RerankItem { id: "c3", text: "Borrowing allows references without ownership transfer", document_name: "rust-book.md", original_index: 2, original_score: 0.78 },
]
query = "What is Rust ownership?"
top_k = 2
```

---

### TC-002: Rerank with empty chunk list returns empty result

**Priority:** High
**Type:** Negative / Edge case

**Precondition:**
- `rerank::rerank_chunks()` function is imported

**Steps:**
1. Call `rerank_chunks(vec![], query, top_k = 5)`
2. Inspect the return value

**Expected result:**
- Returns an empty `Vec<RerankResult>` (no error)
- No panic occurs
- No side effects (logging is acceptable)

**Test data:**
```
items = []  (empty vec)
query = "any query"
top_k = 5
```

---

### TC-003: Rerank with top_k larger than input size

**Priority:** Medium
**Type:** Edge case

**Precondition:**
- `rerank::rerank_chunks()` function is imported

**Steps:**
1. Create 2 `RerankItem`s
2. Call `rerank_chunks(items, query, top_k = 10)`
3. Inspect the return value

**Expected result:**
- Returns 2 results (all available items, not padded to top_k)
- Scores are sorted descending
- No error or panic

**Test data:**
```
items = [RerankItem { id: "c1", text: "First relevant chunk", ... }, RerankItem { id: "c2", text: "Second relevant chunk", ... }]
query = "test query"
top_k = 10
```

---

### TC-004: RerankVerdict assignment follows scoring thresholds

**Priority:** High
**Type:** Positive

**Precondition:**
- Source code of `rerank_chunks()` known to determine verdict assignment logic
- Verdict is based on score threshold or relative ranking

**Steps:**
1. Create a list of 5 `RerankItem`s with deliberately varied text relevance
2. Call `rerank_chunks(items, query, top_k = 5)`
3. Check each result's `verdict` field

**Expected result:**
- Top-scoring chunks receive `RerankVerdict::Keep`
- Low-scoring chunks receive `RerankVerdict::Discard`
- The verdict assignment is deterministic for the same inputs

**Test data:**
```
items = [
    { id: "c1", text: "Directly relevant content about the query topic", ... },
    { id: "c2", text: "Partially relevant content", ... },
    { id: "c3", text: "Completely unrelated content about sports", ... },
    { id: "c4", text: "Another relevant piece about the topic", ... },
    { id: "c5", text: "Gibberish text with no meaning", ... },
]
query = "the query topic"
```

---

### TC-005: Rerank chunks with extremely long text

**Priority:** Medium
**Type:** Edge case

**Precondition:**
- `rerank::rerank_chunks()` function is imported

**Steps:**
1. Create a `RerankItem` with 100KB of text
2. Call `rerank_chunks(vec![item], query, top_k = 1)`
3. Inspect the result

**Expected result:**
- Returns 1 `RerankResult` with a valid score
- No panic or OOM
- Processing completes within reasonable time (< 5s)

**Test data:**
```
item = RerankItem { id: "c1", text: "A".repeat(100_000), ... }
query = "what happens with long text"
top_k = 1
```

---

### Group 2: File Validation (`file_validation.rs`)

---

### TC-006: Validate CSV file with correct MIME type and magic bytes

**Priority:** High
**Type:** Positive

**Precondition:**
- `file_validation::validate_file()` function is available
- Test CSV file with known valid content

**Steps:**
1. Create a byte buffer with content: `"name,age,city\nAlice,30,NYC\nBob,25,LA"`
2. Call `validate_file(buffer, "data.csv", "text/csv")`
3. Inspect the result

**Expected result:**
- Returns `Ok(FileType::Csv)` or equivalent success
- No error about invalid MIME type or magic bytes

**Test data:**
```
content = "name,age,city\nAlice,30,NYC\nBob,25,LA"
filename = "data.csv"
mime_type = "text/csv"
```

---

### TC-007: Validate JSON file with correct MIME type

**Priority:** High
**Type:** Positive

**Precondition:**
- `file_validation::validate_file()` function is available

**Steps:**
1. Create a byte buffer with content: `[{"title":"A","body":"Hello"},{"title":"B","body":"World"}]`
2. Call `validate_file(buffer, "data.json", "application/json")`
3. Inspect the result

**Expected result:**
- Returns `Ok(FileType::Json)` or equivalent success

**Test data:**
```
content = '[{"title":"A","body":"Hello"},{"title":"B","body":"World"}]'
filename = "data.json"
mime_type = "application/json"
```

---

### TC-008: Validate HTML file with correct MIME type

**Priority:** High
**Type:** Positive

**Precondition:**
- `file_validation::validate_file()` function is available

**Steps:**
1. Create a byte buffer with content: `"<html><body><p>Hello world</p></body></html>"`
2. Call `validate_file(buffer, "doc.html", "text/html")`
3. Inspect the result

**Expected result:**
- Returns `Ok(FileType::Html)` or equivalent success

**Test data:**
```
content = "<html><body><p>Hello world</p></body></html>"
filename = "doc.html"
mime_type = "text/html"
```

---

### TC-009: Binary file with misleading CSV extension is rejected

**Priority:** High
**Type:** Negative

**Precondition:**
- `file_validation::validate_file()` function is available

**Steps:**
1. Create a byte buffer with PNG file header bytes `[0x89, 0x50, 0x4E, 0x47, ...]`
2. Call `validate_file(buffer, "data.csv", "text/csv")`
3. Inspect the result

**Expected result:**
- Returns `Err(AppError)` indicating MIME type mismatch or invalid content
- Error message mentions the actual detected type vs declared type

**Test data:**
```
content = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, /* PNG header */ ...]
filename = "data.csv"
mime_type = "text/csv"
```

---

### TC-010: Empty CSV file is rejected

**Priority:** Medium
**Type:** Negative

**Precondition:**
- `file_validation::validate_file()` function is available

**Steps:**
1. Create an empty byte buffer (0 bytes)
2. Call `validate_file(buffer, "empty.csv", "text/csv")`
3. Inspect the result

**Expected result:**
- Returns `Err(AppError)` with error about empty or invalid file

**Test data:**
```
content = []  (0 bytes)
filename = "empty.csv"
mime_type = "text/csv"
```

---

### TC-011: JSON with non-array/object structure is rejected

**Priority:** Medium
**Type:** Negative

**Precondition:**
- `file_validation::validate_file()` function is available

**Steps:**
1. Create a byte buffer with content: `"just a plain string"`
2. Call `validate_file(buffer, "data.json", "application/json")`
3. Inspect the result

**Expected result:**
- Returns `Err(AppError)` indicating invalid JSON document structure

**Test data:**
```
content = "just a plain string"
filename = "data.json"
mime_type = "application/json"
```

---

### Group 3: Multi-format Document Parsing (`documents/service.rs`)

---

### TC-012: Parse CSV file into document chunks

**Priority:** High
**Type:** Positive

**Precondition:**
- `DocumentService` is initialized with `DocumentRepository` and `CollectionRepository`
- A test collection exists in the database
- Valid CSV content is prepared

**Steps:**
1. Upload a valid CSV file through `DocumentService.process_upload()` or the CSV parsing method directly
2. Verify the document is saved in the database
3. Verify chunks are created from CSV rows

**Expected result:**
- Document is saved with `file_type = "text/csv"`
- Each CSV row (excluding header) becomes a separate chunk
- Chunk text includes column context (e.g., `"name: Alice, age: 30, city: NYC"`)
- Chunks are indexed and queryable

**Test data:**
```
CSV content:
name,age,city
Alice,30,NYC
Bob,25,LA
Charlie,35,SF

Expected chunks: 3 (one per data row)
```

---

### TC-013: Parse JSON file into document chunks

**Priority:** High
**Type:** Positive

**Precondition:**
- `DocumentService` is initialized
- A test collection exists

**Steps:**
1. Upload a valid JSON file
2. Verify document metadata is saved
3. Verify chunks are created from JSON objects

**Expected result:**
- Document saved with `file_type = "application/json"`
- Each JSON object in the array becomes a chunk
- Chunk text contains the JSON object content
- Chunks are indexed and queryable

**Test data:**
```
JSON content:
[
  {"title": "Introduction", "content": "First chapter content..."},
  {"title": "Methods", "content": "Second chapter content..."},
  {"title": "Results", "content": "Third chapter content..."}
]

Expected chunks: 3 (one per object)
```

---

### TC-014: Parse HTML file into document chunks

**Priority:** High
**Type:** Positive

**Precondition:**
- `DocumentService` is initialized
- A test collection exists

**Steps:**
1. Upload a valid HTML file
2. Verify document metadata is saved
3. Verify chunks are created from extracted text

**Expected result:**
- Document saved with `file_type = "text/html"`
- HTML tags are stripped, plain text is extracted
- Text is split into chunks using the standard chunking strategy
- Chunks are indexed and queryable

**Test data:**
```
HTML content:
<html>
  <body>
    <h1>Documentation</h1>
    <p>This is the first paragraph about the topic.</p>
    <p>This is the second paragraph with more details.</p>
  </body>
</html>

Expected: 1+ chunks containing the extracted plain text
```

---

### TC-015: Malformed CSV causes error, not panic

**Priority:** High
**Type:** Negative

**Precondition:**
- `DocumentService` is initialized

**Steps:**
1. Attempt to upload a CSV file with inconsistent column counts per row
2. Call `process_upload()` or the CSV parser directly
3. Inspect the result

**Expected result:**
- Returns `Err(AppError)`, not a panic
- Error message indicates CSV parsing failure (e.g., "CSV parse error: ...")
- No partial document or chunks are saved

**Test data:**
```
CSV content:
name,age
Alice,30,NYC
Bob,25
```

---

### TC-016: Malformed JSON causes error, not panic

**Priority:** High
**Type:** Negative

**Precondition:**
- `DocumentService` is initialized

**Steps:**
1. Attempt to upload a JSON file with unclosed brace
2. Inspect the result

**Expected result:**
- Returns `Err(AppError)`
- Error message indicates JSON parse failure
- No partial data is persisted

**Test data:**
```
JSON content:
[{"title": "Incomplete", "body": "Missing closing brace"
```

---

### TC-017: File exceeding size limit is rejected

**Priority:** Medium
**Type:** Negative

**Precondition:**
- `MAX_FILE_SIZE` is defined (expected: ~50 MB)
- `DocumentService` is initialized

**Steps:**
1. Attempt to upload a file with size > `MAX_FILE_SIZE`
2. Inspect the result

**Expected result:**
- Returns `Err(AppError)` with 413 Payload Too Large equivalent
- Error message mentions the file size limit

**Test data:**
```
file_size = MAX_FILE_SIZE + 1  // e.g., 50_000_001 bytes
```

---

### Group 4: Token Counting (`context_window.rs`)

---

### TC-018: count_tokens returns accurate count for ASCII text

**Priority:** High
**Type:** Positive

**Precondition:**
- `context_window::count_tokens()` function is available
- tiktoken BPE vocabulary is loadable (or test with both paths)

**Steps:**
1. Call `count_tokens("Hello, world!")`
2. Call `count_tokens("Rust ownership is a system of rules for managing memory.")`
3. Compare results with known cl100k_base token counts

**Expected result:**
- "Hello, world!" returns 3 tokens (known cl100k_base value)
- "Rust ownership..." returns the correct cl100k_base token count
- Empty string returns 0

**Test data:**
```
text_ascii = "Hello, world!"
text_long = "Rust ownership is a system of rules for managing memory."
text_empty = ""
```

---

### TC-019: count_tokens handles CJK and emoji text

**Priority:** High
**Type:** Positive

**Precondition:**
- `context_window::count_tokens()` is available

**Steps:**
1. Call `count_tokens` with Chinese characters: "你好世界"
2. Call `count_tokens` with Japanese: "こんにちは"
3. Call `count_tokens` with emoji: "🚀🌟💡"
4. Verify all return non-zero results

**Expected result:**
- All return integer token counts > 0
- Results are deterministic (same input → same output)

**Test data:**
```
chinese = "你好世界"
japanese = "こんにちは"
emoji = "🚀🌟💡"
```

---

### TC-020: count_tokens falls back to word-count when tiktoken fails

**Priority:** High
**Type:** Positive

**Precondition:**
- Code path for tiktoken init failure is known and can be triggered

**Steps:**
1. Simulate tiktoken initialization failure (e.g., by setting environment or mocking)
2. Call `count_tokens("Hello world from fallback")`
3. Inspect the result

**Expected result:**
- Returns token count based on `split_whitespace().count()` heuristic
- Word "Hello world from fallback" returns 4
- Does not panic or return an error
- A warning is logged about fallback activation

**Test data:**
```
text = "Hello world from fallback"
expected_fallback_count = 4  // words
```

---

### TC-021: trim_history respects max_messages limit

**Priority:** High
**Type:** Positive

**Precondition:**
- `context_window::trim_history()` function is available
- A list of LLM `Message`s with alternating user/assistant roles is prepared

**Steps:**
1. Create a history of 8 messages (4 user + 4 assistant pairs)
2. Call `trim_history(messages, max_messages = 3, token_budget = 100_000)`
3. Inspect the result

**Expected result:**
- Returns a vector with exactly 3 messages (1 user + 1 assistant pair + 1 system message if present)
- The most recent pair is preserved
- Older messages are dropped

**Test data:**
```
messages = [user1, asst1, user2, asst2, user3, asst3, user4, asst4]
max_messages = 3
token_budget = 100_000  // large enough to not trigger token-based trimming
expected: preserves newest 3 messages (user4, asst4, user3) or similar based on implementation
```

---

### TC-022: trim_history respects token_budget when max_messages is large

**Priority:** High
**Type:** Positive

**Precondition:**
- `context_window::trim_history()` is available
- Multi-turn history with known token counts

**Steps:**
1. Create a history of 6 messages (3 pairs) where each message is ~2000 tokens
2. Call `trim_history(messages, max_messages = 10, token_budget = 5000)`
3. Inspect the result

**Expected result:**
- Returns messages whose total token count is ≤ 5000
- Latest messages are preserved
- Oldest messages are dropped first
- The system message (if any) is always kept

**Test data:**
```
messages = 6 messages, each ~2000 tokens
max_messages = 10
token_budget = 5000
expected: 2 messages preserved (one pair = ~4000 tokens) or fewer
```

---

### Group 5: LLM Fallback (`llm.rs`)

---

### TC-023: Primary LLM endpoint fails → fallback endpoint is called

**Priority:** High
**Type:** Positive

**Precondition:**
- `LlmClient` is configured with a primary URL that returns an error
- Fallback URL is configured to return a valid response
- Mock HTTP server or sandpit API keys available

**Steps:**
1. Configure `LlmClient` with primary endpoint pointing to an unreachable URL
2. Configure fallback endpoint pointing to a mock server that returns a valid response
3. Call `query_single("test prompt")` or the streaming equivalent

**Expected result:**
- Returns a successful response (from the fallback)
- A log message indicates fallback was invoked
- No error is propagated to the caller if fallback succeeds

**Test data:**
```
primary_base_url = "http://localhost:19999/nonexistent"  // unreachable
fallback_base_url = "http://localhost:18002"  // mock server returning valid response
prompt = "What is Rust?"
```

---

### TC-024: Both primary and fallback endpoints fail → error returned

**Priority:** Medium
**Type:** Negative

**Precondition:**
- `LlmClient` is configured with both endpoints pointing to unreachable URLs

**Steps:**
1. Set both primary and fallback URLs to unreachable endpoints
2. Call `query_single("test prompt")`
3. Inspect the result

**Expected result:**
- Returns `Err(AppError)` with a meaningful error message
- Error indicates that both endpoints were attempted and both failed
- No panic occurs

**Test data:**
```
primary_base_url = "http://localhost:19999/primary"
fallback_base_url = "http://localhost:19998/fallback"
both unreachable
```

---

### TC-025: Fallback returns response in expected format

**Priority:** Medium
**Type:** Positive

**Precondition:**
- `LlmClient` is configured with fallback endpoint
- Mock server returns a valid streaming response

**Steps:**
1. Configure primary endpoint to fail immediately (e.g., HTTP 500)
2. Configure fallback to return a well-formed response
3. Call the streaming endpoint
4. Collect all SSE events

**Expected result:**
- Response contains `chunk`, `sources`, and `done` events
- Sources include document citations
- The response content is a coherent answer to the query
- No malformed or incomplete events

**Test data:**
```
prompt = "Explain Rust borrowing in one sentence"
expected: streaming response with text, sources, and done event
```

---

### Group 6: BM25 Configurable Parameters & RRF Fusion

---

### TC-026: BM25 with alpha=1.0 behaves like pure vector search

**Priority:** Medium
**Type:** Positive

**Precondition:**
- BM25 index is populated with known documents
- Chroma vector index has the same documents
- Hybrid search orchestrator is available

**Steps:**
1. Set `rrf_alpha = 1.0` (pure vector weight)
2. Execute a search query
3. Compare results with a pure vector search (alpha not applicable)

**Expected result:**
- Results are dominated by vector similarity ranking
- BM25 keyword score contribution is negligible
- The returned chunks are semantically similar to the query

**Test data:**
```
query = "memory management in systems programming"
rrf_alpha = 1.0  // 100% vector, 0% BM25
```

---

### TC-027: BM25 with alpha=0.0 behaves like pure keyword search

**Priority:** Medium
**Type:** Positive

**Precondition:**
- BM25 index is populated
- Chroma vector index has the same documents

**Steps:**
1. Set `rrf_alpha = 0.0` (pure BM25 weight)
2. Execute a search query with specific keywords
3. Compare results

**Expected result:**
- Results contain documents with exact keyword matches
- Semantic matches without keywords may be ranked lower or absent
- BM25 scores dominate the ranking

**Test data:**
```
query = "garbage collection memory safety"
rrf_alpha = 0.0  // 0% vector, 100% BM25
```

---

### Group 7: Regression Checks

---

### TC-028: Existing RAG pipeline tests pass with new config fields

**Priority:** Medium
**Type:** Regression

**Precondition:**
- Docker Compose with Chroma, PostgreSQL, and test services running
- `DATABASE_URL` and `CHROMA_URL` environment variables set

**Steps:**
1. Run `cargo test --test rag_pipeline`
2. Run `cargo test --test integration`
3. Run `cargo test --test health_integration`

**Expected result:**
- All existing tests pass (green)
- No compilation errors related to new config fields
- No test failures caused by the new `llm_fallback_base_url` field

**Test data:**
```
CARGO_FLAGS = none (default)
```

---

### TC-029: Git sync works with documents containing new file formats

**Priority:** Medium
**Type:** Regression

**Precondition:**
- GitSyncService is available
- A test git repository contains CSV, JSON, and HTML files alongside existing formats

**Steps:**
1. Create a test git repository with a mix of markdown, CSV, JSON, and HTML files
2. Run the git sync pipeline (clone → parse → chunk → embed → index)
3. Verify all documents are indexed

**Expected result:**
- CSV, JSON, and HTML files are parsed and indexed alongside markdown files
- Chunks from new formats appear in search results
- No errors specific to new format parsing

**Test data:**
```
repo structure:
  docs/
    readme.md
    data.csv
    config.json
    index.html
```

---

### TC-030: Existing PDF/MD/DOCX upload still works

**Priority:** Medium
**Type:** Regression

**Precondition:**
- `DocumentService` is initialized
- Test collection exists
- Valid PDF, Markdown, and DOCX files are available

**Steps:**
1. Upload a PDF file through `process_upload()`
2. Upload a Markdown file
3. Upload a DOCX file
4. Query the collection

**Expected result:**
- All three files are parsed successfully
- Chunks from all files are returned in search results
- No regression from the multi-format refactor

**Test data:**
```
files:
  - sample.pdf (standard PDF with text)
  - readme.md (markdown with headers and paragraphs)
  - report.docx (DOCX with formatted text)
```

---

### Summary of Test Cases

| ID | Area | Priority | Type |
|----|------|----------|------|
| TC-001 | Reranker scoring | High | Positive |
| TC-002 | Reranker empty input | High | Negative |
| TC-003 | Reranker top_k > input | Medium | Edge case |
| TC-004 | Reranker verdict assignment | High | Positive |
| TC-005 | Reranker long text | Medium | Edge case |
| TC-006 | File validation CSV | High | Positive |
| TC-007 | File validation JSON | High | Positive |
| TC-008 | File validation HTML | High | Positive |
| TC-009 | File validation MIME mismatch | High | Negative |
| TC-010 | File validation empty file | Medium | Negative |
| TC-011 | File validation invalid JSON | Medium | Negative |
| TC-012 | Multi-format CSV parsing | High | Positive |
| TC-013 | Multi-format JSON parsing | High | Positive |
| TC-014 | Multi-format HTML parsing | High | Positive |
| TC-015 | Multi-format malformed CSV | High | Negative |
| TC-016 | Multi-format malformed JSON | High | Negative |
| TC-017 | File size limit exceeded | Medium | Negative |
| TC-018 | Token count ASCII | High | Positive |
| TC-019 | Token count CJK/emoji | High | Positive |
| TC-020 | Token count fallback | High | Positive |
| TC-021 | trim_history message limit | High | Positive |
| TC-022 | trim_history token budget | High | Positive |
| TC-023 | LLM fallback primary down | High | Positive |
| TC-024 | LLM fallback both down | Medium | Negative |
| TC-025 | LLM fallback response format | Medium | Positive |
| TC-026 | BM25 pure vector (alpha=1.0) | Medium | Positive |
| TC-027 | BM25 pure keyword (alpha=0.0) | Medium | Positive |
| TC-028 | Regression: existing tests | Medium | Regression |
| TC-029 | Regression: git sync new formats | Medium | Regression |
| TC-030 | Regression: existing format uploads | Medium | Regression |

**Coverage:**
- 🔴 High priority: 17 cases
- 🟡 Medium priority: 13 cases
- 🔴 High negative: 5 cases
- 🟡 Medium regression: 3 cases
