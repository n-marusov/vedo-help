# Project Rules

> Short, actionable rules and conventions for this project. Loaded automatically by /aif-implement.

## Rules

- Always run `npm run ai:validate` after any implementation task and verify it exits with code 0
- Follow strict TDD ordering for feature work: all e2e, integration, and unit tests/specs must be written in the first plan phases before any production implementation tasks begin
- Treat tests as executable specification: implementation agents must read and satisfy the new tests before changing production code, and must not reorder implementation ahead of test-writing tasks
- When planning with checkboxes, group every test-writing task before schema, backend, frontend, docs, or validation implementation tasks
