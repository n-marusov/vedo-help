[← User Interface Guide](gui.md) · [Back to README](../README.md) · [Auth →](auth.md)

# API Reference

## Base URL

| Environment | URL |
|-------------|-----|
| Development | `http://localhost:3000` |
| Production | `https://your-domain.com/api` |

## Authentication

All API routes (except `/health` and `/api/health/deep`) require a KeyCloak-issued Bearer JWT token:

```
Authorization: Bearer <ACCESS_TOKEN>
```

Requests without a valid token return `401 Unauthorized`.

Obtain an access token via the OAuth 2.0 Authorization Code flow with PKCE (see [Auth](auth.md)).

## Standard Error Format

```json
{
  "error": {
    "type": "error_type",
    "message": "Human-readable description"
  }
}
```

| Status | Error Type | Description |
|--------|-----------|-------------|
| 400 | `bad_request` | Invalid input |
| 401 | `unauthorized` | Missing or invalid token |
| 404 | `not_found` | Resource not found |
| 415 | `file_error` | Unsupported or corrupt file |
| 413 | `payload_too_large` | ZIP exceeds 10-file limit or body > 50 MB |
| 429 | `rate_limited` | Too many requests |
| 502 | `embedding_error` | Embedding service unavailable |
| 502 | `chroma_error` | Chroma unavailable |
| 502 | `llm_error` | LLM API error |
| 422 | `unprocessable_entity` | Validation error (e.g., editing assistant messages) |

## Endpoints

### Health

#### `GET /health`

Liveness probe — does not require authentication.

```bash
curl http://localhost:3000/health
# → OK
```

#### `GET /api/health/deep`

Deep healthcheck — probes all downstream dependencies (Chroma, Embedding, LLM, PostgreSQL). Returns aggregated status with per-service latency and error details. Does not require authentication.

**Response `200` (healthy or degraded):**

```json
{
  "status": "healthy",
  "checks": [
    {
      "name": "Chroma",
      "status": "healthy",
      "latency_ms": 5
    },
    {
      "name": "PostgreSQL",
      "status": "healthy",
      "latency_ms": 1
    }
  ],
  "timestamp": "2026-06-28T12:00:00Z"
}
```

**Response `503` (unhealthy — critical dependency down):**

Same JSON structure with `"status": "unhealthy"`. Status values: `healthy`, `degraded`, `unhealthy`. Each check has status `healthy`, `degraded`, or `unhealthy` with optional `error` field.

### Documents

#### `POST /api/documents/upload`

Upload a single document to a collection. For ZIP archives, use `POST /api/documents/upload-zip`.

**Request:** `multipart/form-data`

| Field | Type | Description |
|-------|------|-------------|
| `file` | File | PDF, Markdown, or DOCX (max 50 MB) |
| `collection_id` | string | Target collection UUID |

```bash
curl -X POST http://localhost:3000/api/documents/upload \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -F "file=@docs/spec.pdf" \
  -F "collection_id=550e8400-e29b-41d4-a716-446655440000"
```

**Response:** `201 Created`

```json
{
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "spec.pdf",
  "file_type": "pdf",
  "file_size": 204800,
  "chunks_indexed": 42
}
```

#### `GET /api/documents`

List all documents.

```bash
curl http://localhost:3000/api/documents \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

#### `DELETE /api/documents/{id}`

Soft delete a document and its chunks. The document row and chunks remain in SQLite with `is_active=0` but are excluded from queries. Chroma entries are also cleaned up.

```bash
curl -X DELETE http://localhost:3000/api/documents/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

#### `DELETE /api/documents/batch`

Delete multiple documents by their IDs in a single request. Uses the same soft-delete mechanism as single-document deletion. Chroma embeddings are cleaned up per-document.

**Request:**

```json
{
  "ids": ["550e8400-e29b-41d4-a716-446655440000", "660e8400-e29b-41d4-a716-446655440001"]
}
```

```bash
curl -X DELETE http://localhost:3000/api/documents/batch \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"ids":["550e8400-e29b-41d4-a716-446655440000","660e8400-e29b-41d4-a716-446655440001"]}'
```

**Response:** `200 OK`

```json
{
  "deleted_count": 2,
  "ids": ["550e8400-e29b-41d4-a716-446655440000", "660e8400-e29b-41d4-a716-446655440001"]
}
```

**Error responses:**

| Status | Error Type | Description |
|--------|-----------|-------------|
| 400 | `bad_request` | No document IDs provided |
| 404 | `not_found` | No matching documents found |

#### `POST /api/documents/reload/{id}`

Reload/re-index an existing document. Deactivates old chunks, parses the new file content, chunks it, and saves new active chunks. The document identity (UUID) remains the same.

**Request:** `multipart/form-data`

| Field | Type | Description |
|-------|------|-------------|
| `file` | File | New file content (PDF, Markdown, or DOCX, max 50 MB) |

```bash
curl -X POST http://localhost:3000/api/documents/reload/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -F "file=@updated-spec.pdf"
```

**Response:** `200 OK`

```json
{
  "document_id": "550e8400-e29b-41d4-a716-446655440000",
  "chunks_indexed": 15,
  "document_name": "updated-spec.pdf"
}
```

#### `POST /api/documents/upload-zip`

Upload a ZIP archive for batch processing (up to 10 files, max 50 MB).

**Request:** `multipart/form-data`

| Field | Type | Description |
|-------|------|-------------|
| `file` | File | ZIP archive (max 50 MB, max 10 files inside) |
| `collection_id` | string | Target collection UUID |

```bash
curl -X POST http://localhost:3000/api/documents/upload-zip \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -F "file=@docs.zip" \
  -F "collection_id=550e8400-e29b-41d4-a716-446655440000"
```

**Response:** `200 OK`

```json
{
  "total_files": 3,
  "processed": 2,
  "failed": 1,
  "items": [
    {
      "filename": "readme.md",
      "status": "success",
      "document_id": "660e8400-e29b-41d4-a716-446655440000",
      "error": null
    },
    {
      "filename": "notes.txt",
      "status": "skipped",
      "document_id": null,
      "error": "Unsupported file extension"
    },
    {
      "filename": "corrupt.docx",
      "status": "failed",
      "document_id": null,
      "error": "Parse error: ..."
    }
  ]
}
```

**Error responses:**

| Status | Error Type | Description |
|--------|-----------|-------------|
| 413 | `payload_too_large` | ZIP contains >10 files or exceeds 50 MB |

### Collections

#### `POST /api/collections`

Create a new collection.

```json
{
  "name": "my-docs",
  "description": "Technical documentation for Project X"
}
```

#### `GET /api/collections`

List all collections.

#### `GET /api/collections/{id}`

Get a single collection.

#### `DELETE /api/collections/{id}`

Delete a collection and all its documents.

#### `GET /api/collections/{id}/stats`

Get comprehensive statistics for a collection, including document/chunk counts by source, file size, and file type breakdown.

```bash
curl http://localhost:3000/api/collections/550e8400-e29b-41d4-a716-446655440000/stats \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Response:** `200 OK`

```json
{
  "total_documents": 10,
  "total_chunks": 156,
  "total_git_repos": 2,
  "upload_documents": 7,
  "git_documents": 3,
  "upload_chunks": 112,
  "git_chunks": 44,
  "total_file_size_bytes": 2048000,
  "document_types": {
    "markdown": 5,
    "pdf": 3,
    "docx": 2
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `total_documents` | number | Total active documents |
| `total_chunks` | number | Total active chunks |
| `total_git_repos` | number | Registered git repositories |
| `upload_documents` | number | Documents uploaded manually |
| `git_documents` | number | Documents synced from git |
| `upload_chunks` | number | Chunks from uploaded documents |
| `git_chunks` | number | Chunks from git-synced documents |
| `total_file_size_bytes` | number | Total file size in bytes |
| `document_types` | object | File type → count map |

#### `GET /api/collections/{id}/chunks`

Search chunks within a collection. Supports text search (PostgreSQL ILIKE) and semantic search (Chroma vector search).

```bash
curl "http://localhost:3000/api/collections/550e8400-e29b-41d4-a716-446655440000/chunks?q=rate+limit&search_type=text&source=upload&limit=20&offset=0" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Query Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `q` | string | — | Search query (text or semantic) |
| `search_type` | string | `text` | `text` (PG ILIKE) or `semantic` (Chroma) |
| `source` | string | — | Filter by source: `upload` or `git` |
| `limit` | number | `20` | Max results (text search) |
| `offset` | number | `0` | Pagination offset (text search) |
| `top_k` | number | `20` | Max results (semantic search) |

**Response:** `200 OK`

```json
[
  {
    "chunk_id": "660e8400-e29b-41d4-a716-446655440001",
    "document_id": "550e8400-e29b-41d4-a716-446655440000",
    "document_name": "config-guide.md",
    "chunk_index": 2,
    "text": "Rate limiting is configured via the RATE_LIMIT env var.",
    "source": "upload",
    "score": null,
    "file_path": null
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `chunk_id` | string | Chunk UUID |
| `document_id` | string | Parent document UUID |
| `document_name` | string | Document name |
| `chunk_index` | number | Chunk position within document |
| `text` | string | Chunk content |
| `source` | string | `upload` or `git` |
| `score` | number |null` | Relevance score (semantic only) |
| `file_path` | string |null` | Original repo path (git docs only) |

### Query

#### `POST /api/query`

Ask a question. The response streams via SSE.

```json
{
  "query": "How do I configure the rate limiter?",
  "collection_id": "550e8400-e29b-41d4-a716-446655440000",
  "session_id": "660e8400-e29b-41d4-a716-446655440000"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | Yes | User's question |
| `collection_id` | string | Yes | Collection to search |
| `session_id` | string | No | Existing session for conversation history |

**Response:** SSE stream of text chunks, followed by a final event with citations.

```
data: {"type": "chunk", "content": "The rate limiter "}
data: {"type": "chunk", "content": "is configured via "}
data: {"type": "done", "sources": [
  {"document": "config-guide.md", "chunk": 2, "score": 0.92}
]}
```

#### `GET /api/query/{session_id}/subscribe`

Recovery endpoint used after a page reload while a RAG pipeline is still running.
Emits exactly one SSE event when the active pipeline finishes (or immediately if already complete).

Requires the same authentication and session ownership rules as `GET /api/sessions/{id}`.

**Response:** SSE stream with a single event.

```
data: {"type":"done","data":{}}
```

```
data: {"type":"error","data":{"text":"LLM rate limit exceeded"}}
```

| Event type | When emitted |
|------------|-------------|
| `done` | Pipeline completed successfully (or was already complete when subscribed) |
| `error` | Pipeline failed with an error description |

**404 Not Found** — no active pipeline exists for this session and no completed assistant message was persisted. The frontend should stop recovery when it receives 404.

### Conversations

#### `GET /api/sessions`

List all chat sessions.

#### `POST /api/sessions`

Create a new session.

```json
{
  "title": "My Chat",
  "collection_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### `GET /api/sessions/{id}`

Get session details and messages.

#### `DELETE /api/sessions/{id}`

Delete a specific session.

#### `DELETE /api/sessions`

Delete all sessions.

#### `GET /api/sessions/{id}/export?format={json|markdown}`

Export session messages. Default format is `json`.

| Query Param | Values | Description |
|------------|--------|-------------|
| `format` | `json` or `markdown` | Export format (default: `json`) |

`format=markdown` returns `Content-Type: text/markdown` with:
- `# Session Title` as H1
- `## role · timestamp` headers per message
- `(edited)` suffix on edited messages
- Soft-deleted messages excluded

### Sessions (Admin)

#### `GET /api/admin/sessions`

Search all sessions with optional filters. Requires admin role.

```bash
curl "http://localhost:3000/api/admin/sessions?search=rate+limit&from=2026-01-01T00:00:00Z&to=2026-12-31T23:59:59Z&user_id=550e8400-e29b-41d4-a716-446655440000" \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Query Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| `search` | string | Search by session title or message content (ILIKE) |
| `from` | string (RFC 3339) | Filter sessions created on or after this date |
| `to` | string (RFC 3339) | Filter sessions created on or before this date |
| `user_id` | string | Filter sessions owned by a specific user |

**Response:** `200 OK` — array of [SessionSummary](#session-summary) objects.

### SSE Query Events

#### `pipeline_stage` event

When `ADVANCED_RAG_ENABLED=true` (default), the query pipeline emits per-stage progress events during processing:

```json
{
  "type": "pipeline_stage",
  "stage_name": "multi_query",
  "data": {
    "original_query": "How do I configure the rate limiter?",
    "variants": ["How to set up rate limiting?", "Rate limiter configuration steps", "Configure rate limit parameters"]
  }
}
```

Available stages: `multi_query`, `hyde`, `embedding_search`, `keyword_search`, `merge_dedup`, `reranking`, `final_answer`.

#### `subscribe` recovery event

When the page is reloaded during a pipeline run, the backend persists the
pipeline state to localStorage. On the next load, the frontend calls
`GET /api/query/{session_id}/subscribe` instead of polling. That endpoint
emits exactly one event when the pipeline finishes:

```json
{"type": "done", "data": {}}
```

or an error event on failure.

### Message Actions

#### `PATCH /api/sessions/{session_id}/messages/{message_id}`

Edit a message. Only **user** messages can be edited — editing assistant messages returns `422 Unprocessable Entity`.

```json
{
  "content": "Updated question text"
}
```

**Response:** `200 OK` with the updated message including `edited_at` and `original_content`.

| Error | Status | Description |
|-------|--------|-------------|
| 422 | `unprocessable_entity` | Attempting to edit an assistant message |
| 404 | `not_found` | Message not found or soft-deleted |

#### `DELETE /api/sessions/{session_id}/messages/{message_id}`

Soft-delete a message. The message is hidden from history and export but preserved in the database.

**Response:** `204 No Content`

### SSE Query `done` Event

The `done` SSE event includes message IDs for temp-ID reconciliation:

```json
{
  "type": "done",
  "user_message_id": "550e8400-e29b-41d4-a716-446655440000",
  "assistant_message_id": "660e8400-e29b-41d4-a716-446655440000"
}
```

#### Recovery route format

The same `done` event shape (with empty `data: {}`) is emitted by the recovery
endpoint `GET /api/query/{session_id}/subscribe` after the pipeline completes.

### Auth

#### `GET /api/auth/me`

Returns current user info from JWT claims.

```bash
curl http://localhost:3000/api/auth/me \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Response:**

```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "name": "John Doe",
  "email": "john@example.com",
  "preferred_username": "johndoe",
  "provider": "keycloak"
}
```

#### `POST /api/auth/logout`

Client-side logout acknowledgement. The frontend should discard its stored tokens and redirect to KeyCloak's `end_session_endpoint` for RP-initiated logout.

```bash
curl -X POST http://localhost:3000/api/auth/logout \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Response:**

```json
{
  "status": "ok",
  "message": "Logged out successfully. Remove the token on the client side."
}
```

### Git Sync

Sync documents from remote Git repositories. Each repository is cloned, parsed, chunked, and indexed into a collection.

#### `POST /api/git-sync/repos`

Register a new Git repository for document syncing.

```json
{
  "url": "https://github.com/example/docs.git",
  "branch": "main",
  "access_token": "ghp_xxxxxxxx",
  "collection_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | Repository URL (HTTPS or SSH format) |
| `branch` | string | No | Branch to sync (default: `main`) |
| `access_token` | string | No | Access token for private repos |
| `collection_id` | string | Yes | Target collection UUID |

**Response:** `201 Created`

```json
{
  "id": "660e8400-e29b-41d4-a716-446655440000",
  "url": "https://github.com/example/docs.git",
  "branch": "main",
  "local_path": "/data/git-repos/example-docs",
  "collection_id": "550e8400-e29b-41d4-a716-446655440000",
  "collection_name": "my-docs",
  "status": "idle",
  "created_at": "2026-06-18T12:00:00Z",
  "updated_at": "2026-06-18T12:00:00Z"
}
```

#### `GET /api/git-sync/repos`

List all registered Git repositories.

#### `GET /api/git-sync/repos/{id}`

Get a single repository details.

#### `POST /api/git-sync/repos/{id}/sync`

Trigger an immediate sync (clone or pull) for a registered repository.

```bash
curl -X POST http://localhost:3000/api/git-sync/repos/{id}/sync \
  -H "Authorization: Bearer $ACCESS_TOKEN"
```

**Response:**

```json
{
  "repo_id": "660e8400-e29b-41d4-a716-446655440000",
  "status": "syncing",
  "files_indexed": 12,
  "chunks_total": 156
}
```

#### `GET /api/git-sync/repos/{id}/status`

Get the sync status of a repository.

#### `DELETE /api/git-sync/repos/{id}`

Delete a registered repository and its local clone.

---

**Error responses:**

| Status | Error Type | Description |
|--------|-----------|-------------|
| 400 | `bad_request` | Invalid URL format (must start with `https://` or `git@`) |

## OpenAPI Specification

A machine-readable [OpenAPI 3.1 specification](openapi.yaml) is also available.

## See Also

- [Configuration](configuration.md) — environment variables and API keys
- [Architecture](architecture.md) — data flow and service interaction
- [Deployment](deployment.md) — production configuration
