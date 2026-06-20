# Implementation Plan: PostgreSQL Migration

Branch: feature/postgresql-migration
Created: 2026-06-20

## Settings
- Testing: yes
- Logging: verbose
- Docs: yes

## Roadmap Linkage
Milestone: "v0.3.1 — Migration to PostgreSQL"
Rationale: Direct mapping — this plan implements the entire v0.3.1 milestone from the roadmap.

## Research Context
Source: codebase analysis

Goal: Migrate the VEDO hub backend from SQLite to PostgreSQL, consolidate all database containers into a single `db` service (renamed from `keycloak-db`), and use separate databases for each service within that container.

Constraints:
- Hard switch: no dual SQLite/PostgreSQL support — remove SQLite dependency entirely
- Use `sqlx migrate` CLI for schema migrations (replace inline `run_migrations`)
- Use `sqlx::test` attribute for unit/integration tests (no `:memory:` SQLite)
- Docker service `keycloak-db` → `db`; backend connects to database `vedo` inside same Postgres container
- Backend Cargo.toml changes from `sqlite` feature to `postgres` feature in sqlx
- All SQL parameter placeholders change from `?` (SQLite) to `$1, $2, ...` (PostgreSQL)
- All `INTEGER NOT NULL DEFAULT 1` booleans → `BOOLEAN NOT NULL DEFAULT TRUE`
- All `TEXT PRIMARY KEY` UUID columns → `UUID PRIMARY KEY`
- All `TEXT NOT NULL` timestamps → `TIMESTAMPTZ NOT NULL DEFAULT NOW()`

Decisions:
- Hard switch (no backward compatibility with SQLite)
- `sqlx migrate` for versioned migrations (timestamped `.sql` files)
- `sqlx::test` macro + test database provisioning for tests
- Separate databases in one PostgreSQL container: `vedo` (backend), `keycloak` (auth)
- Init script using `POSTGRES_MULTIPLE_DATABASES` for multi-db setup

Open questions: None — all resolved by roadmap scope.

## Commit Plan
- **Commit 1** (after tasks 1–3): "feat: switch backend to PostgreSQL — Cargo.toml, config, Docker Compose"
- **Commit 2** (after tasks 4–6): "feat: rewrite schemas, repositories, and main.rs for PostgreSQL"
- **Commit 3** (after task 7): "feat: migrate all tests to sqlx::test and PostgreSQL"
- **Commit 4** (after tasks 8–10): "feat: update backup/restore scripts, CI, and documentation"
- **Commit 5** (after task 11): "chore: final validation and cleanup"

## URL Scope Matrix (Docker/Infrastructure)

| URL / Service | Context | Compose File |
|---------------|---------|--------------|
| `db:5432` | Docker-internal (backend → Postgres, keycloak → Postgres) | docker-compose.yml |
| `http://backend:3000` | Docker-internal (Caddy → backend) | docker-compose.yml |
| `http://chroma:8000` | Docker-internal (backend → Chroma) | docker-compose.yml |
| `http://embedding:8001` | Docker-internal (backend → embedding) | docker-compose.yml |
| `http://keycloak:8080` | Docker-internal (backend → KeyCloak) | docker-compose.yml |
| `DATABASE_URL=postgres://vedo:...@db:5432/vedo` | Backend env var | docker-compose.yml, .env.example |
| `KC_DB_URL=jdbc:postgresql://db:5432/keycloak` | KeyCloak env var | docker-compose.yml |
| `localhost:3000` | Host-local for dev | docker-compose.override.yml |
| `localhost:8080` | Host-local for KeyCloak | docker-compose.override.yml |

## Tasks

### Phase 1: Infrastructure — Docker Compose & Cargo Config

- [x] **Task 1: Rename `keycloak-db` to `db` and add `vedo` database**
  - In `docker-compose.yml`:
    - Rename service `keycloak-db` → `db`
    - Change image user/db: add `POSTGRES_MULTIPLE_DATABASES: "vedo,keycloak"` or use an init script that creates both `vedo` and `keycloak` databases
    - Add `POSTGRES_USER: postgres` with a superuser, create dedicated users `vedo` and `keycloak`
    - Update volume `keycloak_db_data` → `db_data`
    - Update `KC_DB_URL` env var: `jdbc:postgresql://db:5432/keycloak`
    - Update `depends_on` in `keycloak` service: `keycloak-db` → `db`
    - Add `depends_on: db` to `backend` service with `condition: service_healthy`
    - Add `DATABASE_URL` env var to `backend` service
    - Remove `db_data:/data` volume mount from `backend` service (no longer needed for SQLite file)
    - Update healthcheck to use `-U postgres` or the appropriate user
  - In `docker-compose.override.yml`:
    - Rename `keycloak-db` reference → `db`
    - Remove any SQLite-specific backend volume mounts
  - In `docker-compose.production.yml`:
    - Check for `keycloak-db` references, rename → `db`
    - Add security hardening for the `db` service (resource limits)
  - In `backend/Dockerfile`:
    - Replace `libsqlite3-dev` with `libpq-dev` in build deps stage
    - Replace `libsqlite3-0` with `libpq5` in runtime stage
    - Remove `mkdir -p /data && chown vedo:vedo /data` line
  - In `.env.example`:
    - Change `DATABASE_URL=sqlite:///data/vedo.db?mode=rwc` → `DATABASE_URL=postgres://vedo:${VEDO_DB_PASSWORD}@db:5432/vedo`
    - Add `VEDO_DB_PASSWORD` variable
  - Add `scripts/init-db.sh` or use the `postgres:16-alpine` official init mechanism (`/docker-entrypoint-initdb.d/`) for multi-database creation

  LOGGING REQUIREMENTS:
  - Log backend startup: database connection URL (redacted password)
  - Log `db` container health status checks
  - Use format: `[main] database connect: postgresql://db:5432/vedo`

  Files: docker-compose.yml, docker-compose.override.yml, docker-compose.production.yml, backend/Dockerfile, .env.example, scripts/init-db.sh (new)

- [x] **Task 2: Switch sqlx feature from `sqlite` to `postgres`**
  - In `backend/Cargo.toml`:
    - Change `sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "chrono", "uuid"] }` to `sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "chrono", "uuid", "migrate"] }`
    - Add `"migrate"` feature to enable `sqlx migrate` subcommands and runtime migration support
    - Remove `libsqlite3-sys` or any SQLite-related build dependencies if present
  - Run `cargo check` to verify compilation (will fail until code is updated — that's expected in this phase)
  - In `backend/src/config.rs`:
    - Change default `DATABASE_URL` from `sqlite:data/vedo.db?mode=rwc` to `postgres://vedo:vedo@localhost:5432/vedo`

  LOGGING REQUIREMENTS:
  - N/A for this task (config-only change)

  Files: backend/Cargo.toml, backend/src/config.rs

- [x] **Task 3: Rewrite `main.rs` startup — remove SQLite path logic, add PostgreSQL pool**
  - Remove the entire SQLite path-extraction block (lines 53–74): the `strip_prefix("sqlite:///")` logic, `split('?')` for query params, `std::fs::create_dir_all`
  - Replace `SqlitePoolOptions` with `PgPoolOptions`:
    ```rust
    use sqlx::postgres::PgPoolOptions;
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .unwrap_or_else(|e| { /* error handling */ });
    ```
  - Add health-check retry loop when connecting to PostgreSQL (it may not be ready immediately in Docker):
    ```rust
    let mut retries = 0;
    let max_retries = 30;
    loop {
        match PgPoolOptions::new().max_connections(1).connect(&config.database_url).await {
            Ok(pool) => { db = pool; break; }
            Err(e) if retries < max_retries => { retries += 1; tokio::time::sleep(Duration::from_secs(1)).await; }
            Err(e) => { tracing::error!("Failed to connect to database after {max_retries} retries: {e}"); std::process::exit(1); }
        }
    }
    ```
  - Update doc comments: remove all "SQLite" references
  - Update `run_migrations` call comment and function name

  LOGGING REQUIREMENTS:
  - Log database connection attempt and retry count: `[main] connecting to PostgreSQL (attempt {n}/{max_retries})`
  - Log successful connection: `[main] database connected: {redacted_url}`
  - Log migration success/failure with table names

  Files: backend/src/main.rs

### Phase 2: Schema & Data Layer Rewrite

- [x] **Task 4: Create sqlx migration files (replace inline `run_migrations`)**
  - Create `backend/migrations/` directory
  - Create initial migration `00000000000001_create_collections.sql`:
    ```sql
    CREATE TABLE IF NOT EXISTS collections (
        id UUID PRIMARY KEY,
        name VARCHAR(255) NOT NULL UNIQUE,
        description TEXT,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );
    ```
  - Create `00000000000002_create_documents.sql`:
    ```sql
    CREATE TABLE IF NOT EXISTS documents (
        id UUID PRIMARY KEY,
        name VARCHAR(255) NOT NULL,
        file_type VARCHAR(50) NOT NULL,
        file_size BIGINT NOT NULL,
        uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        collection_id UUID NOT NULL,
        is_active BOOLEAN NOT NULL DEFAULT TRUE,
        FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
    );
    ```
  - Create `00000000000003_create_chunks.sql`:
    ```sql
    CREATE TABLE IF NOT EXISTS chunks (
        id UUID PRIMARY KEY,
        document_id UUID NOT NULL,
        "index" INTEGER NOT NULL,
        text TEXT NOT NULL,
        is_active BOOLEAN NOT NULL DEFAULT TRUE,
        FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
    );
    ```
  - Create `00000000000004_create_sessions.sql`:
    ```sql
    CREATE TABLE IF NOT EXISTS sessions (
        id UUID PRIMARY KEY,
        title VARCHAR(255) NOT NULL DEFAULT 'New Chat',
        collection_id UUID,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE SET NULL
    );
    ```
  - Create `00000000000005_create_messages.sql`:
    ```sql
    CREATE TABLE IF NOT EXISTS messages (
        id UUID PRIMARY KEY,
        session_id UUID NOT NULL,
        role VARCHAR(20) NOT NULL CHECK(role IN ('user', 'assistant')),
        content TEXT NOT NULL,
        sources JSONB,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
    );
    ```
  - Create `00000000000006_create_git_repositories.sql`:
    ```sql
    CREATE TABLE IF NOT EXISTS git_repositories (
        id UUID PRIMARY KEY,
        url TEXT NOT NULL,
        branch VARCHAR(255) NOT NULL DEFAULT 'main',
        access_token TEXT,
        local_path TEXT NOT NULL,
        last_commit_hash TEXT,
        last_synced_at TIMESTAMPTZ,
        collection_id UUID NOT NULL,
        status VARCHAR(20) NOT NULL DEFAULT 'idle' CHECK(status IN ('idle','syncing','error')),
        webhook_secret TEXT,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_git_repos_collection ON git_repositories(collection_id);
    ```
  - Replace `run_migrations` function in `main.rs` with `sqlx::migrate!().run(&db).await` call
  - Remove the inline `CREATE TABLE` and `ALTER TABLE` SQL from `main.rs`
  - Ensure the `migrate` feature is enabled in Cargo.toml (already in Task 2)

  LOGGING REQUIREMENTS:
  - Log each migration applied: `[main] migration applied: {migration_name}`
  - Log migration errors with full context

  Files: backend/migrations/ (new directory, 6 .sql files), backend/src/main.rs

- [x] **Task 5: Rewrite all repository files — `SqlitePool` → `PgPool`, `?` → `$N`, `is_active = 1` → `TRUE`**
  - **`backend/src/modules/collections/repository.rs`**:
    - `use sqlx::SqlitePool` → `use sqlx::PgPool`
    - `SqlitePool` → `PgPool` in struct fields, constructors, and all function signatures
    - All `?` placeholders in SQL → `$1, $2, ...` numbered parameters
    - Update doc comments: "SQLite" → "PostgreSQL" where appropriate
  - **`backend/src/modules/conversations/repository.rs`**:
    - Same pattern: `SqlitePool` → `PgPool`, `?` → `$N`
  - **`backend/src/modules/documents/repository.rs`**:
    - `use sqlx::{QueryBuilder, Sqlite, SqlitePool}` → `use sqlx::{QueryBuilder, Postgres, PgPool}`
    - `QueryBuilder<Sqlite>` → `QueryBuilder<Postgres>` (3 instances)
    - `SqlitePool` → `PgPool`
    - All `?` placeholders → numbered `$1, $2, ...`
    - `is_active = 1` → `is_active = TRUE`, `is_active = 0` → `is_active = FALSE` (throughout)
    - `fn db_pool(&self) -> &SqlitePool` → `&PgPool`
  - **`backend/src/modules/git_sync/repository.rs`**:
    - Same pattern: `SqlitePool` → `PgPool`, `?` → `$N`
    - All `SqliteRow`/`FromRow` usage stays the same (derive macro is DB-agnostic)
  - **`backend/src/modules/query/repository.rs`**:
    - Same pattern: `SqlitePool` → `PgPool`, `?` → `$N`
    - Dynamic IN-clause builder: change `"?".repeat(n)` → `"$1", "$2", ...` using `(1..=n).map(|i| format!("${i}"))`
  - **`backend/src/modules/query/service.rs`**:
    - `SqlitePool` → `PgPool` in constructor

  LOGGING REQUIREMENTS:
  - Each repository method should log at DEBUG level with parameters and results
  - Use format: `[Repository.method] message {key=value}`

  Files: collections/repository.rs, conversations/repository.rs, documents/repository.rs, git_sync/repository.rs, query/repository.rs, query/service.rs

- [x] **Task 6: Rewrite `documents/service.rs` and `git_sync/service.rs` test helpers**
  - In `documents/service.rs` test module:
    - Change `SqlitePool::connect("sqlite:file:test-{db_name}?mode=memory&cache=shared")` → `PgPoolOptions::new().max_connections(1).connect(&pg_url)`
    - Or better: switch to `sqlx::test` macro with database provisioning
    - All `?` → `$N` in any inline SQL within test setups
  - In `git_sync/service.rs` test module:
    - Same: `SqlitePool::connect(":memory:")` → proper Pg test setup
  - In `git_sync/models.rs` and `git_sync/repository.rs`:
    - Verify all `FromRow` derives work with PostgreSQL types (UUID, TIMESTAMPTZ)
    - Update any `sqlx::FromRow` usage that references SQLite-specific type mappings

  LOGGING REQUIREMENTS:
  - Test helpers should log: `[test_setup] connecting to test database: {redacted_url}`

  Files: backend/src/modules/documents/service.rs, backend/src/modules/git_sync/service.rs, backend/src/modules/git_sync/models.rs

### Phase 3: Test Migration

- [x] **Task 7: Migrate all test modules to `sqlx::test` + PostgreSQL**
  - Add `sqlx::test` feature to `[dev-dependencies]` in `Cargo.toml` if not present
  - Replace all `:memory:` SQLite pools with `sqlx::test` attribute macro:
    - `#[sqlx::test]` creates a test database per test, runs migrations automatically
    - Each test function gets a `PgPool` parameter injected by the macro
  - **`backend/tests/common/mod.rs`**:
    - Replace `setup_test_db()` to use `sqlx::test` or create a helper that returns `PgPool`
    - Remove `PRAGMA table_info` queries — replace with `information_schema.columns` checks or just let `sqlx::test` handle migrations
  - **`backend/src/modules/collections/repository.rs` (mod tests)**:
    - Replace `SqlitePoolOptions::new().connect(":memory:")` with `sqlx::test`-based setup
  - **`backend/src/modules/documents/repository.rs` (mod tests)**:
    - Same replacement
  - **`backend/src/modules/documents/reindex_tests.rs`**:
    - Replace both `setup_db_no_is_active()` and `setup_db_with_is_active()`: these currently create a pool without the `is_active` column, then apply the migration. With `sqlx::test`, use migration fixtures or custom fixtures.
  - **`backend/src/modules/git_sync/repository.rs` (mod tests)**:
    - Replace all 3 `setup_git_test_db()` and inline `:memory:` pools
  - **`backend/src/modules/auth/` tests**:
    - Update `database_url: ":memory:"` → proper test database URL or `sqlx::test` fixture
  - **`backend/tests/integration.rs`**:
    - Update to use `PgPool` instead of `SqlitePool`
  - Ensure `sqlx::test` uses `migrations = "../migrations"` path (relative to test file)
  - Set `SQLX_TESTING=true` or the appropriate env variable for offline mode if desired

  LOGGING REQUIREMENTS:
  - Each test should log at start: `[test_name] starting with PostgreSQL test database`
  - Each test should log at end: `[test_name] completed successfully`

  Files: backend/tests/common/mod.rs, backend/tests/integration.rs, all repository *_tests* modules, reindex_tests.rs

### Phase 4: Scripts, CI & Documentation

- [x] **Task 8: Update backup and restore scripts for PostgreSQL**
  - In `scripts/backup.sh`:
    - Replace SQLite file copy (`cp ./data/vedo.db`) with `pg_dump`:
      ```bash
      VEDO_BACKUP="$BACKUP_DIR/vedo-$TIMESTAMP.sql.gz"
      docker compose exec -T db pg_dump -U vedo vedo | gzip > "$VEDO_BACKUP"
      KC_BACKUP="$BACKUP_DIR/keycloak-$TIMESTAMP.sql.gz"
      docker compose exec -T db pg_dump -U keycloak keycloak | gzip > "$KC_BACKUP"
      ```
    - Replace `stop backend` → no need to stop backend for `pg_dump` (PostgreSQL handles concurrent dumps)
    - Update pruning: `vedo-*.db` → `vedo-*.sql.gz`, add `keycloak-*.sql.gz`
    - Update Chroma backup as-is (unchanged)
  - In `scripts/restore.sh`:
    - Replace SQLite file copy with `psql` restore:
      ```bash
      gunzip -c "$DB_FILE" | docker compose exec -T db psql -U vedo vedo
      gunzip -c "$KC_FILE" | docker compose exec -T db psql -U keycloak keycloak
      ```
    - Update usage text and argument parsing for `.sql.gz` files
    - Stop backend before restore (needed for data consistency)
    - Restart backend after restore

  LOGGING REQUIREMENTS:
  - Log each step in backup/restore with timestamps
  - Log backup file sizes and paths

  Files: scripts/backup.sh, scripts/restore.sh

- [x] **Task 9: Update CI — PostgreSQL service container for tests**
  - In `.github/workflows/ci.yml`:
    - Add a PostgreSQL service container to the `backend` job:
      ```yaml
      services:
        postgres:
          image: postgres:16-alpine
          env:
            POSTGRES_USER: vedo
            POSTGRES_PASSWORD: vedo_test
            POSTGRES_DB: vedo
          ports:
            - 5432:5432
          options: >-
            --health-cmd "pg_isready -U vedo"
            --health-interval 10s
            --health-timeout 5s
            --health-retries 5
      ```
    - Add `DATABASE_URL: postgres://vedo:vedo_test@localhost:5432/vedo` env var to the test steps
    - Keep the existing Chroma service for integration tests
    - Update the integration test command to use the PostgreSQL URL
  - Verify E2E workflow (`.github/workflows/e2e.yml`) — no changes needed since E2E tests use Docker Compose

  LOGGING REQUIREMENTS:
  - N/A for CI configuration

  Files: .github/workflows/ci.yml

- [x] **Task 10: Update documentation**
  - In `docs/architecture.md`:
    - Replace all `SQLite` references with `PostgreSQL`
    - Update architecture diagram: `SQL[(SQLite<br/>Metadata)]` → `SQL[(PostgreSQL<br/>vedo + keycloak)]` or similar
    - Update service description for the `db` container
  - In `docs/configuration.md`:
    - Change `DATABASE_URL` default from `sqlite:/data/vedo.db?mode=rwc` to `postgres://vedo:password@db:5432/vedo`
    - Update volume table: remove `db_data` (SQLite), rename `keycloak_db_data` → `db_data` (PostgreSQL)
    - Add `VEDO_DB_PASSWORD` env var documentation
  - In `docs/deployment.md`:
    - Update architecture diagram (remove SQLite, add PostgreSQL)
    - Update backup instructions: `pg_dump` instead of file copy
  - In `docs/getting-started.md`:
    - Update service table: rename `keycloak-db` → `db`
    - Add `VEDO_DB_PASSWORD` to the env var setup
  - In `docs/auth.md`:
    - Update `keycloak_db_data` → `db_data` references
  - In `ARCHITECTURE.md` (if it references SQLite):
    - Update all `SQLite` → `PostgreSQL` mentions
    - Update the `SqlitePool` reference in the code example to `PgPool`
  - In `.ai-factory/DESCRIPTION.md`:
    - Change "SQLite (metadata)" to "PostgreSQL (metadata + auth)"
    - Update tech stack line: "Database: PostgreSQL 16 (metadata + auth)" or similar
  - In `AGENTS.md`:
    - Update project structure references (remove SQLite-specific comments)
    - Update docker-compose service descriptions

  LOGGING REQUIREMENTS:
  - N/A for documentation

  Files: docs/architecture.md, docs/configuration.md, docs/deployment.md, docs/getting-started.md, docs/auth.md, .ai-factory/ARCHITECTURE.md, .ai-factory/DESCRIPTION.md, AGENTS.md

### Phase 5: Validation & Cleanup

- [x] **Task 11: Full-stack validation — build, lint, test, and smoke-test**
  - Run `cargo fmt` and `cargo clippy` in `backend/`
  - Run `cargo test` (unit tests) — must pass with `DATABASE_URL` pointing to a real PostgreSQL
  - Run `cargo test --test integration` — must pass against PostgreSQL + Chroma services
  - Run `ruff format` and `ruff check` in `embedding/`
  - Run `npx @biomejs/biome ci .` in `frontend/`
  - Run `npm run test -- --run` in `frontend/`
  - Run `docker compose build backend` — verify the image builds without SQLite deps
  - Run `docker compose up -d` — verify all services start and backend can connect to `db`
  - Run `docker compose exec backend curl -s http://localhost:3000/api/health` — verify health check
  - Run `scripts/backup.sh` — verify pg_dump works
  - Verify `scripts/restore.sh` (manual smoke test)
  - Fix any issues found during validation

  LOGGING REQUIREMENTS:
  - Log each validation step result
  - Log any errors with full stack traces

  Files: All backend Rust files, all modified config/docs files