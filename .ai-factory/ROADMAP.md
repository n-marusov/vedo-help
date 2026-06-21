# Roadmap: VEDO hub RAG Assistant

> Карта развития продукта от MVP к промышленной эксплуатации.
> `[x]` — завершено / `[~]` — частично / `[ ]` — не начато

---

## Milestone: v0.1 — MVP ✅

Полный цикл: загрузка документа → индексация → вопрос-ответ с цитированием.

- [x] **Phase 1** — Project scaffolding & shared infrastructure (7 задач)
- [x] **Phase 2** — Backend core: document management & embedding (3 задачи)
- [x] **Phase 3** — Query & RAG pipeline (2 задачи)
- [x] **Phase 4** — Collections & conversation management (2 задачи)
- [x] **Phase 5** — Frontend: complete SPA (3 задачи)
- [x] **Phase 6** — Integration, deployment & testing (3 задачи)

---

## Milestone: v0.2 — GUI Redesign (DeepSeek-стиль) ✅

Полный редизайн интерфейса в стиле https://chat.deepseek.com/ — минималистичный чат, тёмная/светлая тема, сайдбар сессий, качественный рендеринг Markdown и кода.

- [x] **Chat UI overhaul (Phases 0–4)** — дизайн-токены (`chat-tokens.css`), компонент `UserAvatar` (SVG, 3 размера), переработка `MessageBubble` (ролевые лейблы, typing indicator, источники), `ChatWindow` (хедер с селектором коллекций, +New), адаптивная вёрстка (768px/480px), E2E-тесты Playwright (42 теста, 5 spec-файлов), юнит-тесты (26)
- [x] **Dark/light theme** — переключаемая тема с сохранением в localStorage, CSS-переменные для всей палитры
- [x] **Session sidebar** — редизайн сайдбара сессий в стиле Pencil (312px, card bg, radius-xl)
- [x] **Admin panel redesign** — страница управления приведена к единому стилю (admin.pen, dialogs.pen)
- [x] **UI component refactor** — вынос атомарных компонентов в `src/components/ui/`, единая типографика IBM Plex Mono
- [x] **Login page** — страница входа с OAuth-провайдерами (login.pen)
- [x] **Auth documentation** — docs/auth.md with KeyCloak setup, social providers, OAuth flow, troubleshooting

---

## Milestone: v0.2.1 — Markdown & Code Rendering ✅

Полноценный рендеринг Markdown и подсветка синтаксиса в сообщениях чата.

- [x] **Markdown & code rendering** — полноценный рендеринг Markdown (remark/rehype), подсветка синтаксиса (shiki/prism), кнопка копирования кода

---

## Milestone: v0.3 — Admin Panel & Production Polish ⏳

Закрытие пробелов MVP: управление коллекциями и загрузка документов через админ-панель, E2E-тесты, ZIP-загрузка, re-indexing, confidence score, graceful degradation.

- [x] **E2E tests** — Playwright: upload → query → sources, запуск в CI
- [x] **Chroma integration tests** — убрать `--ignored`, развернуть Chroma в CI
- [x] **Collection management in admin panel** — UI для CRUD коллекций в админ-панели (создание, удаление, переименование, список)
- [x] **Document upload through admin panel** — интерфейс загрузки документов с дроп-зоной, прогресс-баром, валидацией
- [x] **ZIP batch upload** — до 10 файлов, HTTP 413 при превышении, batch-эндпоинт `/api/documents/upload-zip`
- [x] **Git repository sync** — подключение Git-репозитория (GitHub/GitLab/Bitbucket): клонирование/пулл, парсинг Markdown-документов из репозитория, индексация в Chroma, webhook-уведомления при обновлении
  - ✅ **Бэкенд (Phase 1–4, Tasks 1–10):** полный API (6 эндпоинтов), GitSyncService (clone/pull → parse → chunk → embed → index), E2E-тесты (9), unit-тесты (14), интеграционные тесты (8), миграция БД
  - ✅ **Webhook + polling (Phase 5, Tasks 11–12):** POST /api/git-sync/webhook (HMAC-валидация для GitHub/GitLab), периодический поллинг (tokio::interval, экспоненциальный backoff, graceful shutdown через broadcast)
  - ✅ **Фронтенд (Phase 6, Tasks 13–14):** реализовано
  - ✅ **E2E (Phase 7):** E2E-тесты для Git Sync (9 тестов), фикс Vite-ошибки (v-model), замена selectOption на клики VSelect
- [x] **Remove ADMIN_API_KEY** — выпилить legacy API key аутентификацию, оставить только KeyCloak JWT
- [x] **Document re-indexing** — деактивация старых чанков при перезагрузке
- [x] **Confidence indicator** — relevance score в UI (sources)
- [x] **Bulk document deletion from collection** — чекбокс у каждого документа, чекбокс «toggle all», кнопка удалить выбранные; optimistic UI с rollback; VToast-уведомления. Бэкенд: `DELETE /api/documents/batch` с мягким удалением и очисткой Chroma. Unit-тесты (4) + тесты бэкенда (2)
- [x] **Delete result feedback via VToast** — компонент `VToast` с типами info/success/error/warning, border-left по типу, auto-dismiss. Интегрирован в DocumentList для результатов массового удаления
- [x] **Optimistic deletion UX** — мгновенное подтверждение (remove из массива + show toast success), асинхронный `DELETE /api/documents/batch` на бэкенде, при провале — откат (вернуть документы в список + toast error), блокировка повторного нажатия через `isDeleting`
- [x] **Embedding submission in upload pipeline** — `process_upload` и `process_zip_upload` вызывают `index_chunks_in_chroma` (embed + add_embeddings). При ошибке эмбеддинга — откат: деактивация документа и чанков (soft delete)
- [~] **Graceful degradation** — retry-логика (3 попытки, 1s delay) для LLM ✅; кэширование эмбеддингов ✅; fallback LLM намеренно убран из spec (Retry primary; fail if down) — не планируется; кэширование ответов на частые вопросы — не реализовано

---

## Milestone: v0.3.1 — Basic Q&A Logic & Chat Rework ✅

Реализация базовой логики ответов и доработка чата: улучшение потокового вывода, обработка ошибок LLM, сохранение контекста, UI для редактирования/удаления сообщений.

- [x] **Streaming response improvements** — SSE с chunk/sources/done событиями, NDJSON-парсинг на фронтенде
- [x] **LLM error handling** — retry (3 попытки), 60s timeout, error-события SSE, отображение ошибок в UI
- [x] **Message editing & deletion** — UI для редактирования и удаления сообщений в чате
- [x] **Context management** — sliding window + token budget (сейчас передаётся вся история)
- [x] **Chat export** — экспорт истории чата (JSON/Markdown, backend API готов, нет кнопки в UI)
- [x] **Empty state & loading skeletons** — базовые empty state есть, нет анимированных скелетонов

---

## Milestone: v0.4 — Observability & Reliability ⏳

Мониторинг, алёртинг, автоматический бэкап, per-route rate limiting.

- [ ] **Deep healthcheck** — проверка зависимостей (Chroma, embedding)
- [ ] **Per-route rate limiting** — защита `/api/query`
- [ ] **Automated backup** — cron-контейнер или host cron для SQLite + Chroma (скрипты backup.sh/restore.sh есть, автоматизации нет)
- [ ] **Failure notifications** — Telegram / email webhook
- [ ] **Graceful shutdown** — корректное завершение всех контейнеров (бэкенд частично: SIGINT/SIGTERM через broadcast, контейнерная координация не реализована)

---

## Milestone: v0.5 — Advanced RAG ⏳

Улучшение качества ответов: hybrid search, reranker, query expansion.

- [ ] **Hybrid search** — vector + BM25/FTS fusion
- [ ] **Cross-encoder reranker** — переранжирование top-k результатов
- [ ] **Query expansion** — альтернативные формулировки через LLM
- [ ] **Smarter multi-turn context** — sliding window
- [ ] **Additional formats** — CSV, JSON, HTML-to-text

---

## Milestone: v0.6 — Multi-user & Security ⏳

Аутентификация, multi-tenancy, audit log, security hardening.

- [ ] **User authentication** — JWT, login/register
- [ ] **Multi-tenancy** — изоляция данных по пользователю
- [ ] **RBAC** — admin / user роли
- [ ] **Audit log** — логи всех API-вызовов с user_id + action
- [ ] **CORS hardening** — строгие origin
- [ ] **SAST scanning** — зависимостей в CI

---

## Milestone: v1.0 — Production Ready ⏳

CI/CD, performance testing, SLA, документация, мониторинг.

- [ ] **CI/CD** — авто-деплой на VPS при push в main
- [ ] **Load testing** — k6/locust, target P99 < 10s per query
- [ ] **SLA + auto-recovery** — документированные процедуры
- [ ] **Documentation** — GUI guide, runbook, C4 diagrams
- [ ] **Monitoring dashboard** — cAdvisor + Prometheus или эквивалент

---

## Summary

| Milestone | Status | Фокус |
|-----------|--------|-------|
| v0.1 — MVP | ✅ 20/20 | Full RAG pipeline |
| v0.2 — GUI Redesign | ✅ **6/6** | DeepSeek-style chat UI, UI atoms, session sidebar, admin redesign, login page, dark/light theme |
| v0.2.1 — Markdown & Code Rendering | ✅ **1/1** | Markdown rendering, syntax highlighting, copy button |
| v0.3 — Admin Panel & Production Polish | ⏳ **13/14** | Collection & document management, confidence indicator, ZIP upload ✅; Git sync ✅; ADMIN_API_KEY removed ✅; document re-indexing ✅; bulk deletion ✅; VToast feedback ✅; optimistic UX ✅; embedding submission ✅; graceful degradation ~ (retry + embedding cache ✅, fallback LLM out of scope, response caching ❌) |
| v0.3.1 — Basic Q&A Logic & Chat Rework | ✅ **6/6** | Streaming ✅; LLM error handling ✅; message editing & deletion ✅; context management ✅; chat export UI ✅; empty state & loading skeletons ✅ |
| v0.4 — Observability & Reliability | ⏳ 0/5 | Healthcheck, rate limit, backup automation, alerts, graceful shutdown coordination |
| v0.5 — Advanced RAG | ⏳ 0/5 | Hybrid search, reranker, query expansion, multi-turn, formats |
| v0.6 — Multi-user & Security | ⏳ 0/6 | Auth, multi-tenancy, RBAC, audit, CORS, SAST |
| v1.0 — Production Ready | ⏳ 0/5 | CI/CD, perf, SLA, docs, monitoring |

**Старт:** 2026-06-14
**MVP завершён:** 2026-06-15
**Chat UI overhaul (Phases 0–4):** 2026-06-16
**Document re-indexing:** 2026-06-19
**Bulk deletion + VToast + optimistic UX + embedding pipeline:** 2026-06-21
**v0.3.1 chat rework complete (message edit/delete, context window, chat export, skeletons):** 2026-06-21
**Что дальше:** `/aif-implement` — завершение v0.4 (deep healthcheck, rate limiting, backup automation)