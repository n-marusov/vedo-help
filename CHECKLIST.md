# CHECKLIST

Пройтись по всем пунктам после любой реализации прежде чем считать задачу завершённой.

## Code gates
- `npm run ai:validate` — exit 0
- `uvx ruff check --fix` — exit 0
- `bash scripts/validate-migrations.sh` — exit 0
- `bash scripts/validate-migrations.sh --git` — exit 0 (если origin/main доступен)
- `bash scripts/check-container-health.sh` — exit 0 (все контейнеры healthy)

## Contract
- Типы в `src/api/types.ts` 1:1 соответствуют `workflow/task-manager/openapi.yaml` (envelope-обёртки, имена полей)
- `src/api/client.ts` разворачивает envelopes и маппит `ErrorEnvelope.error` в `ApiError`
- Вне `src/api/client.ts` нет прямых `fetch`-вызовов

## UI
- Все атомы в `src/components/ui` сверены через `pencil` MCP (`ui-kit.lib.pen`)
- `.ai-factory/references/ui-kit-atoms.md` отражает реальный набор вариантов и расхождений
- `/ui-preview` отрисовывает все 9 атомов без регрессий

## Roadmap sync
- `.ai-factory/ROADMAP.md` отражает реальное состояние:
  - Завершённые задачи помечены `[x]`
  - Задачи в работе помечены `[~]` (если применимо)
  - Таблица Summary (Tasks total / Tasks done) пересчитана
  - Milestone Status актуален (не начато / в работе / завершено)

## Database & Migration Hygiene

Проблема «грязной БД» возникает когда:

1. **Изменён файл миграции после её применения.** sqlx хранит контрольную сумму каждой
   миграции в таблице `_sqlx_migrations`. Если файл `.sql` изменился (содержимое,
   пробелы, комментарии), но версия осталась той же — sqlx откажется запускать backend.
2. **Старый Docker volume с предыдущей схемой.** При смене веток или после изменения
   миграций старый volume содержит несовместимую схему.

### Профилактика

```bash
# Перед запуском тестового окружения — удалить старые volumes
docker compose --env-file .env.test -f docker-compose.test.yml down -v

# Проверить, что миграции не были изменены после применения:
bash scripts/validate-migrations.sh --git
```

### Если backend не стартует с ошибкой миграции

```
migration N was previously applied but is missing / has been modified
```

**Что делать:**

1. `docker compose --env-file .env.test -f docker-compose.test.yml down -v` — удалить
   volumes и пересоздать БД с нуля
2. Убедиться, что `bash scripts/validate-migrations.sh` проходит
3. Если ошибка повторяется — проверить, что файлы миграций в `backend/migrations/`
   соответствуют тому, что ожидает `sqlx::migrate!("./migrations")` (пересобрать
   backend: `cargo build`)

### Если DB round-trip тесты падают каскадно (14/14)

После смены веток или при stale volumes **все** тесты `documents_db_unit` могут
упасть с `NotFound("Collection ...")` — это симптом «грязной» БД, а не реальная
поломка. Решение: удалить volumes (`docker compose ... down -v`) и перезапустить
окружение.

## Test environment

Перед любыми тестами запусти тестовое окружение. **Один запуск** — все сервисы
поднимаются сразу. `--env-file .env.test` обязателен для корректной работы KeyCloak,
Chroma и БД.

> ⚠️ **Важно:** Перед запуском убедись, что Docker volumes чистые (см. Database & Migration Hygiene выше).

```bash
# Удалить старые volumes (если не делали)
docker compose --env-file .env.test -f docker-compose.test.yml down -v

# Запустить все сервисы тестового окружения
docker compose --env-file .env.test -f docker-compose.test.yml up -d

# Дождаться, пока все сервисы станут healthy
# (6 сервисов: chroma, backend, frontend, db, keycloak, llm-mock)
docker compose --env-file .env.test -f docker-compose.test.yml ps
```

### Frontend unit-тесты (Vitest)

Изолированы — не требуют тестового окружения. Прогоняются локально.

```bash
cd frontend && npm test
```

### Backend unit-тесты (`cargo test --lib`)

Чистые unit-тесты не требуют никакой инфраструктуры — ни PostgreSQL, ни Chroma.
Выполняются на хосте сразу.

```bash
cd backend && cargo test --lib
```

### Backend DB round-trip тесты (`cargo test --test *_unit`)

Тесты, проверяющие работу с PostgreSQL напрямую (репозитории, сервисы).
Требуют только PostgreSQL из тестового окружения (`localhost:15432`).

> ⚠️ **Важно:** Каждый `--test *_unit` запускается **отдельно** — не используй `--test *_unit` (wildcard).
> Каждый бинарник заново подключается к той же БД, дропает `_sqlx_migrations` и
> перезапускает миграции. При параллельном запуске они конфликтуют.

```bash
# Терминал 1: тестовое окружение уже запущено (см. выше)
# Терминал 2:
export DATABASE_URL=postgres://vedo:test-vedo-password@localhost:15432/vedo
cd backend && cargo test --test git_sync_unit -- --test-threads=1
# После завершения git_sync_unit — запустить documents_db_unit:
cd backend && cargo test --test documents_db_unit -- --test-threads=1
```

### Backend интеграционные тесты (`cargo test --test integration`)

Требуют **реальный Chroma и PostgreSQL** из тестового окружения.
Используют `setup_test_config()` для настройки клиентов, но сами ходят в работающие
сервисы. Выполняются на хосте.

```bash
# Терминал 1: тестовое окружение уже запущено (см. выше)
# Терминал 2:
export DATABASE_URL=postgres://vedo:test-vedo-password@localhost:15432/vedo
export CHROMA_URL=http://localhost:18000
cd backend && cargo test --test integration -- --test-threads=1
```



### E2E тесты (Playwright)

Playwright целиком выполняется внутри `frontend-tests`-контейнера из
`docker-compose.test.yml` (образ `mcr.microsoft.com/playwright`) в сети `test_internal`.
Браузер и Vite dev-server запускаются в том же контейнере, а все запросы к backend и
KeyCloak идут по Docker service names — `http://backend:3000` и `http://keycloak:8080`.
Порты на `localhost` для E2E **не пробрасываются и не используются**.

Переменные окружения уже выставлены в compose (сервис `frontend-tests`):

- `VITE_API_PROXY_TARGET=http://backend:3000`
- `VITE_KEYCLOAK_PROXY_TARGET=http://keycloak:8080`
- `E2E_API_URL=http://backend:3000`
- `E2E_KEYCLOAK_URL=http://keycloak:8080`

`localhost:5173` в `playwright.config.ts` (`baseURL`/`webServer`) — это адрес Vite
**внутри самого Playwright-контейнера** (контейнерный `127.0.0.1`, не host-порт). Это
согласуется с правилом «`localhost` только для browser-URL, публичных host-портов и
self-healthcheck'ов контейнера».

```bash
# Терминал 1: тестовое окружение уже запущено (см. выше)
# Терминал 2: запустить Playwright-контейнер из профиля test-runner
docker compose --env-file .env.test -f docker-compose.test.yml \
  --profile test-runner run --rm frontend-tests
```

Для отладки — интерактивная shell в том же контейнере (остаётся в `test_internal`):

```bash
docker compose --env-file .env.test -f docker-compose.test.yml \
  --profile test-runner run --rm --entrypoint sh frontend-tests
# внутри контейнера:
npm ci
npx playwright test --debug
npx playwright test e2e/login.spec.ts   # отдельный spec
# HTML-отчёт сохраняется в ./frontend/playwright-report (volume примонтирован)
```

> UI-режим (`npx playwright test --ui`) требует проброса `DISPLAY`/X-сервера из
> контейнера — для локальной отладки используй `--debug` (headless с трейсами).

### E2E API-тесты (без браузера)

Для быстрой проверки бэкенда без браузера используются API-тесты из
`api-backend.spec.ts` (37 тестов). Они работают через `APIRequestContext` Playwright
и не требуют Vite dev-server. Запускаются в том же `frontend-tests`-контейнере.

```bash
# Внутри контейнера frontend-tests — все API-тесты
npx playwright test e2e/api-backend.spec.ts

# Одна группа (например, Collections)
npx playwright test e2e/api-backend.spec.ts --grep "Collections"

# Один тест по ID
npx playwright test e2e/api-backend.spec.ts --grep "TC-API-020"
```

### После завершения всех тестов

```bash
# Остановить и очистить volumes (тестовые данные не нужны)
docker compose --env-file .env.test -f docker-compose.test.yml down -v
```

### Полный прогон одной командой

Использует `make` target, который поднимает окружение и прогоняет все тесты
последовательно. Подходит для CI или быстрой проверки перед коммитом:

```bash
make test-env       # docker compose up + ожидание healthcheck'ов
# в отдельном терминале:
make test           # unit-тесты (backend --lib + frontend npm test)
make test:e2e       # E2E через Playwright-контейнер в test_internal (без localhost)
make test-env-down  # остановка и очистка
```

> **Важно:** все `make test-*` target'ы передают `--env-file .env.test` явно.
> `make test:e2e` запускает Playwright внутри `frontend-tests`-контейнера в сети
> `test_internal` (Docker service names, без проброса host-портов).

## Docs / rules
- `AGENTS.md` актуален (структура, ключевые файлы)
- Нет shell-команд через `&&`/`||`/`;` в инструкциях и коммитах
- `npm run format:check` должен проходить (входит в `ai:validate`; форматирование — `npx biome format --write .`)
