#!/usr/bin/env bash

# ===================================================================
# Production Deployment Script
# ===================================================================
# Initial deployment with pre-flight checks, service startup,
# and health verification.
#
# Usage: ./deploy/scripts/deploy.sh
# ===================================================================

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
COMPOSE_FILE="${PROJECT_ROOT}/docker-compose.yml"
COMPOSE_PROD="${PROJECT_ROOT}/docker-compose.production.yml"
ENV_FILE="${PROJECT_ROOT}/.env"
CADDYFILE="${PROJECT_ROOT}/Caddyfile"

# Logging
log_info()    { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1"; }
error_exit()  { log_error "$1"; exit 1; }

# ===================================================================
# Pre-flight Checks
# ===================================================================

log_info "Starting pre-flight checks..."

# Check Docker
command -v docker &>/dev/null || error_exit "Docker is not installed"
docker compose version &>/dev/null || error_exit "Docker Compose v2 is not installed"
docker info &>/dev/null || error_exit "Docker daemon is not running"

# Check files
[ -f "${COMPOSE_FILE}" ] || error_exit "docker-compose.yml not found"
[ -f "${COMPOSE_PROD}" ] || error_exit "docker-compose.production.yml not found"
[ -f "${ENV_FILE}" ] || error_exit ".env not found. Copy .env.example to .env and configure it."

# Load and verify environment
set -a
source "${ENV_FILE}" 2>/dev/null || error_exit "Failed to load .env file"
set +a

# Verify required variables
REQUIRED_VARS=("ADMIN_API_KEY" "DATABASE_URL" "OPENROUTER_API_KEY")
for var in "${REQUIRED_VARS[@]}"; do
    [ -z "${!var:-}" ] && error_exit "Required variable $var is not set in .env"
done

# Check if ports are already in use (Caddy: 80, 443)
for port in 80 443; do
    if command -v lsof &>/dev/null; then
        lsof -iTCP:"${port}" -sTCP:LISTEN &>/dev/null && error_exit "Port ${port} is already in use"
    elif command -v ss &>/dev/null; then
        ss -tlnp | grep -q ":${port} " && error_exit "Port ${port} is already in use"
    fi
done

log_success "Pre-flight checks passed"

# ===================================================================
# Pull & Build
# ===================================================================

cd "${PROJECT_ROOT}"

VERSION="${VERSION:-$(git describe --tags --always --dirty 2>/dev/null || date +%Y%m%d-%H%M%S)}"
export VERSION

log_info "Building application images (version: ${VERSION})..."
docker compose -f "${COMPOSE_FILE}" -f "${COMPOSE_PROD}" build || error_exit "Build failed"
log_success "Images built successfully"

# ===================================================================
# Start Services
# ===================================================================

log_info "Starting all services..."
docker compose -f "${COMPOSE_FILE}" -f "${COMPOSE_PROD}" up -d || error_exit "Failed to start services"

# ===================================================================
# Health Verification
# ===================================================================

log_info "Verifying deployment health..."
MAX_WAIT=120
WAIT=0

while [ $WAIT -lt $MAX_WAIT ]; do
    ALL_HEALTHY=true

    for svc in caddy backend embedding chroma frontend; do
        HEALTH=$(docker compose -f "${COMPOSE_FILE}" -f "${COMPOSE_PROD}" ps --format '{{.Health}}' "${svc}" 2>/dev/null || echo "starting")
        if [ "$HEALTH" != "healthy" ]; then
            ALL_HEALTHY=false
        fi
    done

    if [ "$ALL_HEALTHY" = true ]; then
        log_success "All services are healthy"
        break
    fi

    [ $((WAIT % 15)) -eq 0 ] && log_info "Waiting for services... (${WAIT}s / ${MAX_WAIT}s)"
    sleep 2
    WAIT=$((WAIT + 2))
done

if [ $WAIT -ge $MAX_WAIT ]; then
    log_error "Some services failed health check"
    log_info "Recent logs:"
    docker compose -f "${COMPOSE_FILE}" -f "${COMPOSE_PROD}" logs --tail=30
    error_exit "Deployment failed"
fi

# ===================================================================
# Summary
# ===================================================================

log_success "════════════════════════════════════════════"
log_success "  Deployment completed successfully!"
log_success "════════════════════════════════════════════"

echo ""
log_info "Service Status:"
docker compose -f "${COMPOSE_FILE}" -f "${COMPOSE_PROD}" ps

echo ""
log_info "Useful Commands:"
echo "  View logs:      ./deploy/scripts/logs.sh"
echo "  Health check:   ./deploy/scripts/health-check.sh"
echo "  Update:         ./deploy/scripts/update.sh"
echo "  Rollback:       ./deploy/scripts/rollback.sh"
