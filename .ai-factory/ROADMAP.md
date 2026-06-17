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

## Milestone: v0.2 — GUI Redesign (DeepSeek-стиль) 🔜

Полный редизайн интерфейса в стиле https://chat.deepseek.com/ — минималистичный чат, тёмная/светлая тема, сайдбар сессий, качественный рендеринг Markdown и кода.

- [x] **Chat UI overhaul (Phases 0–4)** — дизайн-токены (`chat-tokens.css`), компонент `UserAvatar` (SVG, 3 размера), переработка `MessageBubble` (ролевые лейблы, typing indicator, источники), `ChatWindow` (хедер с селектором коллекций, +New), адаптивная вёрстка (768px/480px), E2E-тесты Playwright (42 теста, 5 spec-файлов), юнит-тесты (26)
- [x] **Dark/light theme** — переключаемая тема с сохранением в localStorage, CSS-переменные для всей палитры
- [x] **Session sidebar** — редизайн сайдбара сессий в стиле Pencil (312px, card bg, radius-xl)
- [x] **Admin panel redesign** — страница управления приведена к единому стилю (admin.pen, dialogs.pen)
- [x] **UI component refactor** — вынос атомарных компонентов в `src/components/ui/`, единая типографика IBM Plex Mono
- [x] **Login page** — страница входа с OAuth-провайдерами (login.pen)
- [x] **Auth documentation** — docs/auth.md with KeyCloak setup, social providers, OAuth flow, troubleshooting

---

## Milestone: v0.2.1 — Markdown & Code Rendering

Полноценный рендеринг Markdown и подсветка синтаксиса в сообщениях чата.

- [x] **Markdown & code rendering** — полноценный рендеринг Markdown (remark/rehype), подсветка синтаксиса (shiki/prism), кнопка копирования кода

---

## Milestone: v0.3 — Admin Panel & Production Polish

Закрытие пробелов MVP: управление коллекциями и загрузка документов через админ-панель, E2E-тесты, ZIP-загрузка, re-indexing, confidence score, graceful degradation.

- [x] **E2E tests** — Playwright: upload → query → sources, запуск в CI
- [x] **Chroma integration tests** — убрать `--ignored`, развернуть Chroma в CI
- [x] **Collection management in admin panel** — UI для CRUD коллекций в админ-панели (создание, удаление, переименование, список)
- [x] **Document upload through admin panel** — интерфейс загрузки документов с дроп-зоной, прогресс-баром, валидацией
- [ ] **ZIP batch upload** — до 10 файлов, HTTP 413 при превышении (фронтенд принимает .zip, бэкенд не обрабатывает)
- [ ] **Git repository sync** — подключение Git-репозитория (GitHub/GitLab/Bitbucket): клонирование/пулл, парсинг Markdown-документов из репозитория, индексация в Chroma, webhook-уведомления при обновлении
- [ ] **Document re-indexing** — деактивация старых чанков при перезагрузке
- [x] **Confidence indicator** — relevance score в UI (sources)
- [ ] **Graceful degradation** — fallback-модель + кэширование ответов (есть retry + кэш эмбеддингов, нет fallback LLM)

---

## Milestone: v0.3.1 — Basic Q&A Logic & Chat Rework

Реализация базовой логики ответов и доработка чата: улучшение потокового вывода, обработка ошибок LLM, сохранение контекста, UI для редактирования/удаления сообщений.

- [x] **Streaming response improvements** — SSE с chunk/sources/done событиями, NDJSON-парсинг на фронтенде
- [x] **LLM error handling** — retry (3 попытки), 60s timeout, error-события SSE, отображение ошибок в UI
- [ ] **Message editing & deletion** — UI для редактирования и удаления сообщений в чате
- [ ] **Context management** — sliding window + token budget (сейчас передаётся вся история)
- [ ] **Chat export** — экспорт истории чата (JSON/Markdown, backend API готов, нет кнопки в UI)
- [ ] **Empty state & loading skeletons** — базовые empty state есть, нет анимированных скелетонов

---

## Milestone: v0.4 — Observability & Reliability

Мониторинг, алёртинг, автоматический бэкап, per-route rate limiting.

- [ ] **Deep healthcheck** — проверка зависимостей (Chroma, embedding)
- [ ] **Per-route rate limiting** — защита `/api/query`
- [ ] **Automated backup** — cron-контейнер или host cron для SQLite + Chroma
- [ ] **Failure notifications** — Telegram / email webhook
- [ ] **Graceful shutdown** — корректное завершение всех контейнеров

---

## Milestone: v0.5 — Advanced RAG

Улучшение качества ответов: hybrid search, reranker, query expansion.

- [ ] **Hybrid search** — vector + BM25/FTS fusion
- [ ] **Cross-encoder reranker** — переранжирование top-k результатов
- [ ] **Query expansion** — альтернативные формулировки через LLM
- [ ] **Smarter multi-turn context** — sliding window
- [ ] **Additional formats** — CSV, JSON, HTML-to-text

---

## Milestone: v0.6 — Multi-user & Security

Аутентификация, multi-tenancy, audit log, security hardening.

- [ ] **User authentication** — JWT, login/register
- [ ] **Multi-tenancy** — изоляция данных по пользователю
- [ ] **RBAC** — admin / user роли
- [ ] **Audit log** — логи всех API-вызовов с user_id + action
- [ ] **CORS hardening** — строгие origin
- [ ] **SAST scanning** — зависимостей в CI

---

## Milestone: v1.0 — Production Ready

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
| v0.3 — Admin Panel & Production Polish | ⏳ 5/9 | Collection & document management, confidence indicator ✅; ZIP, Git sync, re-indexing, graceful degradation ❌ |
| v0.3.1 — Basic Q&A Logic & Chat Rework | ⏳ 2/6 | Streaming, LLM error handling ✅; message editing, context, export, empty state ❌ |
| v0.4 — Observability & Reliability | ⏳ 0/5 | Healthcheck, rate limit, backup, alerts, shutdown |
| v0.5 — Advanced RAG | ⏳ 0/5 | Hybrid search, reranker, query expansion, multi-turn, formats |
| v0.6 — Multi-user & Security | ⏳ 0/6 | Auth, multi-tenancy, RBAC, audit, CORS, SAST |
| v1.0 — Production Ready | ⏳ 0/5 | CI/CD, perf, SLA, docs, monitoring |

**Старт:** 2026-06-14
**MVP завершён:** 2026-06-15
**Chat UI overhaul (Phases 0–4):** 2026-06-16
**Что дальше:** `/aif-implement` — завершение v0.3 (ZIP batch upload, document re-indexing, graceful degradation) и v0.3.1 (message editing, context window, chat export UI, loading skeletons)
