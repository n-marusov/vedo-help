#!/usr/bin/env bash
set -euo pipefail

# VEDO hub restore script
# Restores PostgreSQL databases (vedo + keycloak) and optionally Chroma data
#
# Usage:
#   ./scripts/restore.sh <vedo_dump> <keycloak_dump> [chroma_archive]           # Development
#   ./scripts/restore.sh --prod <vedo_dump> <keycloak_dump> [chroma_archive]    # Production
#   ./scripts/restore.sh -f docker-compose.yml -f custom.yml <vedo_dump> <keycloak_dump> [chroma_archive]
#
# Options:
#   --prod, -p    Use production compose files (must come before positional args)
#   -f <file>     Additional compose file (repeatable, must come before positional args)
#
# Arguments:
#   vedo_dump       Path to vedo database SQL dump (.sql)
#   keycloak_dump   Path to keycloak database SQL dump (.sql)
#   chroma_archive  Path to Chroma backup archive (.tar.gz, optional)
#
# Examples:
#   ./scripts/restore.sh backups/vedo-2024-01-01.sql backups/keycloak-2024-01-01.sql
#   ./scripts/restore.sh backups/vedo-2024-01-01.sql backups/keycloak-2024-01-01.sql backups/chroma-2024-01-01.tar.gz
#   ./scripts/restore.sh --prod backups/vedo-2024-01-01.sql backups/keycloak-2024-01-01.sql

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_DIR"

COMPOSE_FILES=()

# --- Parse arguments ----------------------------------------------------------

# Consume leading flags before positional args
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
        -*)
            echo "[ERROR] Unknown option: $1" >&2
            echo "Usage: $0 [--prod|-p] [-f <compose_file>...] <vedo_dump> <keycloak_dump> [chroma_archive]" >&2
            exit 1
            ;;
        *)
            # First positional arg reached
            break
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

# --- Validate positional arguments --------------------------------------------

if [ $# -lt 2 ]; then
    echo "Usage: $0 [options] <vedo_dump> <keycloak_dump> [chroma_archive]" >&2
    echo ""
    echo "Arguments:"
    echo "  vedo_dump       Path to vedo database SQL dump (.sql)"
    echo "  keycloak_dump   Path to keycloak database SQL dump (.sql)"
    echo "  chroma_archive  Path to Chroma backup archive (.tar.gz, optional)"
    exit 1
fi

VEDO_DUMP="$1"
KEYCLOAK_DUMP="$2"
CHROMA_ARCHIVE="${3:-}"
shift 3 2>/dev/null || true

# Validate all input files exist before making any changes
ERRORS=0
if [ ! -f "$VEDO_DUMP" ]; then
    echo "[ERROR] vedo dump file not found: $VEDO_DUMP" >&2
    ERRORS=1
fi
if [ ! -f "$KEYCLOAK_DUMP" ]; then
    echo "[ERROR] keycloak dump file not found: $KEYCLOAK_DUMP" >&2
    ERRORS=1
fi
if [ -n "$CHROMA_ARCHIVE" ] && [ ! -f "$CHROMA_ARCHIVE" ]; then
    echo "[ERROR] Chroma archive not found: $CHROMA_ARCHIVE" >&2
    ERRORS=1
fi
if [ $ERRORS -ne 0 ]; then
    exit 1
fi

# --- Rollback trap ------------------------------------------------------------
# Always restart services, even if the restore fails.

_restart_services() {
    echo "[INFO] Restarting services..."
    "${COMPOSE_CMD[@]}" start chroma backend 2>/dev/null || \
        "${COMPOSE_CMD[@]}" up -d chroma backend 2>/dev/null || true
    sleep 2
}

cleanup() {
    local exit_code=$?
    _restart_services
    if [ $exit_code -ne 0 ]; then
        echo "[WARN] Restore completed with errors — verify system health." >&2
    else
        echo "[INFO] Restore complete. Verify system health at /health endpoint."
    fi
    exit $exit_code
}
trap cleanup EXIT

# Stop `set -e` for the main restore logic so we capture every step's outcome.
# We restart services in the EXIT trap regardless of failure.
echo "[INFO] Starting VEDO hub restore"
echo "[INFO]   Compose profile: $COMPOSE_DESC"
echo "[INFO]   vedo dump:       $VEDO_DUMP"
echo "[INFO]   keycloak dump:   $KEYCLOAK_DUMP"
[ -n "$CHROMA_ARCHIVE" ] && echo "[INFO]   Chroma archive:  $CHROMA_ARCHIVE"

# --- Stop backend and chroma --------------------------------------------------
echo "[INFO] Stopping backend and chroma..."
"${COMPOSE_CMD[@]}" stop backend chroma 2>/dev/null || \
    echo "[WARN] Failed to stop containers — continuing" >&2

# --- Restore vedo database ----------------------------------------------------
echo "[INFO] Restoring vedo database..."

# Drop and recreate to get a clean state
"${COMPOSE_CMD[@]}" exec -T db psql -U postgres -c "DROP DATABASE IF EXISTS vedo;" 2>/dev/null
"${COMPOSE_CMD[@]}" exec -T db psql -U postgres -c "CREATE DATABASE vedo OWNER vedo;" 2>/dev/null

# Import the dump
"${COMPOSE_CMD[@]}" exec -T db psql -U vedo -d vedo < "$VEDO_DUMP" 2>/dev/null
echo "[INFO] vedo database restored from $VEDO_DUMP"

# --- Restore keycloak database ------------------------------------------------
echo "[INFO] Restoring keycloak database..."

# Drop and recreate to get a clean state
"${COMPOSE_CMD[@]}" exec -T db psql -U postgres -c "DROP DATABASE IF EXISTS keycloak;" 2>/dev/null
"${COMPOSE_CMD[@]}" exec -T db psql -U postgres -c "CREATE DATABASE keycloak OWNER keycloak;" 2>/dev/null

# Grant schema permissions (init-db.sh does this; replication ensures it works)
"${COMPOSE_CMD[@]}" exec -T db psql -U postgres -d keycloak -c "GRANT ALL ON SCHEMA public TO keycloak;" 2>/dev/null || true

# Import the dump
"${COMPOSE_CMD[@]}" exec -T db psql -U keycloak -d keycloak < "$KEYCLOAK_DUMP" 2>/dev/null
echo "[INFO] keycloak database restored from $KEYCLOAK_DUMP"

# --- Restore Chroma data ------------------------------------------------------
if [ -n "$CHROMA_ARCHIVE" ]; then
    echo "[INFO] Restoring Chroma data..."
    if "${COMPOSE_CMD[@]}" run --rm --no-deps -v chroma_data:/dest \
        alpine sh -c 'rm -rf /dest/* && tar xzf - -C /dest' < "$CHROMA_ARCHIVE" 2>/dev/null; then
        echo "[INFO] Chroma data restored from $CHROMA_ARCHIVE"
    else
        echo "[WARN] Chroma restore failed — chroma_data volume may not exist" >&2
    fi
fi

echo "[INFO] All restore operations completed (services will restart on exit)."
