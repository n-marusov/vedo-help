# Production Runbook

> Operational procedures for the VEDO hub RAG Assistant production deployment.

## Deployment

### CI/CD Auto-Deploy

Pushing to `main` triggers the **Deploy** workflow (`.github/workflows/deploy.yml`):

1. **Build & push** — multi-arch Docker images to GitHub Container Registry
2. **E2E tests** — Playwright against full Docker test stack
3. **Deploy to VPS** — SSH into production, pull new images, roll update
4. **Smoke test** — verify health after deployment

Monitor the workflow at: `https://github.com/<org>/vedo-rag-assistant/actions`

### Manual Deploy

```bash
# Pull latest images
docker compose -f docker-compose.yml -f docker-compose.production.yml pull backend frontend

# Roll update (no-deps to avoid restarting infrastructure)
docker compose -f docker-compose.yml -f docker-compose.production.yml up -d --no-deps backend frontend

# Verify
bash scripts/smoke-test.sh --production --quick
```

### Rollback

```bash
# Pull a specific version
docker compose -f docker-compose.yml -f docker-compose.production.yml pull backend:<previous-version>

# Tag and deploy
docker tag ghcr.io/<org>/backend:<previous-version> ghcr.io/<org>/backend:latest
docker compose -f docker-compose.yml -f docker-compose.production.yml up -d --no-deps backend
```

## Backup & Restore

### Automated Backup (systemd Timer)

```bash
# Install daily backup timer (runs at 03:00)
sudo ./scripts/install-backup-timer.sh --prod

# Verify
sudo systemctl list-timers --all | grep vedo
```

### Manual Backup

```bash
# Development
./scripts/backup.sh

# Production
./scripts/backup.sh --prod
```

Backups are stored in `backups/` with three files per run:

| File | Description |
|------|-------------|
| `vedo-<TIMESTAMP>.sql` | PostgreSQL dump of the `vedo` database |
| `keycloak-<TIMESTAMP>.sql` | PostgreSQL dump of the `keycloak` database |
| `chroma-<TIMESTAMP>.tar.gz` | Chroma vector store archive |

Backups older than 30 days are automatically pruned.

### Restore

```bash
# Validate files exist, then restore
./scripts/restore.sh backups/vedo-2024-01-01.sql backups/keycloak-2024-01-01.sql

# With Chroma data
./scripts/restore.sh backups/vedo-2024-01-01.sql backups/keycloak-2024-01-01.sql backups/chroma-2024-01-01.tar.gz

# Production
./scripts/restore.sh --prod backups/vedo-2024-01-01.sql backups/keycloak-2024-01-01.sql
```

The restore script:
- Validates all input files **before** stopping containers
- Drops and recreates databases for a clean state
- Restarts services even on failure (rollback-safe)

## Health Checks

| Endpoint/Command | Purpose | Expected Response |
|---|---|---|
| `GET /health` | Basic liveness | `OK` |
| `GET /api/health/deep` | Dependency check | JSON with per-service status |
| `bash scripts/smoke-test.sh --production --quick` | Comprehensive | Exit 0 |
| `docker compose ps` | Container status | All `healthy` |

### Interpreting Health Status

- **Backend unhealthy**: Check logs with `docker compose logs --tail=50 backend`
- **Chroma unhealthy**: Restart with `docker compose restart chroma`
- **Database unhealthy**: Check PostgreSQL logs `docker compose logs db`
- **KeyCloak unhealthy**: Verify realm import `docker compose logs keycloak`

## Incident Response

### Chroma Unavailable

```bash
docker compose logs --tail=50 chroma
docker compose restart chroma
# If persistent: check disk space, volume integrity
```

### LLM API Errors

```bash
# Verify API key and rate limits
docker compose exec backend env | grep LLM
# Check backend logs for upstream errors
docker compose logs --tail=50 backend | grep -i "llm\|routerai\|error"
```

### High Memory Usage

```bash
# Check per-container memory
docker stats --no-stream

# Check OTel collector memory limiter
docker compose logs --tail=20 otel-collector | grep -i memory

# View cAdvisor metrics (via SSH tunnel)
# http://localhost:18080/metrics
```

### Database Corruption

```bash
# 1. Identify latest valid backup
ls -lt backups/vedo-*.sql

# 2. Restore
./scripts/restore.sh --prod backups/vedo-<LATEST>.sql backups/keycloak-<LATEST>.sql

# 3. Verify
bash scripts/smoke-test.sh --production --quick
```

### KeyCloak Auth Failures

```bash
# Check KeyCloak logs
docker compose logs --tail=50 keycloak

# Verify realm was imported
docker compose exec keycloak ls /opt/keycloak/data/import/

# Force re-import (restart keycloak-init + keycloak)
docker compose up -d keycloak-init keycloak
```

## Monitoring Access

All monitoring services are internal-only (no public ports). Access via SSH tunnel:

```bash
# Grafana
ssh -L 3000:grafana:3000 user@host
# Open http://localhost:3000

# Prometheus
ssh -L 9090:prometheus:9090 user@host
# Open http://localhost:9090

# cAdvisor
ssh -L 8080:cadvisor:8080 user@host
# Open http://localhost:8080
```

See [Monitoring](monitoring.md) for detailed dashboard and alert information.

## Password & Key Rotation

### Grafana Admin Password

1. Set `GF_SECURITY_ADMIN_PASSWORD` in `.env`
2. Restart Grafana: `docker compose -f docker-compose.yml -f docker-compose.production.yml up -d grafana`

### KeyCloak Admin Password

1. Set `KEYCLOAK_ADMIN_PASSWORD` in `.env`
2. Restart KeyCloak: `docker compose -f docker-compose.yml -f docker-compose.production.yml up -d keycloak`

### PostgreSQL Passwords

1. Update passwords in `.env` (`POSTGRES_PASSWORD`, `VEDO_DB_PASSWORD`, `KEYCLOAK_DB_PASSWORD`)
2. Restart all services: `docker compose -f docker-compose.yml -f docker-compose.production.yml down && docker compose -f docker-compose.yml -f docker-compose.production.yml up -d`

> ⚠️ Rotating PostgreSQL passwords requires updating all services that connect to the database.

## Logs Access

### Docker Logs

```bash
# Follow all services
docker compose logs -f

# Follow a specific service
docker compose logs -f backend

# Last N lines
docker compose logs --tail=100 frontend

# Search for errors
docker compose logs backend 2>&1 | grep -i error
```

### Production Logs (json-file driver)

All services use the json-file logging driver with rotation:
- Max size: 20 MB per file
- Max files: 5

### Systemd Journal (Backup Timer)

```bash
# View backup timer logs
journalctl -u vedo-backup.service

# Follow backup logs
journalctl -u vedo-backup.service -f
```

### OTel Collector

Structured logs, traces, and metrics are exported via OTLP to the OTel collector:

```bash
# View collector logs (debug exporter output)
docker compose logs --tail=50 otel-collector
```

## See Also

- [Deployment Guide](deployment.md) — environment setup and configuration
- [Monitoring](monitoring.md) — dashboards, alerts, and metrics
- [Architecture](c4-architecture.md) — C4 model diagrams
