# CHECKLIST

Пройтись по всем пунктам после любой реализации прежде чем считать задачу завершённой.

## Code gates
- `npm run ai:validate` — exit 0 (known pre-existing: ai:perf workspace missing)
- `uvx ruff check --fix` — exit 0

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

## Test environment

Перед прогоном e2e или интеграционных тестов убедись, что тестовое окружение запущено:

```bash
# Запустить все сервисы тестового окружения
docker compose -f docker-compose.test.yml up -d

# Проверить здоровье всех сервисов
docker compose -f docker-compose.test.yml ps
```

После завершения тестов:
```bash
# Остановить и очистить volumes (тестовые данные не нужны)
docker compose -f docker-compose.test.yml down -v
```

### E2E тесты (Playwright)

Требуют работающего тестового окружения + Vite dev server. Playwright автоматически
запускает Vite через `webServer` в `playwright.config.ts`, но в тестовом режиме Vite
должен проксировать API на тестовый backend, а не на дев-сервер:

```bash
# 1. Запустить тестовое окружение (в одном терминале)
docker compose -f docker-compose.test.yml up -d

# 2. Запустить e2e тесты (в другом терминале)
VITE_API_PROXY_TARGET=http://localhost:13000 \
VITE_KEYCLOAK_PROXY_TARGET=http://localhost:18080 \
npm run test:e2e
```

Для debug-режима:
```bash
VITE_API_PROXY_TARGET=http://localhost:13000 \
VITE_KEYCLOAK_PROXY_TARGET=http://localhost:18080 \
npm run test:e2e:debug
```

### Integration тесты (Rust)

Требуют только Postgres. Chroma и Embedding мокаются через setup_test_config():

```bash
# 1. Запустить тестовое окружение
docker compose -f docker-compose.test.yml up -d

# 2. Запустить unit-тесты
export DATABASE_URL=postgres://vedo:test-vedo-password@localhost:15432/vedo
cd backend && cargo test --lib

# 3. Запустить интеграционные тесты (Chroma)
export DATABASE_URL=postgres://vedo:test-vedo-password@localhost:15432/vedo
export CHROMA_URL=http://localhost:18000
export EMBEDDING_SERVICE_URL=http://localhost:18001
cd backend && cargo test --test integration
```

### Embedding тесты (Python)

```bash
export EMBEDDING_SERVICE_URL=http://localhost:18001
cd embedding && pytest tests/ -v
```

### Полный прогон в тестовом окружении

```bash
make test-env   # запускает docker-compose.test.yml + ожидает healthcheck'и
# в отдельном терминале:
make test       # прогоняет все тесты (unit + embedding)
make test:e2e   # прогоняет e2e тесты
```

## Docs / rules
- `AGENTS.md` актуален (структура, ключевые файлы)
- Нет shell-команд через `&&`/`||`/`;` в инструкциях и коммитах
- `npm run format:check` может падать на pre-existing biome-форматировании, не связанном с фичей
