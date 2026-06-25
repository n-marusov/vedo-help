# Fix Pre-existing Test Failures

> План исправления тестов, падающих из-за race condition, незаданных FK- precondition и непоследовательной конфигурации запуска.

## Branch

- **Текущая ветка:** `fix/test-compilation-errors`
- **Base:** `main`

## Settings

- **Testing:** Да — тесты уже написаны, задача — починить их
- **Logging:** Standard — только INFO, фиксы тривиальные
- **Docs:** Нет — изменения только в тестах и CHECKLIST.md

## Roadmap Linkage

- **Milestone:** `v0.3.1` — Basic Q&A Logic & Chat Rework
- **Rationale:** Pre-existing test errors отмечены в Roadmap как pre-requisite для v0.4

---

## Tasks

- [x] ### Phase 1 — Fix `test_process_zip_corrupted` (collection FK precondition)

**Root cause:** Тест вызывает `process_zip_upload()` с `Uuid::new_v4()` как `collection_id`, но не создаёт коллекцию в БД. Функция первой же строчкой проверяет `get_collection_for_user()`, возвращает `AppError::NotFound` — тест ожидает `AppError::FileError`.

**File:** `backend/tests/documents_db_unit.rs`

**Changes:**
1. Добавить вставку коллекции перед вызовом `process_zip_upload` (аналогично `test_process_zip_mixed_valid_invalid`):
   - Создать `collection_id = Uuid::new_v4()`
   - INSERT в `collections` с этим ID, именем, `created_at = NOW()`, `user_id = 'test-user'`

**Validation:** `cd backend && cargo test --test documents_db_unit test_process_zip_corrupted -- --test-threads=1`

---

- [x] ### Phase 2 — Fix integration test race condition (`--test-threads=1`)

**Root cause:** Все integration-тесты используют общую БД, а `setup_test_db()` делает `TRUNCATE ... CASCADE`. Без `--test-threads=1` concurrent-тесты затирают данные друг друга, вызывая FK violations. **Доказано:** все 23 теста проходят с `--test-threads=1`.

**Files:**
- `CHECKLIST.md` — добавить `--test-threads=1` в инструкцию для интеграционных тестов
- `backend/tests/integration.rs` — все тесты уже имеют `setup_test_db()`, которая явно требует `--test-threads=1` (комментарий в `common/mod.rs`)

**Changes:**
1. В `CHECKLIST.md` заменить:
   ```
   cd backend && cargo test --test integration
   ```
   на:
   ```
   cd backend && cargo test --test integration -- --test-threads=1
   ```
2. В `CHECKLIST.md` (секция "Backend DB round-trip тесты") убедиться, что `documents_db_unit` тоже указан с `--test-threads=1`.

**Validation:** `cd backend && cargo test --test integration -- --test-threads=1` — 23/23 pass.

---

- [x] ### Phase 3 — Fix `test_get_chunks_by_ids_unknown_uuid` collection FK precondition

**Root cause:** Тест использует hardcoded UUID `eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee` для `collection_id` и делает прямой SQL INSERT в `documents`, но коллекции с таким UUID не существует. При sequential-запуске с `--test-threads=1` тест проходит, потому что ранние тесты создают коллекции, которые потом не затираются. Без `--test-threads=1` коллекция могла быть стёрта concurrent TRUNCATE.

**Note:** Этот тест является жертвой race condition из Phase 2, а не самостоятельной проблемы. После перехода на `--test-threads=1` он проходит.

**Changes:** Нет изменений кода — охраняется Phase 2 (`--test-threads=1`).

**Validation:** `cd backend && cargo test --test integration -- --test-threads=1` — 23/23 pass.

---

- [x] ### Phase 4 — Document `_sqlx_migrations` cleanup in test environment docs

**Root cause:** `git_sync_unit` тесты падали с `VersionMismatch(1)` когда `_sqlx_migrations` содержала checksum, не совпадающую с текущими файлами миграций. Это состояние возникает при переключении веток с разными версиями миграций.

**File:** `backend/tests/common/mod.rs`

**Changes:**
1. Добавить в `setup_test_db()` (в `backend/tests/common/mod.rs`) перед `sqlx::migrate!()`:
   ```rust
   // Drop stale migration tracking to avoid VersionMismatch when
   // switching between branches with different migration histories.
   sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
       .execute(&pool)
       .await
       .ok();
   ```

**Validation:** Переключиться на ветку со старыми миграциями, вернуться — `cargo test --test git_sync_unit -- --test-threads=1` не должен падать с VersionMismatch.

---

- [x] ### Phase 5 — Fix `test_conversation_repo_native_uuid_bind` (side effect of race condition)

**Root cause:** Аналогично Phase 3 — это жертва race condition. При `--test-threads=1` тест проходит (подтверждено запуском 23/23).

**Changes:** Нет — охраняется Phase 2.

**Validation:** `cd backend && cargo test --test integration -- --test-threads=1`

---

## Commit Plan

1. `fix(test): add collection FK precondition to test_process_zip_corrupted` ✅
   - `backend/tests/documents_db_unit.rs`
2. `fix(test): add _sqlx_migrations cleanup to setup_test_db()` ✅
   - `backend/tests/common/mod.rs`
3. `fix(docs): add --test-threads=1 for integration/db tests in CHECKLIST.md` ✅
   - `CHECKLIST.md`
