#!/usr/bin/env bash
# =============================================================================
# Smoke Test — VEDO hub RAG Assistant
# =============================================================================
# Runs a full startup smoke test: compose up + port checks + health endpoints.
#
# Usage:
#   ./scripts/smoke-test.sh                       # development smoke test (no teardown)
#   ./scripts/smoke-test.sh --cleanup             # teardown only
#   ./scripts/smoke-test.sh --full                # development test + teardown
#   ./scripts/smoke-test.sh --production          # production smoke test (uses -f docker-compose.production.yml)
#   ./scripts/smoke-test.sh --production --full   # production test + teardown
#   ./scripts/smoke-test.sh --quick               # quick smoke (health endpoints only)
#
# Exit codes:
#   0 — all checks passed
#   1 — one or more checks failed
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPOSE_FILE="$PROJECT_DIR/docker-compose.yml"
COMPOSE_PROD_FILE="$PROJECT_DIR/docker-compose.production.yml"

# Determine compose files to use
COMPOSE_FILES=(-f "$COMPOSE_FILE")
MODE="development"
IS_PRODUCTION=false

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

    if $IS_PRODUCTION; then
        if [[ ! -f "$COMPOSE_PROD_FILE" ]]; then
            fail "docker-compose.production.yml not found at $COMPOSE_PROD_FILE"
            return 1
        fi
        pass "docker-compose.production.yml found"
    fi

    return 0
}

start_services() {
    info "Starting all services via docker compose..."
    docker compose "${COMPOSE_FILES[@]}" up --build -d 2>&1 || {
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
        status=$(docker compose "${COMPOSE_FILES[@]}" ps --format json "$service" 2>/dev/null | python -c "import sys,json; print(json.load(sys.stdin).get('Health',''))" 2>/dev/null || echo "")
        if [[ "$status" == "healthy" ]]; then
            pass "$name is healthy"
            return 0
        fi
        # Fallback: check if container is running and log no crash
        if docker compose "${COMPOSE_FILES[@]}" ps "$service" 2>/dev/null | grep -q "Up"; then
            sleep 2
            ((attempt+=2)) || true
            continue
        fi
        sleep 2
        ((attempt+=2)) || true
    done

    # Final attempt with verbose output
    info "Last attempt — checking $name logs..."
    docker compose "${COMPOSE_FILES[@]}" logs --tail=20 "$service" 2>/dev/null || true

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
        docker compose "${COMPOSE_FILES[@]}" logs --tail=10 backend 2>/dev/null || true
        all_ok=false
    fi

    # Frontend responds — try port 80 (production/CI), fall back to 5173 (dev override)
    local frontend_port=80
    info "Checking frontend (nginx) responds..."
    if curl -sf "http://localhost:${frontend_port}/" >/dev/null 2>&1; then
        pass "Frontend (nginx) — 200 OK on port ${frontend_port}"
    elif curl -sf "http://localhost:5173/" >/dev/null 2>&1; then
        frontend_port=5173
        pass "Frontend (Vite dev) — 200 OK on port ${frontend_port}"
    else
        fail "Frontend failed (tried ports 80 and 5173)"
        docker compose "${COMPOSE_FILES[@]}" logs --tail=10 frontend 2>/dev/null || true
        all_ok=false
    fi

    # KeyCloak health (token endpoint)
    info "Checking KeyCloak health..."
    if curl -sf "http://localhost:8080/realms/vedo/.well-known/openid-configuration" >/dev/null 2>&1; then
        pass "KeyCloak — realm reachable"
    else
        fail "KeyCloak health check failed"
        docker compose "${COMPOSE_FILES[@]}" logs --tail=10 keycloak 2>/dev/null || true
        all_ok=false
    fi

    # PostgreSQL health
    info "Checking PostgreSQL health..."
    if docker compose "${COMPOSE_FILES[@]}" exec -T db pg_isready -U postgres >/dev/null 2>&1; then
        pass "PostgreSQL — pg_isready"
    else
        fail "PostgreSQL health check failed"
        all_ok=false
    fi

    # OTel collector health (gRPC port probe via HTTP extension)
    info "Checking OTel collector health..."
    if docker compose "${COMPOSE_FILES[@]}" exec -T otel-collector wget -q -O /dev/null http://localhost:13133 2>/dev/null || \
       docker compose "${COMPOSE_FILES[@]}" ps --format json otel-collector 2>/dev/null | python -c "import sys,json; d=json.load(sys.stdin); exit(0 if d.get('Health')=='healthy' else 1)" 2>/dev/null; then
        pass "OTel collector — healthy"
    else
        # Fallback: check container is up
        if docker compose "${COMPOSE_FILES[@]}" ps otel-collector 2>/dev/null | grep -q "Up"; then
            pass "OTel collector — running (no health endpoint)"
        else
            fail "OTel collector not running"
            all_ok=false
        fi
    fi

    # Caddy routing verification (production mode only)
    if $IS_PRODUCTION; then
        info "Checking Caddy routing..."
        if curl -sf "http://localhost:80/api/health" >/dev/null 2>&1; then
            pass "Caddy — routes /api/* to backend"
        else
            fail "Caddy routing check failed"
            docker compose "${COMPOSE_FILES[@]}" logs --tail=10 caddy 2>/dev/null || true
            all_ok=false
        fi

        info "Checking Caddy serves frontend..."
        if curl -sf "http://localhost:80/" >/dev/null 2>&1; then
            pass "Caddy — serves frontend"
        else
            fail "Caddy frontend routing failed"
            all_ok=false
        fi
    else
        info "Skipping Caddy routing checks (development mode — no Caddy)"
    fi

    if ! $all_ok; then
        return 1
    fi
    return 0
}

check_no_crash_loops() {
    local all_ok=true
    local services=(chroma backend frontend)

    # In production mode, also check Caddy and keycloak
    if $IS_PRODUCTION; then
        services+=(caddy keycloak)
    fi

    for service in "${services[@]}"; do
        local restart_count
        restart_count=$(docker compose "${COMPOSE_FILES[@]}" ps --format json "$service" 2>/dev/null | python -c "import sys,json; d=json.load(sys.stdin); print(d.get('RestartCount',0))" 2>/dev/null || echo "0")
        if [[ "$restart_count" -gt 2 ]]; then
            fail "$service has restarted $restart_count times (crash loop?)"
            docker compose "${COMPOSE_FILES[@]}" logs --tail=20 "$service" 2>/dev/null || true
            all_ok=false
        else
            pass "$service restart count: $restart_count"
        fi
    done

    if ! $all_ok; then
        return 1
    fi
    return 0
}

check_service_logs_no_errors() {
    local all_ok=true

    # Backend should have no ERROR-level logs after startup
    local errors
    errors=$(docker compose "${COMPOSE_FILES[@]}" logs --tail=50 backend 2>/dev/null | grep -c '"level":"ERROR"' || true)
    if [[ "$errors" -gt 0 ]]; then
        fail "Backend logs contain $errors ERROR entries"
        docker compose "${COMPOSE_FILES[@]}" logs --tail=30 backend 2>/dev/null | grep '"level":"ERROR"' || true
        all_ok=false
    else
        pass "Backend logs have no ERROR entries"
    fi

    if ! $all_ok; then
        return 1
    fi
    return 0
}

cleanup() {
    info "Stopping and removing containers..."
    docker compose "${COMPOSE_FILES[@]}" down -v 2>/dev/null || true
    pass "Cleanup complete"
}

# =============================================================================
# Main
# =============================================================================

DO_CLEANUP=false
DO_FULL=false
DO_QUICK=false

for arg in "$@"; do
    case "$arg" in
        --cleanup)    DO_CLEANUP=true ;;
        --full)       DO_FULL=true ;;
        --quick)      DO_QUICK=true ;;
        --production|-p)
            IS_PRODUCTION=true
            MODE="production"
            COMPOSE_FILES=(-f "$COMPOSE_FILE" -f "$COMPOSE_PROD_FILE")
            ;;
        *)         echo "Unknown option: $arg"; exit 1 ;;
    esac
done

echo "═══════════════════════════════════════════════════════════════"
echo "  VEDO hub — Smoke Test ($MODE)"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# If --cleanup only, just tear down and exit
if $DO_CLEANUP && ! $DO_FULL; then
    cleanup
    exit 0
fi

# Phase 0: Quick check (health endpoints only, no service management)
if $DO_QUICK; then
    info "Phase 0: Quick health check"
    check_health_endpoints || ((FAILURES++))
    echo ""
    # Still check for crash loops in quick mode
    check_no_crash_loops || ((FAILURES++))
    echo ""
    # Summary for quick mode
    echo "═══════════════════════════════════════════════════════════════"
    if [[ "$FAILURES" -eq 0 ]]; then
        echo -e "  ${GREEN}All quick smoke checks passed!${NC}"
    else
        echo -e "  ${RED}$FAILURES quick smoke check failure(s)${NC}"
    fi
    echo "═══════════════════════════════════════════════════════════════"
    exit "$FAILURES"
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
wait_for_service "Backend"    "backend"   || ((FAILURES++))
wait_for_service "KeyCloak"   "keycloak"  || ((FAILURES++))
wait_for_service "PostgreSQL" "db"        || ((FAILURES++))
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
