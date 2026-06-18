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

## Docs / rules
- `AGENTS.md` актуален (структура, ключевые файлы)
- Нет shell-команд через `&&`/`||`/`;` в инструкциях и коммитах
- `npm run format:check` может падать на pre-existing biome-форматировании, не связанном с фичей
