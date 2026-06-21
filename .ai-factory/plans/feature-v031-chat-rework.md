# Implementation Plan: v0.3.1 — Basic Q&A Logic & Chat Rework

Branch: `feature/v031-chat-rework`
Created: 2026-06-21
Linked Milestone: v0.3.1 — Basic Q&A Logic & Chat Rework

## Settings

- **Testing:** yes — strict TDD (per `AGENTS.md`): all e2e/integration/unit test-writing tasks run in early phases before any production implementation. Implementation must read the new tests first and satisfy their behavior.
- **Logging:** verbose — DEBUG-level `tracing` logs for backend flow, structured `[module.method]` format; DEBUG `console` for frontend during development, env-controlled.
- **Docs:** yes — mandatory docs checkpoint in `/aif-implement`: route docs updates through `/aif-docs` (touch `docs/api.md`, `docs/gui.md`, `docs/configuration.md`).
- **UI source of truth:** all frontend UI for this plan MUST be derived from the Pencil `.pen` design files in `design/`. The implementation agent MUST load the `pencil-design` skill at the start of any frontend UI task and follow its rules (reuse design-system components, use design tokens not hardcoded values, visually verify). See the "UI Design Source Files" constraint below for the strict mapping.
- **Frontend skill precondition:** before any frontend task in Phase 3, the agent invokes `skill(pencil-design)` and reads the relevant `.pen` files listed there. Do not write new UI atom / variant / visual style in code that contradicts these designs; if a design is missing, design it in the appropriate `.pen` file first (e.g. extend `chat.pen` with edit/delete hover row, export toolbar button, loading skeleton variants), then generate code from it.

## Roadmap Linkage

- **Milestone:** "v0.3.1 — Basic Q&A Logic & Chat Rework"
- **Rationale:** Scope for this plan = the four remaining incomplete v0.3.1 roadmap items (message editing & deletion, context management, chat export UI, empty state & loading skeletons). Completing these moves the milestone from 2/6 to 6/6.

## Scope (4 v0.3.1 items)

| # | Roadmap item | Area | Status before plan |
|---|--------------|------|--------------------|
| 1 | Message editing & deletion | backend + frontend | `[ ]` no API, no UI |
| 2 | Context management (sliding window + token budget) | backend | `[ ]` full history sent to LLM |
| 3 | Chat export (JSON/Markdown; button in UI) | backend (Markdown) + frontend (button) | `[~]` backend JSON-ready, no Markdown, no UI |
| 4 | Empty state & loading skeletons | frontend | `[~]` basic empty states exist; no skeletons |

## Recon Findings (synthesized from codebase exploration)

### Backend

- `backend/src/modules/conversations/models.rs` — `Message { id, session_id, role, content, sources, created_at }`. No `edited_at`, `deleted_at`, `original_content`. No `UpdateMessageRequest`.
- `backend/src/modules/conversations/repository.rs` — `ConversationRepository` with `add_message`, `get_messages` (no soft-delete filter, no LIMIT), `get_message_count` (N+1 source of bugs after soft-delete). Missing: `get_message(id)`, `update_message(id, content)`, `soft_delete_message(id)`.
- `backend/src/modules/conversations/handlers.rs` — routes wired in `main.rs:220-240`: `GET/POST/DELETE /api/sessions`, `GET /api/sessions/:id`, `DELETE /api/sessions/:id`, `GET /api/sessions/:id/export` (JSON only). **No** `PATCH/DELETE /api/sessions/:id/messages/:mid`.
- `backend/src/modules/conversations/service.rs:103` — `export_session` returns `serde_json::Value` JSON only. No markdown serializer.
- `backend/src/modules/query/service.rs:180-205` — `load_conversation_history` runs `SELECT role, content FROM messages WHERE session_id = $1 ORDER BY created_at ASC` unbounded. Passes to `llm_client.query_stream` at L132. **Single insertion point for context window trimming.**
- `backend/src/shared/llm.rs:21-24` — `Message { role: String, content: String }`; `build_messages` (L114-149) drops history verbatim into the OpenRouter request. No `max_tokens` field in request body.
- `backend/src/config.rs` — env: `OPENROUTER_API_KEY`, `OPENROUTER_BASE_URL`, `OPENROUTER_MODEL`. **No** `LLM_MAX_HISTORY_MESSAGES`, `LLM_CONTEXT_TOKEN_BUDGET`, `LLM_MAX_TOKENS`. Needs additions.
- `backend/migrations/` — latest = `00000000000006_create_git_repositories.sql`. Next migration filename: `00000000000007_<name>.sql`. Existing precedent: chunks table uses `is_active` boolean (migration 00000000000003).
- `backend/tests/integration.rs` is Chroma-only; **no conversations integration test exists** — must establish the pattern (mirror `auth_integration.rs`'s `TestApp` setup).
- `messages.role` has CHECK `('user', 'assistant')` (migration 5); `messages.session_id` has `ON DELETE CASCADE` — safe for per-message DELETE.
- `query/repository.rs:72` already filters `AND c.is_active = TRUE` for chunks — follow this precedent for `messages.deleted_at IS NULL`.

### Frontend

- `frontend/src/components/MessageBubble.vue` — no edit/delete affordance; only Sources toggle and per-code-block copy button. User vs assistant distinguished via `message.role`. Hover action pattern to copy: `ChatView.vue:225-231` (`.session-item-delete` revealed on `.session-item:hover` with CSS opacity transition at `ChatView.vue:522-528`).
- `frontend/src/stores/chat.ts:12-17` — state `{ messages, isLoading, activeSessionId, sessions, error }`. Actions: `sendMessage` (raw `fetch('/api/query')` at L111-116), `cancelStream`, `fetchSessions`, `createSession`, `deleteSession`, `loadSession` (GETs `/sessions/:id/messages` then sets `messages.value`), `clearMessages`. **Missing:** `editMessage`, `deleteMessage`, `exportSession`, `isSessionLoading` flag.
- Temp-ID leak: `sendMessage` optimistically pushes `temp-${Date.now()}` user + `temp-assist-${Date.now()}` assistant; after `done` event the IDs are not reconciled with server-assigned IDs. `editMessage`/`deleteMessage` by `id` will break until this is fixed.
- `frontend/src/api/client.ts:44-81` — `api` object exports `get/post/del/batchDeleteDocuments` + Git Sync wrappers + `upload`. **Missing:** `exportSession`, `editMessage`, `deleteMessage`. Streaming path bypasses the api object.
- `frontend/src/api/types.ts:53-60` — `Message { id, session_id, role, content, sources, created_at }`. **Missing:** optional `edited_at`. No `EditMessageRequest` or `SessionExport` type.
- `frontend/src/components/ui/` — has VButton, VAvatar, VBadge, VDialog, VDropZone, VInput, VLabel, VProgressBar, VSelect, VThemeToggle, VToast. **Missing:** `VSkeleton.vue`.
- Loading states are inline text-only (`session-empty`, `dl-empty`, `grm-empty`, welcome screen). Welcome screen shows during `loadSession` until `messages.length > 0` — flicker bug.
- Tests: Vitest pattern `frontend/src/**/__tests__/{PascalCase}.spec.ts`, includes `src/**/*.spec.ts` + `src/**/*.test.ts`, excludes `e2e/**`; jsdom env. Playwright e2e at `frontend/e2e/{kebab-case}.spec.ts`, config at `frontend/playwright.config.ts`. Existing `MessageBubble.spec.ts` uses `vi.hoisted()` mocks + factory fixtures. Existing `e2e/rag-flow.spec.ts` shows e2e pattern using `helpers.ts` (`setupAuthAndCollection`, `setActiveCollection`).
- **`frontend/src/stores/__tests__/chat.spec.ts` does NOT exist** — chat store is the only store without unit tests; creating it is in scope as part of TDD.

## Test-First Policy (per `AGENTS.md`)

Strict order:

1. **Phase 1 — Red:** Write all e2e + integration + unit tests for ALL four features. Compile/runtime may fail; the tests are executable specification for Phase 2–3 implementation agents to satisfy.
2. **Phase 2 — Backend Impl (satisfy Red):** Implement migration → repository → service → handlers → routes → context window → markdown export. Make backend tests green.
3. **Phase 3 — Frontend Impl (satisfy Red):** Add `VSkeleton`, message edit/delete UI, store actions, export button, skeleton integration, temp-ID reconciliation. Make frontend tests green.
4. **Phase 4 — Docs & Validation:** API/gui/configuration docs, roadmap updates, lint+format+validate.

Implementation agents MUST read the new tests before code, and must NOT reorder schema/backend/frontend implementation ahead of test-writing tasks.

## Constraints & Decisions

- **Soft-delete model:** mirror `chunks.is_active` precedent using `deleted_at TIMESTAMPTZ NULL` (NULL = live). Add `edited_at TIMESTAMPTZ NULL` and `original_content TEXT NULL`. Update `get_messages`, `get_message_count`, `load_conversation_history`, `list_sessions` count query, and `export_session` to filter `deleted_at IS NULL`.
- **Editing UX:** connects with leading message in segment, `original_content` preserves audit trail. Editing is only allowed on USER messages (own messages); assistant messages can be deleted but not edited (consistent with DeepSeek-style UI).
- **Temp-ID reconciliation:** SSE `done` event MUST include the server-assigned user message ID and assistant message ID. The store replaces temp IDs in `messages.value`. This unblocks `editMessage`/`deleteMessage` by ID. Backend change in `query/handlers.rs` SSE done payload; frontend pinia store patch path.
- **Context trimming:** simple tokenizer (no heavy dependency). Choose approach: (a) word-count-based budget (`content.split_whitespace().count()` as proxy) — easy; or (b) integrate `tiktoken-rs` for accurate OpenAI-style tokens. **Decision: option (a)** for v0.3.1 — simpler, no new Rust dep, documented limitation. Revisit in v0.5 Advanced RAG if quality suffers.
- **Sliding window policy:** when over budget, drop oldest non-system message pairs user+assistant together until under budget. Keep at least the 2 most recent messages (1 turn) regardless. Configurable via `LLM_MAX_HISTORY_MESSAGES` (default 20) and `LLM_CONTEXT_TOKEN_BUDGET` (default 6000).
- **Export format selection:** `GET /api/sessions/:id/export?format={json|markdown}`. Default `json` for backward compat. Markdown uses session title as H1, per-message H2 `## ${role} · ${created_at}` followed by content. Soft-deleted messages excluded.
- **VSkeleton API:** prop `variant: 'text' | 'circle' | 'card'`, prop `rows: number` (for stacked line skeleton), CSS shimmer animation reusing `chat-tokens.css` animation tokens, respects dark/light theme via existing CSS variables.
- **Skeleton placement precedence:** chat messages area during `loadSession` first (most visible flicker), then sessions sidebar, then `DocumentList` during `fetchDocuments`, then `GitRepoManager` repos list.
- **Tests require:** conversations integration tests will need a fresh test-migration fixture (mirror `auth_integration.rs` TestApp that boots axum with in-memory `PgPool` or test DB). If no test DB is configured already, document test preconditions explicitly in the task description (per skill-context rule about exact preconditions).
- **Docker inter-service URLs:** no new docker compose wiring required (no new services). Existing network scope matrix (browser/public, host-local, Docker-internal) applies unchanged.

### UI Design Source Files (MANDATORY)

The `design/` directory contains 5 Pencil `.pen` files — these are the source of truth for the frontend UI in this plan. The implementation agent MUST:

1. Load the `pencil-design` skill (`skill(pencil-design)`) before any frontend UI task in Phase 3 — its 6 Critical Rules (reuse design-system components, use variables not hardcoded values, prevent overflow, visually verify, reuse existing assets, load `frontend-design`-style aesthetic skill) are binding.
2. Use `ui-kit.lib.pen` as the design-system library — every reusable atom (buttons, inputs, dialogs, badges, avatars, etc.) MUST be sourced from here via `pencil_batch_get` with `patterns: [{ reusable: true }]` and inserted as instances, never recreated from scratch.
3. Follow the strict mapping below per task. If a needed surface does not yet exist in the design files, the agent first designs it in the appropriate `.pen` file (extending the existing chat/admin/dialogs conventions), visually verifies via `pencil_get_screenshot` + `pencil_snapshot_layout`, then generates the production code.

| Plan task | Primary `.pen` source | What to extract/extend |
|-----------|----------------------|------------------------|
| T9 — `VSkeleton.vue` | `ui-kit.lib.pen` | Add a `VSkeleton` component to the kit (new atom); use existing `chat-tokens.css` `--animation-*` tokens and the kit's variable layer for shimmer. Look at existing `VProgressBar` as a sibling pattern. |
| T12 — `MessageBubble` edit/delete hover row | `chat.pen` | Extend the existing message bubble design with the hover action row (edit + delete buttons for user messages; delete-only for assistant), edit-mode textarea + Save/Cancel buttons, and the `· edited` badge. Use button instances from `ui-kit.lib.pen`. |
| T13 — `ChatView` export toolbar button + format `<VSelect>` + messages-area skeleton | `chat.pen` (toolbar layout), `ui-kit.lib.pen` (VButton/VSelect instances) | Add an "Export" `VButton` (ghost variant) and a format `VSelect` to the chat toolbar's right slot, matching the existing top-bar composition. Add the messages-area skeleton placeholder using the `VSkeleton` from T9. |
| T14 — Skeletons in sessions sidebar, DocumentList, GitRepoManager | `chat.pen` (sessions sidebar), `admin.pen` (DocumentList, GitRepoManager layouts) | Replace the existing inline text-only loaders with `VSkeleton` instances (variant `card`, `rows` count per list). Match the card/row compositions already designed. |
| T15 — Empty state for active session | `chat.pen` | Add a "No messages in this session" empty-state placeholder consistent with the existing welcome screen / no-sessions empty states already designed in `chat.pen`. Reuse the existing icon + title + subtitle pattern. |
| T17 — `docs/gui.md` | `chat.pen`, `admin.pen` | Screenshots / behavior descriptions should reflect the final implemented Pencil designs. |

Rules for the agent:
- Do NOT introduce new hex colors, paddings, radii, or typography outside the existing variables in `ui-kit.lib.pen` and `chat-tokens.css` (already mirrored as CSS custom properties in `frontend/src/assets/chat-tokens.css`).
- The existing `frontend/src/components/ui/V*.vue` components were code-generated from `ui-kit.lib.pen`; any new atom (`VSkeleton`) MUST be designed in the `.pen` first and then generated to `frontend/src/components/ui/` to keep the kit and code in sync.
- After generating code from a `.pen`, run the project's biome format/check (`npx biome format`, `npx biome check`) before considering the task complete — per the code-style gate in `AGENTS.md`.

## Commit Plan

- **Commit 1** (after Phase 1 test-writing): `test: add e2e + unit + integration tests for v0.3.1 chat rework`  
- **Commit 2** (after Tasks T5–T8 message edit/delete backend): `feat(messages): soft-delete + edit API with token-window aware query`
- **Commit 3** (after Tasks T9–T11 context + export backend): `feat(query): sliding window token budget + markdown export format`
- **Commit 4** (after Tasks T12–T15 frontend): `feat(chat): message edit/delete UI, VSkeleton, export button, temp-ID reconciliation`
- **Commit 5** (after Task T16): `docs: v0.3.1 chat rework API, gui, configuration`

## Tasks

### Phase 1 — Red: Test Authoring (executable specification)

- [x] **T1: Create feature branch** — `git checkout main` → `git pull origin main` → `git checkout -b feature/v031-chat-rework` (run as separate steps per project rule).
  - File: branch reference only.
  - Active form: "Creating feature branch"
  - Status: ✅ completed during planning (verify `git rev-parse --abbrev-ref HEAD` outputs `feature/v031-chat-rework`).

- [x] **T2: Backend integration tests — message edit/delete + export markdown (TDD red)** — Create `backend/tests/conversations_integration.rs`. Mirror `auth_integration.rs` TestApp setup. Add failing/ignored `#[tokio::test]` cases:
  - `test_patch_message_updates_content` — POST session → POST message via query? No direct message POST; use `add_message` repo in tests. PATCH `/api/sessions/:sid/messages/:mid` body `{content: "updated"}` returns 200 with updated message; `original_content` retained; `edited_at` set.
  - `test_delete_message_soft_delete_then_excluded_from_history` — DELETE `/api/sessions/:sid/messages/:mid` returns 204; subsequent GET `/api/sessions/:sid/messages` excludes it; `get_message_count` decrements; collection session list shows updated `message_count`.
  - `test_cannot_edit_assistant_message` — PATCH on assistant-role message returns 422.
  - `test_edit_user_message_reappears_in_export` — export after edit reflects new content; deleted messages excluded from export.
  - `test_export_markdown_format` — `GET /api/sessions/:id/export?format=markdown` returns `Content-Type: text/markdown`, body contains `# {title}` H1 and `## user` / `## assistant` headers.
  - `test_export_default_json_unchanged` — backward compat: no `format` param returns JSON with prior shape.
  - `test_conversation_history_filters_soft_deleted` — after soft-delete of one user message, `/api/query` payload built by `load_conversation_history` excludes it (assert via `build_messages` test helper or process_query mock).
  - Mark each `#[ignore]` until backend impl exists, so `cargo test` stays green during the red phase.

  **Logging requirements (in tests):** Use `tracing` `debug!` in test setup; assert via `TestApp` logs that handlers log `[conv.update_message]` and `[conv.soft_delete]` DEBUG lines (optional — pin via `tracing-subscriber` test layer if straightforward, otherwise skip as flaky).
  File: `backend/tests/conversations_integration.rs`.
  Active form: "Writing backend integration tests (red)".

- [x] **T3: Backend unit tests — repository round-trip + context trimming (TDD red)** — Mirror skill-context rule (SQLite UUIDs / `TEXT` round-trip; here Postgres `UUID`, equivalent pattern). Add to existing inline `#[cfg(test)]` modules (or `repository_test.rs` if that pattern exists in the project; else inline `#[cfg(test)] mod tests { ... }`):
  - `repository.rs`: `test_update_message_sets_edited_at_and_original_content` (insert, update, fetch — assert fields round-trip); `test_soft_delete_sets_deleted_at`; `test_get_messages_excludes_soft_deleted` (insert 3, soft-delete 1, assert 2 returned); `test_get_message_count_excludes_soft_deleted`.
  - `service.rs`: `test_export_markdown_includes_all_live_messages_only` (build session with 3 messages, soft-delete 1, expect markdown with 2).
  - New `backend/src/modules/query/context_window.rs` (or inline `mod context_window`): `test_trim_history_drops_oldest_until_under_budget`; `test_trim_history_preserves_at_least_one_recent_turn`; `test_trim_history_max_history_messages_cap`; `test_trim_history_under_budget_is_noop`; `test_count_tokens_word_approach_approximates_size`.
  - Mark new impl-dependent tests `#[ignore]` until impl exists.

  **Logging requirements:** unit tests assert behavior only; no log assertions.
  Files: `backend/src/modules/conversations/repository.rs` (test mod), `backend/src/modules/conversations/service.rs` (test mod), `backend/src/modules/query/context_window.rs` (new) + `backend/src/modules/query/mod.rs` (export `context_window`).
  Active form: "Writing backend unit tests (red)".

- [x] **T4: Frontend Vitest specs — VSkeleton, MessageBubble edit/delete, chat store (TDD red)** —
  - Create `frontend/src/components/ui/__tests__/VSkeleton.spec.ts`: assert renders with `data-testid="skeleton"`; prop `variant="circle"` adds class `skeleton-circle`; prop `rows=4` renders 4 child lines; shimmer CSS class present (assert via `wrapper.classes()` or class on root).
  - Extend `frontend/src/components/__tests__/MessageBubble.spec.ts` with: `renders edit button on user messages only`; `renders delete button on both user and assistant messages`; `emits edit event with message id when edit clicked`; `emits delete event with message id when delete clicked`; `enters edit mode and shows textarea + Save/Cancel`; `emits save-edit event with new content`; `displays edited_at indicator when message.edited_at is set`.
  - Create `frontend/src/stores/__tests__/chat.spec.ts` (does NOT exist today): cover new actions — `editMessage(sid, mid, content)` calls api, replaces content in `messages.value`, sets `message.edited_at`; `deleteMessage(sid, mid)` optimistic removes from `messages.value`, on api error reverts; `exportSession(sid, 'json'|'md')` triggers blob download (mock `URL.createObjectURL`); `loadSession(sid)` sets `isSessionLoading=true` during fetch then `false` after; `sendMessage` `done` event updates temp user + assistant IDs with server-returned IDs (assert ID synchronization removing `temp-` prefix).
  - All new assertions fail initially (interfaces don't exist yet). Use `vi.hoisted()` mocks; factory fixtures matching existing style.
  **Logging requirements:** no log assertions in unit specs.
  Files listed above.
  Active form: "Writing frontend Vitest specs (red)".

- [x] **T5: Frontend Playwright e2e specs (TDD red)** — Add three e2e spec files using `frontend/playwright.config.ts` and helpers from `frontend/e2e/helpers.ts`:
  - `frontend/e2e/chat-edit-delete.spec.ts`: sends a user query (mock LLM via `page.route('**/api/query', ...)` if route-intercept needed; else use a test collection with minimal docs), asserts message bubbles render; hover user message → edit button visible → click → edit textarea → save → message content updates; hover assistant message → delete button → confirm → message removed from list. Assert HTTP calls hit `/api/sessions/:sid/messages/:mid` PATCH/DELETE (`page.waitForRequest`).
  - `frontend/e2e/chat-export.spec.ts`: open a session with messages → click toolbar "Export" → select "Markdown" → file dialog / blob generated (intercept `URL.createObjectURL`) → assert content starts with `# {session title}` and contains `## user`; same with JSON → assert JSON shape `{session, messages}`.
  - `frontend/e2e/loading-skeletons.spec.ts`: stub `page.route('**/api/sessions*', async r => { await new Promise(res => setTimeout(res, 300)); r.continue(); })` to slow session GET → assert `[data-testid="skeleton"]` elements appear in messages area; slow `GET /sessions` → assert sidebar shows skeleton rows; slow `GET /api/documents` → assert documents area shows skeleton rows.
  - Each spec uses `data-testid` attributes that Phase 3 implementation will add to components. Mark specs with `test.skip` until impl lands if the test runner would otherwise block CI.
  **Logging requirements:** e2e tests log via Playwright's test step annotations, not application logs.
  Files listed above.
  Active form: "Writing Playwright e2e specs (red)".

### Phase 2 — Backend Implementation (satisfy red)

- [x] **T6: Backend message migration + repository + service + handlers + routes** —
  - Create migration `backend/migrations/00000000000007_add_message_edit_and_soft_delete.sql`: `ALTER TABLE messages ADD COLUMN edited_at TIMESTAMPTZ NULL; ADD COLUMN original_content TEXT NULL; ADD COLUMN deleted_at TIMESTAMPTZ NULL;` + `CREATE INDEX idx_messages_deleted_at ON messages (deleted_at) WHERE deleted_at IS NULL;` (partial index to keep live-message scans fast). Document rationale inline in SQL.
  - `models.rs`: extend `Message` with `edited_at: Option<DateTime<Utc>>`, `original_content: Option<String>`, `deleted_at: Option<DateTime<Utc>>`. Add `UpdateMessageRequest { content: String }` with `serde` validate length `1..=8000`.
  - `repository.rs`: add `get_message(&self, id) -> Result<Message, AppError>` (NotFound if missing or soft-deleted); `update_message(&self, id, content) -> Result<Message, AppError>` that sets `edited_at = NOW()`, `original_content = old.content` only on first edit (preserve original). Update `get_messages`, `get_message_count`, `list_sessions` count query to filter `deleted_at IS NULL`. Add `soft_delete_message(&self, id)` setting `deleted_at = NOW()`.
  - `service.rs`: `update_message(session_id, msg_id, req)` enforces role == "user" else `AppError::UnprocessableEntity("Assistant messages cannot be edited")`; returns updated message. `delete_message(session_id, msg_id)` calls repo soft-delete. Update `export_session` to filter deleted + populate `edited_at`/`original_content` in the JSON.
  - `handlers.rs`: `patch_message` — `PATCH /api/sessions/:session_id/messages/:message_id` accepts `UpdateMessageRequest`, returns `Json<Message>`. `delete_message` — `DELETE /api/sessions/:session_id/messages/:message_id` returns 204.
  - `main.rs`: wire the two new routes alongside existing session routes (around L220–240) inside the auth-protected `route_layer`.
  - **Logging requirements (DEBUG):** `[conv.update_message] session={sid} msg={mid} role={role} old_len={} new_len={}` at INFO; `[conv.soft_delete] session={sid} msg={mid}` at INFO; `[conv.update_message] rejected: not user role` at WARN. Verbose level default; env-controlled.
  Files: `backend/migrations/00000000000007_*.sql`, `backend/src/modules/conversations/{models,repository,service,handlers}.rs`, `backend/src/main.rs`.
  Active form: "Implementing backend message edit/delete".
  Make green: T2 (integration tests), T3 (repository/service unit tests). Un-ignore the affected `#[ignore]` tests.

- [x] **T7: Backend context window: config + tokenizer + trim_history integration** —
  - `config.rs`: add `llm_max_history_messages: usize` (env `LLM_MAX_HISTORY_MESSAGES`, default `20`) and `llm_context_token_budget: usize` (env `LLM_CONTEXT_TOKEN_BUDGET`, default `6000`). Parse in `AppConfig::from_env`.
  - New `backend/src/modules/query/context_window.rs` (exported from `query/mod.rs`): `count_tokens(text: &str) -> usize` (word-count heuristic via `text.split_whitespace().count()`); `trim_history(history: &[Message], max_messages: usize, token_budget: usize) -> (Vec<Message>, usize)` — drops oldest user+assistant pairs until both `max_messages` and `token_budget` are satisfied; preserves at least the last 2 messages (1 turn). Returns trimmed history + dropped-count.
  - `query/service.rs`: in `load_conversation_history` (L180-205) and at the call site L124-127, wrap with `trim_history`. Already filters `deleted_at IS NULL` after T6 migration.
  - **Logging requirements (INFO):** `[query.trim_history] session={sid} in={} out={} dropped={} kept_tokens={}` — INFO level so per-query trimming is observable; DEBUG includes `budget={}` and `max_messages={}`. Log at this point per verbose policy.
  Files: `backend/src/config.rs`, `backend/src/modules/query/context_window.rs` (new), `backend/src/modules/query/mod.rs`, `backend/src/modules/query/service.rs`.
  Active form: "Implementing context window trimming".
  Make green: T3 context_window unit tests; T2 `test_conversation_history_filters_soft_deleted`.

- [x] **T8: Backend markdown export** —
  - `service.rs`: add `build_markdown_export(session: &Session, messages: &[Message]) -> String` — H1 `# {title}`; per message `## {role} · {created_at.rfc3339}`; blank line; content; **line**; skip soft-deleted (already filtered before call); include `(edited)` suffix when `edited_at` is set.
  - `handlers.rs`: `export_session` parse `Query<ExportFormat>` where `ExportFormat { format: Option<String> }` (default "json"); on "markdown" return `Content-Type: text/markdown` with `body::full` markdown string; on "json" keep existing path. 422 on unknown format.
  - Update `docs/api.md` (L309-312): document `?format=json|markdown`.
  - **Logging requirements (DEBUG):** `[conv.export_session] session={sid} format={format} bytes={n}` at INFO.
  Files: `backend/src/modules/conversations/{service,handlers}.rs`, `docs/api.md`.
  Active form: "Implementing markdown export".
  Make green: T2 `test_export_markdown_format`, `test_export_default_json_unchanged`, `test_edit_user_message_reappears_in_export`.

### Phase 3 — Frontend Implementation (satisfy red)

- [x] **T9: VSkeleton component** —
  - **DESIGN-FIRST (mandatory):** load `pencil-design` skill, open `design/ui-kit.lib.pen`, design a new `VSkeleton` atom there (props: `variant: 'text' | 'circle' | 'card'`, `rows`), mirroring the existing `VProgressBar` atom composition and using variables from the kit's token layer + `chat-tokens.css` `--animation-*` tokens for the shimmer. Visually verify via `pencil_get_screenshot` + `pencil_snapshot_layout` BEFORE generating code.
  - Create `frontend/src/components/ui/VSkeleton.vue` — props `{ variant: 'text' | 'circle' | 'card', rows: number = 1 }`; root `div` with `data-testid="skeleton"` and class `skeleton-{variant}`; for `variant="text"` and `rows>1` render `.skeleton-line` children with `v-for`; CSS shimmer animation reusing `chat-tokens.css` `--animation-*` tokens; respects dark/light theme via existing CSS custom properties (do NOT hardcode hex). Code generated from the `.pen` must keep the kit and code in sync.
  - Export from `components/ui/index.ts` if barrel exists; else import directly.
  - **Logging requirements:** `console.debug('[VSkeleton] render variant=%s rows=%d', ...)` in dev only.
  Files: `frontend/src/components/ui/VSkeleton.vue`, optional `components/ui/index.ts`.
  Active form: "Implementing VSkeleton component".
  Make green: T4 VSkeleton.spec.ts.

- [x] **T10: API client extensions + Message type** —
  - `api/types.ts`: extend `Message` with optional `edited_at?: string` and `original_content?: string`. Add `EditMessageRequest { content: string }`. Add `ExportFormat = 'json' | 'md'`.
  - `api/client.ts`: add to `api` object `editMessage: (sessionId, messageId, req: EditMessageRequest) => api.patch<Message>(\`/sessions/${sessionId}/messages/${messageId}\`, req)` (requires adding `patch` to the api object — currently only get/post/del/…), `deleteMessage: (sessionId, messageId) => api.del<{}>(\`/sessions/${sessionId}/messages/${messageId}\`)`, `exportSession: (sessionId, format: ExportFormat) => fetch(\`/api/sessions/${sessionId}/export?format=${format}\`)` returning a Blob (mirror `upload`'s non-JSON handling). StreamEvent `done` type extended in T11 with `{ user_message_id?: string; assistant_message_id?: string }`.
  - **Logging:** Debug `console.debug('[api.${method}] ...')` consistent with existing pattern (none exist today, mirror DocumentList's debug style if available).
  Files: `frontend/src/api/types.ts`, `frontend/src/api/client.ts`.
  Active form: "Extending API client and types".
  Make green: parts of T4 chat.spec.ts that mock these calls.

- [x] **T11: Chat store actions + temp-ID reconciliation** —
  - `stores/chat.ts`: add refs `isSessionLoading = ref(false)`, `isExporting = ref(false)`.
  - New actions: `editMessage(sessionId, messageId, content)` — calls `api.editMessage`, finds local message by `id` (post temp-ID reconciliation) and updates `content` + `edited_at` in place; `deleteMessage(sessionId, messageId)` — optimistic remove from `messages.value`, capture index + prev value, on api error revert and toast-error via existing `VToast` pattern; `exportSession(sessionId, format)` — sets `isExporting=true`, calls `api.exportSession` returning Blob, create object URL, programmatically click an `<a download>` link, revoke URL, set `isExporting=false`; `loadSession(sessionId)` — set `isSessionLoading=true` before GET, `false` after (today it just sets `messages.value` directly).
  - Temp-ID reconciliation in `sendMessage` — when the `done` StreamEvent arrives, if it contains `user_message_id` / `assistant_message_id`, replace the matching local `temp-${...}` IDs in `messages.value`. Backend `query/handlers.rs` must echo these IDs in the `done` payload:
    - `backend/src/modules/query/handlers.rs` SSE `done` event: include `{ user_message_id, assistant_message_id }` from the persisted messages (the assistant message is appended at query time like today; the user message stored at query time as well).
    - `backend/src/modules/query/service.rs` `process_query`: persist user message BEFORE streaming, persist assistant message at `done`; return both IDs via the `StreamEvent::Done` payload.
  - **Logging (frontend, DEBUG via console):** `[chat.editMessage] session=%s msg=%s` on entry; `[chat.deleteMessage] optimistic remove idx=%d`; `[chat.exportSession] format=%s bytes=%d`; `[chat.sendMessage] reconciled temp IDs user=%s->%s assist=%s->%s` at done.
  - **Logging (backend, INFO):** `[query.process_query] persisted user_message_id={mid}` after insert; `[query.process_query] persisted assistant_message_id={mid}` after stream done.
  Files: `frontend/src/stores/chat.ts`, `backend/src/modules/query/{handlers,service,models}.rs`.
  Active form: "Implementing chat store actions and temp-ID reconciliation".
  Make green: T4 chat.spec.ts (front), parts of T2 (back) for done-payload.

- [x] **T12: MessageBubble edit/delete UI** —
  - **DESIGN-FIRST (mandatory):** load `pencil-design` skill, open `design/chat.pen`. Extend the existing message bubble design in `chat.pen` with: (a) hover action row showing edit + delete buttons for user messages and delete-only for assistant messages; (b) edit-mode variant showing a textarea + Save/Cancel buttons; (c) a small `· edited` badge when the message has an `edited_at`. Reuse `VButton` instances from `ui-kit.lib.pen`. Visually verify via `pencil_get_screenshot` + `pencil_snapshot_layout` BEFORE generating code.
  - Add hover action row to `MessageBubble.vue` revealed via CSS opacity transition (mirror `.session-item-delete` pattern at `ChatView.vue:522-528`).
  - For user messages: show edit + delete buttons. For assistant messages: show delete only. Buttons emit events `@edit` `{ id }`, `@delete` `{ id }`.
  - Edit mode: when `editing` prop/ref is true, replace content rendering with a `<textarea>` bound to local `draftContent`, plus Save (emit `@save-edit` `{ id, content: draftContent }`) and Cancel (revert + emit `@cancel-edit`) buttons. Render `edited_at` indicator as a small `· edited` pill text when `message.edited_at` is set.
  - Add `data-testid` attributes: `message-edit-btn`, `message-delete-btn`, `message-edit-textarea`, `message-save-btn`, `message-cancel-edit-btn`, `message-edited-badge`.
  - **Logging (DEBUG):** `[MessageBubble] enter edit mid=%s` / `[MessageBubble] save edit` / `[MessageBubble] cancel edit` via `console.debug` (env-gated).
  Files: `frontend/src/components/MessageBubble.vue`.
  Active form: "Implementing MessageBubble edit/delete UI".
  Make green: T4 MessageBubble.spec.ts extension (excluding chat-store mock wiring in T4 — store tests are independent).

- [x] **T13: ChatView wire edit/delete + export button + skeletons** —
  - **DESIGN-FIRST (mandatory):** load `pencil-design` skill, open `design/chat.pen`. Extend the existing chat top toolbar layout with an "Export" `VButton` (ghost variant) and a format `VSelect` in the toolbar's right slot, matching the existing top-bar composition. Place the messages-area skeleton placeholder using the `VSkeleton` from T9 (variant `text`, `rows: 6`). Reuse `VButton`/`VSelect` instances from `ui-kit.lib.pen`. Visually verify BEFORE generating code.
  - `ChatView.vue`: listen for `@edit`, `@save-edit`, `@cancel-edit`, `@delete` on `<MessageBubble>` and call chat store actions `editMessage` / `deleteMessage` with `activeSessionId`.
  - Add toolbar right slot next to the existing collection `VSelect` (`ChatView.vue:245-257`): an "Export" `VButton` variant="ghost" + a `<VSelect>` of `[ {label: 'Markdown', value: 'md'}, {label: 'JSON', value: 'json'} ]` (default md). Click → `chatStore.exportSession(activeSessionId, format)`. Disable when `isExporting`.
  - Replace the inline "welcome screen" flash during `loadSession` with `<VSkeleton variant="text" :rows="6" />` gated by `chatStore.isSessionLoading`. Keep the welcome screen only for the no-active-session state (no session selected yet).
  - Add `data-testid="export-btn"`, `data-testid="export-format-select"`, `data-testid="messages-loading-skeleton"`.
  - **Logging (DEBUG):** `[ChatView] export format=%s` on click; `[ChatView] entering edit for mid=` on user action.
  Files: `frontend/src/views/ChatView.vue`.
  Active form: "Wiring chat view edit/delete + export + skeleton".
  Make green: parts of T5 e2e specs (chat-export, loading-skeletons).

- [x] **T14: Skeletons across sessions sidebar, DocumentList, GitRepoManager** —
  - **DESIGN-FIRST (mandatory):** load `pencil-design` skill. For the sessions sidebar open `design/chat.pen`; for `DocumentList` and `GitRepoManager` open `design/admin.pen`. Replace the existing text-only loaders in those designs with `VSkeleton` instances (variant `card`, with `rows` matching the list/card composition already in the `.pen`). Reuse kit atoms. Visually verify each updated layout BEFORE generating code.
  - `ChatView.vue` sessions sidebar: replace `v-if="chatStore.sessions.length === 0"` "No sessions yet" with "loading skeleton rows while `chatStore.isLoadingSessions`" (new ref — add to chat store, set during `fetchSessions`, default false). Add `data-testid="sessions-loading-skeleton"`.
  - Add `isLoadingSessions` flag to `stores/chat.ts` `fetchSessions` (set true at start, false at end; today no flag exists).
  - `DocumentList.vue` loading block (around L311-314): replace plain "Loading documents..." text with `<VSkeleton variant="card" :rows="5" />` during `documentStore.isLoading && documents.length === 0`.
  - `GitRepoManager.vue` loading block (around L205-209): replace `<p>Loading repositories...</p>` with `<VSkeleton variant="card" :rows="3" />` during `isLoadingRepos`.
  - Add `data-testid` attributes matching e2e spec: `data-testid="sessions-loading-skeleton"`, `data-testid="documents-loading-skeleton"`, `data-testid="repos-loading-skeleton"`.
  - **Logging (DEBUG):** `[chat.fetchSessions] loaded %d sessions` (already INFO-ish in dev); add `console.debug('[DocumentList] skeleton rows=5')` no-op — or skip; verbose env-gated.
  Files: `frontend/src/views/ChatView.vue`, `frontend/src/stores/chat.ts`, `frontend/src/components/DocumentList.vue`, `frontend/src/components/GitRepoManager.vue`.
  Active form: "Adding skeletons across list views".
  Make green: T5 loading-skeletons.spec.ts parts for sidebar/documents/repos.

- [x] **T15: Empty state refinement for active session** —
  - **DESIGN-FIRST (mandatory):** load `pencil-design` skill, open `design/chat.pen`. Add a "No messages in this session" empty-state placeholder that is visually consistent with the existing welcome screen / no-sessions empty states already designed in `chat.pen`. Reuse the existing icon + title + subtitle pattern from those empty states. Visually verify BEFORE generating code.
  - `ChatView.vue`: when a session IS active but its messages are empty (after `loadSession` resolves with `[]`), show a refined "No messages in this session. Ask a question to start." placeholder inside the messages area — distinct from the no-session welcome screen.
  - Add `data-testid="session-empty-messages"` for testability.
  - **Logging:** no-op.
  Files: `frontend/src/views/ChatView.vue`.
  Active form: "Refining empty state for active session".
  Make green: T5 loading-skeletons.spec.ts empty-state assertions (if any) or new assertions added to T5 at impl time without reordering (per AGENTS rules the spec already authored; add minimal assertion updates here in T15).

- [x] **T16: E2E specs enable + final Playwright pass** —
  - Remove `test.skip` from the three e2e spec files added in T5; ensure specs pass against the implemented system in local dev (`docker compose up` + Playwright) and in CI (`.github/workflows/e2e.yml` already runs the suite).
  - If any spec reveals a contract gap (e.g. `data-testid` mismatch), fix the implementation in this task — do NOT weaken the e2e assertions.
  - **Logging:** Playwright test steps via `test.step()` for clarity in HTML report.
  Files: `frontend/e2e/{chat-edit-delete,chat-export,loading-skeletons}.spec.ts`.
  Active form: "Enabling and finalizing e2e specs".
  Make green: full e2e suite for the four v0.3.1 features.

### Phase 4 — Docs & Final Validation

- [x] **T17: Documentation updates** —
  - `docs/api.md`: document `PATCH /api/sessions/:id/messages/:mid` and `DELETE /api/sessions/:id/messages/:mid` (responses, errors including 422 for editing assistant messages); extend the export row to show `?format=json|markdown`; document the new done-SSE payload `{ user_message_id, assistant_message_id }`.
  - `docs/gui.md`: add section "Message actions" (edit user messages, delete any message); "Chat export" (toolbar button, format selector); "Loading skeletons" (messages area, sessions sidebar, documents, repos); updated temp-ID reconciliation note (server-assigned IDs replace client temp IDs on `done`).
  - `docs/configuration.md`: add `LLM_MAX_HISTORY_MESSAGES` (default 20) and `LLM_CONTEXT_TOKEN_BUDGET` (default 6000). Update `.env.example` and `docker-compose.yml` service env for backend.
  - `.ai-factory/ROADMAP.md`: mark all four v0.3.1 items complete; bump milestone line `v0.3.1 — Basic Q&A Logic & Chat Rework | ⏳ 2/6` → `✅ 6/6` (the planning skill does not own ROADMAP edits — note for `/aif-implement` to invoke `/aif-roadmap` owner skill, OR if ROADMAP edits are conventionally done directly during impl per project history, document the expected resulting state without writing).
  - `frontend/src/api/types.ts` JSDoc tweaks if any — optional.
  - **Logging:** no-op.
  Files listed above.
  Active form: "Updating project docs".
  Make green: docs checkpoint gate in `/aif-implement`.

- [x] **T18: Final lint + format + verify** —
  - Run per project convention (`AGENTS.md`):
    - `backend/`: `cargo fmt` + `cargo clippy` (default rustfmt settings, no custom config).
    - `embedding/`: `ruff format` + `ruff check` (unchanged, but include if any embedding change — likely none).
    - `frontend/`: `npx biome format` + `npx biome check` (per `frontend/biome.json`).
    - `npm run ai:validate` (per `CHECKLIST.md`) — must pass before marking done.
  - Read `CHECKLIST.md` (mandatory per AGENTS.md post-implementation gate) and verify every listed gate passes (contract, UI, docs/rules sections).
  - If any validation fails, fix the root cause in this task — do NOT weaken checks.
  - **Logging:** no-op (this task runs existing tooling).
  Files: n/a (validation only).
  Active form: "Running final lint/format/verify".
  Make green: full v0.3.1 acceptance — milestone complete.

## Next Steps

Plan created with **18 tasks** across 4 phases (TDD test-first → backend impl → frontend impl → docs+validation).

Plan file: `.ai-factory/plans/feature-v031-chat-rework.md`

To start implementation, run:
```
/aif-implement
```

To view tasks:
```
/tasks
```

### Critical TDD reminder for `/aif-implement`

The implementation agent MUST:
1. Read the new tests (T2, T3, T4, T5) FIRST, before touching any production code.
2. Satisfy test behavior in order — backend red tests green by end of Phase 2, frontend red tests green by end of Phase 3.
3. Not reorder schema/migration/backend/frontend implementation tasks ahead of the test-writing tasks in Phase 1.
4. Preserve the `#[ignore]` (Rust) / `test.skip` (Playwright) gating until the corresponding impl task is committed — flip them to active only when the impl lands.
5. **UI design contract:** for every frontend task with a `DESIGN-FIRST (mandatory)` preamble (T9, T12, T13, T14, T15), the agent MUST load the `pencil-design` skill first, design or extend the relevant `.pen` file under `design/` per the mapping in the "UI Design Source Files" constraint, visually verify via Pencil MCP tools, and only then generate the production Vue code. Code that contradicts the `.pen` files or uses hardcoded values in place of design tokens is a bug. Backend tasks (T6, T7, T8, T11 backend parts) are unaffected by this design contract.
6. **Design system discipline:** all UI atoms must be sourced from `ui-kit.lib.pen` via `pencil_batch_get` with `patterns: [{ reusable: true }]` and inserted as instances — never recreated from scratch. New atoms (`VSkeleton`) are designed in the kit first, then code-generated to `frontend/src/components/ui/`.

### Context cleanup

Once `/aif-implement` is launched, consider `/clear` (full reset) or `/compact` (compress history) to free context space — this plan carries significant recon context.