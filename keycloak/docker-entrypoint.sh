#!/bin/bash
# ====================================================================
# docker-entrypoint.sh — VEDO hub KeyCloak bootstrap
#
# 1. Substitute ${VAR} placeholders in realm-import.json.template
#    with values from environment variables.
# 2. Import the resolved realm via kcadm.sh / import.
# 3. Start KeyCloak in production mode.
# ====================================================================

set -e

IMPORT_DIR="/opt/keycloak/data/import"
TEMPLATE="${IMPORT_DIR}/realm-import.json.template"
RESOLVED="${IMPORT_DIR}/realm-import.json"

if [ -f "$TEMPLATE" ]; then
  echo "[INFO] Resolving realm template: ${TEMPLATE}"

  # Use envsubst if available (from gettext-base), otherwise fall back to
  # a minimal sed-based substitution for common vars.
  if command -v envsubst >/dev/null 2>&1; then
    # Export all KC_*, KEYCLOAK_*, VEDO_*, YANDEX_*, VK_*, MAILRU_* vars
    # so envsubst can see them
    export_vars=$(env | grep -oE '^(KC_|KEYCLOAK_|VEDO_|YANDEX_|VK_|MAILRU_)[^=]+' | tr '\n' '|' | sed 's/|$//')
    if [ -n "$export_vars" ]; then
      envsubst "$(echo "$export_vars" | sed 's/[^|]\+/$\0/g; s/|/|$/g')" < "$TEMPLATE" > "$RESOLVED"
      echo "[INFO] Template resolved with envsubst -> ${RESOLVED}"
    else
      echo "[WARN] No known env vars found; copying template as-is"
      cp "$TEMPLATE" "$RESOLVED"
    fi
  else
    echo "[INFO] envsubst not available; using sed for common substitutions"
    cp "$TEMPLATE" "$RESOLVED"
    # Replace common placeholders
    sed -i \
      -e "s|\${KEYCLOAK_ADMIN_PASSWORD}|${KEYCLOAK_ADMIN_PASSWORD:-admin}|g" \
      -e "s|\${VEDO_BACKEND_CLIENT_SECRET}|${VEDO_BACKEND_CLIENT_SECRET:-changeme-vedo-backend-secret}|g" \
      -e "s|\${YANDEX_CLIENT_ID}|${YANDEX_CLIENT_ID:-}|g" \
      -e "s|\${YANDEX_CLIENT_SECRET}|${YANDEX_CLIENT_SECRET:-}|g" \
      -e "s|\${VK_CLIENT_ID}|${VK_CLIENT_ID:-}|g" \
      -e "s|\${VK_CLIENT_SECRET}|${VK_CLIENT_SECRET:-}|g" \
      -e "s|\${MAILRU_CLIENT_ID}|${MAILRU_CLIENT_ID:-}|g" \
      -e "s|\${MAILRU_CLIENT_SECRET}|${MAILRU_CLIENT_SECRET:-}|g" \
      "$RESOLVED"
    echo "[INFO] Template resolved with sed -> ${RESOLVED}"
  fi

  echo "[INFO] Importing realm vedo-hub..."
  # Use KeyCloak's built-in import (available in production mode)
  /opt/keycloak/bin/kc.sh import --file "$RESOLVED" 2>&1
  echo "[INFO] Realm vedo-hub imported successfully"
else
  echo "[WARN] No realm template found at ${TEMPLATE}; skipping import"
fi

echo "[INFO] Starting KeyCloak in production mode..."
exec /opt/keycloak/bin/kc.sh start "$@"
