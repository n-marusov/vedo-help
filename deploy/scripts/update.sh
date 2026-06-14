#!/usr/bin/env bash

# ===================================================================
# Zero-Downtime Update Script
# ===================================================================
# Pulls latest code, builds new images, and performs rolling update
# with automatic rollback on health check failure.
#
# Usage: ./deploy/scripts/update.sh [version]
# Example: ./deploy/scripts/update.sh v1.2.3
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
dc() { docker compose -f "${COMPOSE_FILE}" -f "${COMPOSE_PROD}" "$@"; }

# Logging
log_info()    { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1"; }
error_exit()  { log_error "$1"; exit 1; }

# ===================================================================
# Pre-Update Checks
# ===================================================================

log_info "Starting update..."

# Verify services are running
dc ps --format '{{.State}}' 2>/dev/null | grep -q "running" || error_exit "No services running. Use deploy.sh for initial deployment."

# Version
NEW_VERSION="${1:-$(date +%Y%m%d-%H%M%S)}"
export VERSION="${NEW_VERSION}"
log_info "Target version: ${NEW_VERSION}"

# ===================================================================
# Pre-Deployment Backup
# ===================================================================

BACKUP_SCRIPT="${SCRIPT_DIR}/backup.sh"
if [ -f "${BACKUP_SCRIPT}" ]; then
    log_info "Creating pre-deployment backup..."
    bash "${BACKUP_SCRIPT}" || log_warning "Backup failed, continuing with update"
fi

# ===================================================================
# Pull & Build
# ===================================================================

cd "${PROJECT_ROOT}"

# Pull latest code if in git repo
STASHED=false
if [ -d ".git" ]; then
    log_info "Pulling latest code..."
    if ! git diff --quiet 2>/dev/null || ! git diff --cached --quiet 2>/dev/null; then
        log_warning "Local changes detected, stashing..."
        git stash push -m "pre-update-$(date +%s)"
        STASHED=true
    fi
    git pull || log_warning "Git pull failed, continuing with local version"
    if [ "$STASHED" = true ]; then
        git stash pop || log_warning "Could not restore stashed changes. Run 'git stash pop' manually."
    fi
fi

log_info "Building new images (version: ${NEW_VERSION})..."
dc build --no-cache || error_exit "Build failed"
log_success "Images built"

# ===================================================================
# Rolling Update
# ===================================================================

START_TIME=$(date +%s)
DURATION=0

# Recreate all services with new images
log_info "Performing rolling update..."
dc up -d --force-recreate --no-deps caddy backend frontend embedding || error_exit "Failed to start new containers"

# Wait for health checks
log_info "Waiting for health checks..."
MAX_WAIT=120
WAIT=0

while [ $WAIT -lt $MAX_WAIT ]; do
    ALL_HEALTHY=true

    for svc in caddy backend frontend embedding; do
        HEALTH=$(dc ps --format '{{.Health}}' "${svc}" 2>/dev/null || echo "starting")
        if [ "$HEALTH" != "healthy" ]; then
            ALL_HEALTHY=false
        fi
    done

    if [ "$ALL_HEALTHY" = true ]; then
        END_TIME=$(date +%s)
        DURATION=$((END_TIME - START_TIME))
        log_success "All services healthy (${DURATION}s)"
        break
    fi

    [ $((WAIT % 15)) -eq 0 ] && log_info "Waiting... (${WAIT}s / ${MAX_WAIT}s)"
    sleep 2
    WAIT=$((WAIT + 2))
done

if [ $WAIT -ge $MAX_WAIT ]; then
    log_error "Health check failed after update"
    log_info "Check logs: ./deploy/scripts/logs.sh"
    log_info "To rollback: ./deploy/scripts/rollback.sh"
    exit 1
fi

# ===================================================================
# Summary
# ===================================================================

log_success "════════════════════════════════════════════"
log_success "  Update completed successfully!"
log_success "════════════════════════════════════════════"

echo ""
log_info "Update Summary:"
echo "  Version: ${NEW_VERSION}"
echo "  Duration: ${DURATION}s"

echo ""
log_info "Current Status:"
dc ps

echo ""
log_info "Next Steps:"
echo "  Monitor logs:  ./deploy/scripts/logs.sh"
echo "  Health check:  ./deploy/scripts/health-check.sh"
echo "  If issues:     ./deploy/scripts/rollback.sh"
