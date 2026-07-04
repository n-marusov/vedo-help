#!/usr/bin/env bash
# ============================================================================
# Parallel Backend Test Runner
#
# Runs backend test binaries in parallel using separate PostgreSQL databases.
# Each binary gets its own database via TEST_DATABASE_ID, eliminating TRUNCATE
# race conditions.
#
# Prerequisites:
#   - Docker test environment is running (docker compose ... up -d)
#   - `dc` alias or docker compose available
#   - PostgreSQL user has CREATEDB (granted by setup)
#
# Usage:
#   bash scripts/run-backend-tests.sh            # Run all tests in parallel
#   bash scripts/run-backend-tests.sh --seq      # Run sequentially (debug)
#   bash scripts/run-backend-tests.sh --list     # List test binaries only
# ============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

# ── Docker compose helper ──
DC="docker compose --env-file .env.test -f docker-compose.test.yml"

# ── Test binaries ──
# Format: "binary_name|description|is_lib"
TESTS=(
  "lib|Library unit tests (59 tests)|true"
  "documents_db_unit|Document DB operations (15 tests)|false"
  "git_sync_unit|Git sync repo/service (18 tests)|false"
  "conversations_unit|Conversations repo (9 tests)|false"
  "health_unit|Health check logic (11 tests)|false"
  "integration|Chroma + DB integration (23 tests)|false"
  "auth_integration|Auth integration (3 tests)|false"
  "auth_middleware_test|Auth middleware (6 tests)|false"
  "conversations_integration|Conversations integration (7 tests)|false"
  "git_sync_integration|Git sync integration (8 tests)|false"
  "health_integration|Health integration (7 ignored)|false"
  "multi_tenancy|Multi-tenancy (8 tests, via sqlx::test)|false"
)

# ── Colors ──
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ── Parse arguments ──
if [ "${1:-}" = "--list" ]; then
  echo "Available test binaries:"
  for entry in "${TESTS[@]}"; do
    IFS='|' read -r name desc _ <<< "$entry"
    printf "  %-30s %s\n" "$name" "$desc"
  done
  exit 0
fi

SEQ_MODE=false
if [ "${1:-}" = "--seq" ]; then
  SEQ_MODE=true
fi

# ── Create test databases (parallel mode only) ──
if [ "$SEQ_MODE" = false ]; then
  echo -e "${BLUE}[setup] Creating test databases...${NC}"
  for entry in "${TESTS[@]}"; do
    IFS='|' read -r name _ is_lib <<< "$entry"
    [ "$is_lib" = "true" ] && continue           # lib tests don't need DB
    [ "$name" = "multi_tenancy" ] && continue    # sqlx::test manages its own

    echo -n "  Creating vedo_test_$name ... "
    $DC exec -T db psql -U postgres -c \
      "CREATE DATABASE \"vedo_test_$name\" OWNER vedo;" 2>/dev/null && \
      echo -e "${GREEN}ok${NC}" || echo -e "${YELLOW}already exists${NC}"
  done
  echo -e "${GREEN}[setup] All test databases ready${NC}"
fi

echo ""
echo "=============================================="
echo "  Running backend tests..."
echo "=============================================="
echo ""

RESULTS_DIR=$(mktemp -d)
START_TIME=$(date +%s)

run_test() {
  local name="$1"
  local is_lib="$2"
  local log_file="$RESULTS_DIR/$name.log"

  local cmd=""
  if [ "$SEQ_MODE" = true ]; then
    # Sequential: shared database, --test-threads=1
    if [ "$is_lib" = "true" ]; then
      cmd="cargo test --manifest-path backend/Cargo.toml --lib 2>&1"
    elif [ "$name" = "multi_tenancy" ]; then
      cmd="DATABASE_URL='postgres://vedo:test-vedo-password@localhost:15432' cargo test --manifest-path backend/Cargo.toml --test $name -- --test-threads=1 2>&1"
    else
      cmd="cargo test --manifest-path backend/Cargo.toml --test $name -- --test-threads=1 2>&1"
    fi
  else
    # Parallel: separate databases
    if [ "$is_lib" = "true" ]; then
      cmd="cargo test --manifest-path backend/Cargo.toml --lib 2>&1"
    elif [ "$name" = "multi_tenancy" ]; then
      cmd="DATABASE_URL='postgres://vedo:test-vedo-password@localhost:15432' cargo test --manifest-path backend/Cargo.toml --test $name -- --test-threads=1 2>&1"
    else
      cmd="TEST_DATABASE_ID='$name' cargo test --manifest-path backend/Cargo.toml --test $name -- --test-threads=1 2>&1"
    fi
  fi

  local test_start=$(date +%s)
  eval "$cmd" > "$log_file" 2>&1
  local exit_code=$?
  local test_end=$(date +%s)
  local duration=$((test_end - test_start))

  local summary
  summary=$(grep "^test result:" "$log_file" | tail -1)

  if [ $exit_code -eq 0 ]; then
    echo -e "${GREEN}  ✅ $name${NC} — $summary (${duration}s)"
  else
    echo -e "${RED}  ❌ $name${NC} — $summary (${duration}s)"
  fi
}

# Launch ALL tests in parallel (background)
for entry in "${TESTS[@]}"; do
  IFS='|' read -r name _ is_lib <<< "$entry"
  run_test "$name" "$is_lib" &
done

# Wait for all parallel jobs
wait

END_TIME=$(date +%s)
TOTAL_SECONDS=$((END_TIME - START_TIME))

# ── Results ──
echo ""
echo "=============================================="
echo "  Results"
echo "=============================================="
echo ""

PASSED=0
FAILED=0
for entry in "${TESTS[@]}"; do
  IFS='|' read -r name desc _ <<< "$entry"
  log_file="$RESULTS_DIR/$name.log"
  summary=$(grep "^test result:" "$log_file" | tail -1)

  if grep -q "^test result:.* 0 failed" "$log_file" 2>/dev/null; then
    echo -e "  ${GREEN}✅ $name${NC} — $summary"
    PASSED=$((PASSED + 1))
  else
    echo -e "  ${RED}❌ $name${NC} — $summary"
    FAILED=$((FAILED + 1))
    # Show first failure details
    fail_line=$(grep -m1 "panicked\|FAILED" "$log_file" 2>/dev/null || true)
    if [ -n "$fail_line" ]; then
      echo -e "     ${YELLOW}First failure:${NC} $fail_line"
    fi
  fi
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ "$SEQ_MODE" = true ]; then
  echo -e "  Mode: ${YELLOW}Sequential${NC} (shared database)"
else
  echo -e "  Mode: ${GREEN}Parallel${NC} (separate databases per binary)"
fi
echo -e "  Passed: ${GREEN}$PASSED${NC} / Failed: ${RED}$FAILED${NC}"
echo -e "  Total time: ${BLUE}${TOTAL_SECONDS}s${NC} (wall clock)"

if [ "$SEQ_MODE" = false ]; then
  echo ""
  echo -e "${YELLOW}  Note: Individual test durations include first-run migration time.${NC}"
  echo -e "${YELLOW}  Subsequent runs skip migrations (idempotent).${NC}"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Cleanup
rm -rf "$RESULTS_DIR"

[ "$FAILED" -eq 0 ]
