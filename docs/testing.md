[← Deployment](deployment.md) · [Back to README](../README.md) · [Technical Spec →](technical-specification-rag-system.md)

# Тестирование

> Инструкция по ручному запуску тестов в тестовой среде.

## Содержание

1. [Обзор тестовой инфраструктуры](#обзор-тестовой-инфраструктуры)
2. [Подготовка локального окружения](#подготовка-локального-окружения)
3. [Тестовое окружение (Docker Compose)](#тестовое-окружение-docker-compose)
4. [Модульные тесты](#модульные-тесты)
5. [Интеграционные тесты](#интеграционные-тесты)
6. [E2E-тесты (Playwright)](#e2e-тесты-playwright)
7. [Полный прогон всех тестов](#полный-прогон-всех-тестов)
8. [CI-цели (аналог GitHub Actions локально)](#ci-цели-аналог-github-actions-локально)
9. [Форматирование и линтинг](#форматирование-и-линтинг)
10. [Полезные советы](#полезные-советы)

---

## Обзор тестовой инфраструктуры

Проект содержит три тестируемых сервиса и отдельные E2E-тесты:

| Сервис | Язык | Фреймворк | Модульные (без инфраструктуры) | Интеграционные (с БД, `tests/`) | Интеграционные (HTTP, `tests/`) | E2E |
|--------|------|-----------|:---:|:---:|:---:|:---:|
| **backend** | Rust | cargo test | ✅ `cargo test --lib` | ✅ `cargo test --test git_sync_unit`
  `cargo test --test documents_db_unit`
  `cargo test --test conversations_unit` | ✅ `cargo test --test integration`
  `cargo test --test *_integration` | — |
| **embedding** | Python | pytest | ✅ `pytest tests/` | — | — | — |
| **frontend** | Vue 3 / TS | Vitest / Playwright | ✅ `npm test` | — | — | ✅ `npm run test:e2e` |

---

## Подготовка локального окружения

Перед запуском тестов убедитесь, что все необходимые инструменты установлены.

### Backend (Rust)

Модульные тесты backend требуют только Rust toolchain. Интеграционные тесты дополнительно
требуют Docker Compose для PostgreSQL и других сервисов.

**Минимальные требования:**

- **Rust** — через [rustup](https://rustup.rs/). Версия синхронизирована с `rust-toolchain.toml`:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup show  # установит версию из rust-toolchain.toml
  ```
- **Git** — для клонирования репозиториев и работы `git2`:
  ```bash
  # Windows (winget):
  winget install --id Git.Git -e --source winget
  # macOS:
  brew install git
  # Linux (Debian/Ubuntu):
  sudo apt-get install git
  ```
- **Docker Desktop** — для тестового окружения (PostgreSQL, Chroma, LLM mock):
  - [Docker Desktop for Windows](https://docs.docker.com/desktop/setup/install/windows-install/)
  - [Docker Desktop for macOS](https://docs.docker.com/desktop/setup/install/mac-install/)
  - Linux: `sudo apt-get install docker-ce docker-ce-cli docker-compose-plugin`

**Проверка установки:**

```bash
rustc --version
cargo --version
docker --version
docker compose version
```

### Embedding (Python)

```bash
# Установить uv (рекомендуемый менеджер пакетов):
# Windows (powershell):
powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"

# macOS / Linux:
curl -LsSf https://astral.sh/uv/install.sh | sh

# Создать виртуальное окружение и установить зависимости (включая dev-зависимости для тестов):
cd embedding
uv sync --extra dev
```

> **Примечание:** Dev-зависимости (`pytest`, `pytest-asyncio`, `ruff`, `coverage`, `httpx`)
> объявлены в группе `[project.optional-dependencies] dev` в `pyproject.toml`.
> Используйте `--extra dev`, а не `--dev` — на uv 0.8.x флаг `--dev` не активирует
> опциональную группу. Команда `uv sync` без `--extra dev` устанавливает только
> production-зависимости.

### Frontend (Vue 3 / TypeScript)

```bash
# Установить Node.js (рекомендуется LTS):
#   https://nodejs.org/ (v20 или новее)
# Или через менеджер версий:
#   nvm install --lts    # macOS / Linux / WSL
#   nvm use --lts

# Установить зависимости:
cd frontend
npm ci
```

### Проверка готовности окружения

```bash
# Всё одной командой:
make check-tools
```

Если `make check-tools` не настроен, выполните вручную:

```bash
rustc --version && cargo --version && node --version && npm --version && uv --version && docker --version
```

---

## Тестовое окружение (Docker Compose)

Интеграционные и E2E-тесты требуют запущенной тестовой инфраструктуры. Для этого используется `docker-compose.test.yml`.

### Запуск тестового окружения

```bash
make test-env
```

Эта команда эквивалентна:

```bash
docker compose --env-file .env.test -f docker-compose.test.yml up -d
```

После запуска ожидается ~10 секунд для инициализации всех сервисов и выводится таблица их статуса.

### Состав тестового окружения

В `docker-compose.test.yml` поднимаются следующие сервисы:

| Сервис | Проброшенный порт (по умолчанию) | Назначение |
|--------|----------------------------------|------------|
| chroma | `18000` | Векторная база данных |
| embedding | `18081` | Сервис эмбеддингов |
| backend | `13000` | REST API (Rust/axum) |
| frontend | `15173` | SPA (Vue 3/Vite) |
| db | `15432` | PostgreSQL 16 |
| keycloak | `18080` | OIDC/OAuth2-провайдер |
| keycloak-init | — | Инициализация realm в KeyCloak (однократно) |
| llm-mock | — | Мок LLM для тестов |

### Остановка и очистка тестового окружения

```bash
make test-env-down
```

Останавливает все контейнеры и удаляет volume'ы (`-v`).

### Переменные окружения (.env.test)

Перед запуском тестового окружения необходимо настроить `.env.test` в корне проекта. Обязательные переменные:

| Переменная | Описание |
|-----------|----------|
| `LLM_API_KEY` | Ключ API для RouterAI (требуется backend) |
| `POSTGRES_PASSWORD` | Пароль суперпользователя PostgreSQL |
| `VEDO_DB_PASSWORD` | Пароль БД приложения |
| `KEYCLOAK_DB_PASSWORD` | Пароль БД KeyCloak |
| `KEYCLOAK_ADMIN_PASSWORD` | Пароль админа KeyCloak |
| `VEDO_BACKEND_CLIENT_SECRET` | Client secret для backend |
| `VEDO_ADMIN_PASSWORD` | Пароль тестового пользователя admin |
| `VEDO_ALICE_PASSWORD` | Пароль тестового пользователя alice |
| `VEDO_GUEST_PASSWORD` | Пароль тестового пользователя guest |

---

## Модульные тесты

Модульные тесты проверяют изолированные компоненты без внешних зависимостей. Они не требуют запуска тестового окружения (`make test-env`).

### Backend (Rust) — без инфраструктуры

Чистые unit-тесты, не требующие БД, Chroma или других сервисов — запускаются без какого-либо тестового окружения:

```bash
cd backend
cargo test --lib
```

Результат: **~49 тестов** (конфигурация, chunking, file validation, HMAC, token injection, LLM messages, chroma client mock, context window).

Опции:
- `cargo test --lib -- --nocapture` — показать stdout в консоли
- `cargo test --lib <фильтр>` — запустить только тесты, чьё имя содержит `<фильтр>`
  ```bash
  cargo test --lib documents
  ```

### Backend (Rust) — с PostgreSQL (DB round-trip)

Тесты, проверяющие работу с хранилищем (document upload, ZIP processing, soft-delete, batch delete,
reindex, git sync lock, conversation history). Они находятся в отдельных бинарниках в `tests/`
и **требуют работающего PostgreSQL**:

```bash
# 1. Запустить тестовое окружение (требуется только PostgreSQL)
make test-env

# 2. Запустить все DB round-trip тесты (последовательно)
cd backend
cargo test --test git_sync_unit -- --test-threads=1
cargo test --test documents_db_unit -- --test-threads=1
cargo test --test conversations_unit -- --test-threads=1
```

Эти тесты используют `common::setup_test_db()`, которая подключается к PostgreSQL,
накатывает миграции и чистит таблицы. Они не требуют Chroma, Embedding или других сервисов.

### Embedding (Python)

```bash
cd embedding
uv run pytest tests/ -v
```

Опции:
- `uv run pytest tests/ -v -k "test_embed"` — фильтрация по имени теста
- `uv run pytest tests/ -v --cov=src --cov-report=term` — с отчётом о покрытии

### Frontend (Vue 3 / TypeScript)

```bash
cd frontend
npm test
```

Команда выполняет `vitest run`. Опции:
- `npm run test:watch` — watch-режим (перезапуск при изменениях)
- `npx vitest run --reporter=verbose` — подробный вывод

Vitest настроен на:
- Окружение: `jsdom`
- Глобальные функции: `true`
- Подключение тестов: `src/**/*.spec.ts` и `src/**/*.test.ts`
- Исключение: `e2e/`

---

## Интеграционные тесты

Интеграционные тесты подключаются к реальным сервисам (Chroma, PostgreSQL, LLM mock) и **требуют запущенного тестового окружения**.

### Backend — интеграционные тесты (`tests/`)

Backend имеет три категории тестов в директории `tests/`:

1. **HTTP-интеграционные** — проверяют HTTP-эндпоинты через реальные запросы:
   `git_sync_integration`, `conversations_integration`, `auth_integration`, `integration`, `auth_middleware_test`
2. **DB round-trip** — проверяют репозитории и сервисы напрямую (без HTTP):
   `git_sync_unit`, `documents_db_unit`, `conversations_unit`
3. **Mock-тесты** — чисто логические тесты без БД (RED-спецификация):
   некоторые тесты в `conversations_unit` (контекстное окно, токены)

Все тесты, использующие `common::setup_test_db()`, подключаются к PostgreSQL, накатывают миграции и чистят таблицы.

```bash
# 1. Запустить тестовое окружение
make test-env

# 2. Запустить конкретную группу интеграционных тестов
cd backend
cargo test --test git_sync_integration -- --test-threads=1
```

**Важно:** Все тесты, использующие `common::setup_test_db()`, должны выполняться последовательно (`--test-threads=1`), так как `TRUNCATE ... CASCADE` сбрасывает все таблицы.

Интеграционные тесты покрывают:
- **Chroma CRUD** — создание, чтение, удаление коллекций
- **QueryRepository** — взаимодействие с Chroma через `ChromaClient`
- **Auth middleware** — проверка JWT-токенов
- **Conversations** — CRUD сессий и сообщений через HTTP
- **Git sync** — клонирование и синхронизация репозиториев
- **Documents** — загрузка, ZIP-пакеты, soft-delete, batch delete
- **RAG pipeline** — upload → chunk → embed → query

Настройка подключения (через переменные окружения):

```bash
CHROMA_URL=http://chroma:8000 \
  DATABASE_URL=postgres://vedo:password@db:5432/vedo \
  cargo test --test integration -- --test-threads=1
```

### Интеграционные тесты с `#[ignore]`

Некоторые тесты в `tests/` помечены `#[ignore]` — они представляют собой RED-спецификацию для ещё не реализованных фич (фаза executable specification). Для их запуска:

```bash
cargo test --test git_sync_integration -- --ignored
```

## E2E-тесты (Playwright)

E2E-тесты проверяют работу приложения через браузер. Они запускаются в специальном контейнере `frontend-tests` внутри тестовой сети Docker.

### Запуск E2E-тестов

```bash
# 1. Запустить тестовое окружение
make test-env

# 2. Запустить E2E-тесты
make test:e2e
```

Команда `make test:e2e` выполняет:

```bash
docker compose --env-file .env.test -f docker-compose.test.yml \
  --profile test-runner run --rm frontend-tests
```

Контейнер `frontend-tests`:
- Использует образ `mcr.microsoft.com/playwright:v1.61.0-noble`
- Устанавливает зависимости через `npm ci`
- Запускает `npm run test:e2e` (что эквивалентно `playwright test`)
- Подключается к тестовым сервисам по внутренней сети Docker

### Локальный запуск E2E (без Docker)

Для отладки можно запустить Playwright локально:

```bash
cd frontend
# Убедиться, что тестовое окружение запущено
npm run test:e2e
# С UI-режимом для отладки:
npm run test:e2e:ui
# С debug-режимом:
npm run test:e2e:debug
```

Playwright настроен на:
- Браузер: Chromium
- Workers: 1 (последовательный прогон)
- Retries: 0 (локально), 2 (в CI)
- Базовый URL: `http://localhost:5173`
- Репортеры: HTML (`playwright-report/`) и list

### Состав E2E-тестов

| Файл | Что проверяет |
|------|---------------|
| `login.spec.ts` | Аутентификация через KeyCloak |
| `chat-window.spec.ts` | Чат-интерфейс и ввод сообщений |
| `message-bubble.spec.ts` | Отрисовка сообщений и источников |
| `navigation.spec.ts` | Роутинг и адаптивная вёрстка |
| `rag-flow.spec.ts` | Полный RAG-флоу: загрузка → запрос → источники |
| `avatar.spec.ts` | Компонент аватара |
| `theme-switching.spec.ts` | Переключение тёмной/светлой темы |
| `auth-regression.spec.ts` | Регрессия авторизации |
| `chat-edit-delete.spec.ts` | Редактирование и удаление сообщений |
| `chat-export.spec.ts` | Экспорт истории чата |
| `document-reindexing.spec.ts` | Переиндексация документов |
| `git-sync.spec.ts` | Синхронизация git-репозиториев |
| `loading-skeletons.spec.ts` | Скелетоны загрузки |
| `zip-upload.spec.ts` | Загрузка ZIP-архивов |

---

## Полный прогон всех тестов

### Быстрый запуск (через Makefile)

```bash
make test
```

Эта команда последовательно запускает:

```bash
cd backend && cargo test --lib                                      # модульные тесты backend (без инфраструктуры)
cd backend && cargo test --test integration -- --test-threads=1     # HTTP-интеграционные тесты backend
cd frontend && npm test                                             # модульные тесты frontend
cd embedding && pytest tests/ -v                                    # модульные тесты embedding
```

### Полная проверка с тестами на БД

```bash
make test-env

# DB round-trip тесты (только PostgreSQL)
cargo test --test git_sync_unit -- --test-threads=1                 # git sync repository + service
cargo test --test documents_db_unit -- --test-threads=1             # document upload, ZIP, soft-delete
cargo test --test conversations_unit -- --test-threads=1            # conversations repository + context

# HTTP-интеграционные тесты (все сервисы)
cargo test --test git_sync_integration -- --test-threads=1          # интеграционные git-sync
cargo test --test conversations_integration -- --test-threads=1     # интеграционные conversations
cargo test --test auth_integration -- --test-threads=1              # интеграционные auth

make test:e2e                                                        # E2E-тесты
make test-env-down
```

### Полная проверка (форматирование + линтинг + тесты)

```bash
make check
```

Эквивалентно `make format && make lint && make test`.

---

## CI-цели (аналог GitHub Actions локально)

Эти цели дублируют шаги из GitHub Actions CI, позволяя прогнать ту же проверку локально.

### Backend

```bash
make ci-backend
```

Выполняет: `cargo fmt --check && cargo clippy -- -D warnings && cargo test --lib && cargo test --test integration -- --test-threads=1`

### Embedding

```bash
make ci-embedding
```

Выполняет: `ruff format src/ --check && ruff check src/ && uv run pytest tests/ -v --cov=src --cov-report=term`

### Frontend

```bash
make ci-frontend
```

Выполняет: `npm run lint:ci && npm run format:check && npm run test -- --run && npm run build`

---

## Форматирование и линтинг

Перед запуском тестов рекомендуется проверить форматирование и линтинг. Каждый сервис имеет свою конфигурацию:

| Сервис | Форматирование | Линтинг |
|--------|---------------|---------|
| **backend** | `cargo fmt` | `cargo clippy -- -D warnings` |
| **embedding** | `ruff format src/` | `ruff check src/` |
| **frontend** | `npx biome format --write .` | `npx biome check .` |

Быстрый запуск через Makefile:

```bash
make format   # форматирование всех сервисов
make lint     # линтинг всех сервисов
```

---

## Полезные советы

1. **Порядок запуска:** E2E-тесты требуют запущенного тестового окружения (`make test-env`). Не забудьте остановить его после завершения (`make test-env-down`).

2. **Параллельный запуск dev и test:** Тестовое окружение использует отдельные порты (например, `18000` для Chroma, `13000` для backend), поэтому dev-окружение (`docker compose up`) и test-окружение могут работать одновременно.

3. **Покрытие кода:** Для генерации отчётов о покрытии используйте `make coverage`. Требуется `cargo-tarpaulin` для Rust (устанавливается через `cargo install cargo-tarpaulin`).

4. **Smoke-тест:** Перед запуском полного набора тестов можно выполнить smoke-тест:

    ```bash
    make smoke
    ```

    Smoke-тест поднимает все сервисы и проверяет их health endpoints.

5. **Проверка логов тестового окружения:**

    ```bash
    docker compose --env-file .env.test -f docker-compose.test.yml logs -f backend
    ```

6. **Запуск одного E2E-теста:** Для запуска конкретного файла внутри контейнера можно изменить команду:

    ```bash
    docker compose --env-file .env.test -f docker-compose.test.yml \
      --profile test-runner run --rm frontend-tests \
      /bin/sh -c "npm ci && npx playwright test e2e/login.spec.ts"
    ```

7. **ChatOps в Makefile:** `make help` покажет все доступные цели.

## See Also

- [Getting Started](getting-started.md) — установка и первый запуск
- [Configuration](configuration.md) — переменные окружения
- [Deployment](deployment.md) — развёртывание на продакшн
