[← User Interface Guide](gui.md) · [Back to README](../README.md) · [Auth →](auth.md)

# API Reference

## Base URL

| Environment | URL |
|-------------|-----|
| Development | `http://localhost:3000` |
| Production | `https://your-domain.com/api` |

## Authentication

All API routes (except `/health`) require a Bearer token:

```
Authorization: Bearer <ADMIN_API_KEY>
```

Requests without a valid token return `401 Unauthorized`.

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
| 401 | `unauthorized` | Missing or invalid API key |
| 404 | `not_found` | Resource not found |
| 415 | `file_error` | Unsupported or corrupt file |
| 413 | `payload_too_large` | ZIP exceeds 10-file limit or body > 50 MB |
| 429 | `rate_limited` | Too many requests |
| 502 | `embedding_error` | Embedding service unavailable |
| 502 | `chroma_error` | Chroma unavailable |
| 502 | `llm_error` | OpenRouter error |

## Endpoints

### Health

#### `GET /health`

Liveness probe — does not require authentication.

```bash
curl http://localhost:3000/health
# → OK
```

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
  -H "Authorization: Bearer $ADMIN_API_KEY" \
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
  -H "Authorization: Bearer $ADMIN_API_KEY"
```

#### `DELETE /api/documents/{id}`

Delete a document and its chunks.

```bash
curl -X DELETE http://localhost:3000/api/documents/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer $ADMIN_API_KEY"
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
  -H "Authorization: Bearer $ADMIN_API_KEY" \
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

#### `GET /api/sessions/{id}/export`

Export session messages as JSON.

## See Also

- [Configuration](configuration.md) — environment variables and API keys
- [Architecture](architecture.md) — data flow and service interaction
- [Deployment](deployment.md) — production configuration
