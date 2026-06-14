# Roadmap: VEDO hub RAG Assistant

> Отслеживание состояния реализации плана `.ai-factory/plans/feature-rag-assistant-full.md`.
> `[ ]` — не начато / `[~]` — в работе / `[x]` — завершено

---

## Milestone: v0.1 — MVP (базовый RAG pipeline)

Полный цикл: загрузка документа → индексация → вопрос-ответ с цитированием.

### Phase 1: Project Scaffolding & Shared Infrastructure

- [x] Task 1.1 — Backend Rust project skeleton
- [x] Task 1.2 — Backend shared module: error, auth, types
- [x] Task 1.3 — Backend shared: LLM client, chunking, file validation
- [x] Task 1.4 — Python embedding service skeleton
- [x] Task 1.5 — Vue 3 frontend skeleton
- [x] Task 1.6 — CI pipeline (GitHub Actions) — multi-service
- [x] Task 1.7 — Developer tooling: Makefile, rust-toolchain, .gitignore, .editorconfig

### Phase 2: Backend Core — Document Management & Embedding

- [x] Task 2.1 — Documents module: data layer
- [x] Task 2.2 — Documents module: parsing and chunking pipeline
- [x] Task 2.3 — Embedding service: wire up with backend

### Phase 3: Query & RAG Pipeline

- [x] Task 3.1 — Query module: RAG pipeline
- [x] Task 3.2 — SSE streaming implementation

### Phase 4: Collections & Conversation Management

- [x] Task 4.1 — Collections module
- [x] Task 4.2 — Conversations module

### Phase 5: Frontend — Complete SPA

- [x] Task 5.1 — Frontend API layer and stores
- [x] Task 5.2 — Frontend: Chat interface
- [x] Task 5.3 — Frontend: Document & collection management

### Phase 6: Integration, Deployment & Testing

- [x] Task 6.1 — Backend router wiring
- [x] Task 6.2 — Integration tests
- [x] Task 6.3 — Docker Compose and deployment config
- [x] Task 6.4 — Backup & restore scripts
- [x] Task 6.5 — Documentation checkpoint

---

## Summary

| Milestone | Status | Tasks total | Tasks done |
|-----------|--------|-------------|------------|
| v0.1 — MVP | Complete | 20 | 20 |

**Start date:** 2026-06-14
**Target:** Full implementation of RAG Q&A system with Rust backend, Python embedding, Chroma vector DB, Vue 3 frontend, Docker Compose deployment.
