#!/usr/bin/env bash
# =============================================================================
# Validate Keycloak Realm Template Substitution
# =============================================================================
# Regression test for the keycloak-init template substitution logic.
# Verifies that sed-based substitution produces correct realm-import.json
# with all variables replaced and no unresolved ${...} placeholders.
#
# This mirrors the actual substitution done in docker-compose.yml's
# keycloak-init service (sed-based, not envsubst).
#
# Usage:
#   ./scripts/validate-keycloak-template.sh
#
# Exit codes:
#   0 — all checks passed
#   1 — one or more checks failed
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TEMPLATE="$PROJECT_DIR/keycloak/realm-import.json.template"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}✓${NC} $1"; }
fail() { echo -e "  ${RED}✗${NC} $1"; }
info() { echo -e "  ${YELLOW}→${NC} $1"; }

FAILURES=0

# --------------------------------------------------------------------------
# Prerequisites
# --------------------------------------------------------------------------
check_prereqs() {
  if [[ ! -f "$TEMPLATE" ]]; then
    fail "Template file not found: $TEMPLATE"
    return 1
  fi
  pass "Template file exists"
  return 0
}

# --------------------------------------------------------------------------
# Test: substitution produces no unresolved placeholders
# --------------------------------------------------------------------------
test_no_unresolved_placeholders() {
  info "Checking that sed substitution resolves all \${...} placeholders..."

  local output
  output=$(mktemp)
  # shellcheck disable=SC2155
  trap 'rm -f "$output"' RETURN

  # Simulate the exact substitution from docker-compose.yml
  cp "$TEMPLATE" "$output"
  sed -i 's|${VEDO_BACKEND_CLIENT_SECRET}|test-client-secret|g' "$output"
  sed -i 's|${VEDO_ADMIN_PASSWORD}|test-admin-pw|g' "$output"
  sed -i 's|${VEDO_ALICE_PASSWORD}|test-alice-pw|g' "$output"
  sed -i 's|${VEDO_GUEST_PASSWORD}|test-guest-pw|g' "$output"

  # Check for any remaining ${...} placeholders
  local remaining
  remaining=$(grep -c '\${[A-Z_]*}' "$output" || true)

  if [[ "$remaining" -eq 0 ]]; then
    pass "No unresolved \${...} placeholders remain after substitution"
    return 0
  else
    fail "$remaining unresolved \${...} placeholders found after substitution"
    grep -n '\${[A-Z_]*}' "$output" || true
    return 1
  fi
}

# --------------------------------------------------------------------------
# Test: substituted values appear correctly
# --------------------------------------------------------------------------
test_substituted_values() {
  info "Checking that substituted values match expected output..."

  local output
  output=$(mktemp)
  # shellcheck disable=SC2155
  trap 'rm -f "$output"' RETURN

  cp "$TEMPLATE" "$output"
  sed -i 's|${VEDO_BACKEND_CLIENT_SECRET}|regression-test-secret|g' "$output"
  sed -i 's|${VEDO_ADMIN_PASSWORD}|regression-test-admin|g' "$output"
  sed -i 's|${VEDO_ALICE_PASSWORD}|regression-test-alice|g' "$output"
  sed -i 's|${VEDO_GUEST_PASSWORD}|regression-test-guest|g' "$output"

  local ok=true

  if grep -q '"regression-test-secret"' "$output"; then
    pass "VEDO_BACKEND_CLIENT_SECRET substituted correctly"
  else
    fail "VEDO_BACKEND_CLIENT_SECRET not found in output"
    ok=false
  fi

  if grep -q '"regression-test-admin"' "$output"; then
    pass "VEDO_ADMIN_PASSWORD substituted correctly"
  else
    fail "VEDO_ADMIN_PASSWORD not found in output"
    ok=false
  fi

  if grep -q '"regression-test-alice"' "$output"; then
    pass "VEDO_ALICE_PASSWORD substituted correctly"
  else
    fail "VEDO_ALICE_PASSWORD not found in output"
    ok=false
  fi

  if grep -q '"regression-test-guest"' "$output"; then
    pass "VEDO_GUEST_PASSWORD substituted correctly"
  else
    fail "VEDO_GUEST_PASSWORD not found in output"
    ok=false
  fi

  if $ok; then
    return 0
  else
    return 1
  fi
}

# --------------------------------------------------------------------------
# Test: output is valid JSON after substitution
# --------------------------------------------------------------------------
test_valid_json() {
  info "Checking that substituted output is valid JSON..."

  local output
  output=$(mktemp)
  # shellcheck disable=SC2155
  trap 'rm -f "$output"' RETURN

  cp "$TEMPLATE" "$output"
  sed -i 's|${VEDO_BACKEND_CLIENT_SECRET}|test-secret|g' "$output"
  sed -i 's|${VEDO_ADMIN_PASSWORD}|test-admin|g' "$output"
  sed -i 's|${VEDO_ALICE_PASSWORD}|test-alice|g' "$output"
  sed -i 's|${VEDO_GUEST_PASSWORD}|test-guest|g' "$output"

  if python3 -c "import json" 2>/dev/null; then
    if python3 -c "import json; json.load(open('$output'))" 2>/dev/null; then
      pass "Substituted output is valid JSON"
      return 0
    else
      fail "Substituted output is not valid JSON"
      python3 -c "import json; json.load(open('$output'))" 2>&1 || true
      return 1
    fi
  elif python -c "import json" 2>/dev/null; then
    if python -c "import json; json.load(open('$output'))" 2>/dev/null; then
      pass "Substituted output is valid JSON"
      return 0
    else
      fail "Substituted output is not valid JSON"
      python -c "import json; json.load(open('$output'))" 2>&1 || true
      return 1
    fi
  else
    # Fallback: basic structural check (braces balance)
    local open_braces close_braces
    open_braces=$(grep -o '{' "$output" | wc -l)
    close_braces=$(grep -o '}' "$output" | wc -l)
    if [[ "$open_braces" -eq "$close_braces" ]] && [[ "$open_braces" -gt 0 ]]; then
      pass "Substituted output has balanced braces (Python not available for strict validation)"
      return 0
    else
      fail "Substituted output has unbalanced braces (open=$open_braces, close=$close_braces)"
      return 1
    fi
  fi
}

# ============================================================================
# Main
# ============================================================================

echo "═══════════════════════════════════════════════════════════════════"
echo "  Keycloak Realm Template Substitution Validation"
echo "═══════════════════════════════════════════════════════════════════"
echo ""

check_prereqs || ((FAILURES++))
echo ""

echo "--- Substitution Tests ---"
test_no_unresolved_placeholders || ((FAILURES++))
test_substituted_values || ((FAILURES++))
test_valid_json || ((FAILURES++))
echo ""

echo "═══════════════════════════════════════════════════════════════════"
if [[ "$FAILURES" -eq 0 ]]; then
  echo -e "  ${GREEN}All template validation checks passed!${NC}"
else
  echo -e "  ${RED}$FAILURES template validation failure(s)${NC}"
fi
echo "═══════════════════════════════════════════════════════════════════"

exit "$FAILURES"
