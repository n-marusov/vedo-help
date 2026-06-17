# Implementation Plan: Markdown & Code Rendering

Branch: feature/markdown-code-rendering
Created: 2026-06-17

## Settings
- Testing: yes (TDD — test-first approach)
- Logging: verbose (DEBUG logs for development)
- Docs: yes (mandatory docs checkpoint)

## Roadmap Linkage
Milestone: "v0.2.1 — Markdown & Code Rendering"
Rationale: Этот план реализует полноценный рендеринг Markdown, подсветку синтаксиса и кнопку копирования кода — единственную задачу вехи v0.2.1.

## Requirements Summary

Текущее состояние:
- `MessageBubble.vue` использует `marked.parse()` (sync mode) для Markdown → HTML
- `highlight.js` v11.9 уже в `package.json` как dependency, но **не импортируется** и не используется
- Нет подсветки синтаксиса — блоки кода имеют только тёмный фон (`#12122a`)
- Нет кнопки копирования кода
- Нет языковых меток на блоках кода
- Нет поддержки GFM-таблиц (или других расширений markdown)
- `marked` используется с настройками по умолчанию, без GFM

Цель:
1. Подключить `highlight.js` для автоматического определения языка и подсветки синтаксиса
2. Добавить кнопку "Copy" на каждый блок кода
3. Добавить языковую метку (`lang-*`) на блоки кода
4. Настроить `marked` на GFM, async safe
5. Стилизовать блоки кода, inline-код и таблицы через design tokens
6. Покрыть изменения unit- и e2e-тестами

## Commit Plan
- **Commit 1** (after Task 1): "feat: configure marked with GFM and highlight.js integration"
- **Commit 2** (after Tasks 2-3): "feat: add syntax highlighting, copy button, and language labels to code blocks"
- **Commit 3** (after Tasks 4-5): "test: unit and E2E tests for markdown and code rendering"
- **Commit 4** (after Task 6): "docs: update CHECKLIST.md and document markdown rendering"

## Tasks

### Phase 1: Markdown Engine & Syntax Highlighting

- [ ] **Task 1: Upgrade marked configuration and integrate highlight.js**

  **Deliverable:** Настроить `marked` с GFM и интегрировать `highlight.js` для подсветки синтаксиса в `MessageBubble.vue`.

  **Changes:**
  - `frontend/src/components/MessageBubble.vue`:
    - Импортировать `highlight.js` (`import hljs from 'highlight.js'`)
    - Создать отдельный модуль `frontend/src/utils/markdown.ts` с конфигурацией marked:
      - `marked.use({ gfm: true, breaks: true })`
      - Кастомный `renderer.code` для подсветки через `hljs.highlightAuto()`
      - fallback на `escapeHtml()` если подсветка не удалась
    - В `MessageBubble.vue` заменить прямой `marked.parse()` на вызов из нового модуля
    - Добавить CSS для подсветки синтаксиса (тёмная тема highlight.js или кастомные токены)
    - Обновить `renderedContent` computed — использовать асинхронный `marked.parse()` с обработкой через `async/await` (watch effect вместо computed)

  **TDD approach:**
  1. Написать unit-тест для `markdown.ts` (проверка: код с тегом ```` ```python ```` получает подсветку)
  2. Написать unit-тест для `MessageBubble.vue` (проверка: `<pre>` содержит `hljs`-классы)
  3. Реализовать функциональность

  **LOGGING REQUIREMENTS:**
  - `[markdown]` DEBUG: log markdown parse start/finish with content length
  - `[markdown]` WARN: log highlight.js failures with detected language attempt
  - `[MessageBubble]` DEBUG: log when renderedContent is recomputed (content length, block count)

  **Files:**
  - `frontend/src/utils/markdown.ts` (новый)
  - `frontend/src/components/MessageBubble.vue` (изменения)
  - `frontend/src/components/__tests__/MessageBubble.spec.ts` (дополнение)
  - `frontend/src/utils/__tests__/markdown.spec.ts` (новый)

  **Dependencies:** нет

### Phase 2: Code Block Enhancements (Copy Button & Language Labels)

- [ ] **Task 2: Add language label to code blocks**

  **Deliverable:** Каждый блок кода должен отображать язык подсветки в виде метки (`Python`, `JavaScript`, `Rust` и т.д.).

  **Changes:**
  - В кастомном `renderer.code` в `markdown.ts` добавить `<div class="code-header">` с языковой меткой
  - Языковая метка берётся из класса `language-*` или результата `hljs.highlightAuto()`
  - Добавить CSS для `.code-header` и `.code-lang-label` в `MessageBubble.vue` через design tokens
  - Если язык не определён — показывать "Code"

  **TDD approach:**
  1. Написать unit-тест: код ```` ```rust ```` → в рендере есть `.code-lang-label` с текстом "Rust"
  2. Написать unit-тест: код без явного языка → метка "Code"
  3. Реализовать

  **LOGGING REQUIREMENTS:**
  - `[markdown]` DEBUG: log detected language for each code block
  - `[markdown]` INFO: log when language is auto-detected vs explicitly set

  **Files:**
  - `frontend/src/utils/markdown.ts` (изменения)
  - `frontend/src/components/MessageBubble.vue` (добавить CSS)
  - `frontend/src/components/__tests__/MessageBubble.spec.ts` (дополнение)

  **Dependencies:** Task 1

- [ ] **Task 3: Add copy-to-clipboard button on code blocks**

  **Deliverable:** Каждый блок кода должен иметь кнопку "Copy", которая копирует содержимое в буфер обмена.

  **Changes:**
  - В кастомном `renderer.code` добавить `<button class="code-copy-btn">Copy</button>`
  - После нажатия: текст меняется на "Copied!" на 2 секунды, затем возвращается на "Copy"
  - Использовать `navigator.clipboard.writeText()`
  - Добавить CSS для `.code-copy-btn` — абсолютное позиционирование в `.code-header`
  - Обработать ошибки clipboard API: fallback с `console.warn` и показывать "Failed" на кнопке

  **TDD approach:**
  1. Написать unit-тест: кнопка Copy присутствует в рендере блока кода
  2. Написать unit-тест: после клика текст кнопки меняется на "Copied!"
  3. Написать unit-тест: через 2 секунды после клика текст возвращается на "Copy"
  4. Реализовать

  **LOGGING REQUIREMENTS:**
  - `[MessageBubble]` INFO: log copy action with code block language and content length
  - `[MessageBubble]` WARN: log clipboard API failures
  - `[MessageBubble]` DEBUG: log copy button state transitions (Copy → Copied! → Copy)

  **Files:**
  - `frontend/src/utils/markdown.ts` (изменения в `renderer.code`)
  - `frontend/src/components/MessageBubble.vue` (CSS)
  - `frontend/src/components/__tests__/MessageBubble.spec.ts` (дополнение)

  **Dependencies:** Task 2

### Phase 3: Markdown Rendering Polish

- [ ] **Task 4: Enhance markdown rendering and styling with design tokens**

  **Deliverable:** Улучшить рендеринг Markdown: GFM-таблицы, стили для списков, blockquotes, горизонтальных линий через design tokens.

  **Changes:**
  - Настроить `marked` на GFM (уже в Task 1): таблицы, зачёркивание, авто-ссылки
  - Обновить CSS в `MessageBubble.vue`:
    - `table` — стили для GFM-таблиц (бордеры, `thead`, `tbody`, чередование строк)
    - `blockquote` — левая граница + отступ
    - `ul/ol` — отступы, маркеры/цифры
    - `hr` — стилизованная линия
    - `h1`-`h6` — пропорциональные размеры
    - `img` — max-width, border-radius
  - Все цвета — через CSS custom properties (design tokens)
  - Добавить поддержку `chat-tokens.css` для code-специфичных переменных

  **TDD approach:**
  1. Написать unit-тест: таблица `| A | B |` → рендерится как `<table>` с `<thead>` и `<tbody>`
  2. Написать unit-тест: blockquote → `<blockquote>` с корректными стилями
  3. Написать unit-тест: список `- item` → `<ul>` с `<li>`
  4. Реализовать

  **LOGGING REQUIREMENTS:**
  - `[MessageBubble]` DEBUG: log when markdown body contains tables/blockquotes

  **Files:**
  - `frontend/src/components/MessageBubble.vue` (scoped CSS)
  - `frontend/src/assets/chat-tokens.css` (добавить токены для code)
  - `frontend/src/components/__tests__/MessageBubble.spec.ts` (дополнение)

  **Dependencies:** Task 1

### Phase 4: Testing

- [ ] **Task 5: Add E2E tests for syntax highlighting, copy button, and markdown features**

  **Deliverable:** Playwright E2E-тесты для проверки подсветки кода, кнопки Copy и Markdown-элементов.

  **Changes:**
  - `frontend/e2e/message-bubble.spec.ts` — добавить тесты:
    - `TC-CODE-001`: code block renders with syntax highlighting classes (`.hljs`)
    - `TC-CODE-002`: code block has language label
    - `TC-CODE-003`: copy button is visible on code blocks
    - `TC-CODE-004`: copy button copies content to clipboard
    - `TC-CODE-005`: copy button shows "Copied!" state
    - `TC-MD-001`: table renders with correct HTML structure
    - `TC-MD-002`: blockquote renders with correct styling
    - `TC-MD-003`: inline code has distinct background

  **Files:**
  - `frontend/e2e/message-bubble.spec.ts` (изменения)

  **Dependencies:** Tasks 1-4 (full implementation done)

### Phase 5: Documentation & Cleanup

- [ ] **Task 6: Update documentation and CHECKLIST.md**

  **Deliverable:** Обновить документацию, отразить изменения в AGENTS.md, проверить CHECKLIST.md.

  **Changes:**
  - `docs/gui.md` — добавить раздел о рендеринге Markdown и кода
  - `AGENTS.md` — обновить структуру (добавить `frontend/src/utils/markdown.ts`)
  - `CHECKLIST.md` — проверить все gates (включая `npm run ai:validate`)
  - Убедиться, что `.ai-factory/ROADMAP.md` отражает статус `[x]` для v0.2.1

  **Files:**
  - `docs/gui.md` (изменения)
  - `AGENTS.md` (изменения)
  - `CHECKLIST.md` (проверка)
  - `.ai-factory/ROADMAP.md` (обновление)

  **Dependencies:** Tasks 1-5
