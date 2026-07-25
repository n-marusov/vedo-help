#!/usr/bin/env bash
# ============================================================================
# Validate Docker Compose Configuration & Port Uniqueness
# ============================================================================
# Checks:
#   1. docker-compose.yml config is valid (YAML + env interpolation)
#   2. docker-compose.test.yml config is valid
#   3. No host port overlap between dev and test stacks (prevents
#      "port already allocated" errors when profiles run concurrently)
#
# Usage:
#   ./scripts/validate-compose-ports.sh
#
# Exit codes:
#   0 — all checks passed
#   1 — one or more checks failed
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass()  { echo -e "  ${GREEN}✓${NC} $1"; }
fail()  { echo -e "  ${RED}✗${NC} $1"; }
info()  { echo -e "  ${YELLOW}→${NC} $1"; }

FAILURES=0

# --------------------------------------------------------------------------
# Check 1: Dev compose config
# --------------------------------------------------------------------------
check_dev_config() {
  info "Validating docker-compose.yml..."
  if docker compose --env-file "$PROJECT_DIR/.env" config --quiet 2>&1; then
    pass "docker-compose.yml — valid config"
    return 0
  else
    fail "docker-compose.yml — config validation failed"
    return 1
  fi
}

# --------------------------------------------------------------------------
# Check 2: Test compose config
# --------------------------------------------------------------------------
check_test_config() {
  info "Validating docker-compose.test.yml..."
  # .env.test may not exist in CI; if missing, fall back to empty env
  local env_flag=""
  [[ -f "$PROJECT_DIR/.env.test" ]] && env_flag="--env-file $PROJECT_DIR/.env.test"
  # shellcheck disable=SC2086
  if docker compose $env_flag -f "$PROJECT_DIR/docker-compose.test.yml" config --quiet 2>&1; then
    pass "docker-compose.test.yml — valid config"
    return 0
  else
    fail "docker-compose.test.yml — config validation failed"
    return 1
  fi
}

# --------------------------------------------------------------------------
# Check 3: Host port uniqueness (dev vs test)
# --------------------------------------------------------------------------
check_port_uniqueness() {
  info "Checking host port uniqueness between dev and test stacks..."
  local overlap=""
  local dev_ports test_ports

  # Extract host ports from dev compose (only services with ports published)
  # Matches lines like "  - \"18001:8001\"" or "${BACKEND_PORT:-3000}:3000"
  dev_ports=$(docker compose --env-file "$PROJECT_DIR/.env" config \
    | sed -n 's/^\s*-\s*"\([0-9]*\):[0-9]*".*/\1/p' \
    | sort -u)

  local env_flag=""
  [[ -f "$PROJECT_DIR/.env.test" ]] && env_flag="--env-file $PROJECT_DIR/.env.test"
  # shellcheck disable=SC2086
  test_ports=$(docker compose $env_flag -f "$PROJECT_DIR/docker-compose.test.yml" config \
    | sed -n 's/^\s*-\s*"\([0-9]*\):[0-9]*".*/\1/p' \
    | sort -u)

  # Find intersection
  overlap=$(comm -12 <(echo "$dev_ports") <(echo "$test_ports") 2>/dev/null || true)

  if [[ -z "$overlap" ]]; then
    pass "Dev and test profiles use disjoint host port ranges"
    info "  Dev ports:  $(echo "$dev_ports"  | tr '\n' ' ')"
    info "  Test ports: $(echo "$test_ports" | tr '\n' ' ')"
    return 0
  else
    fail "Host port collision(s) between dev and test profiles: $(echo "$overlap" | tr '\n' ' ')"
    info "  Dev ports:  $(echo "$dev_ports"  | tr '\n' ' ')"
    info "  Test ports: $(echo "$test_ports" | tr '\n' ' ')"
    info "  Fix: change the overlapping port(s) in docker-compose.test.yml"
    return 1
  fi
}

# ============================================================================
# Main
# ============================================================================

echo "═══════════════════════════════════════════════════════════════════"
echo "  Docker Compose Configuration Validation"
echo "═══════════════════════════════════════════════════════════════════"
echo ""

check_dev_config   || ((FAILURES++))
echo ""
check_test_config  || ((FAILURES++))
echo ""
check_port_uniqueness || ((FAILURES++))
echo ""

echo "═══════════════════════════════════════════════════════════════════"
if [[ "$FAILURES" -eq 0 ]]; then
  echo -e "  ${GREEN}All compose validation checks passed!${NC}"
else
  echo -e "  ${RED}$FAILURES compose validation failure(s)${NC}"
fi
echo "═══════════════════════════════════════════════════════════════════"

exit "$FAILURES"
