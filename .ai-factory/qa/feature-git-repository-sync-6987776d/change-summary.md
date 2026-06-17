## Change Summary

**Commits:** 1 (plan only) + 3 staged test files
**Changed files:** 4 (1 plan, 3 test files)
**Risk level:** 🟢 Low (Phase 1 tests-first — no production code yet)

---

### What Changed

Phase 1 of the Git Repository Sync feature implements **TDD tests-first**: E2E (Playwright), integration (Rust + Chroma), and unit tests for the new `git_sync` module. No production code has been written yet — these tests serve as executable specifications for the upcoming implementation phases (2–6). All three test files are staged but not yet committed.

---

### Affected Areas

| Component     | Change type | Description |
|---------------|-------------|-------------|
| `frontend/e2e/git-sync.spec.ts` | Added | 6 E2E tests: register, sync, delete, validation, error state, multi-repo listing |
| `backend/tests/git_sync_integration.rs` | Added | 7 integration tests: create/list contract, sync fixture, incremental sync, delete cleanup, empty repo, nested dirs, status transitions contract |
| `backend/tests/git_sync_unit.rs` | Added | 13 unit tests: repository CRUD (5), service contracts (token injection, file parsing, pipeline order, metadata) |
| `.ai-factory/plans/feature-git-repository-sync.md` | Added | Implementation plan defining the complete feature scope (phases 1–6) |

---

### Evidence

| Finding | Evidence |
|---------|----------|
| E2E tests cover all 6 scenarios from Task 1 | `frontend/e2e/git-sync.spec.ts` L49–482 — TC-GIT-001 through TC-GIT-006 |
| Integration tests cover all 7 scenarios from Task 2 | `backend/tests/git_sync_integration.rs` L125–716 — 7 async tests |
| Unit tests cover all 13 scenarios from Task 3 | `backend/tests/git_sync_unit.rs` L25–745 — 13 tests |
| Access token is never serialized in summaries | Unit test L129–140 + L137–140; Integration contract L110–143 |
| Pipeline order is documented: clone→parse→chunk→embed→index | Unit test L595–630 |
| Incremental sync path is documented: pull→diff→parse_changed→reindex | Unit test L634–671 |
| Three status values enforced: idle, syncing, error | Integration test L708–714; Unit test DB CHECK constraint |
| Metadata contract: source="git" + repo_id + file_path | Unit test L712–745 |

---

### Risks

🔴 **Critical** (must verify):

- **Contract-only tests won't catch real integration bugs:** `test_create_and_list_repo_contract` (L125–144) and `test_sync_status_transitions_contract` (L661–716) only validate JSON shapes — they do not exercise actual HTTP handlers or the service pipeline. These are "Red phase" stubs that will pass green without ever calling real code.
- **No test for concurrent sync safety:** No test verifies behavior when two sync requests arrive simultaneously. Risk of database corruption or duplicate indexing.
- **Missing auth/session tests:** None of the E2E tests verify that an unauthenticated user cannot access Git repo endpoints.

🟡 **Medium** (should verify):

- **No UI test for empty repos state:** When no repos are registered, the E2E tests don't verify that an empty-state message or placeholder is shown.
- **No test for token UI masking:** E2E tests fill the token field but don't verify the token is masked in the UI (password-type input) or excluded from client-accessible state.
- **No test for delete dialog cancel/escape:** TC-GIT-003 only tests confirm; doesn't test dismissing the dialog via cancel or Escape key.
- **No webhook tests in Phase 1:** Webhook payload validation and signature verification are pushed to Phase 5 — no integration test for invalid webhook signatures.
- **`test_parse_markdown_finds_only_md_files` is non-recursive:** The unit test helper (L529–546) only scans the top-level directory, not nested directories like the real pipeline must. The integration test covers nested dirs, but this unit test is misleadingly shallow.

🟢 **Low** (nice to verify):

- No edge-case tests for git@ (SSH) URL token injection
- No test for branch name with special characters (`/`, `#`, emoji)
- No test for markdown frontmatter parsing (YAML/TOML in `.md` files)
- E2E tests use `data-testid` selectors that don't exist yet (component not built)

---

### Testing Recommendations

**First priority:**

- [ ] Add integration test that exercises actual backend handlers (not just JSON shape contracts) once Phase 4 is complete
- [ ] Add test for concurrent sync safety — two threads attempting sync on the same repo
- [ ] Add E2E test for protected routes: unauthenticated request to `/api/git-repos` returns 401

**Regression:**

- [ ] Ensure existing Chroma test infrastructure (`tests/integration.rs`) still passes alongside git_sync fixtures
- [ ] Verify `CHROMA_URL` env variable behavior when not set (tests should skip or fail with clear message)
