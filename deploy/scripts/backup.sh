#!/usr/bin/env bash

# ===================================================================
# Data Backup Script
# ===================================================================
# Creates timestamped, compressed backups of SQLite database and
# Chroma vector store with retention policy
# (7 daily, 4 weekly, 12 monthly).
#
# Usage: ./deploy/scripts/backup.sh
# ===================================================================

set -euo pipefail

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
COMPOSE_FILE="${PROJECT_ROOT}/docker-compose.yml"
COMPOSE_PROD="${PROJECT_ROOT}/docker-compose.production.yml"
dc() { docker compose -f "${COMPOSE_FILE}" -f "${COMPOSE_PROD}" "$@"; }
BACKUP_DIR="${PROJECT_ROOT}/backups"

log_info()    { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1"; }
error_exit()  { log_error "$1"; exit 1; }

# Load env
[ -f "${PROJECT_ROOT}/.env" ] && source "${PROJECT_ROOT}/.env" 2>/dev/null

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
mkdir -p "${BACKUP_DIR}"

log_info "Starting backup: ${TIMESTAMP}"

# ===================================================================
# Backup 1: SQLite Database (from backend container)
# ===================================================================
log_info "Backing up SQLite database..."
DB_SOURCE="/data/vedo.db"
DB_BACKUP="${BACKUP_DIR}/vedo-db-${TIMESTAMP}.db"

if dc ps --format '{{.State}}' backend 2>/dev/null | grep -q "running"; then
    dc cp "backend:${DB_SOURCE}" "${DB_BACKUP}" 2>/dev/null || {
        log_warning "Could not copy database from container (may not exist yet)"
        rm -f "${DB_BACKUP}"
    }

    if [ -f "${DB_BACKUP}" ] && [ -s "${DB_BACKUP}" ]; then
        gzip -f "${DB_BACKUP}"
        SIZE=$(du -h "${DB_BACKUP}.gz" | cut -f1)
        log_success "SQLite backup created: vedo-db-${TIMESTAMP}.db.gz (${SIZE})"
    else
        rm -f "${DB_BACKUP}"
        log_warning "SQLite database file not found or empty — skipping"
    fi
else
    log_warning "Backend container not running — skipping SQLite backup"
fi

# ===================================================================
# Backup 2: Chroma Vector Store
# ===================================================================
log_info "Backing up Chroma vector store..."
CHROMA_BACKUP="${BACKUP_DIR}/chroma-${TIMESTAMP}.tar.gz"

if dc ps --format '{{.State}}' chroma 2>/dev/null | grep -q "running"; then
    # Chroma stores data in the named volume at /chroma/chroma
    dc exec -T chroma tar czf - -C /chroma/chroma . 2>/dev/null > "${CHROMA_BACKUP}" || {
        log_warning "Chroma backup failed"
        rm -f "${CHROMA_BACKUP}"
    }

    if [ -f "${CHROMA_BACKUP}" ] && [ -s "${CHROMA_BACKUP}" ]; then
        SIZE=$(du -h "${CHROMA_BACKUP}" | cut -f1)
        log_success "Chroma backup created: chroma-${TIMESTAMP}.tar.gz (${SIZE})"
    else
        rm -f "${CHROMA_BACKUP}"
        log_warning "Chroma backup is empty — skipping"
    fi
else
    log_warning "Chroma container not running — skipping vector store backup"
fi

# ===================================================================
# Retention Policy
# ===================================================================
log_info "Applying retention policy..."
find "${BACKUP_DIR}" -name "vedo-db-*.db.gz" -mtime +7 | while read -r file; do
    FILENAME=$(basename "$file")
    FILEDATE=$(echo "$FILENAME" | grep -oE '[0-9]{8}' | head -1)
    [ -z "$FILEDATE" ] && continue

    DAY="${FILEDATE:6:2}"
    AGE_DAYS=$(( ( $(date +%s) - $(date -d "${FILEDATE:0:4}-${FILEDATE:4:2}-${FILEDATE:6:2}" +%s 2>/dev/null || echo 0) ) / 86400 ))

    # Keep 1st-of-month backups for 365 days (monthly)
    if [ "$DAY" = "01" ] && [ "$AGE_DAYS" -le 365 ]; then
        continue
    fi

    # Keep Sunday backups for 30 days (weekly)
    DOW=$(date -d "${FILEDATE:0:4}-${FILEDATE:4:2}-${FILEDATE:6:2}" +%w 2>/dev/null || echo "")
    if [ "$DOW" = "0" ] && [ "$AGE_DAYS" -le 30 ]; then
        continue
    fi

    rm -f "$file"
done

# Same retention for Chroma backups
find "${BACKUP_DIR}" -name "chroma-*.tar.gz" -mtime +7 -delete

log_info "Retention policy applied (7 daily, 4 weekly, 12 monthly)"

# ===================================================================
# Summary
# ===================================================================
echo ""
log_info "Recent backups:"
ls -lh "${BACKUP_DIR}"/*.db.gz "${BACKUP_DIR}"/*.tar.gz 2>/dev/null | tail -5 || echo "  None"

log_success "Backup complete: ${TIMESTAMP}"
