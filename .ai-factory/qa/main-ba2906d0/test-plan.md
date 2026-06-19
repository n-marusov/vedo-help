# Test Plan: Document Upload (UTF-8 Chunking Fix)

> Plan for verifying the UTF-8 chunking boundary fix and the full document upload flow through the admin panel.

## In Scope

- Text chunking with multi-byte UTF-8 characters (Cyrillic, Chinese, emoji, accented Latin)
- Single-file document upload via admin panel (`POST /api/documents/upload`)
- ZIP batch upload (`POST /api/documents/upload-zip`)
- Document listing after upload (`GET /api/documents`)
- Git sync indexing path (also uses `chunk_document`)
- Frontend upload UI: drop zone, progress bar, document list refresh

## Out of Scope

- Query/chat functionality (covered by separate E2E tests, TC-RAG-003/004)
- Authentication flows (covered by auth regression tests)
- Collection CRUD (covered by separate E2E tests)
- Non-document-related admin panel features

## Test Types

| Type | Description | Priority |
|------|-------------|----------|
| Functional | Verify document upload succeeds for various file types and encodings | High |
| Edge Case | Multi-byte UTF-8 files, very large files, empty files | High |
| Negative | Invalid file types, no collection selected, upload without auth | Medium |
| Regression | ASCII-only documents still work, no data loss, existing E2E tests pass | High |
| Integration | Backend crash recovery — verify the service recovers after a failed request | Medium |

## Verification Checklist

- [ ] **High** — Upload a Markdown file with Russian Cyrillic text → success, no crash
- [ ] **High** — Upload a PDF file with mixed ASCII/UTF-8 content → success
- [ ] **High** — Upload a plain ASCII text file → success (regression check)
- [ ] **High** — Verify document appears in the document list after upload
- [ ] **High** — All 5 chunking unit tests pass (`cargo test chunking`)
- [ ] **High** — All RAG Flow E2E tests pass (15 tests)
- [ ] **Medium** — Upload a ZIP archive containing mixed-file-type documents
- [ ] **Medium** — Upload an empty file → proper error, no crash
- [ ] **Medium** — Upload a file to a non-existent collection → proper error
- [ ] **Medium** — Upload without auth token → 401 response
- [ ] **Low** — Upload a very large file (>10 MB) → proper handling
- [ ] **Low** — Upload a file with emoji in content → no crash
- [ ] **Low** — Upload an unsupported file type (.exe, .sh) → rejected with validation error

## Environment

- Docker Compose with all 6 services running (chroma, embedding, backend, frontend, keycloak, keycloak-db)
- Jest/Vitest for unit tests, Playwright for E2E tests
- Frontend dev server on `localhost:5173`, backend on `localhost:3000`
