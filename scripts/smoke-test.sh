#!/usr/bin/env bash
# =============================================================================
# Smoke Test — VEDO hub RAG Assistant
# =============================================================================
# Runs a full startup smoke test: compose up + port checks + health endpoints.
#
# Usage:
#   ./scripts/smoke-test.sh            # full smoke test (no teardown)
#   ./scripts/smoke-test.sh --cleanup  # teardown only
#   ./scripts/smoke-test.sh --full     # full test + teardown
#
# Exit codes:
#   0 — all checks passed
#   1 — one or more checks failed
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPOSE_FILE="$PROJECT_DIR/docker-compose.yml"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass()  { echo -e "  ${GREEN}✓${NC} $1"; }
fail()  { echo -e "  ${RED}✗${NC} $1"; }
info()  { echo -e "  ${YELLOW}→${NC} $1"; }

FAILURES=0

check_docker_installed() {
    if ! command -v docker &>/dev/null; then
        fail "Docker is not installed"
        return 1
    fi
    if ! command -v docker compose &>/dev/null; then
        fail "docker compose plugin is not installed"
        return 1
    fi
    pass "Docker and docker compose are available"
    return 0
}

check_compose_file() {
    if [[ ! -f "$COMPOSE_FILE" ]]; then
        fail "docker-compose.yml not found at $COMPOSE_FILE"
        return 1
    fi
    pass "docker-compose.yml found"
    return 0
}

start_services() {
    info "Starting all services via docker compose..."
    docker compose -f "$COMPOSE_FILE" up --build -d 2>&1 || {
        fail "docker compose up failed"
        return 1
    }
    return 0
}

wait_for_service() {
    local name="$1"
    local service="$2"
    local max_attempts=45
    local attempt=0

    info "Waiting for $name to be healthy (up to ${max_attempts}s)..."
    while [[ $attempt -lt $max_attempts ]]; do
        # Check container health via Docker Compose healthcheck status
        local status
        status=$(docker compose -f "$COMPOSE_FILE" ps --format json "$service" 2>/dev/null | python -c "import sys,json; print(json.load(sys.stdin).get('Health',''))" 2>/dev/null || echo "")
        if [[ "$status" == "healthy" ]]; then
            pass "$name is healthy"
            return 0
        fi
        # Fallback: check if container is running and log no crash
        if docker compose -f "$COMPOSE_FILE" ps "$service" 2>/dev/null | grep -q "Up"; then
            sleep 2
            ((attempt+=2)) || true
            continue
        fi
        sleep 2
        ((attempt+=2)) || true
    done

    # Final attempt with verbose output
    info "Last attempt — checking $name logs..."
    docker compose -f "$COMPOSE_FILE" logs --tail=20 "$service" 2>/dev/null || true

    fail "$name did not become healthy within ${max_attempts}s"
    return 1
}

check_health_endpoints() {
    local all_ok=true

    # Backend health endpoint (public)
    info "Checking backend /health endpoint..."
    if curl -sf "http://localhost:3000/health" >/dev/null 2>&1; then
        pass "Backend /health — 200 OK"
    else
        fail "Backend /health failed"
        docker compose -f "$COMPOSE_FILE" logs --tail=10 backend 2>/dev/null || true
        all_ok=false
    fi

    # Embedding service health — check via exec since port isn't exposed
    info "Checking embedding /health endpoint via exec..."
    if docker compose -f "$COMPOSE_FILE" exec -T embedding python -c "import urllib.request; print(urllib.request.urlopen('http://localhost:8001/health').status)" 2>/dev/null | grep -q "200"; then
        pass "Embedding /health — 200 OK"
    else
        fail "Embedding /health failed"
        docker compose -f "$COMPOSE_FILE" logs --tail=10 embedding 2>/dev/null || true
        all_ok=false
    fi

    # Frontend serves on 80
    info "Checking frontend (nginx) responds..."
    if curl -sf "http://localhost:80/" >/dev/null 2>&1; then
        pass "Frontend (nginx) — 200 OK"
    else
        fail "Frontend (nginx) failed"
        docker compose -f "$COMPOSE_FILE" logs --tail=10 frontend 2>/dev/null || true
        all_ok=false
    fi

    $all_ok
}

check_no_crash_loops() {
    local all_ok=true

    for service in chroma embedding backend frontend; do
        local restart_count
        restart_count=$(docker compose -f "$COMPOSE_FILE" ps --format json "$service" 2>/dev/null | python -c "import sys,json; d=json.load(sys.stdin); print(d.get('RestartCount',0))" 2>/dev/null || echo "0")
        if [[ "$restart_count" -gt 2 ]]; then
            fail "$service has restarted $restart_count times (crash loop?)"
            docker compose -f "$COMPOSE_FILE" logs --tail=20 "$service" 2>/dev/null || true
            all_ok=false
        else
            pass "$service restart count: $restart_count"
        fi
    done

    $all_ok
}

check_service_logs_no_errors() {
    local all_ok=true

    # Backend should have no ERROR-level logs after startup
    local errors
    errors=$(docker compose -f "$COMPOSE_FILE" logs --tail=50 backend 2>/dev/null | grep -c '"level":"ERROR"' || true)
    if [[ "$errors" -gt 0 ]]; then
        fail "Backend logs contain $errors ERROR entries"
        docker compose -f "$COMPOSE_FILE" logs --tail=30 backend 2>/dev/null | grep '"level":"ERROR"' || true
        all_ok=false
    else
        pass "Backend logs have no ERROR entries"
    fi

    $all_ok
}

cleanup() {
    info "Stopping and removing containers..."
    docker compose -f "$COMPOSE_FILE" down -v 2>/dev/null || true
    pass "Cleanup complete"
}

# =============================================================================
# Main
# =============================================================================

DO_CLEANUP=false
DO_FULL=false

for arg in "$@"; do
    case "$arg" in
        --cleanup) DO_CLEANUP=true ;;
        --full)    DO_FULL=true ;;
        *)         echo "Unknown option: $arg"; exit 1 ;;
    esac
done

echo "═══════════════════════════════════════════════════════════════"
echo "  VEDO hub — Smoke Test"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# If --cleanup only, just tear down and exit
if $DO_CLEANUP && ! $DO_FULL; then
    cleanup
    exit 0
fi

# Phase 1: Prerequisites
info "Phase 1: Prerequisites"
check_docker_installed || ((FAILURES++))
check_compose_file || ((FAILURES++))
echo ""

# Phase 2: Start services
info "Phase 2: Starting services"
start_services || { cleanup; exit 1; }
echo ""

# Phase 3: Wait for health
info "Phase 3: Service health checks"
wait_for_service "Chroma"     "chroma"    || ((FAILURES++))
wait_for_service "Embedding"  "embedding" || ((FAILURES++))
wait_for_service "Backend"    "backend"   || ((FAILURES++))
echo ""

# Phase 4: Endpoint verification
info "Phase 4: Endpoint verification"
check_health_endpoints || ((FAILURES++))
echo ""

# Phase 5: Stability checks
info "Phase 5: Stability checks"
check_no_crash_loops || ((FAILURES++))
check_service_logs_no_errors || ((FAILURES++))
echo ""

# Summary
echo "═══════════════════════════════════════════════════════════════"
if [[ "$FAILURES" -eq 0 ]]; then
    echo -e "  ${GREEN}All smoke tests passed!${NC}"
    echo "═══════════════════════════════════════════════════════════════"
else
    echo -e "  ${RED}$FAILURES smoke test failure(s)${NC}"
    echo "═══════════════════════════════════════════════════════════════"
fi
echo ""

# Cleanup if --full
if $DO_FULL; then
    cleanup
fi

exit "$FAILURES"
