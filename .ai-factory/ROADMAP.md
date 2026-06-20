# Roadmap: VEDO hub RAG Assistant

> Карта развития продукта от MVP к промышленной эксплуатации.
> `[x]` — завершено / `[ ]` — не начато

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

## Milestone: v0.3 — Admin Panel & Production Polish ✅

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
- [x] **Bulk document deletion from collection** — чекбокс у каждого документа, toggle all, подтверждение через VDialog, вызов `DELETE /api/documents/batch` с массивом ID
- [x] **Delete result feedback via VToast** — обратная связь через `VToast` (success/error): "Deleted N document(s)" или сообщение об ошибке
- [x] **Optimistic deletion UX** — Selection через `Set<string>`, toggle/toggleAll, toast-уведомление после批量 deletion, откат при ошибке
- [x] **Embedding submission in upload pipeline** — `process_upload` и `process_zip_upload` теперь вызывают `index_chunks_in_chroma()`: эмбеддинг через `EmbeddingClient` + `chroma_client.add_embeddings()`, при ошибке — деактивация документа и чанков (откат)

---

## Milestone: v0.3.1 — Migration to PostgreSQL ✅

Перевод всей инфраструктуры на единую СУБД PostgreSQL: замена SQLite в backend на Postgres, объединение контейнера БД (keycloak-db → db), выделение отдельных баз для каждого сервиса.

- [x] **PostgreSQL container rename** — переименовать `keycloak-db` в `db` в `docker-compose.yml`, `docker-compose.override.yml`, `docker-compose.production.yml`; том `keycloak_db_data` → `db_data`; обновить ссылку `KC_DB_URL: jdbc:postgresql://db:5432/keycloak`
- [x] **Add backend database** — создать отдельную базу `vedo` в контейнере `db` (init-скрипт `scripts/init-db.sh`); добавить переменную `DATABASE_URL=postgres://vedo:password@db:5432/vedo` в `.env.example` и `docker-compose.yml`
- [x] **sqlx feature switch** — заменить `features = ["sqlite"]` на `["postgres", "migrate"]` в `backend/Cargo.toml`; убрать `sqlite` из фичей; обновить dev-зависимости для тестов
- [x] **Repository layer refactor** — заменить `SqlitePool` → `PgPool`, `sqlx::sqlite::SqlitePoolOptions` → `sqlx::postgres::PgPoolOptions`, `QueryBuilder<Sqlite>` → `QueryBuilder<Postgres>` во всех 5 модулях (collections, conversations, documents, git_sync, query) и `main.rs`
- [x] **Schema migration** — переписать `run_migrations()` с SQLite-специфики на PostgreSQL-совместимый DDL (убрать `INTEGER NOT NULL DEFAULT 1` для булевых → `BOOLEAN NOT NULL DEFAULT TRUE`, `TEXT` → `VARCHAR`/`TEXT`, `UUID PRIMARY KEY`); заменить ручные миграции на `sqlx migrate!` с timestamped `.sql` файлами
- [x] **Config & pool init** — убрать SQLite-специфику из `AppConfig::from_env()` и `main.rs` (создание директории для DB-файла, `sqlite:` префикс); подключение к PostgreSQL через `PgPoolOptions`, healthcheck-цикл ожидания готовности БД
- [x] **Test migration** — перевести все unit/integration тесты с `:memory:` SQLite на PostgreSQL test pool с `sqlx::migrate!`; обновить `setup_test_db()` / `setup_git_test_db()` во всех модулях; `$?` → `$N` параметры
- [x] **Backup script update** — обновить `scripts/backup.sh` и `scripts/restore.sh`: `pg_dump`/`pg_restore` вместо SQLite file copy; бэкап двух баз (vedo, keycloak) из контейнера `db`
- [x] **CI update** — обновить `.github/workflows/ci.yml`: добавить PostgreSQL service container для интеграционных тестов
- [x] **Documentation update** — обновить docs/, AGENTS.md, ARCHITECTURE.md, DESCRIPTION.md: все ссылки SQLite → PostgreSQL

---

## Milestone: v0.3.2 — Security Hardening ⏳

Критические и высокоприоритетные исправления безопасности, выявленные аудитом. Блокируют продакшен-деплой.

- [ ] **XSS sanitization for LLM output** — добавить `DOMPurify` или `rehype-sanitize` в Markdown-рендерер (`frontend/src/utils/markdown.ts`); сейчас `v-html` рендерит LLM-ответ без HTML-санитизации (stored XSS через prompt injection)
- [ ] **Per-route rate limiting** — реализовать rate limit для `/api/query` и `/api/documents/upload`; сейчас только body-size limit, нет per-route лимитов (подвержены абузу)
- [ ] **SAST in CI** — добавить `cargo audit` и `npm audit` в `.github/workflows/ci.yml`; нет проверки зависимостей на известные уязвимости
- [ ] **CORS hardening** — заменить `CorsLayer::permissive()` на конкретные origin из `.env` / `FRONTEND_URL` для продакшена
- [ ] **Security headers** — добавить `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Strict-Transport-Security` (через Caddy или axum middleware)
- [ ] **Error message sanitization** — не отправлять SQL/DB детали в `AppError::InternalError` на клиент; возвращать generic `INTERNAL_ERROR` в продакшене, полные ошибки — только в логах
- [ ] **Deep healthcheck endpoint** — `GET /api/health` проверяет `db`, `chroma`, `embedding` вместо возврата `"OK"`

---

## Milestone: v0.3.3 — Basic Q&A Logic & Chat Rework ⏳

Реализация базовой логики ответов и доработка чата: улучшение потокового вывода, обработка ошибок LLM, сохранение контекста, UI для редактирования/удаления сообщений.

- [x] **Streaming response improvements** — SSE с chunk/sources/done событиями, NDJSON-парсинг на фронтенде
- [x] **LLM error handling** — retry (3 попытки), 60s timeout, error-события SSE, отображение ошибок в UI
- [ ] **Message editing & deletion** — UI для редактирования и удаления сообщений в чате
- [ ] **Context management** — sliding window + token budget (сейчас передаётся вся история)
- [ ] **Chat export** — экспорт истории чата (JSON/Markdown), backend API готов (`GET /api/sessions/:id/export`), нет кнопки в UI
- [ ] **Empty state & loading skeletons** — базовые empty state есть, нет анимированных скелетонов

---

## Milestone: v0.4 — Observability & Reliability ⏳

Мониторинг, алёртинг, автоматический бэкап.

- [ ] **Failure notifications** — Telegram / email webhook — не реализовано
- [x] **Automated backup** — скрипты `scripts/backup.sh` и `scripts/restore.sh` (pg_dump + Chroma, с прунингом старых бэкапов)
- [x] **Graceful shutdown** — корректное завершение: `broadcast::channel` для сигнализации фоновым задачам (git-sync scheduler), `with_graceful_shutdown` для axum, обработка SIGINT/SIGTERM

---

## Milestone: v0.5 — Advanced RAG ⏳

Улучшение качества ответов: hybrid search, reranker, query expansion.

- [ ] **Hybrid search** — vector + BM25/FTS fusion
- [ ] **Cross-encoder reranker** — переранжирование top-k результатов
- [ ] **Query expansion** — альтернативные формулировки через LLM
- [ ] **Smarter multi-turn context** — sliding window
- [ ] **Additional formats** — CSV, JSON, HTML-to-text

---

## Milestone: v0.6 — Multi-user & Audit ⏳

Multi-tenancy, RBAC, audit log (CORS и SAST перенесены в v0.3.2).

- [x] **User authentication** — KeyCloak 26 (OIDC/OAuth2), JWT Bearer token, `/api/auth/me` и `/api/auth/logout`, realm-import.json.template, PostgreSQL для KeyCloak
- [ ] **Multi-tenancy** — изоляция данных по пользователю (нет фильтрации user_id в запросах)
- [ ] **RBAC** — admin / user роли (модель `UserContext` не содержит поле role)
- [ ] **Audit log** — логи всех API-вызовов с user_id + action — не реализовано

---

## Milestone: v1.0 — Production Ready ⏳

CI/CD, performance testing, SLA, документация, мониторинг.

- [ ] **CI/CD** — авто-деплой на VPS при push в main (CI есть: fmt/clippy/test/e2e, нет деплоя)
- [ ] **Load testing** — k6/locust, target P99 < 10s per query
- [ ] **SLA + auto-recovery** — документированные процедуры
- [ ] **Documentation** — GUI guide, runbook, C4 diagrams (docs/api.md и docs/architecture.md есть, C4 нет)
- [ ] **Monitoring dashboard** — cAdvisor + Prometheus или эквивалент

---

## Summary

| Milestone | Status | Фокус |
|-----------|--------|-------|
| v0.1 — MVP | ✅ 20/20 | Full RAG pipeline |
| v0.2 — GUI Redesign | ✅ 6/6 | DeepSeek-style chat UI, UI atoms, session sidebar, admin redesign, login page, dark/light theme |
| v0.2.1 — Markdown & Code Rendering | ✅ 1/1 | Markdown rendering, syntax highlighting, copy button |
| v0.3 — Admin Panel & Production Polish | ✅ 14/14 | Collection & document management, confidence indicator, ZIP upload, Git sync, ADMIN_API_KEY removed, re-indexing, bulk deletion, VToast feedback, optimistic UX, embedding pipeline |
| v0.3.1 — Migration to PostgreSQL | ✅ 10/10 | Container rename, backend DB, sqlx pg, schema migration, config, tests, backup, CI, docs |
| v0.3.2 — Security Hardening | ⏳ 0/7 | XSS sanitization, rate limiting, SAST in CI, CORS hardening, security headers, error sanitization, deep healthcheck |
| v0.3.3 — Basic Q&A Logic & Chat Rework | ⏳ 2/6 | Streaming, LLM error handling ✅; message editing, context, export UI, empty state ❌ |
| v0.4 — Observability & Reliability | ⏳ 2/3 | Automated backup ✅, graceful shutdown ✅; failure notifications ❌ | |
| v0.5 — Advanced RAG | ⏳ 0/5 | Hybrid search, reranker, query expansion, multi-turn, formats |
| v0.6 — Multi-user & Audit | ⏳ 1/5 | KeyCloak auth ✅; multi-tenancy, RBAC, audit log ❌ |
| v1.0 — Production Ready | ⏳ 0/5 | CI/CD, perf, SLA, docs, monitoring |

**Старт:** 2026-06-14
**MVP завершён:** 2026-06-15
**Chat UI overhaul (Phases 0–4):** 2026-06-16
**Document re-indexing:** 2026-06-19
**Bulk document deletion + embedding pipeline:** 2026-06-20
**Что дальше:** `/aif-plan` — v0.3.2 (Security Hardening: XSS, rate limiting, SAST, CORS, headers), затем v0.3.3 (message editing, context window, chat export UI, loading skeletons)