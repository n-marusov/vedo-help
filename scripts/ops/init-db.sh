#!/bin/bash
# init-db.sh — PostgreSQL initialization script for Docker entrypoint.
set -e

echo "[init-db] Creating databases and users..."

# Create vedo user
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL
    CREATE USER vedo WITH PASSWORD '${VEDO_DB_PASSWORD:-CHANGEME-vedo-password}';
EOSQL

echo "[init-db] vedo user ready"

# Create vedo database
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL
    CREATE DATABASE vedo OWNER vedo;
EOSQL

echo "[init-db] vedo database ready"

# Grant schema permissions
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "vedo" <<-EOSQL
    GRANT ALL ON SCHEMA public TO vedo;
EOSQL

# Create keycloak user
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL
    CREATE USER keycloak WITH PASSWORD '${KEYCLOAK_DB_PASSWORD:-CHANGEME-keycloak-password}';
EOSQL

echo "[init-db] keycloak user ready"

# Create keycloak database
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL
    CREATE DATABASE keycloak OWNER keycloak;
EOSQL

echo "[init-db] keycloak database ready"

# Grant schema permissions
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "keycloak" <<-EOSQL
    GRANT ALL ON SCHEMA public TO keycloak;
EOSQL

echo "[init-db] Database initialization complete."
