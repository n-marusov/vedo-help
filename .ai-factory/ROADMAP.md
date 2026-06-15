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

- [ ] **Chat UI overhaul** — переработка ChatWindow + MessageBubble: минималистичные сообщения, аватарки, плавные анимации, адаптивная вёрстка
- [ ] **Dark/light theme** — переключаемая тема с сохранением в localStorage, CSS-переменные для всей палитры
- [ ] **Session sidebar** — список чатов слева: поиск, группировка по датам, контекстное меню (переименовать/удалить/экспорт)
- [ ] **Markdown & code rendering** — полноценный рендеринг Markdown (remark/rehype), подсветка синтаксиса (shiki/prism), кнопка копирования кода
- [ ] **Admin panel redesign** — приведение страницы управления (документы, коллекции) к единому стилю
- [ ] **UI component refactor** — вынос атомарных компонентов в `src/components/ui/`, единая типографика, иконки Lucide

---

## Milestone: v0.3 — Production Polish

Закрытие пробелов MVP: E2E-тесты, ZIP-загрузка, re-indexing, confidence score, graceful degradation.

- [ ] **E2E tests** — Playwright: upload → query → sources, запуск в CI
- [ ] **Chroma integration tests** — убрать `--ignored`, развернуть Chroma в CI
- [ ] **ZIP batch upload** — до 10 файлов, HTTP 413 при превышении
- [ ] **Document re-indexing** — деактивация старых чанков при перезагрузке
- [ ] **Confidence indicator** — relevance score в UI (sources)
- [ ] **Graceful degradation** — fallback-модель + кэширование ответов

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
| **v0.2 — GUI Redesign** | ⏳ **0/6** | **DeepSeek-style chat UI, dark/light theme, session sidebar, Markdown rendering** |
| v0.3 — Production Polish | ⏳ 0/6 | E2E, ZIP, re-indexing, confidence, graceful degradation |
| v0.4 — Observability & Reliability | ⏳ 0/5 | Healthcheck, rate limit, backup, alerts, shutdown |
| v0.5 — Advanced RAG | ⏳ 0/5 | Hybrid search, reranker, query expansion, multi-turn, formats |
| v0.6 — Multi-user & Security | ⏳ 0/6 | Auth, multi-tenancy, RBAC, audit, CORS, SAST |
| v1.0 — Production Ready | ⏳ 0/5 | CI/CD, perf, SLA, docs, monitoring |

**Старт:** 2026-06-14
**MVP завершён:** 2026-06-15
**Что дальше:** `/aif-plan v0.2 — GUI Redesign (DeepSeek-стиль)` → `/aif-implement`
