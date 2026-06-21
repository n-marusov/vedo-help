# Implementation Plan: Fix E2E Test Failures

Branch: feature/v031-chat-rework
Created: 2026-06-21

## Settings
- Testing: yes (E2E tests are the subject of the fix)
- Logging: verbose (adding DEBUG logging for Chroma propagation diagnosis)
- Docs: no (no documentation changes needed)

## Roadmap Linkage
Milestone: "v0.3 — Admin Panel & Production Polish"
Rationale: Стабильность E2E-тестов — условие завершения milestone (13/14 задач, остаётся graceful degradation)

## Commit Plan
- **Commit 1** (after Task 1): "fix: check ZIP entry count before per-file validation in process_zip_upload"
- **Commit 2** (after Task 2): "fix: add Chroma propagation retry loop to query service"
- **Commit 3** (after Tasks 3-4): "fix: stabilize loading skeleton E2E tests with route timing"
- **Commit 4** (after Task 5): "chore: set Playwright workers=1 and fullyParallel=false"

## Tasks

### Phase 1: Backend fixes

#### Task 1: Fix ZIP validation order — check entry count before per-file validation

**Проблема:** `process_zip_upload` извлекает все файлы из ZIP, вызывает `validate_file` для каждого, и только потом проверяет количество файлов (`file_count > 10`). Если Playwright multipart повреждает ZIP так, что один из файлов не проходит `validate_zip_magic`, возвращается HTTP 415 вместо ожидаемого 413.

**Решение:**
1. Сразу после `zip::ZipArchive::new(reader)` (строка 392) проверить `archive.len() > 10`
2. Если превышает — сразу вернуть `AppError::PayloadTooLarge`, до извлечения и валидации файлов
3. Удалить дублирующую проверку `file_count > 10` из строк 459-467

**Файл:** `backend/src/modules/documents/service.rs` (метод `process_zip_upload`, ~стр. 390-467)
**Logging:** `tracing::info!("ZIP has {count} entries — exceeds limit of 10, rejecting early")`
**Тесты:** существующий `test_process_zip_with_11_files_returns_413` — должен всё ещё проходить

---

#### Task 2: Add Chroma propagation retry loop for queries with 0 results

**Проблема:** После индексации документа в Chroma, запрос через 400ms возвращает 0 результатов (propagation delay фильтра `is_active`). E2E тесты RAG-003 и chat-export не дожидаются появления чанков в поиске.

**Решение:**
В `process_query` (строка 95-111), после первого запроса к Chroma, если `chroma_results.is_empty()` — выполнить до 3 повторных попыток с интервалом 500ms:
```rust
if chroma_results.is_empty() {
    for attempt in 1..=3 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        chroma_results = self.repo.query_chroma(&collection_name, &embedding, 5).await?;
        if !chroma_results.is_empty() {
            tracing::info!("Chroma results found after retry {attempt}");
            break;
        }
    }
}
```

**Файлы:**
- `backend/src/modules/query/service.rs` (метод `process_query`)
- `backend/Cargo.toml` — `tokio` уже есть с `time` feature, проверять не надо

**Logging:** `tracing::info!("Chroma results found after retry {attempt}")` / `tracing::warn!("Chroma still empty after {MAX_RETRIES} retries, continuing with 0 results")`
**Тесты:** существующий `test_query_empty_collection_returns_no_results` — должен всё ещё проходить (не меняет поведение для пустой коллекции)

---

### Phase 2: E2E test stabilization

#### Task 3: Fix loading skeleton tests — route registration timing

**Проблема:** `page.route('**/api/documents', ..., { times: 1 })` перехватывает запрос другого параллельного теста. В последовательном режиме `page.route` может регистрироваться после того, как запрос уже отправлен (при установке `activeCollectionId` до `page.goto`).

**Решение:**
1. Перенести `page.route` **перед** `page.goto('/admin')`
2. Для `repos-loading-skeleton` — регистрировать route **до** переключения на git-таб
3. Опционально: добавить `page.waitForTimeout(100)` после `setActiveCollection` для гарантии, что route перехватит запрос

**Файл:** `frontend/e2e/loading-skeletons.spec.ts`
**Конкретные изменения:**
- Тест "slow GET /api/documents": `page.route` перенести до `page.goto('/admin')`, `setActiveCollection` после `goto`
- Тест "slow GET /api/git-sync": route регистрировать до клика по git-табу
- Удалить `import { getTestAccessToken }` — не используется

---

#### Task 4: Fix chat-export and rag-flow — stabilise message flow

**Проблема:** E2E тесты чата не дожидаются появления assistant-сообщения, потому что:
1. Chroma propagation delay даёт 0 результатов → query идёт без контекста
2. Без сессии кнопка экспорта не показывается (уже исправлено в `ChatView.vue`)

**Решение:**
- В `rag-flow.spec.ts` — после загрузки документа и ожидания `.dl-item__name`, добавить небольшую паузу (1-2s) на Chroma propagation
- В `chat-export.spec.ts` — увеличить таймауты `waitForSelector` с 10s до 20s для assistant-сообщения
- Убедиться, что `handleSend` создаёт сессию (уже исправлено)

**Файлы:**
- `frontend/e2e/rag-flow.spec.ts` (TC-RAG-003)
- `frontend/e2e/chat-export.spec.ts` (оба теста)

---

### Phase 3: Infrastructure

#### Task 5: Finalize Playwright sequential config

**Проверка:** Убедиться, что `playwright.config.ts` имеет:
- `fullyParallel: false`
- `workers: 1`
- Quote style single (Biome format)

**Файл:** `frontend/playwright.config.ts`
