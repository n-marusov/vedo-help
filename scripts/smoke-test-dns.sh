#!/usr/bin/env bash
# =============================================================================
# DNS Smoke Test — VEDO hub RAG Assistant
# =============================================================================
# Checks that the embedding service can resolve external domains
# (huggingface.co) independently of host-system DNS (VPN interference).
#
# Uses explicit public DNS (8.8.8.8, 1.1.1.1) via docker run --dns to
# simulate the configuration from docker-compose.yml.
#
# Usage:
#   ./scripts/smoke-test-dns.sh
#
# Exit codes:
#   0  — DNS resolution works correctly
#   1  — DNS resolution failed
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPOSE_FILE="$PROJECT_DIR/docker-compose.yml"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass()  { echo -e "  ${GREEN}✓${NC} $1"; }
fail()  { echo -e "  ${RED}✗${NC} $1"; }
info()  { echo -e "  ${YELLOW}→${NC} $1"; }

FAILURES=0
IMAGE_NAME="vedo-embedding:latest"

ensure_image() {
    if ! docker image inspect "$IMAGE_NAME" &>/dev/null; then
        info "Image $IMAGE_NAME not found, building..."
        docker compose -f "$COMPOSE_FILE" build embedding 2>&1 || {
            fail "Build failed"
            return 1
        }
        pass "Image built"
    else
        pass "Image $IMAGE_NAME exists"
    fi
    return 0
}

check_dns_resolution() {
    local domain="$1"
    local dns_server="$2"

    info "Checking DNS resolution of $domain via $dns_server..."
    if docker run --rm --dns "$dns_server" "$IMAGE_NAME" python -c "
import socket
try:
    ip = socket.getaddrinfo('$domain', 443)[0][4][0]
    print(f'OK: $domain -> {ip}')
except Exception as e:
    print(f'FAIL: {e}')
    exit(1)
" 2>&1; then
        pass "DNS resolution of $domain via $dns_server works"
        return 0
    else
        fail "DNS resolution of $domain via $dns_server failed"
        return 1
    fi
}

check_external_connectivity() {
    local domain="$1"

    info "Checking HTTPS connectivity to $domain..."
    if docker run --rm --dns 8.8.8.8 --dns 1.1.1.1 "$IMAGE_NAME" python -c "
import urllib.request
try:
    resp = urllib.request.urlopen('https://$domain', timeout=15)
    print(f'OK: $domain reachable (HTTP {resp.status})')
except Exception as e:
    print(f'FAIL: {e}')
    exit(1)
" 2>&1; then
        pass "HTTPS connectivity to $domain works"
        return 0
    else
        fail "HTTPS connectivity to $domain failed"
        return 1
    fi
}

check_compose_dns_config() {
    info "Checking dns: section in docker-compose.yml for embedding..."
    if grep -A5 "dns:" "$COMPOSE_FILE" | grep -q "8.8.8.8"; then
        pass "dns: 8.8.8.8 configured in embedding service"
    else
        fail "dns: 8.8.8.8 NOT found in embedding config"
        return 1
    fi
    if grep -A5 "dns:" "$COMPOSE_FILE" | grep -q "1.1.1.1"; then
        pass "dns: 1.1.1.1 configured in embedding service"
    else
        fail "dns: 1.1.1.1 NOT found in embedding config"
        return 1
    fi
    return 0
}

# =============================================================================
# Main
# =============================================================================

echo "═══════════════════════════════════════════════════════════════"
echo "  VEDO hub — DNS Smoke Test"
echo "  Verifying independence from host-system DNS (VPN)"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Phase 1: Config check
info "Phase 1: Checking docker-compose.yml configuration"
check_compose_dns_config || ((FAILURES++))
echo ""

# Phase 2: Ensure image exists
info "Phase 2: Ensuring image is available"
ensure_image || { echo -e "\n${RED}✗ Image not available — cannot continue${NC}"; exit 1; }
echo ""

# Phase 3: DNS resolution via public DNS
info "Phase 3: DNS resolution via public DNS (8.8.8.8, 1.1.1.1)"
check_dns_resolution "huggingface.co" "8.8.8.8" || ((FAILURES++))
check_dns_resolution "huggingface.co" "1.1.1.1" || ((FAILURES++))
check_dns_resolution "pypi.org" "8.8.8.8" || ((FAILURES++))
echo ""

# Phase 4: HTTPS connectivity
info "Phase 4: HTTPS connectivity to huggingface.co"
check_external_connectivity "huggingface.co" || ((FAILURES++))
echo ""

# Summary
echo "═══════════════════════════════════════════════════════════════"
if [[ "$FAILURES" -eq 0 ]]; then
    echo -e "  ${GREEN}All DNS smoke tests passed!${NC}"
    echo -e "  ${GREEN}Embedding service is independent of host-system DNS (VPN).${NC}"
    echo "═══════════════════════════════════════════════════════════════"
else
    echo -e "  ${RED}$FAILURES DNS smoke test failure(s)${NC}"
    echo "═══════════════════════════════════════════════════════════════"
fi
echo ""

exit "$FAILURES"
