#!/usr/bin/env bash
set -e

# VEDO hub restore script
# Restores SQLite database and Chroma data from backup files
#
# Usage:
#   ./scripts/restore.sh <db_file> [chroma_archive]
#
# Examples:
#   ./scripts/restore.sh backups/vedo-2024-01-01.db
#   ./scripts/restore.sh backups/vedo-2024-01-01.db backups/chroma-2024-01-01.tar.gz

if [ $# -lt 1 ]; then
    echo "Usage: $0 <db_file> [chroma_archive]"
    echo ""
    echo "Arguments:"
    echo "  db_file         Path to SQLite backup file (.db)"
    echo "  chroma_archive  Path to Chroma backup archive (.tar.gz, optional)"
    exit 1
fi

DB_FILE="$1"
CHROMA_ARCHIVE="${2:-}"

# Validate inputs
if [ ! -f "$DB_FILE" ]; then
    echo "[ERROR] Database file not found: $DB_FILE"
    exit 1
fi

if [ -n "$CHROMA_ARCHIVE" ] && [ ! -f "$CHROMA_ARCHIVE" ]; then
    echo "[ERROR] Chroma archive not found: $CHROMA_ARCHIVE"
    exit 1
fi

echo "[INFO] Starting VEDO hub restore"
echo "[INFO]   Database: $DB_FILE"
[ -n "$CHROMA_ARCHIVE" ] && echo "[INFO]   Chroma:   $CHROMA_ARCHIVE"

# Stop containers
echo "[INFO] Stopping containers..."
docker compose stop backend chroma || echo "[WARN] Failed to stop containers"

# Restore SQLite database
mkdir -p ./data
cp "$DB_FILE" ./data/vedo.db
echo "[INFO] SQLite database restored from $DB_FILE"

# Restore Chroma data
if [ -n "$CHROMA_ARCHIVE" ]; then
    rm -rf ./data/chroma
    tar -xzf "$CHROMA_ARCHIVE" -C ./data
    echo "[INFO] Chroma data restored from $CHROMA_ARCHIVE"
fi

# Restart containers
echo "[INFO] Starting containers..."
docker compose up -d chroma backend 2>/dev/null || true

echo "[INFO] Restore complete. Verify system health at /health endpoint."
