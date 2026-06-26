# Implementation Plan: Fix Pre-existing Test Errors

Branch: fix/test-compilation-errors
Created: 2026-06-26

## Settings
- Testing: yes (fixing tests is the task)
- Logging: standard
- Docs: no

## Roadmap Linkage
Milestone: "v0.4 — Observability & Reliability"
Rationale: Pre-requisite from ROADMAP.md: fix pre-existing test errors before starting v0.4 work

## Root Cause Summary

После повторной верификации с чистой БД актуальные поломки:

| Область | Сколько падает | Коренная причина |
|---------|---------------|-----------------|
| `documents_db_unit` | **2 теста** | `process_zip_upload` проверяет существование collection через `get_collection_for_user`, но тесты используют `Uuid::new_v4()` без вставки коллекции в БД |
| `git_sync_unit` | **1 тест** | Сравнение `Utc::now()` (наносекунды) с timestamp из PostgreSQL (микросекунды) — расхождение в последних 3 цифрах |
| `integration` | **0** (при запущенном Chroma) | Падения только когда Chroma/Embedding не запущены — это ожидаемо, тесты требуют полного окружения |
| `ruff check` | **2 fixable** | Неотсортированные импорты в `embedding/tests/test_main.py` |
| E2E (Docker) | **инфраструктура** | Docker-контейнер не может достучаться до npm registry |

## Tasks

### Phase 1: Fix documents_db_unit ZIP tests
- [ ] Task 1: Вставить collection в PostgreSQL перед `process_zip_upload` в `test_process_zip_empty` и `test_process_zip_with_11_files_returns_413`

  **Логирование:** DEBUG-лог при создании тестовой коллекции

  Файлы: `backend/tests/documents_db_unit.rs` (строки ~253-323)

  **Детали:**
  - `test_process_zip_empty` (строка 309): добавить `sqlx::query("INSERT INTO collections ...")` перед `process_zip_upload`
  - `test_process_zip_with_11_files_returns_413` (строка 255): добавить `sqlx::query("INSERT INTO collections ...")` перед `process_zip_upload`. Переиспользовать `collection_id` вместо `Uuid::new_v4()`.
  - Паттерн уже используется в остальных тестах этого же файла (строки 52-59, 100-107 и т.д.)

### Phase 2: Fix git_sync_unit timestamp assertion
- [ ] Task 2: Исправить сравнение timestamp в `test_create_repo_persists_all_fields`

  **Логирование:** DEBUG-лог с указанием precision mismatch

  Файлы: `backend/tests/git_sync_unit.rs` (строка 90)

  **Детали:**
  - Заменить `assert_eq!(row.10, repo_created_at)` на сравнение с допуском
  - Опция A: округлить оба timestamp до микросекунд: `repo_created_at.timestamp_micros()`
  - Опция B: использовать `chrono::Duration::milliseconds(1)` допуск
  - Предпочтение: опция A (микросекундное округление), т.к. PostgreSQL хранит `timestamptz` с микросекундной точностью

### Phase 3: Fix ruff lint issues
- [ ] Task 3: Исправить сортировку импортов в embedding тестах

  **Логирование:** не требуется (автофикс)

  Файлы: `embedding/tests/test_main.py`

  **Команда:** `uvx ruff check --fix`

### Phase 4: Integration test verification
- [ ] Task 4: Запустить integration тесты с полным тестовым окружением (Chroma + Embedding) и убедиться, что 0 падений

  **Логирование:** INFO при старте тестов

  **Команда:**
  ```bash
  docker compose --env-file .env.test -f docker-compose.test.yml up -d
  # wait for healthy
  cd backend && DATABASE_URL=postgres://vedo:test-vedo-password@localhost:15432/vedo CHROMA_URL=http://localhost:18000 EMBEDDING_SERVICE_URL=http://localhost:18001 cargo test --test integration -- --test-threads=1
  ```

## Commit Plan
- **Commit 1** (после Phase 1): "fix: insert collection before zip upload in documents_db_unit tests"
- **Commit 2** (после Phase 2): "fix: compare timestamps with microsecond tolerance in git_sync_unit test"
- **Commit 3** (после Phase 3): "style: fix ruff import sorting in embedding tests"
- **Commit 4** (после Phase 4): "chore: verify integration tests pass with full test env"

## Known Infrastructure Issues (не блокируют, но задокументированы)
1. **E2E Playwright (frontend-tests):** Docker-контейнер не может загрузить npm-зависимости (`Idle timeout`). Требуется настройка сети хоста или mirror registry для `npm ci` внутри контейнера.
2. **E2E API (на хосте):** 32 теста падают с 401 при запуске с хоста, т.к. рассчитаны на выполнение внутри Docker-сети. Не требуют исправления — это ожидаемое поведение.
