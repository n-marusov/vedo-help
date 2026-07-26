#!/bin/sh
# check-container-health.sh
# Проверяет, что все Docker контейнеры из docker-compose.yml имеют статус healthy.
# Exit 0 если все healthy, 1 если есть unhealthy или starting.
#
# Usage:
#   check-container-health.sh <compose-file> [override-file] [env-file]
#
# Examples:
#   check-container-health.sh deploy/docker/compose.yml
#   check-container-health.sh deploy/docker/compose.test.yml "" .env.test

set -e

COMPOSE_FILE="${1:-deploy/docker/compose.yml}"
OVERRIDE_FILE="${2:-deploy/docker/compose.override.yml}"
ENV_FILE="${3:-}"

if [ ! -f "$COMPOSE_FILE" ]; then
	echo "ERROR: Compose file not found: $COMPOSE_FILE"
	exit 1
fi

echo "=== Container Health Check ==="
echo "Compose file: $COMPOSE_FILE"
if [ -n "$OVERRIDE_FILE" ] && [ -f "$OVERRIDE_FILE" ]; then
	echo "Override file: $OVERRIDE_FILE"
fi
if [ -n "$ENV_FILE" ]; then
	echo "Env file: $ENV_FILE"
fi
echo ""

# Build docker compose ps command with optional flags
COMPOSE_PS="docker compose"
if [ -n "$ENV_FILE" ] && [ -f "$ENV_FILE" ]; then
	COMPOSE_PS="$COMPOSE_PS --env-file $ENV_FILE"
fi
COMPOSE_PS="$COMPOSE_PS -f $COMPOSE_FILE"
if [ -n "$OVERRIDE_FILE" ] && [ -f "$OVERRIDE_FILE" ]; then
	COMPOSE_PS="$COMPOSE_PS -f $OVERRIDE_FILE"
fi
COMPOSE_PS="$COMPOSE_PS ps --format \"table {{.Name}}\t{{.Status}}\t{{.Health}}\""

STATUS=$(eval "$COMPOSE_PS")

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
