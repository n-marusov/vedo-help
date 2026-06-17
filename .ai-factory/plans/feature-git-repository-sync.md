# Implementation Plan: Git Repository Sync

Branch: feature/git-repository-sync
Created: 2026-06-18

## Settings
- Testing: yes (TDD mandatory per RULES.md — tests written FIRST, implementation follows)
- Logging: verbose
- Docs: yes

## Roadmap Linkage
Milestone: "v0.3 — Admin Panel & Production Polish"
Rationale: Git repository sync is a v0.3 task (currently unchecked). It extends the document ingestion pipeline to pull Markdown files from remote Git repositories, reuse existing chunking + embedding + Chroma indexing, and notify on updates via webhooks.

## Overview

**Git Repository Sync** connects external Git repositories (GitHub, GitLab, Bitbucket) to the VEDO RAG pipeline. A registered repository is cloned into a local working directory, all Markdown (`.md`) files are parsed, chunked, embedded, and indexed into a Chroma collection. Subsequent syncs pull the latest changes and re-index only modified files. Webhook endpoints allow automatic sync triggers.

### TDD Flow

Tests are written **before** implementation code and serve as executable specifications. The agent reads test files to understand expected contracts (API shapes, error conditions, edge cases) and generates implementation that satisfies them. Workflow per task group:

```
Red phase:   write tests → cargo test → FAIL (expected: no implementation yet)
Green phase: implement code → cargo test → PASS
```

### Key design decisions

1. **Library: `git2`** — libgit2 bindings, production-grade, well-tested. Clone/pull wrapped in `tokio::task::spawn_blocking` for async compatibility.
2. **Sync strategy: incremental** — first sync clones and indexes all `.md` files; subsequent pulls detect file changes via `git diff` and re-index only delta.
3. **Storage: SQLite** — new `git_repositories` table tracks repo URL, local clone path, last synced commit, and linked collection UUID.
4. **Webhook: POST endpoint** — receives GitHub/GitLab push events, validates signature/token, triggers sync asynchronously.
5. **Scope: Markdown only** — first iteration indexes `.md` files exclusively. Future iterations add `.rst`, `.txt`, `.adoc`.
6. **Auth: HTTPS token** — repos are cloned via HTTPS with a personal access token stored per-repo (never logged, never serialized in responses).

## Commit Plan
- **Commit 1** (after tasks 1-3): "test: add E2E, integration, and unit tests for Git sync"
- **Commit 2** (after tasks 4-6): "feat: add DB migration, models, and repository for Git sync"
- **Commit 3** (after tasks 7-8): "feat: implement GitSyncService — clone, pull, parse, index"
- **Commit 4** (after tasks 9-10): "feat: add Git sync API endpoints and route wiring"
- **Commit 5** (after tasks 11-12): "feat: add webhook endpoint and polling scheduler"
- **Commit 6** (after tasks 13-14): "feat: add frontend Git repo manager UI"

## Tasks

### Phase 1: Tests First (E2E → Integration → Unit)

- [x] **Task 1: Write E2E tests for Git sync UI (Playwright)**
  - Create `frontend/e2e/git-sync.spec.ts`:
    - **Test: `register new git repo`** — navigate Admin → "Git Repositories" tab → fill form (URL, branch, collection, token) → submit → verify new repo row appears in the table with status badge "idle"
    - **Test: `trigger sync and observe results`** — register repo → click "Sync Now" button → verify status badge transitions to "syncing" (spinner visible) → wait for completion → verify badge shows "idle" with `last_synced_at` timestamp and `files_indexed` count displayed
    - **Test: `delete repo`** — register repo → click "Delete" button → confirm in VDialog → verify row removed from table
    - **Test: `form validation errors`** — submit empty form → verify inline error messages for required fields → enter invalid URL (e.g. `ftp://bad`) → verify "Must be https:// or git@" error
    - **Test: `sync error state`** — register repo with intentionally broken URL (e.g. `https://nonexistent.invalid/repo.git`) → click "Sync Now" → verify status badge turns "error" (red) with tooltip showing error message
    - **Test: `list shows multiple repos`** — register 2 repos with different names → verify both appear in table with correct collection names
  - Follow existing Playwright patterns from `frontend/e2e/` (login via Keycloak test user, `page.goto('/admin')`, `waitForSelector` on VButton/VInput)
  - These tests describe the expected UI contract that the Vue component must fulfill
  - **Files:** `frontend/e2e/git-sync.spec.ts` (new)
  - **Logging:** N/A (Playwright uses its own tracing)

- [x] **Task 2: Write integration tests for Git sync backend (Rust + Chroma)**
  - Create `backend/tests/git_sync_integration.rs`:
    - **`test_create_and_list_repo`** — POST create repo via handler → GET list → verify response contains the created repo with correct `url`, `branch`, `collection_id`, `status: "idle"`
    - **`test_sync_markdown_repo_from_local_fixture`** — create a local bare git repo in `tests/fixtures/sample-docs/` (2-3 `.md` files with distinct content) → register as `file:///...` repo → trigger sync → verify `SyncStatusResponse.files_indexed >= 2` and `chunks_total > 0` → query Chroma via `ChromaClient::query()` with a known phrase from a fixture file → verify at least one result with correct `document_id` and `score > 0`
    - **`test_incremental_sync_detects_changes`** — initial sync on fixture repo → note chunk count → add a new `.md` file to the fixture repo + commit → trigger sync again → verify incremental chunk count > original
    - **`test_delete_repo_cleans_up`** — create + sync → DELETE repo → verify GET returns 404 → verify local clone directory removed via `std::fs::metadata` → verify Chroma collection deleted (re-create with same name succeeds)
    - **`test_sync_empty_repo`** — create bare repo with no `.md` files → sync → verify `files_indexed: 0`, `status: "idle"`, no error
    - **`test_sync_repo_with_nested_dirs`** — fixture repo has `docs/guide.md` and `docs/api/ref.md` → sync → verify both files indexed → query for content from nested file → verify found
    - **`test_sync_status_transitions`** — register repo → GET status → verify `idle` → start sync (don't await) → GET status → verify `syncing` → await sync → GET status → verify `idle` or `error`
  - Use `backend/tests/common/mod.rs` helpers: `setup_test_db()` (in-memory SQLite), `setup_test_config()`
  - `#[tokio::test]` async; `unique_collection()` for Chroma isolation; env `CHROMA_URL` for service URL
  - Integration tests define the API contract: request/response shapes, status codes, error conditions
  - **Files:** `backend/tests/git_sync_integration.rs` (new), `backend/tests/common/mod.rs` (add git_repo fixture helpers if needed)
  - **Logging:** RUST_LOG=debug during test execution for verbose failure diagnostics

- [x] **Task 3: Write unit tests for GitSyncService and GitRepoRepository (Rust)**
  - Create `backend/tests/git_sync_unit.rs`:
    - **Repository tests** (using in-memory SQLite):
      - `test_create_repo_persists_all_fields` — create `GitRepo` with all fields set → verify `get_repo()` returns identical data (except `access_token` which should be present in DB but omitted in `GitRepoSummary`)
      - `test_list_repos_returns_all` — insert 3 repos → verify list returns 3 items
      - `test_update_sync_status_changes_fields` — create repo → `update_sync_status(id, "abc123", "syncing")` → verify `last_commit_hash`, `status` updated
      - `test_delete_repo_removes_row` — create → delete → verify `get_repo()` returns `AppError::NotFound`
      - `test_create_repo_same_url_allowed` — two repos with same URL but different collection_id → both succeed
    - **Service tests** (using mocked ChromaClient + EmbeddingClient via `mockall`):
      - `test_clone_repo_injects_token` — verify URL transformation: `https://github.com/u/r.git` + token `ghp_xxx` → `https://ghp_xxx@github.com/u/r.git`
      - `test_clone_repo_no_token_uses_url_as_is` — URL without token passed through unchanged
      - `test_parse_markdown_finds_only_md_files` — create temp dir with `readme.md`, `notes.txt`, `image.png` → parse → verify only `readme.md` returned
      - `test_parse_markdown_skips_large_files` — create 11MB `.md` file → parse → verify skipped with WARN log
      - `test_index_chunks_calls_embedding_then_chroma` — mock `EmbeddingClient` to return fixed vectors → mock `ChromaClient::add_embeddings` to succeed → verify Chroma called with correct collection_id, metadata includes `{source: "git", repo_id, file_path}`
      - `test_sync_full_clone_triggers_indexing` — mock `clone_repo` → mock `parse_markdown_files` → mock `index_chunks` → verify pipeline called in order
      - `test_sync_incremental_calls_git_diff` — repo has `last_commit_hash` → verify pull + diff executed, not full clone
      - `test_sync_failure_sets_error_status` — mock `clone_repo` to fail → verify status updated to `"error"` with error message
  - Follow `mockall` pattern from existing project dev-dependencies; use `tempfile` for temp directories
  - Unit tests define the service contract: method signatures, error types, state transitions
  - **Files:** `backend/tests/git_sync_unit.rs` (new)
  - **Logging:** RUST_LOG=debug during test execution
  - **Depends on:** Task 2 (integration tests define API contract; unit tests define service contract)

### Phase 2: Foundation — DB, models, repository

- [x] **Task 4: Add `git2` dependency and `git_repositories` table migration**
  - Add `git2 = "0.19"` to `backend/Cargo.toml` under `[dependencies]`
  - Add `sqlx` migration in `backend/src/main.rs` `run_migrations()`:
    ```sql
    CREATE TABLE IF NOT EXISTS git_repositories (
        id TEXT PRIMARY KEY,
        url TEXT NOT NULL,
        branch TEXT NOT NULL DEFAULT 'main',
        access_token TEXT,
        local_path TEXT NOT NULL,
        last_commit_hash TEXT,
        last_synced_at TEXT,
        collection_id TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'idle' CHECK(status IN ('idle','syncing','error')),
        webhook_secret TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_git_repos_collection ON git_repositories(collection_id);
    ```
  - **Files:** `backend/Cargo.toml`, `backend/src/main.rs` (`run_migrations` fn, lines ~298-377)
  - **Logging:** INFO "Git repositories table migration applied"; DEBUG with column count; ERROR if migration fails with full SQL error
  - **Gate:** `cargo build` succeeds; integration test `test_create_and_list_repo` still FAILS (no handler yet — expected)

- [x] **Task 5: Define `GitRepo` models and request/response DTOs**
  - Create `backend/src/modules/git_sync/models.rs`:
    ```rust
    pub struct GitRepo {
        pub id: Uuid,
        pub url: String,
        pub branch: String,
        pub access_token: Option<String>, // NEVER logged, NEVER serialized in responses
        pub local_path: String,
        pub last_commit_hash: Option<String>,
        pub last_synced_at: Option<DateTime<Utc>>,
        pub collection_id: Uuid,
        pub status: String, // "idle" | "syncing" | "error"
        pub webhook_secret: Option<String>,
        pub created_at: DateTime<Utc>,
        pub updated_at: DateTime<Utc>,
    }
    pub struct CreateRepoRequest { pub url: String, pub branch: Option<String>, pub access_token: Option<String>, pub collection_id: Uuid }
    pub struct GitRepoSummary { /* all fields except access_token, webhook_secret; + collection_name: String */ }
    pub struct SyncStatusResponse { pub repo_id: Uuid, pub status: String, pub files_indexed: usize, pub chunks_total: usize, pub last_commit: Option<String>, pub error: Option<String> }
    pub struct WebhookPayload { pub repo_id: Uuid, pub event: String, pub ref_name: String, pub before: Option<String>, pub after: String }
    ```
  - Derive `Debug, Clone, Serialize, Deserialize`. `sqlx::FromRow` for `GitRepo`, `GitRepoSummary`.
  - `GitRepoSummary` must implement `From<GitRepo>` that strips sensitive fields.
  - **Files:** `backend/src/modules/git_sync/models.rs` (new)
  - **Logging:** N/A — pure data types; DEBUG in `From` impl tracing the stripping of sensitive fields
  - **Gate:** `cargo build` succeeds; unit test `test_create_repo_persists_all_fields` still FAILS (no repository yet — expected)

- [x] **Task 6: Implement `GitRepoRepository` for SQLite CRUD**
  - Create `backend/src/modules/git_sync/repository.rs`:
    ```rust
    pub struct GitRepoRepository { db: SqlitePool }
    impl GitRepoRepository {
        pub fn new(db: SqlitePool) -> Self;
        pub async fn create_repo(&self, repo: &GitRepo) -> Result<Uuid, AppError>;
        pub async fn list_repos(&self) -> Result<Vec<GitRepo>, AppError>;
        pub async fn get_repo(&self, id: Uuid) -> Result<GitRepo, AppError>;
        pub async fn get_repo_with_collection_name(&self, id: Uuid) -> Result<GitRepoSummary, AppError>;
        pub async fn list_repos_with_collection_names(&self) -> Result<Vec<GitRepoSummary>, AppError>;
        pub async fn update_sync_status(&self, id: Uuid, commit_hash: &str, synced_at: &DateTime<Utc>, status: &str) -> Result<(), AppError>;
        pub async fn delete_repo(&self, id: Uuid) -> Result<(), AppError>;
    }
    ```
  - Use `sqlx::query_as` with pattern matching existing `CollectionRepository`
  - `list_repos_with_collection_names` — JOIN with `collections` table
  - `get_repo` returns `AppError::NotFound` when 0 rows
  - **Files:** `backend/src/modules/git_sync/repository.rs` (new)
  - **Logging:** DEBUG `[GitRepoRepository::<method>]` on entry/exit with `repo_id=%id`; ERROR on SQL failures with `error=%e`, `query=%sql`; never log `access_token`
  - **Gate:** `cargo test test_create_repo_persists_all_fields` PASSES (Task 3 unit test now green)

### Phase 3: Core Service — clone, pull, parse, index

- [x] **Task 7: Implement `GitSyncService` — core pipeline**
  - Create `backend/src/modules/git_sync/service.rs`:
    ```rust
    pub struct GitSyncService {
        repo: GitRepoRepository,
        chroma_url: String,
        embedding_url: String,
        clone_root: PathBuf,
    }
    ```
  - Methods:
    - `clone_repo(&self, git_repo: &GitRepo) -> Result<PathBuf, AppError>` — `git2::Repository::clone()` in `spawn_blocking`; inject token into URL `https://{token}@host/path`; clone to `{clone_root}/{repo.id}/`
    - `pull_repo(&self, local_path: &Path, branch: &str) -> Result<(String, String), AppError>` — `git2` fetch + fast-forward; returns `(old_commit, new_commit)` for diff
    - `get_changed_files(&self, local_path: &Path, old_commit: &str, new_commit: &str) -> Result<Vec<String>, AppError>` — `git2::Repository::open`, `diff_tree_to_tree`, collect `.md` file paths
    - `sync_repo(&self, repo_id: Uuid) -> Result<SyncStatusResponse, AppError>` — full orchestrator:
      1. Set status `syncing`
      2. If no `last_commit_hash` → full clone + parse all `.md` files
      3. Else → `pull_repo` → `get_changed_files` → parse only delta
      4. `index_chunks()` for parsed files
      5. Update `last_commit_hash`, `last_synced_at`, status → `idle`
      6. On any failure → status → `error`, return error
    - `parse_markdown_files(&self, dir: &Path, filter: Option<&[String]>) -> Result<Vec<(String, String)>, AppError>` — walk dir, collect `*.md`, filter by `filter` if present, read UTF-8, skip >10MB files
    - `index_chunks(&self, collection_id: Uuid, repo_id: Uuid, files: &[(String, String)]) -> Result<(usize, usize), AppError>` — for each file: `chunk_document()` → `EmbeddingClient::embed()` → `ChromaClient::add_embeddings()`; returns `(files_count, chunks_total)`; metadata `{source, repo_id, file_path, chunk_index}`
    - `delete_repo_local(&self, repo_id: Uuid) -> Result<(), AppError>` — `std::fs::remove_dir_all` in `spawn_blocking`
    - `delete_repo_and_cleanup(&self, repo_id: Uuid) -> Result<(), AppError>` — delete Chroma collection → delete local clone → delete SQLite row
  - Reuses `shared::chunking::chunk_document()`, constructs `EmbeddingClient` and `ChromaClient` per sync call
  - Constructor: `GitSyncService::new(repo, chroma_url, embedding_url, clone_root) -> Self`
  - Sanitize: strip `access_token` from all log messages (redact with `[REDACTED]`)
  - **Files:** `backend/src/modules/git_sync/service.rs` (new)
  - **Logging:** INFO `[GitSyncService::sync_repo] started repo_id=%s` / `completed files=%d chunks=%d`; DEBUG on git operations (branch, commit hash); WARN on skipped files (too large, non-UTF-8); ERROR on git/embedding/Chroma failures with full context; never log `access_token`
  - **Gate:** `cargo test test_sync_full_clone_triggers_indexing` PASSES (Task 3 unit test green); `cargo test test_sync_markdown_repo_from_local_fixture` PASSES (Task 2 integration test green after wiring — expected FAIL before Task 10)

- [x] **Task 8: Add `GIT_CLONE_ROOT` config and directory initialization**
  - Add to `AppConfig` in `backend/src/config.rs`:
    - `pub git_clone_root: String` — env `GIT_CLONE_ROOT`, default `"data/git-repos"`
    - `pub git_sync_interval_secs: u64` — env `GIT_SYNC_INTERVAL_SECS`, default `0`
  - In `main.rs` startup: `std::fs::create_dir_all(&config.git_clone_root)` with ERROR+exit on failure
  - **Files:** `backend/src/config.rs`, `backend/src/main.rs`
  - **Logging:** INFO `Git clone root: {path}`; ERROR if directory creation fails with `error=%e, path=%s`
  - **Gate:** `cargo build` succeeds; `GIT_CLONE_ROOT=/tmp/test cargo run` creates directory

### Phase 4: API — handlers and wiring

- [x] **Task 9: Implement `git_sync` Axum handlers**
  - Create `backend/src/modules/git_sync/handlers.rs`:
    ```rust
    pub async fn create_repo(State(svc): State<GitSyncService>, Json(req): Json<CreateRepoRequest>) -> Result<Json<GitRepoSummary>, AppError>;
    pub async fn list_repos(State(svc): State<GitSyncService>) -> Result<Json<Vec<GitRepoSummary>>, AppError>;
    pub async fn get_repo(State(svc): State<GitSyncService>, Path(id): Path<Uuid>) -> Result<Json<GitRepoSummary>, AppError>;
    pub async fn trigger_sync(State(svc): State<GitSyncService>, Path(id): Path<Uuid>) -> Result<Json<SyncStatusResponse>, AppError>;
    pub async fn get_sync_status(State(svc): State<GitSyncService>, Path(id): Path<Uuid>) -> Result<Json<SyncStatusResponse>, AppError>;
    pub async fn delete_repo(State(svc): State<GitSyncService>, Path(id): Path<Uuid>) -> Result<Json<Value>, AppError>;
    ```
  - `create_repo`: validate URL starts with `https://` or `git@`, reject with `AppError::BadRequest` otherwise. Generate `id` (Uuid::new_v4), `local_path = {clone_root}/{id}`, `status = "idle"`. Returns `GitRepoSummary`.
  - `trigger_sync`: calls `svc.sync_repo(id).await` synchronously. For large repos, a future `202 Accepted` pattern can be added; initial version blocks the request.
  - `delete_repo`: calls `svc.delete_repo_and_cleanup(id).await`, returns `{"status":"deleted","id":"..."}`
  - `get_sync_status`: reads `GitRepo` from DB, returns `SyncStatusResponse` with current status
  - Follow existing handler pattern: extract state, delegate, return `Json<T>`
  - **Files:** `backend/src/modules/git_sync/handlers.rs` (new)
  - **Logging:** INFO `[handler::create_repo] url=%s collection_id=%s`; INFO `[handler::trigger_sync] repo_id=%s`; DEBUG with extracted path params; ERROR on all failures; never log `access_token`
  - **Gate:** `cargo test test_create_and_list_repo` PASSES (Task 2 integration test — after wiring in Task 10)

- [x] **Task 10: Wire `GitSyncService` into `main.rs`, `lib.rs`, `modules/mod.rs`**
  - Create `backend/src/modules/git_sync/mod.rs`:
    ```rust
    pub mod handlers;
    pub mod models;
    pub mod repository;
    pub mod service;
    ```
  - Add `pub mod git_sync;` to `backend/src/modules/mod.rs`
  - Export in `backend/src/lib.rs`: `pub use modules::git_sync;`
  - In `main.rs`:
    - Import: `use vedo_backend::modules::git_sync::{handlers as git_sync_handlers, service::GitSyncService};`
    - Instantiate: `let git_sync_service = GitSyncService::new(git_repo_repo, chroma_url, embedding_service_url, PathBuf::from(&config.git_clone_root));`
    - Add `git_sync_service: GitSyncService` to `AppState`
    - `impl FromRef<AppState> for GitSyncService { fn from_ref(state: &AppState) -> Self { state.git_sync_service.clone() } }`
    - Add routes:
      ```rust
      .route("/api/git-sync/repos", post(git_sync_handlers::create_repo))
      .route("/api/git-sync/repos", get(git_sync_handlers::list_repos))
      .route("/api/git-sync/repos/{id}", get(git_sync_handlers::get_repo))
      .route("/api/git-sync/repos/{id}/sync", post(git_sync_handlers::trigger_sync))
      .route("/api/git-sync/repos/{id}/status", get(git_sync_handlers::get_sync_status))
      .route("/api/git-sync/repos/{id}", delete(git_sync_handlers::delete_repo))
      ```
  - **Files:** `backend/src/modules/git_sync/mod.rs` (new), `backend/src/modules/mod.rs`, `backend/src/lib.rs`, `backend/src/main.rs`
  - **Logging:** INFO `GitSyncService initialized clone_root=%s`
  - **Gate:** `cargo build` succeeds; `cargo test --test git_sync_integration` — all Task 2 tests PASS

### Phase 5: Webhooks & polling

- [x] **Task 11: Implement webhook endpoint with HMAC validation**
  - Add `POST /api/git-sync/webhook` handler in `handlers.rs`:
    - Receives raw JSON body + headers
    - **GitHub mode:** checks `X-GitHub-Event: push`, validates `X-Hub-Signature-256` via HMAC-SHA256 against `webhook_secret` stored in DB
    - **GitLab mode:** checks `X-Gitlab-Event: Push Hook`, validates `X-Gitlab-Token` against `webhook_secret`
    - If no `webhook_secret` configured → skip validation (warn log), accept raw payload
    - Extracts branch from `ref_name` (strip `refs/heads/`), matches against repo's configured branch
    - If match → `tokio::spawn(svc.sync_repo(repo_id))` → returns `202 Accepted` immediately
    - If no match (wrong branch) → returns `200 OK` with `{ "status": "skipped", "reason": "branch mismatch" }`
    - If signature invalid → returns `401 Unauthorized`
    - If repo not found → returns `404 Not Found`
  - Add route in `main.rs`: `.route("/api/git-sync/webhook", post(git_sync_handlers::webhook))`
  - **Files:** `backend/src/modules/git_sync/handlers.rs`, `backend/src/main.rs`
  - **Logging:** INFO `[webhook] received event=%s repo_id=%s ref=%s`; WARN on missing webhook_secret; WARN on signature mismatch; INFO on skipped (branch mismatch); ERROR on handler panic
  - **Gate:** unit tests `test_webhook_signature_valid`, `test_webhook_signature_invalid` added to Task 3 → PASS; integration test `test_webhook_triggers_sync` (Task 2) PASS

- [x] **Task 12: Add configurable polling scheduler**
  - Add `GitSyncScheduler` logic inside `GitSyncService`:
    - `pub async fn start_scheduler(self: Arc<Self>, interval_secs: u64, shutdown: broadcast::Receiver<()>)`
    - If `interval_secs == 0` → INFO `Scheduler disabled (interval=0)` → return immediately
    - `tokio::spawn` loop: `tokio::time::interval` every N seconds
    - Each tick: `repo.list_repos().await` → filter `status != 'syncing'` → for each: `self.sync_repo(id).await`
    - Failures: track consecutive error count per repo; exponential backoff (1m→2m→4m, cap 30m)
    - Shutdown: `tokio::select! { _ = interval.tick() => {...}, _ = shutdown.recv() => { break; } }`
  - Spawn in `main.rs` after router build:
    ```rust
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let scheduler_svc = Arc::new(git_sync_service.clone());
    tokio::spawn(async move { scheduler_svc.start_scheduler(config.git_sync_interval_secs, shutdown_rx).await });
    ```
    Wire `shutdown_tx` into `shutdown_signal()` to send on Ctrl+C
  - **Files:** `backend/src/modules/git_sync/service.rs`, `backend/src/main.rs`
  - **Logging:** INFO `Scheduler started interval=%ds`; DEBUG each poll cycle with `repos_checked=%d`; WARN on consecutive failures `repo_id=%s failures=%d`; INFO `Scheduler stopped`
  - **Gate:** integration test `test_scheduler_polls_and_syncs` (Task 2) PASS

### Phase 6: Frontend — types, client, UI component

- [x] **Task 13: Add API types and client methods**
  - Add to `frontend/src/api/types.ts`:
    ```typescript
    export interface GitRepoSummary {
      id: string; url: string; branch: string; local_path: string;
      last_commit_hash?: string; last_synced_at?: string;
      collection_id: string; collection_name: string;
      status: 'idle' | 'syncing' | 'error';
      created_at: string; updated_at: string;
    }
    export interface CreateRepoRequest {
      url: string; branch?: string; access_token?: string; collection_id: string;
    }
    export interface SyncStatusResponse {
      repo_id: string; status: string; files_indexed: number;
      chunks_total: number; last_commit?: string; error?: string;
    }
    ```
  - Add to `frontend/src/api/client.ts`:
    ```typescript
    getGitRepos: () => api.get<GitRepoSummary[]>('/git-sync/repos'),
    createGitRepo: (req: CreateRepoRequest) => api.post<GitRepoSummary>('/git-sync/repos', req),
    triggerSync: (id: string) => api.post<SyncStatusResponse>(`/git-sync/repos/${id}/sync`),
    deleteGitRepo: (id: string) => api.del<{ status: string; id: string }>(`/git-sync/repos/${id}`),
    ```
  - **Files:** `frontend/src/api/types.ts`, `frontend/src/api/client.ts`
  - **Gate:** `npm run type-check` succeeds (or equivalent TS check)

- [x] **Task 14: Build `GitRepoManager.vue` admin component**
  - Create `frontend/src/components/GitRepoManager.vue`:
    - **Template:**
      - Header: "Git Repositories" + "Connect Repository" button (VButton, primary variant)
      - Table: columns (URL, Branch, Collection, Status, Last Synced, Actions)
      - **Status cell:** VBadge: `idle` → gray, `syncing` → blue with CSS pulse animation, `error` → red destructive variant with tooltip showing `error` message on hover
      - **Actions cell:** "Sync Now" VButton (small/ghost) + "Delete" VButton (small/destructive)
      - Empty state when no repos: "No repositories connected. Connect a Git repository to index its documentation."
    - **Dialog (VDialog):** "Connect Repository" form:
      - VInput: URL (required, placeholder `https://github.com/user/repo.git`, validates `startsWith('https://') || startsWith('git@')`)
      - VInput: Branch (optional, default `main`)
      - VInput: Access Token (password type, optional, placeholder `ghp_...` or `glpat-...`)
      - VSelect: Collection (required, fetches from `api.getCollections()`)
      - VButton "Connect" (primary) + VButton "Cancel" (ghost)
    - **Script setup:**
      - `onMounted` → fetch repos and collections from API
      - `connectRepo()` → validate → API create → push to list → close dialog
      - `syncRepo(id)` → set status `syncing` locally → API trigger → update row with result
      - `deleteRepo(id)` → confirm dialog → API delete → remove from list
      - `formatDate(iso: string)` helper
    - Use existing atoms: VButton, VInput, VSelect, VBadge, VDialog
  - **Files:** `frontend/src/components/GitRepoManager.vue` (new)
  - **Logging:** `console.debug('[GitRepoManager] fetching repos...')`; `console.error('[GitRepoManager] sync failed:', err)`
  - **Gate:** `npm run dev` → Admin page shows "Git Repositories" tab; E2E tests from Task 1 PASS

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Large repos (>1k .md files) cause timeout on sync | Medium | Cap files per sync (default 500); `202 Accepted` + background task in future iteration |
| `git2` crate build failures on Windows | Low | `git2` is well-supported on all platforms; fallback to `std::process::Command("git")` if needed |
| Webhook endpoint exposed without auth | Medium | HMAC signature validation mandatory; `webhook_secret` stored per repo; document setup |
| Token leakage in logs/responses | High | Sanitize `access_token` from all log output; `GitRepoSummary` never serializes `access_token`; redact in DEBUG traces |
| Concurrent syncs on same repo | Low | Status `syncing` prevents re-entry; `tokio::sync::Mutex` per repo ID if needed |

## Dependencies

- **Runtime dependencies:** `git2 = "0.19"`, existing `ChromaClient`, `EmbeddingClient`, `chunking`
- **Environment variables (new):** `GIT_CLONE_ROOT` (default `data/git-repos`), `GIT_SYNC_INTERVAL_SECS` (default `0`)
- **No new Docker services** — sync runs inside the backend container
- **Filesystem:** `data/git-repos/` directory must be writable and persisted via Docker volume
- **Test fixtures:** small git repos created in `backend/tests/fixtures/` for integration tests
