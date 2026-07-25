#!/usr/bin/env bash
set -euo pipefail

# VEDO hub backup script
# Backs up PostgreSQL databases (vedo + keycloak) and Chroma vector store
#
# Usage:
#   ./scripts/backup.sh                          # Development (docker-compose.yml + override)
#   ./scripts/backup.sh --prod                   # Production  (-f docker-compose.yml -f docker-compose.production.yml)
#   ./scripts/backup.sh -f docker-compose.yml -f custom.yml
#
# Options:
#   --prod, -p    Use production compose files
#   -f <file>     Additional compose file (repeatable)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_DIR"

BACKUP_DIR="${BACKUP_DIR:-./backups}"
TIMESTAMP=$(date +%F_%H-%M-%S)
COMPOSE_FILES=()

# --- Parse arguments ----------------------------------------------------------

while [[ $# -gt 0 ]]; do
    case "$1" in
        --prod|-p)
            COMPOSE_FILES=(-f docker-compose.yml -f docker-compose.production.yml)
            shift
            ;;
        -f)
            if [[ $# -lt 2 ]]; then
                echo "[ERROR] -f requires a file argument" >&2
                exit 1
            fi
            COMPOSE_FILES+=(-f "$2")
            shift 2
            ;;
        *)
            echo "[ERROR] Unknown option: $1" >&2
            echo "Usage: $0 [--prod|-p] [-f <compose_file>...]" >&2
            exit 1
            ;;
    esac
done

# Build the docker compose command prefix
if [[ ${#COMPOSE_FILES[@]} -gt 0 ]]; then
    COMPOSE_CMD=(docker compose "${COMPOSE_FILES[@]}")
    COMPOSE_DESC="${COMPOSE_FILES[*]}"
else
    COMPOSE_CMD=(docker compose)
    COMPOSE_DESC="development (default compose)"
fi

# Validate compose files exist
for f in "${COMPOSE_FILES[@]}"; do
    if [[ "$f" != -* ]] && [ ! -f "$f" ]; then
        echo "[ERROR] Compose file not found: $f" >&2
        exit 1
    fi
done

# --- Main ---------------------------------------------------------------------

mkdir -p "$BACKUP_DIR"

echo "[INFO] Starting VEDO hub backup: $TIMESTAMP"
echo "[INFO] Compose profile: $COMPOSE_DESC"

# Stop backend and chroma for consistent snapshots.
# Keep `db` running — pg_dump works against the live database.
echo "[INFO] Stopping backend and chroma..."
if ! "${COMPOSE_CMD[@]}" stop backend chroma 2>/dev/null; then
    echo "[WARN] Failed to stop containers — continuing anyway" >&2
fi

# --- Backup vedo database -----------------------------------------------------
VEDO_DUMP="$BACKUP_DIR/vedo-$TIMESTAMP.sql"
echo "[INFO] Starting vedo database dump..."
if "${COMPOSE_CMD[@]}" exec -T db pg_dump -U vedo vedo > "$VEDO_DUMP" 2>/dev/null; then
    SIZE=$(du -h "$VEDO_DUMP" | cut -f1)
    echo "[INFO] vedo database dump created: $VEDO_DUMP ($SIZE)"
else
    echo "[WARN] vedo database dump failed — check that db service is running" >&2
    rm -f "$VEDO_DUMP"
fi

# --- Backup keycloak database -------------------------------------------------
KEYCLOAK_DUMP="$BACKUP_DIR/keycloak-$TIMESTAMP.sql"
echo "[INFO] Starting keycloak database dump..."
if "${COMPOSE_CMD[@]}" exec -T db pg_dump -U keycloak keycloak > "$KEYCLOAK_DUMP" 2>/dev/null; then
    SIZE=$(du -h "$KEYCLOAK_DUMP" | cut -f1)
    echo "[INFO] keycloak database dump created: $KEYCLOAK_DUMP ($SIZE)"
else
    echo "[WARN] keycloak database dump failed — check that db service is running" >&2
    rm -f "$KEYCLOAK_DUMP"
fi

# --- Backup Chroma data -------------------------------------------------------
CHROMA_BACKUP="$BACKUP_DIR/chroma-$TIMESTAMP.tar.gz"
echo "[INFO] Starting Chroma data backup..."

# Try local bind mount first (development), then fall back to Docker volume.
CHROMA_BACKED_UP=false

if [ -d ./data/chroma ]; then
    tar -czf "$CHROMA_BACKUP" -C ./data chroma 2>/dev/null
    echo "[INFO] Chroma backup created (local): $CHROMA_BACKUP"
    CHROMA_BACKED_UP=true
fi

if [[ "$CHROMA_BACKED_UP" != "true" ]]; then
    # Fall back to Docker volume extraction
    if "${COMPOSE_CMD[@]}" run --rm --no-deps -v chroma_data:/chroma_data:ro \
        alpine tar czf - -C /chroma_data . > "$CHROMA_BACKUP" 2>/dev/null; then
        SIZE=$(du -h "$CHROMA_BACKUP" | cut -f1)
        echo "[INFO] Chroma backup created (volume): $CHROMA_BACKUP ($SIZE)"
        CHROMA_BACKED_UP=true
    fi
fi

if [[ "$CHROMA_BACKED_UP" != "true" ]]; then
    echo "[WARN] Chroma backup failed — neither local bind mount nor chroma_data volume available" >&2
    rm -f "$CHROMA_BACKUP"
fi

# --- Restart containers -------------------------------------------------------
echo "[INFO] Restarting services..."
"${COMPOSE_CMD[@]}" start chroma backend 2>/dev/null || \
    "${COMPOSE_CMD[@]}" up -d chroma backend 2>/dev/null || true

# Wait briefly for health checks
sleep 2

echo "[INFO] Services restarted."

# --- Prune old backups --------------------------------------------------------
echo "[INFO] Pruning backups older than 30 days..."
find "$BACKUP_DIR" -name "vedo-*.sql" -mtime +30 -delete 2>/dev/null || true
find "$BACKUP_DIR" -name "keycloak-*.sql" -mtime +30 -delete 2>/dev/null || true
find "$BACKUP_DIR" -name "chroma-*.tar.gz" -mtime +30 -delete 2>/dev/null || true

echo "[INFO] Backup complete: $TIMESTAMP"
echo "[INFO] All backups in: $BACKUP_DIR"
