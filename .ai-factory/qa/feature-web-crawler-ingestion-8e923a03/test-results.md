# Test Results: Web Crawler & Site Ingestion

**Branch:** `feature/web-crawler-ingestion`
**Date:** 2026-07-11

---

## Summary Table

| Модуль | Тестов | Прошло | Упало | Причина падения | Что исправить |
|--------|--------|--------|-------|-----------------|---------------|
| **Backend: web_crawl_unit** — normalize URL | 3 | 3 | 0 | — | — |
| **Backend: web_crawl_unit** — URL/traversal logic | 5 | 5 | 0 | — | — |
| **Backend: web_crawl_unit** — DB contract (8 тестов) | 8 | 0 | 8 | `relation "web_crawl_jobs" does not exist` — таблицы не созданы | Создать миграции `00000000000016_create_web_crawl_tables.sql` и `00000000000017_add_web_source_to_documents.sql` |
| **Backend: web_crawl_integration** | 6 | 0 | 6 | `relation "web_crawl_jobs" does not exist` — таблицы не созданы | Создать миграции (см. выше) + реализовать модели и репозиторий |
| **Frontend: vitest** (существующие) | 212 | 210 | 0 | 2 skipped (предсуществующие) | Не требуется |
| **Frontend E2E: web-crawl.spec.ts** | 10 | ? | ? | Backend API `/api/web-crawl` не существует | Реализовать API-хендлеры (Phase 3, Task 7) |
| **Backend:cargo check** (существующий) | — | ✓ | — | — | — |

---

## Детальный разбор упавших тестов

### Backend: web_crawl_unit — DB-зависимые тесты (8 шт.)

| Тест | Статус | Ошибка | Корень проблемы |
|------|--------|--------|-----------------|
| `test_create_job_persists_all_fields` | FAIL | `relation "web_crawl_jobs" does not exist` | Нет миграции `00000000000016` |
| `test_job_status_transitions` | FAIL | То же | То же |
| `test_job_cancel_from_idle` | FAIL | То же | То же |
| `test_list_jobs_by_user_filters_correctly` | FAIL | То же | То же |
| `test_delete_job_removes_row` | FAIL | То же | То же |
| `test_create_page_persists_fields` | FAIL | То же | То же |
| `test_delete_job_cascades_to_pages` | FAIL | То же | То же |
| `test_crawl_lock_cas_behavior` | FAIL | То же | То же |

**Что исправить:** Создать файлы миграций:
1. `backend/migrations/00000000000016_create_web_crawl_tables.sql` — таблицы `web_crawl_jobs`, `web_crawl_pages`, индексы
2. `backend/migrations/00000000000017_add_web_source_to_documents.sql` — расширить `chk_documents_source` включить `'web'`

### Backend: web_crawl_integration (6 тестов)

| Тест | Статус | Ошибка | Корень проблемы |
|------|--------|--------|-----------------|
| `test_job_lifecycle_full` | FAIL | `relation "web_crawl_pages" does not exist` | Нет миграций (см. выше) |
| `test_cancel_job_marks_pending_pages` | FAIL | То же | То же |
| `test_multiple_jobs_independent_tracking` | FAIL | То же | То же |
| `test_job_error_stores_message` | FAIL | То же | То же |
| `test_page_scoping_by_job` | FAIL | То же | То же |

**Что исправить:** После создания миграций, также нужно:
1. Добавить модуль `web_crawl` в `backend/src/modules/`
2. Реализовать `models.rs` (CrawlJob, CrawlPage, CrawlJobConfig, и т.д.)
3. Реализовать `repository.rs` (WebCrawlRepository с методами CRUD)

### Frontend E2E: web-crawl.spec.ts (10 тестов)

| Тест | Статус | Ошибка | Корень проблемы |
|------|--------|--------|-----------------|
| TC-WEB-001 | ? | Backend не запущен / API не существует | Нет API-хендлеров (Task 7) |
| TC-WEB-002 | ? | То же | То же + нет UI-компонента (Task 8) |
| TC-WEB-003 | ? | То же | Нет UI-компонента WebCrawlerManager |
| TC-WEB-004 | ? | Endpoint `POST /api/web-crawl` не существует | Нет API-хендлеров (Task 7) |
| TC-WEB-005 | ? | То же | То же |
| TC-WEB-006 | ? | Нет UI-компонента | Нет WebCrawlerManager (Task 8) |
| TC-WEB-007 | ? | Endpoint не существует | Нет API-хендлеров (Task 7) |
| TC-WEB-008 | ? | То же | То же |
| TC-WEB-009 | ? | То же | То же |
| TC-WEB-010 | ? | То же | То же |

**Что исправить:** Реализовать Phase 2-4 плана:
- Phase 2 (Tasks 4-5): Модели, репозиторий, краулер-движок
- Phase 3 (Tasks 6-7): Сервисный слой, API-хендлеры, main.rs
- Phase 4 (Tasks 8-9): WebCrawlerManager UI, AdminView интеграция, API-клиент

---

## Пройденные тесты (Pure Logic)

### Backend: web_crawl_unit — без БД (8 тестов)

| Тест | Статус | Проверяет |
|------|--------|-----------|
| `test_normalize_url_strips_fragment` | ✓ | URL: удаление `#fragment` |
| `test_normalize_url_removes_trailing_slash` | ✓ | URL: удаление замыкающего `/` |
| `test_normalize_url_resolves_relative` | ✓ | URL: разрешение относительных ссылок |
| `test_same_domain_enforcement` | ✓ | Ограничение: только один домен |
| `test_path_prefix_filtering` | ✓ | Фильтр: только по path-префиксу |
| `test_depth_limit` | ✓ | Глубина: не более max_depth |
| `test_max_pages_limit` | ✓ | Страницы: не более max_pages |
| `test_url_deduplication` | ✓ | Дубликаты: один URL не посещается дважды |

---

## Необходимые действия для агента-исполнителя

1. **Создать миграции БД** (`00000000000016` + `00000000000017`)
2. **Реализовать модуль `web_crawl`**: `models.rs`, `repository.rs`, `crawler.rs`
3. **Добавить `scraper` и `robotstxt`** в `Cargo.toml`
4. **Реализовать сервис и API**: `service.rs`, `handlers.rs`, `main.rs` wiring
5. **Реализовать фронтенд**: `WebCrawlerManager.vue`, `api/client.ts` методы, `AdminView.vue` интеграция
6. **Обновить `common/mod.rs`** TRUNCATE список: добавить `web_crawl_jobs`, `web_crawl_pages`
7. **Запустить все тесты**: `cargo test --test web_crawl_unit && cargo test --test web_crawl_integration`
