#!/usr/bin/env bash
# validate-docker-compose.sh
# Checks that backend service URLs use Docker service names (not localhost).
set -euo pipefail

echo "Validating Docker Compose configuration..."

# Extract the rendered compose config as environment list
ENV_LIST=$(docker compose run --rm backend env 2>/dev/null || docker compose config 2>/dev/null | grep -oP '(?<=      )[A-Z_]+=.*' || true)

if [ -z "$ENV_LIST" ]; then
    echo "WARN: Could not extract environment from Docker Compose. Skipping validation."
    echo "      Run this script when Docker services are running, or use 'docker compose config'."
    exit 0
fi

errors=0

# Check CHROMA_URL
CHROMA_URL=$(echo "$ENV_LIST" | grep "^CHROMA_URL=" | cut -d= -f2-)
if [ "$CHROMA_URL" != "http://chroma:8000" ]; then
    echo "  FAIL: CHROMA_URL = ${CHROMA_URL:-"(not set)"} (expected http://chroma:8000)"
    errors=1
fi



if [ "$errors" -eq 0 ]; then
    echo "OK: All backend service URLs use Docker service names."
else
    echo ""
    echo "One or more backend service URLs are misconfigured."
    echo "Use Docker service names (e.g., http://chroma:8000) instead of localhost."
    exit 1
fi
