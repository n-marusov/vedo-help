# Plan: Remove ADMIN_API_KEY from Application

**Branch:** `feature/remove-api-key`
**Created:** 2026-06-18
**Type:** refactor

## Settings

| Setting | Value |
|---------|-------|
| Testing | yes — update existing tests, ensure coverage |
| Logging | verbose — DEBUG-level logging during refactor |
| Docs | yes — update all documentation references |
| Roadmap | v0.3 — Admin Panel & Production Polish |

## Roadmap Linkage

- **Milestone:** v0.3 — Admin Panel & Production Polish
- **Rationale:** Security cleanup — removing legacy dual-auth mechanism that is superseded by KeyCloak OIDC. Fits under production polish scope.

## Overview

Remove the legacy `ADMIN_API_KEY` authentication mechanism from the entire application stack. Currently the backend supports **dual auth**: legacy static API key (`AuthInfo::ApiKey`) and KeyCloak JWT (`AuthInfo::User`). The frontend already uses only KeyCloak. This plan eliminates the API key path, simplifying auth to only JWT-based validation.

### Affected files

| File | Changes |
|------|---------|
| `backend/src/config.rs` | Remove `admin_api_key` field + env var parsing |
| `backend/src/shared/auth.rs` | Remove `AuthInfo::ApiKey`, `AuthToken` struct, simplify `authenticate_request()` |
| `backend/src/shared/mod.rs` | Remove `AuthToken` from re-exports |
| `backend/src/modules/auth/service.rs` | Remove `AuthInfo::ApiKey` match arm |
| `backend/src/modules/auth/models.rs` | Remove `AuthInfo::ApiKey` match arm from `UserContext` |
| `backend/src/modules/auth/handlers.rs` | Remove `AuthInfo::ApiKey` match in `logout` |
| `backend/tests/common/mod.rs` | Remove `admin_api_key` from test config |
| `.ai-factory/DESCRIPTION.md` | Update security section |
| `README.md` | Remove `ADMIN_API_KEY` from quick start |
| `docs/api.md` | Update auth section + all curl examples |
| `docs/architecture.md` | Remove API key fallback mention |
| `docs/auth.md` | Remove dual-auth wording |
| `backend/tests/auth_integration.rs` | New file — TDD auth integration tests |

## Tasks

### Phase 0: Tests First — Specification via Tests (TDD)

> **Strategy:** Write tests that define the *target* behavior before changing any production code.
> These tests will fail initially (red) and pass after Phase 1 implementation (green).
> This ensures the tests serve as executable documentation of the new auth contract.

#### [x] Task 1 — Update test helpers for JWT-only auth

- **Files:** `backend/tests/common/mod.rs`, `backend/src/config.rs` (test section)
- **Deliverable:** Test infrastructure ready for JWT-only auth.
- **Details:**
  1. In `tests/common/mod.rs`: remove `admin_api_key: "test-api-key".to_string()` from `setup_test_config()`
  2. In `config.rs` tests: update `test_config_defaults` — remove assertion on `admin_api_key`
- **Logging:** N/A
- **Dependencies:** None

#### [x] Task 2 — Write auth integration tests that define expected behavior

- **Files:** `backend/tests/auth_integration.rs` (new file)
- **Deliverable:** A new test file with 3 test cases:
  1. `test_no_auth_header_returns_401` — no Authorization → 401
  2. `test_invalid_token_returns_401` — Bearer invalid → 401
  3. `test_old_api_key_rejected_returns_401` — Bearer any-static-key → 401
- **Details:** Tests authenticate_request directly with no JWT validator. Test 3 is the key TDD spec.
- **Dependencies:** Task 1

### Phase 1: Backend Core Refactoring — Make Tests Pass

#### [x] Task 3 — Remove `admin_api_key` from AppConfig

- **Files:** `backend/src/config.rs`
- **Deliverable:** `AppConfig` struct loses `admin_api_key` field.
- **Dependencies:** Task 1

#### [x] Task 4 — Refactor `shared/auth.rs`: remove `AuthInfo::ApiKey`, `AuthToken`, simplify `authenticate_request`

- **Files:** `backend/src/shared/auth.rs`, `backend/src/shared/mod.rs`, `backend/src/main.rs`
- **Deliverable:** Only JWT auth path remains. AuthInfo is now a struct (not enum). AuthToken removed.
- **Dependencies:** Task 2

#### [x] Task 5 — Clean up auth module match arms

- **Files:** `backend/src/modules/auth/service.rs`, `backend/src/modules/auth/models.rs`, `backend/src/modules/auth/handlers.rs`
- **Deliverable:** No references to `AuthInfo::ApiKey` in the auth module.
- **Dependencies:** Task 3, Task 4

### Phase 2: Config & Documentation

#### [x] Task 6 — Clean up deployment configs

- **Files:** `.env.example`, `docker-compose.yml`, `docker-compose.override.yml`, `docker-compose.production.yml`
- **Deliverable:** No `ADMIN_API_KEY` references in deployment configuration (confirmed — none present).
- **Dependencies:** None

#### [x] Task 7 — Update project documentation

- **Files:** `docs/api.md`, `docs/architecture.md`, `docs/auth.md`, `README.md`, `.ai-factory/DESCRIPTION.md`
- **Deliverable:** All docs reflect JWT-only authentication.
- **Dependencies:** Task 3–5

### Phase 3: Polish & Verification

#### [x] Task 8 — Format, lint, and full verify

- **Files:** All changed files
- **Results:**
  - `cargo fmt` ✅
  - `cargo clippy` ✅ (no warnings)
  - `cargo test` — 37 lib + 15 git_sync_unit + 3 auth_integration = **55 tests passed** ✅
  - Ruff/Biome — no changes in those directories
- **Dependencies:** Task 1–7

## Commit Plan

| # | Tasks | Commit Message | Status |
|---|-------|---------------|--------|
| 1 | 1, 2 | `test(auth): add auth integration tests specifying JWT-only contract` | ✅ |
| 2 | 3, 4, 5 | `refactor(auth): remove legacy ADMIN_API_KEY authentication` | ✅ |
| 3 | 6, 7 | `chore(config,docs): remove ADMIN_API_KEY from configs and documentation` | ✅ |
| 4 | 8 | `style: format and lint after auth refactor` | ✅ |
