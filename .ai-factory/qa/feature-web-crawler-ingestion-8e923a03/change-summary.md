# Change Summary: Web Crawler & Site Ingestion

**Branch:** `feature/web-crawler-ingestion`
**Date:** 2026-07-11
**Based on plan:** `.ai-factory/plans/feature-web-crawler-ingestion.md`

## Overview

Added Phase 1 (TDD) test suite for the web crawler feature. The feature adds a new document source type ("web") alongside existing "upload" and "git" sources. Users can enter a starting URL and crawl a website (BFS) to extract text content and index it into Chroma.

The module structure mirrors `git_sync`: crawler engine → models → repository → service → handlers → API + SSE progress.

## Files Added

| File | Type | Purpose |
|------|------|---------|
| `frontend/e2e/web-crawl.spec.ts` | E2E tests | 10 Playwright test scenarios for crawl UI and API |
| `backend/tests/web_crawl_unit.rs` | Unit tests | 16 tests: DB contract, URL normalization, traversal limits, deduplication |
| `backend/tests/web_crawl_integration.rs` | Integration tests | 6 tests: full lifecycle, cancel cascade, multi-job isolation, error handling |

## Test Framework

| Test type | Tool | Config |
|-----------|------|--------|
| E2E | Playwright | `frontend/e2e/` — requires Docker test stack |
| Backend unit | Cargo + sqlx + serial_test | PostgreSQL test database via `setup_test_db()` |
| Backend integration | Cargo + sqlx + serial_test | Same DB with multi-table assertions |

## Changes to Existing Files

| File | Change |
|------|--------|
| `backend/tests/common/mod.rs` | Added 8 missing `AppConfig` fields (`query_cache_ttl_secs`, `query_cache_max_entries`, `query_rate_limit_requests`, `query_rate_limit_window_secs`, `notification_*`) |

## Dependencies Required (from plan)

- `reqwest` (already in Cargo.toml)
- `scraper = "0.22"` (needs to be added)
- `robotstxt = "1.1"` (needs to be added)

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tables don't exist in DB | All DB tests fail | Create migrations `00000000000016` and `00000000000017` |
| Web crawl module not implemented | Code doesn't compile outside tests | Implement Phase 2-4 of the plan |
| URL normalization correctness | Edge cases with malformed URLs | Tests cover fragment stripping, trailing slashes, relative resolution |
