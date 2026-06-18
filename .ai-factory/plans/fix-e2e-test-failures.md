# FIX: E2E Test Failures — 57 failed / 145 total

> Ветка: `fix/e2e-test-failures` | Дата: 2026-06-18

После прогона `npx playwright test --project=chromium` из 145 тестов 57 упало, 88 прошло (2.6m).

---

## Сводка коренных причин

| Группа | Падений | Коренная причина |
|--------|---------|------------------|
| **A: Auth-токен не установлен** | ~40 | `chat-window.spec.ts` и `navigation.spec.ts` не вызывают `setupAuth(page)` перед `goto('/')` → роутер редиректит на `/login` |
| **B: `selectOption` на кастомном VSelect** | 7 | `git-sync.spec.ts` вызывает `.selectOption()` на `<div>`-based кастомном дропдауне (`VSelect`) |

## Текущий статус

- [x] Task 1: setupAuth в chat-window.spec.ts
- [x] Task 2: setupAuth в navigation.spec.ts
- [x] Task 3: VSelect в git-sync.spec.ts
- [x] Task 4: ZIP-upload collection mock
- [x] Task 5: CSS shorthand assertions
- [x] Task 6: theme-switching setupAuth
| **C: ZIP upload без коллекции** | 4 | `zip-upload.spec.ts` ждёт `.dl-label`, но админ-панель не показывает документы без выбранной коллекции |
| **D: CSS shorthand assertions** | 2 | `message-bubble.spec.ts` использует `toHaveCSS("padding", ...)` — shorthand может быть пустым |
| **E: Мелочи (уже пофикшены)** | 4 | `rag-flow.spec.ts: __dirname` в ES module (исправлено), таймауты |

---

## Task 1: Добавить setupAuth в chat-window.spec.ts

**Файл:** `frontend/e2e/chat-window.spec.ts`

Добавить импорт `setupAuth` / `setupAdminAuth` / `VALID_TOKEN` из `./helpers` и вызвать в `beforeEach`.

**Логирование:** `DEBUG [e2e] auth setup added to chat-window beforeEach`

---

## Task 2: Добавить setupAuth в navigation.spec.ts

**Файл:** `frontend/e2e/navigation.spec.ts`

Добавить импорт `setupAuth` + `VALID_TOKEN` из `./helpers` и вызвать в `beforeEach` для тестов, требующих аутентификацию.

**Логирование:** `DEBUG [e2e] auth setup added to navigation beforeEach`

---

## Task 3: Исправить selectOption для VSelect в git-sync.spec.ts

**Файл:** `frontend/e2e/git-sync.spec.ts`

Заменить `collectionSelect.selectOption("col-1")` на последовательность:
1. Кликнуть триггер VSelect (кнопка/триггер)
2. Дождаться появления выпадающего списка
3. Кликнуть по нужному `<button>` с опцией

**Логирование:** `DEBUG [e2e] VSelect interaction: trigger click → option click`

---

## Task 4: Исправить ZIP-upload — установить коллекцию перед тестом

**Файл:** `frontend/e2e/zip-upload.spec.ts`

Перед `page.goto("/admin")` установить API-ключ через `page.addInitScript` (как в `rag-flow.spec.ts`) и загрузить моки для коллекции (GET /api/collections, чтобы admin panel отобразила документы).

**Логирование:** `DEBUG [e2e] zip-upload: mocking collections + setting API key`

---

## Task 5: Исправить CSS shorthand assertions в message-bubble.spec.ts

**Файл:** `frontend/e2e/message-bubble.spec.ts`

- Заменить `toHaveCSS("padding", expect.stringContaining("px"))` на проверку longhand-свойства: `toHaveCSS("padding-left", "2px")` или `padding-top`.
- Убедиться, что `background-color` для inline `<code>` применяется (проверить селектор / добавить fallback).

**Логирование:** `DEBUG [e2e] message-bubble: CSS shorthand → longhand assertion`

---

## Task 6: Исправить theme-switching — admin & chat page setupAuth

**Файл:** `frontend/e2e/theme-switching.spec.ts`

Добавить `setupAuth(page)` в `beforeEach` для тестов, которые заходят на `/` и `/admin` (секции Chat Page, Admin Page).

**Логирование:** `DEBUG [e2e] theme-switching: auth setup for chat/admin tests`

---

## Commit Plan

| Коммит | Задачи | Сообщение |
|--------|--------|-----------|
| 1 | Tasks 1, 2, 6 | fix(e2e): add auth setup to chat, navigation, and theme switching tests |
| 2 | Task 3 | fix(e2e): replace selectOption with click interaction for custom VSelect |
| 3 | Task 4 | fix(e2e): add collection mock and API key setup for ZIP upload tests |
| 4 | Task 5 | fix(e2e): replace CSS shorthand assertions with longhand properties |

---

## После исправлений

```bash
cd frontend
npx playwright test --project=chromium  # ожидается 0 failures
```
