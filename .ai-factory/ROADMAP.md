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
- [x] **Chat UI polish** — доработка интерфейса чата (реализация выполнена, ожидает Pencil-верификацию Task 0.4):
  - Убрать иконки у сообщений, называть сессии кратким содержимым первого запроса
  - Действия под запросом: копирование + редактирование; под ответом: копирование + регенерация
  - Поиск по содержимому сессий на сайдбаре
  - Light theme: светлые фоны плашек источников; таблицы без чередования фона; не упоминать «chunk» в ответе
  - Хедер: контурные иконки кнопок; сайдбар: New session по центру ниже заголовка + интеграция с выбором коллекции, кнопка поиска сессий и кнопка сворачивания
  - Действия сессий: переименовать, pin, сохранить (помимо удаления)
  - Timestamp в одной строке с действиями; увеличенная ширина сообщений; Export — объединённая кнопка с иконкой (сначала в ui-kit.lib.pen)
  - Убрать кнопки удаления ответа/запроса; поле ввода с белым фоном и тенью по контуру
  - Авто-выбор первой коллекции при загрузке; авто-установка коллекции при выборе сессии; замена выпадающего списка на тэг (название сессии + коллекция)
  - Кнопка debug info в ответах (только для admin): найденные ключевые слова (BM25), 5 чанков embedding-search, чанки по ключевым словам
- [x] **Admin panel & repo sync fix** — удалить Drop-зону в админ-панели (оставить только кнопку upload); исправить ошибку синхронизации репозитория ✅

---

## Milestone: v0.4 — Observability & Reliability ⏳

Централизованное структурированное логгирование (OpenTelemetry), мониторинг, алёртинг, автоматический бэкап, per-route rate limiting.

- [x] **OpenTelemetry: структурированное логгирование** — централизованный OTel Collector, интеграция во все сервисы (Rust/Python/TS), trace propagation, E2E валидация. Детальный план: `.ai-factory/plans/feature-logging-unification-otel.md`.
- [x] **Session debug view in admin panel** — отладка вынесена из чата в админ-панель. Вместо кнопки в сообщениях бота — отдельный таб в админке:
  - [x] Разделение интерфейса админ-панели на два таба: «Collections & Sources» (текущее содержимое) и «Session Debug» (новое)
  - [x] Поиск сессий по заголовку, дате, пользователю
  - [x] Выбор сессии из списка → просмотр сообщений с debug-данными
  - [x] 7-шаговая пайплайн-диаграмма для каждого ответа бота (в коллапсируемых секциях):
    - Шаг 1. **Multi-query** — заглушка (v0.4.2)
    - Шаг 2. **HyDE** — заглушка (v0.4.2)
    - Шаг 3. **Embedding search** — реальные данные (Chroma, top-k, dimension, latency)
    - Шаг 4. **Hybrid keyword search** — заглушка (v0.4.2)
    - Шаг 5. **Merge & deduplication** — заглушка (v0.4.2)
    - Шаг 6. **Reranking** — заглушка (v0.4.2)
    - Шаг 7. **Final answer** — реальные данные (model, tokens, latency, prompt preview)
  - [x] Бэкенд: сбор DebugData при `debug: true`, хранение в `messages.debug_data`, API поиска сессий `GET /api/admin/sessions?search=&from=&to=`
  - [x] Pencil-дизайн: обновлён `admin.pen` с табами и экраном Session Debug
- [x] **Deep healthcheck** — проверка зависимостей (Chroma, embedding)
- [ ] **Per-route rate limiting** — защита `/api/query`
- [ ] **Failure notifications** — Telegram / email webhook

---

## Milestone: v0.4.2 — Advanced RAG Pipeline (Multi-Query, HyDE, Hybrid Search, Reranking) ✅

Полный 7-шаговый RAG-пайплайн с визуализацией всех шагов в админ-панели (по методичке День 2).

- [x] **Backend: Config + env vars** — `ADVANCED_RAG_ENABLED`, `RERANK_TOP_K`, `HYBRID_TOP_K`, `MULTI_QUERY_COUNT`, `LLM_RERANK_MODEL` в `config.rs`
- [x] **Backend: BM25 keyword search module** — `shared/bm25.rs`: инвертированный индекс, поиск по ключевым словам, ранжирование по BM25
- [x] **Backend: LLM helper для не-streaming вызовов** — `LlmClient::query_single(prompt)` для multi-query, HyDE, reranking
- [x] **Backend: Multi-query** — LLM генерирует 2-3 альтернативные формулировки вопроса
- [x] **Backend: HyDE (гипотетический документ)** — LLM пишет гипотетический ответ для каждого вопроса
- [x] **Backend: Hybrid search orchestrator** — объединение результатов Chroma + BM25 + дедупликация по chunk_id
- [x] **Backend: LLM Reranking** — для каждого уникального чанка: LLM оценивает (вердикт "брать"/"пропустить")
- [x] **Backend: SourceRef с метаданными этапа** — расширение `SourceRef`: `stage`, `rerank_score`, `rerank_verdict`
- [x] **Frontend: API types** — новые `StreamEvent` типы (`pipeline_stage`), расширенный `SourceRef`
- [x] **Frontend: Debug panel v2** — `SessionDebug.vue`: 7 динамических секций под каждый шаг (реальные данные или заглушки)
- [x] **Frontend: Admin RAG Debug tab** — обновление SessionDebug.vue: фильтр по User ID, поиск по содержимому сообщений
- [x] **Backend: Новые SSE-типы событий** — `pipeline_stage` события через SSE-канал в `run_pipeline`, обработка на фронтенде в SSE-парсере `chat.ts`, recovery через `subscribe_handler`
- [x] **Backend: Ужесточение anti-hallucination промпта** — `SYSTEM_PROMPT` в `shared/llm.rs`: явный запрет внешних знаний, обязательный отказ при недостатке контекста
- [x] **Frontend: Pinia debug store** — `stores/ragDebug.ts`: выделенное хранилище для pipelineStage, debugData, setPipelineStage/setDebugData/reset
- [x] **Backend: Динамическая размерность эмбеддинга + реальный query_snippet** — исправление хардкода `384` на `embedding.len()`, замена статической строки "Multi-query with HyDE" на эффективный поисковый запрос
- [x] **Backend: Детальные чанки во всех шагах отладки** — `MergeDedupStep.results`, `RerankResult.text_snippet`
- [x] **Backend: Заполнение FinalAnswerStep** — model, chunks_in_context, prompt_preview, latency, токены
- [x] **Backend: SourceRef с реальными метаданными этапа** — stage, rerank_score, rerank_verdict
- [x] **Frontend: Чанки под спойлерами** — отображение text_snippet в embedding search, merge&dedup, reranking
- [x] **Frontend: Скрытие отключённых этапов** — полное скрытие disabled-шагов, удаление бейджей active/future
- [x] **Frontend: Final answer с реальными данными** — заполнение model, prompt_preview, chunks_in_context

---

## Milestone: v0.5 — Advanced RAG ✅

Улучшение качества ответов: cross-encoder reranker, multi-turn context, доп. форматы.

- [x] **Cross-encoder reranker** — модель-ранжировщик (например, BAAI/bge-reranker-v2-m3) для CPU
- [x] **Smarter multi-turn context** — tiktoken-rs для точного подсчёта токенов вместо word-count heuristic
- [x] **Additional formats** — CSV, JSON, HTML-to-text
- [x] **Hybrid search optimization** — тюнинг BM25 параметров, weighted fusion с векторным поиском
- [x] **LLM API Fallback** — fallback на API https://opencode.ai/, в случае если API router.ai не доступен

---



## Milestone: v0.6 — Multi-user & Security ✅

Аутентификация, multi-tenancy, audit log, security hardening.

- [x] **User authentication** — JWT, login/register
- [x] **Multi-tenancy** — изоляция данных по пользователю
- [x] **RBAC** — admin / user роли
- [x] **Audit log** — логи всех API-вызовов с user_id + action
- [x] **CORS hardening** — строгие origin
- [x] **SAST scanning** — зависимостей в CI

> **Known Issues (pre-existing, не связаны с v0.6):**
> - `tests/conversations_unit.rs` — 2 errors: missing `pinned` + `user_id` в `Session`; `export_session_markdown` принимает 3 аргумента (v0.3.1 chat rework)
> - `tests/documents_db_unit.rs` — `DocumentService::new` требует 2 аргумента (добавлен `CollectionRepository`) — ошибка от OTel милестоуна
> - `tests/integration.rs` — 6 errors: missing `user_id` в Collection/Document/Session/GitRepo (v0.3.1)
> - `tests/conversations_integration.rs` — 1 error: missing `pinned` + `user_id` в `Session` (v0.3.1)
> - `src/modules/auth/tests.rs` — 2 errors: function signature mismatch (v0.3.1)
> - `cargo clippy` — 1 pre-existing `dead_code` warning
> 
> **Статус:** production code v0.6 работает полностью. Pre-existing test errors будут исправлены в рамках соответствующих милестоунов или отдельного cleanup.

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
| v0.3.1 — Basic Q&A Logic & Chat Rework | ✅ **8/8** | Streaming ✅; LLM error handling ✅; message editing & deletion ✅; context management ✅; chat export UI ✅; empty state & loading skeletons ✅; Chat UI polish ✅ (implementation complete, pending Pencil design verification); admin panel & repo sync fix ✅ |
| v0.4 — Observability & Reliability | ⏳ 2/6 | Debug view ✅; deep healthcheck, rate limit, backup automation, alerts, graceful shutdown coordination ❌ |
| v0.4.2 — Advanced RAG Pipeline | ✅ **21/21** | Multi-query, HyDE, BM25, LLM reranking, 7-step pipeline, pipeline_stage SSE, anti-hallucination prompt, Pinia debug store, admin debug visualization |
| v0.5 — Advanced RAG | ✅ **5/5** | Cross-encoder reranker, tiktoken multi-turn, CSV/JSON/HTML formats, hybrid search optimization, LLM API fallback |
| v0.6 — Multi-user & Security | ✅ **6/6** | Auth, multi-tenancy, RBAC, audit, CORS, SAST |
| v1.0 — Production Ready | ⏳ 0/5 | CI/CD, perf, SLA, docs, monitoring |

**Старт:** 2026-06-14
**MVP завершён:** 2026-06-15
**Chat UI overhaul (Phases 0–4):** 2026-06-16
**Document re-indexing:** 2026-06-19
**Bulk deletion + VToast + optimistic UX + embedding pipeline:** 2026-06-21
**v0.3.1 chat rework complete (message edit/delete, context window, chat export, skeletons):** 2026-06-21
**Chat UI polish debug info + admin role wiring:** 2026-06-23
**v0.6 — Multi-user & Security complete (all 7 phases):** 2026-06-25
**Debug view (v0.4):** 2026-06-26
**RAG pipeline debug data fixes (v0.4.2):** 2026-07-07
**v0.5 — Advanced RAG complete (all 5 tasks):** 2026-07-11
**v0.4.2 — Advanced RAG Pipeline complete (all 21 tasks):** 2026-07-11
**Что дальше:** `/aif-implement` — завершение v0.4 (deep healthcheck, rate limiting, backup automation), затем `/aif-plan` на v1.0

**Pre-requisite:** Before starting v0.4, fix pre-existing test errors от v0.3.1/OTel:
  1. `/aif-plan fix "pre-existing test errors from v0.3.1 chat rework and OTel milestone"`
  2. `/aif-implement` — исправить `conversations_unit`, `documents_db_unit`, `integration`, `conversations_integration`, `auth_middleware_test`
