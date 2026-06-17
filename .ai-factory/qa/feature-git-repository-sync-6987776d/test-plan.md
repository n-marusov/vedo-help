## Test Plan: Git Repository Sync — Phase 1 Tests First

**Date:** 2026-06-18
**Branch / Version:** feature/git-repository-sync
**Environment:** Local development (Chroma service running, no production code yet)

---

### 1. Testing Goal

Verify that Phase 1 (Tests First) test coverage is complete for the Git Repository Sync feature. Assess whether the existing 26 tests (6 E2E + 7 integration + 13 unit) fully specify all contracts defined in Tasks 1–3 of the implementation plan. Identify gaps and add missing tests before Phase 2 implementation begins.

---

### 2. Test Scope

**In Scope** — we test:

- All 6 E2E scenarios from Task 1: register, sync, delete, validation, error, listing
- All 7 integration scenarios from Task 2: CRUD contract, sync-from-fixture, incremental, delete-cleanup, empty-repo, nested-dirs, status-transitions
- All 13 unit scenarios from Task 3: repository CRUD (5), service contracts (8)
- New tests for identified gaps: unprotected route auth, delete dialog cancel, concurrent sync, UI empty state

**Out of Scope** — we don't test:

- Phase 2–6 implementation code (not written yet)
- Webhook payload validation (Phase 5 task)
- Frontend component render tests (Vitest) — the component doesn't exist yet
- Performance under load (Phase 1 is contract validation only)
- Production deployment configuration

---

### 3. Test Types

| Type | Priority | Area |
|------|----------|------|
| Functional | 🔴 High | Core sync pipeline: register → sync → index → query |
| Functional | 🔴 High | Access token security: never exposed in API responses or UI state |
| Regression | 🔴 High | Auth protection: unauthenticated access returns 401 |
| Edge cases | 🟡 Medium | Concurrent sync on same repo |
| Edge cases | 🟡 Medium | Recursive directory scanning for nested .md files |
| Negative | 🟡 Medium | Invalid URL format, empty repo, broken remote |
| Negative | 🟡 Medium | UI dialog cancel/dismiss |
| Edge cases | 🟡 Medium | Empty repo list (zero-state UI) |
| Security | 🔴 High | Token masking in UI (password field type) |

---

### 4. Test Data

| Category | Data | Purpose |
|----------|------|----------|
| Valid data | `https://github.com/user/test-repo.git`, branch `main`, token `ghp_test123`, collection `col-1` | Happy path registration |
| Valid data | Fixture repo with 2–3 `.md` files + nested dirs | Integration sync pipeline |
| Boundary values | 10 MB `.md` file, 10 MB+1 `.md` file | File size skip threshold |
| Invalid data | `ftp://bad-protocol/repo.git`, empty URL, missing collection | Form validation |
| Invalid data | `https://nonexistent.invalid/repo.git` | Sync error state |
| Special cases | No `.md` files, only `.txt`/`.png` files | Empty sync result |
| Auth data | Valid JWT token, missing token, expired token | Auth protection |
| Concurrent | Two parallel sync triggers on same repo ID | Race condition safety |

---

### 5. Preconditions

- [ ] Chroma service is running locally (or `CHROMA_URL` env var set)
- [ ] SQLite in-memory database is available via `setup_test_db()`
- [ ] `git` CLI is available on PATH (for fixture repo creation)
- [ ] `walkdir` and `tempfile` crates are in dev-dependencies
- [ ] Playwright is configured with test user tokens (`vedo_auth_token`, `vedo_api_key`)
- [ ] `.ai-factory/plans/feature-git-repository-sync.md` is available as reference

---

### 6. Acceptance Criteria

- [ ] All 6 E2E tests from Task 1 are present and cover every required scenario
- [ ] All 7 integration tests from Task 2 are present and validated
- [ ] All 13 unit tests from Task 3 are present
- [ ] At least 1 new E2E test added for unauthenticated Git repo access → 401
- [ ] At least 1 new E2E test added for delete dialog cancel/Escape
- [ ] At least 1 new unit test added for recursive .md file discovery
- [ ] At least 1 new integration test added for concurrent sync safety
- [ ] Access token NEVER appears in any test's expected output/summary JSON
- [ ] Pipeline order contract tests are self-documenting (clone→parse→chunk→embed→index→update)
- [ ] No tests silently pass without exercising real code paths (no false greens)

---

### 7. Plan Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Contract-only tests give false green | High — real bugs only caught in Phase 4 | Mark two stubs clearly as Red-phase; add real handler tests in Phase 4 |
| Chroma not available in CI | Medium — integration tests skip/fail | Tests use `CHROMA_URL` env; CI workflow already has Chroma service container |
| `data-testid` selectors don't exist yet | Low — E2E tests can't run until component exists | Tests serve as contract docs; full E2E run deferred to Phase 6 |
| Test fixtures accumulate on disk | Low | All git fixtures use `tempfile::tempdir()` → auto-cleanup |

### 8. Checklist

| Check | Priority |
|-------|----------|
| E2E: register repo → row appears with idle badge (TC-GIT-001) | 🔴 High |
| E2E: trigger sync → status transitions → files_indexed shown (TC-GIT-002) | 🔴 High |
| E2E: delete repo → confirm → row removed (TC-GIT-003) | 🔴 High |
| E2E: empty form submit → required field errors (TC-GIT-004) | 🟡 Medium |
| E2E: invalid URL → format error message (TC-GIT-004) | 🟡 Medium |
| E2E: sync broken URL → error badge + tooltip (TC-GIT-005) | 🔴 High |
| E2E: multiple repos listed → correct collection names (TC-GIT-006) | 🟡 Medium |
| E2E: unauthenticated access → 401 (NEW) | 🔴 High |
| E2E: delete dialog cancel → row stays (NEW) | 🟡 Medium |
| E2E: empty repos list → zero-state message shown (NEW) | 🟡 Medium |
| Integration: POST create → GET list → verify contract shape | 🔴 High |
| Integration: sync fixture repo → files indexed > 0 → query Chroma | 🔴 High |
| Integration: incremental sync detects new file | 🔴 High |
| Integration: delete repo cleans Chroma + local clone dir | 🔴 High |
| Integration: empty repo sync → files_indexed=0, status=idle | 🟡 Medium |
| Integration: nested directories → both files indexed | 🟡 Medium |
| Integration: status transitions idle→syncing→idle/error contract | 🔴 High |
| Integration: concurrent sync on same repo (NEW) | 🟡 Medium |
| Unit: create repo persists all fields, summary omits token | 🔴 High |
| Unit: list repos returns all (3 repos) | 🟡 Medium |
| Unit: update sync status changes commit_hash + status | 🔴 High |
| Unit: delete repo removes row | 🔴 High |
| Unit: same URL with different collection_id allowed | 🟡 Medium |
| Unit: token injection into HTTPS URL | 🔴 High |
| Unit: no-token URL passed through unchanged (HTTPS, SSH, file://) | 🟡 Medium |
| Unit: parse finds only .md files (top-level) | 🟡 Medium |
| Unit: parse skips files > 10 MB | 🟡 Medium |
| Unit: full clone pipeline order (clone→parse→chunk→embed→index→update) | 🔴 High |
| Unit: incremental sync pipeline (pull→diff→parse_changed→reindex) | 🔴 High |
| Unit: sync failure → error status with message | 🔴 High |
| Unit: index metadata contract (source="git", repo_id, file_path) | 🔴 High |
| Unit: recursive .md file discovery in nested dirs (NEW) | 🟡 Medium |
