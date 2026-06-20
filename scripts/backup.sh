#!/usr/bin/env bash
set -e

# VEDO hub backup script
# Backs up SQLite database and Chroma data directory

BACKUP_DIR="${BACKUP_DIR:-./backups}"
TIMESTAMP=$(date +%F_%H-%M-%S)

mkdir -p "$BACKUP_DIR"

echo "[INFO] Starting VEDO hub backup: $TIMESTAMP"

# Stop containers to ensure data consistency
echo "[INFO] Stopping containers..."
docker compose stop backend chroma || echo "[WARN] Failed to stop containers, continuing..."

# Backup SQLite database
if [ -f ./data/vedo.db ]; then
    DB_BACKUP="$BACKUP_DIR/vedo-$TIMESTAMP.db"
    cp ./data/vedo.db "$DB_BACKUP"
    echo "[INFO] SQLite backup created: $DB_BACKUP ($(du -h "$DB_BACKUP" | cut -f1))"
else
    echo "[WARN] SQLite database not found at ./data/vedo.db"
fi

# Backup Chroma data directory
if [ -d ./data/chroma ]; then
    CHROMA_BACKUP="$BACKUP_DIR/chroma-$TIMESTAMP.tar.gz"
    tar -czf "$CHROMA_BACKUP" -C ./data chroma
    echo "[INFO] Chroma backup created: $CHROMA_BACKUP ($(du -h "$CHROMA_BACKUP" | cut -f1))"
else
    echo "[WARN] Chroma data directory not found at ./data/chroma"
fi

# Restart containers
echo "[INFO] Restarting containers..."
docker compose start chroma backend || echo "[WARN] Failed to restart containers"
docker compose up -d chroma backend 2>/dev/null || true

# Prune backups older than 30 days
echo "[INFO] Pruning backups older than 30 days..."
find "$BACKUP_DIR" -name "vedo-*.db" -mtime +30 -delete 2>/dev/null || true
find "$BACKUP_DIR" -name "chroma-*.tar.gz" -mtime +30 -delete 2>/dev/null || true

echo "[INFO] Backup complete: $TIMESTAMP"
