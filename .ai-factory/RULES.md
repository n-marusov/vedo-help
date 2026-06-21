# Project Rules

> Short, actionable rules and conventions for this project. Loaded automatically by /aif-implement.

## Rules

- Always run `npm run ai:validate` after any implementation task and verify it exits with code 0
- Follow strict TDD ordering for feature work: all e2e, integration, and unit tests/specs must be written in the first plan phases before any production implementation tasks begin
- Treat tests as executable specification: implementation agents must read and satisfy the new tests before changing production code, and must not reorder implementation ahead of test-writing tasks
- When planning with checkboxes, group every test-writing task before schema, backend, frontend, docs, or validation implementation tasks
- Use optimistic UI as a general UX principle: the UI must feel instant — eagerly update local state before the API responds, execute the mutation asynchronously, show a VToast confirmation on success, and roll back the local state with a VToast error on failure
- Always lint-format staged .vue files manually before any frontend git commit — Lefthook biome glob `*.{js,ts,...}` excludes `.vue`, so .vue files are silently skipped by the pre-commit hook
