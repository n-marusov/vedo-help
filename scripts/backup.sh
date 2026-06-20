#!/usr/bin/env bash
set -e

# VEDO hub backup script
# Backs up PostgreSQL databases (vedo + keycloak) and Chroma data directory
#
# Usage:
#   ./scripts/backup.sh
#
# Environment variables:
#   BACKUP_DIR     Directory for backup files (default: ./backups)

BACKUP_DIR="${BACKUP_DIR:-./backups}"
TIMESTAMP=$(date +%F_%H-%M-%S)

mkdir -p "$BACKUP_DIR"

echo "[INFO] Starting VEDO hub backup: $TIMESTAMP"

# Backup vedo database (PostgreSQL pg_dump)
VEDO_BACKUP="$BACKUP_DIR/vedo-$TIMESTAMP.sql.gz"
echo "[INFO] Backing up vedo database..."
docker compose exec -T db pg_dump -U postgres vedo | gzip > "$VEDO_BACKUP"
echo "[INFO] vedo database backup created: $VEDO_BACKUP ($(du -h "$VEDO_BACKUP" | cut -f1))"

# Backup keycloak database (PostgreSQL pg_dump)
KC_BACKUP="$BACKUP_DIR/keycloak-$TIMESTAMP.sql.gz"
echo "[INFO] Backing up keycloak database..."
docker compose exec -T db pg_dump -U postgres keycloak | gzip > "$KC_BACKUP"
echo "[INFO] keycloak database backup created: $KC_BACKUP ($(du -h "$KC_BACKUP" | cut -f1))"

# Backup Chroma data directory
if [ -d ./data/chroma ]; then
    CHROMA_BACKUP="$BACKUP_DIR/chroma-$TIMESTAMP.tar.gz"
    tar -czf "$CHROMA_BACKUP" -C ./data chroma
    echo "[INFO] Chroma backup created: $CHROMA_BACKUP ($(du -h "$CHROMA_BACKUP" | cut -f1))"
else
    echo "[WARN] Chroma data directory not found at ./data/chroma"
fi

# Prune backups older than 30 days
echo "[INFO] Pruning backups older than 30 days..."
find "$BACKUP_DIR" \( -name "vedo-*.sql.gz" -o -name "keycloak-*.sql.gz" -o -name "chroma-*.tar.gz" \) -mtime +30 -delete 2>/dev/null || true

echo "[INFO] Backup complete: $TIMESTAMP"
