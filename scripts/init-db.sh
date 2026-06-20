#!/bin/bash
# init-db.sh — PostgreSQL initialization script for Docker entrypoint.
# Creates separate databases and users for vedo-backend and KeyCloak.
# Mounted via docker-compose.yml into /docker-entrypoint-initdb.d/.
set -e

echo "[init-db] Creating databases and users..."

# Create vedo user and database
echo "[init-db] Creating vedo user and database..."
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    DO \$do BEGIN
        IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'vedo') THEN
            CREATE USER vedo WITH PASSWORD '${VEDO_DB_PASSWORD:-vedo}';
        END IF;
    END \$do;

    CREATE DATABASE vedo OWNER vedo;

    GRANT ALL PRIVILEGES ON DATABASE vedo TO vedo;
EOSQL

echo "[init-db] vedo database created."

# Create keycloak user and database
echo "[init-db] Creating keycloak user and database..."
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    DO \$do BEGIN
        IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'keycloak') THEN
            CREATE USER keycloak WITH PASSWORD '${KEYCLOAK_DB_PASSWORD:-keycloak}';
        END IF;
    END \$do;

    CREATE DATABASE keycloak OWNER keycloak;

    GRANT ALL PRIVILEGES ON DATABASE keycloak TO keycloak;
EOSQL

echo "[init-db] keycloak database created."

# Grant schema permissions for vedo user on vedo database
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "vedo" <<-EOSQL
    GRANT ALL ON SCHEMA public TO vedo;
EOSQL

# Grant schema permissions for keycloak user on keycloak database
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "keycloak" <<-EOSQL
    GRANT ALL ON SCHEMA public TO keycloak;
EOSQL

echo "[init-db] Database initialization complete."
