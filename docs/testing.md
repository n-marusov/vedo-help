[← Deployment](deployment.md) · [Back to README](../README.md) · [Technical Spec →](technical-specification-rag-system.md)

# Тестирование

> Инструкция по ручному запуску тестов в тестовой среде.

## Содержание

1. [Обзор тестовой инфраструктуры](#обзор-тестовой-инфраструктуры)
2. [Тестовое окружение (Docker Compose)](#тестовое-окружение-docker-compose)
3. [Модульные тесты](#модульные-тесты)
4. [Интеграционные тесты](#интеграционные-тесты)
5. [E2E-тесты (Playwright)](#e2e-тесты-playwright)
6. [Полный прогон всех тестов](#полный-прогон-всех-тестов)
7. [CI-цели (аналог GitHub Actions локально)](#ci-цели-аналог-github-actions-локально)
8. [Форматирование и линтинг](#форматирование-и-линтинг)
9. [Полезные советы](#полезные-советы)

---

## Обзор тестовой инфраструктуры

Проект содержит три тестируемых сервиса и отдельные E2E-тесты:

| Сервис | Язык | Фреймворк тестов | Модульные | Интеграционные | E2E |
|--------|------|-------------------|-----------|----------------|-----|
| **backend** | Rust | cargo test | ✅ `cargo test --lib` | ✅ `cargo test --test integration` | — |
| **embedding** | Python | pytest | ✅ `pytest tests/` | — | — |
| **frontend** | Vue 3 / TS | Vitest / Playwright | ✅ `npm test` | — | ✅ `npm run test:e2e` |

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

Модульные тесты не требуют запуска тестового окружения и выполняются изолированно.

### Backend (Rust)

```bash
cd backend
cargo test --lib
```

Опции:
- `cargo test --lib -- --nocapture` — показать stdout в консоли
- `cargo test --lib <фильтр>` — запустить только тесты, чьё имя содержит `<фильтр>`
  ```bash
  cargo test --lib documents
  ```

### Embedding (Python)

```bash
cd embedding
pytest tests/ -v
```

Опции:
- `pytest tests/ -v -k "test_embed"` — фильтрация по имени теста
- `pytest tests/ -v --coverage --cov=src --cov-report=term` — с отчётом о покрытии

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

Интеграционные тесты подключаются к реальным сервисам (Chroma, PostgreSQL) и **требуют запущенного тестового окружения**.

### Backend — интеграционные тесты

```bash
# 1. Запустить тестовое окружение
make test-env

# 2. Запустить интеграционные тесты
cd backend
cargo test --test integration -- --test-threads=1
```

**Важно:** Интеграционные тесты должны выполняться последовательно (`--test-threads=1`), так как они используют общую базу данных и операция `TRUNCATE ... CASCADE` сбрасывает все таблицы.

Интеграционные тесты проверяют:
- **Chroma CRUD** — создание, чтение, удаление коллекций в векторной БД
- **Репозиторий QueryRepository** — взаимодействие с Chroma через `ChromaClient`
- **Auth middleware** — проверка JWT-токенов
- **Conversations** — CRUD для сессий и сообщений
- **Git sync** — клонирование и синхронизация репозиториев

Настройка подключения (через переменные окружения):

```bash
CHROMA_URL=http://chroma:8000 \
  DATABASE_URL=postgres://vedo:password@db:5432/vedo \
  cargo test --test integration -- --test-threads=1
```

---

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
cd backend && cargo test --lib              # модульные тесты backend
cd backend && cargo test --test integration -- --test-threads=1  # интеграционные тесты backend
cd frontend && npm test                     # модульные тесты frontend
cd embedding && pytest tests/ -v            # модульные тесты embedding
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

Выполняет: `ruff format src/ --check && ruff check src/ && pytest tests/ -v --cov=src --cov-report=term`

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
