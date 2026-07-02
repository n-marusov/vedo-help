#!/bin/sh
# check-container-health.sh
# Проверяет, что все Docker контейнеры из docker-compose.yml имеют статус healthy.
# Exit 0 если все healthy, 1 если есть unhealthy или starting.

set -e

COMPOSE_FILE="${1:-docker-compose.yml}"
OVERRIDE_FILE="${2:-docker-compose.override.yml}"

if [ ! -f "$COMPOSE_FILE" ]; then
	echo "ERROR: Compose file not found: $COMPOSE_FILE"
	exit 1
fi

echo "=== Container Health Check ==="
echo "Compose files: $COMPOSE_FILE ${OVERRIDE_FILE:+$OVERRIDE_FILE}"
echo ""

# Get container status in table format
if [ -f "$OVERRIDE_FILE" ]; then
	STATUS=$(docker compose -f "$COMPOSE_FILE" -f "$OVERRIDE_FILE" ps --format "table {{.Name}}\t{{.Status}}\t{{.Health}}")
else
	STATUS=$(docker compose -f "$COMPOSE_FILE" ps --format "table {{.Name}}\t{{.Status}}\t{{.Health}}")
fi

echo "$STATUS"
echo ""

# Parse health column (skip header line)
UNHEALTHY=$(echo "$STATUS" | awk 'NR>1 && $NF !~ /healthy/ { print $0 }')

if [ -n "$UNHEALTHY" ]; then
	echo "❌ Some containers are not healthy:"
	echo "$UNHEALTHY" | while IFS= read -r line; do
		echo "   - $line"
	done
	exit 1
else
	echo "✅ All containers are healthy!"
	exit 0
fi
