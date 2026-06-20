#!/usr/bin/env bash
set -e

# VEDO hub restore script
# Restores PostgreSQL databases (vedo + keycloak) and Chroma data from backup files
#
# Usage:
#   ./scripts/restore.sh <vedo_backup> [keycloak_backup] [chroma_archive]
#
# Examples:
#   ./scripts/restore.sh backups/vedo-2024-01-01.sql.gz
#   ./scripts/restore.sh backups/vedo-2024-01-01.sql.gz backups/keycloak-2024-01-01.sql.gz
#   ./scripts/restore.sh backups/vedo-2024-01-01.sql.gz backups/keycloak-2024-01-01.sql.gz backups/chroma-2024-01-01.tar.gz

if [ $# -lt 1 ]; then
    echo "Usage: $0 <vedo_backup> [keycloak_backup] [chroma_archive]"
    echo ""
    echo "Arguments:"
    echo "  vedo_backup       Path to vedo database backup (.sql.gz)"
    echo "  keycloak_backup   Path to keycloak database backup (.sql.gz, optional)"
    echo "  chroma_archive    Path to Chroma backup archive (.tar.gz, optional)"
    exit 1
fi

VEDO_BACKUP="$1"
KC_BACKUP="${2:-}"
CHROMA_ARCHIVE="${3:-}"

# Validate inputs
if [ ! -f "$VEDO_BACKUP" ]; then
    echo "[ERROR] vedo backup file not found: $VEDO_BACKUP"
    exit 1
fi

if [ -n "$KC_BACKUP" ] && [ ! -f "$KC_BACKUP" ]; then
    echo "[ERROR] keycloak backup file not found: $KC_BACKUP"
    exit 1
fi

if [ -n "$CHROMA_ARCHIVE" ] && [ ! -f "$CHROMA_ARCHIVE" ]; then
    echo "[ERROR] Chroma archive not found: $CHROMA_ARCHIVE"
    exit 1
fi

echo "[INFO] Starting VEDO hub restore"
echo "[INFO]   vedo database:   $VEDO_BACKUP"
[ -n "$KC_BACKUP" ] && echo "[INFO]   keycloak database: $KC_BACKUP"
[ -n "$CHROMA_ARCHIVE" ] && echo "[INFO]   Chroma:          $CHROMA_ARCHIVE"

# Stop backend before restore (needed for data consistency)
echo "[INFO] Stopping backend container..."
docker compose stop backend || echo "[WARN] Failed to stop backend"

# Restore vedo database
echo "[INFO] Restoring vedo database from $VEDO_BACKUP..."
gunzip -c "$VEDO_BACKUP" | docker compose exec -T db psql -U postgres vedo
echo "[INFO] vedo database restored"

# Restore keycloak database (if provided)
if [ -n "$KC_BACKUP" ]; then
    echo "[INFO] Restoring keycloak database from $KC_BACKUP..."
    gunzip -c "$KC_BACKUP" | docker compose exec -T db psql -U postgres keycloak
    echo "[INFO] keycloak database restored"
fi

# Restore Chroma data
if [ -n "$CHROMA_ARCHIVE" ]; then
    docker compose stop chroma || echo "[WARN] Failed to stop chroma"
    rm -rf ./data/chroma
    tar -xzf "$CHROMA_ARCHIVE" -C ./data
    echo "[INFO] Chroma data restored from $CHROMA_ARCHIVE"
fi

# Restart containers
echo "[INFO] Starting containers..."
docker compose up -d || true

echo "[INFO] Restore complete. Verify system health at /health endpoint."
