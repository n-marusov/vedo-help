#!/usr/bin/env bash
# =============================================================================
# Validate SQLx Migrations
# =============================================================================
# Checks that migration files under backend/migrations/ are internally
# consistent: no duplicate versions, no gaps, no version 0, no files
# with non-numeric prefixes, and (optionally, git-aware) no modified
# migrations that were already applied upstream.
#
# This prevents runtime failures like:
#   "migration N was previously applied but has been modified"
#
# Usage:
#   ./scripts/validate-migrations.sh               # basic validation
#   ./scripts/validate-migrations.sh --git          # also check vs origin/main
#
# Exit codes:
#   0 — all checks passed
#   1 — one or more checks failed
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MIGRATIONS_DIR="$PROJECT_DIR/backend/migrations"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}✓${NC} $1"; }
fail() { echo -e "  ${RED}✗${NC} $1"; }
warn() { echo -e "  ${YELLOW}⚠${NC} $1"; }
info() { echo -e "  ${YELLOW}→${NC} $1"; }

FAILURES=0
CHECK_GIT=false

for arg in "$@"; do
  case "$arg" in
    --git) CHECK_GIT=true ;;
    *)     echo "Unknown option: $arg"; exit 1 ;;
  esac
done

echo "═══════════════════════════════════════════════════════════════════"
echo "  Migration Validation"
echo "═══════════════════════════════════════════════════════════════════"
echo ""

# --------------------------------------------------------------------------
# Check 0: Directory exists
# --------------------------------------------------------------------------
if [[ ! -d "$MIGRATIONS_DIR" ]]; then
  fail "Migrations directory not found: $MIGRATIONS_DIR"
  exit 1
fi

# --------------------------------------------------------------------------
# Parse all migration files
# --------------------------------------------------------------------------
declare -a FILES
declare -a VERSIONS
declare -a DESCRIPTIONS

while IFS='|' read -r filename version desc; do
  FILES+=("$filename")
  VERSIONS+=("$version")
  DESCRIPTIONS+=("$desc")
done < <(
  for f in "$MIGRATIONS_DIR"/*.sql; do
    base=$(basename "$f" .sql)
    # Extract the numeric prefix (everything before the first underscore)
    # and the rest (description after the first underscore).
    ver="${base%%_*}"
    desc="${base#*_}"
    echo "$(basename "$f")|$ver|$desc"
  done
)

# --------------------------------------------------------------------------
# Check 1: All files match the expected naming pattern
# --------------------------------------------------------------------------
info "Check 1: Naming pattern (NNNNNNNNNNNN_description.sql)..."
BAD_NAMES=0
for f in "$MIGRATIONS_DIR"/*.sql; do
  base=$(basename "$f" .sql)
  if ! echo "$base" | grep -qE '^[0-9]+_.+$'; then
    fail "File '$base.sql' does not match pattern NNNNNNNNNNNN_description.sql"
    ((FAILURES++))
    ((BAD_NAMES++))
  fi
done
if [[ "$BAD_NAMES" -eq 0 ]]; then
  pass "All migration files follow the naming pattern"
fi
echo ""

# --------------------------------------------------------------------------
# Check 2: No duplicate version numbers
# --------------------------------------------------------------------------
info "Check 2: Unique version numbers..."
DUPLICATES=$(printf '%s\n' "${VERSIONS[@]}" | sort | uniq -d)
if [[ -n "$DUPLICATES" ]]; then
  while IFS= read -r ver; do
    DUPE_FILES=$(printf '%s\n' "${FILES[@]}" | while IFS= read -r f; do
      b=$(basename "$f" .sql)
      v="${b%%_*}"
      [[ "$v" == "$ver" ]] && echo "    $f"
    done)
    fail "Duplicate version $ver:"
    echo -e "$DUPE_FILES"
    ((FAILURES++))
  done <<< "$DUPLICATES"
else
  pass "All version numbers are unique"
fi
echo ""

# --------------------------------------------------------------------------
# Check 3: No version 0 (sqlx ignores it silently, but it's confusing)
# --------------------------------------------------------------------------
info "Check 3: No version 0..."
HAS_ZERO=false
for i in "${!VERSIONS[@]}"; do
  # 10# prefix forces decimal interpretation (leading zeros are ignored)
  if [[ $((10#${VERSIONS[$i]})) -eq 0 ]]; then
    fail "Version 0 is ignored by sqlx: ${FILES[$i]}"
    ((FAILURES++))
    HAS_ZERO=true
  fi
done
if ! $HAS_ZERO; then
  pass "No version-0 migrations"
fi
echo ""

# --------------------------------------------------------------------------
# Check 4: Versions are sequential (no gaps)
# --------------------------------------------------------------------------
info "Check 4: Sequential version numbers..."
SORTED_VERSIONS=()
while IFS= read -r v; do
  SORTED_VERSIONS+=("$v")
done < <(printf '%s\n' "${VERSIONS[@]}" | sort -n)

GAP_FOUND=false
for i in "${!SORTED_VERSIONS[@]}"; do
  expected=$((i + 1))
  # 10# prefix forces decimal interpretation
  actual=$((10#${SORTED_VERSIONS[$i]}))
  if [[ "$actual" -ne "$expected" ]]; then
    fail "Version gap: expected $expected but found $actual"
    ((FAILURES++))
    GAP_FOUND=true
    break
  fi
done
if ! $GAP_FOUND; then
  pass "Versions are sequential (1..${#SORTED_VERSIONS[@]})"
fi
echo ""

# --------------------------------------------------------------------------
# Check 5: (Git-aware) Detect modifications to already-applied migrations
# --------------------------------------------------------------------------
if $CHECK_GIT; then
  info "Check 5: Stale migration detection (diff vs origin/main)..."
  BASE_BRANCH="${BASE_BRANCH:-main}"
  # Resolve the base branch ref
  BASE_REF="origin/$BASE_BRANCH"
  if ! git -C "$PROJECT_DIR" rev-parse --verify "$BASE_REF" &>/dev/null; then
    warn "Base ref '$BASE_REF' not found — skipping stale migration check"
  else
    STALE=0
    for i in "${!VERSIONS[@]}"; do
      filename="${FILES[$i]}"
      relative="backend/migrations/$filename"
      # Check if this file exists on the base branch at all
      if git -C "$PROJECT_DIR" cat-file -e "$BASE_REF:$relative" 2>/dev/null; then
        # File exists on base branch — check for diff
        if ! git -C "$PROJECT_DIR" diff --quiet "$BASE_REF" -- "$relative" 2>/dev/null; then
          warn "Migration '$filename' differs from $BASE_REF (was it modified after being applied?)"
          ((STALE++))
        fi
      fi
    done
    if [[ "$STALE" -eq 0 ]]; then
      pass "No stale migrations detected vs $BASE_REF"
    else
      info "Found $STALE migration(s) modified vs $BASE_REF — verify they are intentional"
    fi
  fi
  echo ""
fi

# ============================================================================
# Summary
# ============================================================================
echo "═══════════════════════════════════════════════════════════════════"
if [[ "$FAILURES" -eq 0 ]]; then
  echo -e "  ${GREEN}All migration validation checks passed!${NC}"
else
  echo -e "  ${RED}$FAILURES migration validation failure(s)${NC}"
fi
echo "═══════════════════════════════════════════════════════════════════"

exit "$FAILURES"
