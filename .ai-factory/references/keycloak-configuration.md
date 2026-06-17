# Keycloak Configuration Reference

> Source: https://www.keycloak.org/documentation, https://www.keycloak.org/guides
> Created: 2026-06-17
> Updated: 2026-06-17
> Version: Keycloak 26.6.3 (Nightly)

## Overview

Keycloak is an open-source Identity and Access Management (IAM) solution developed under the Cloud Native Computing Foundation (CNCF). It supports OpenID Connect, OAuth 2.0, and SAML 2.0 protocols. This reference covers server configuration for Keycloak 26.x (Quarkus distribution), focusing on production deployment, database setup, hostname configuration, reverse proxy integration, TLS, container deployment, and logging.

## Core Concepts

- **Realm**: A tenant managing a set of users, credentials, roles, and groups. The `master` realm is the admin realm.
- **Client**: An application or service that requests authentication from Keycloak.
- **User**: An entity that can authenticate to Keycloak.
- **OpenID Connect (OIDC)**: The primary protocol for authentication and authorization (based on OAuth 2.0).
- **SAML 2.0**: Alternative protocol for authentication.
- **Client Secret**: A shared secret used by confidential clients (backend services) for the `client_secret_basic` or `client_secret_post` token endpoint authentication methods.
- **Service Account**: A special type of user that represents a client (used for machine-to-machine communication).

## Configuration Sources (Priority Order)

Keycloak loads configuration from four sources in order of priority:

1. **Command-line parameters**: `--db-url=cliValue`
2. **Environment variables**: `KC_DB_URL=envVarValue` (prefix `KC_` or `KCRAW_` for raw values)
3. **Configuration file**: `conf/keycloak.conf` (key=value format, default location; override with `--config-file=/path/to/conf`)
4. **Java KeyStore file**: `kc.db-password` stored as a PBE secret in a PKCS12 keystore

When an option is set in more than one source, the one higher in the list wins.

### Format Translation

| Source | Format | Example |
|--------|--------|---------|
| CLI | `--<key-with-dashes>=<value>` | `--db-url-host=myhost` |
| Env var | `KC_<KEY_WITH_UNDERSCORES>=<value>` | `KC_DB_URL_HOST=myhost` |
| Config file | `<key-with-dashes>=<value>` | `db-url-host=myhost` |
| KeyStore | `kc.<key-with-dashes>` (alias) | `kc.db-password` |

### Special Prefixes

- **`KCRAW_` prefix**: Use `KCRAW_DB_PASSWORD` instead of `KC_DB_PASSWORD` to preserve `$` characters literally (avoids expression evaluation). Cannot be used together with `KC_` for the same key.
- **`KCKEY_` prefix**: For environment variable keys with special characters (e.g., datasource names containing `_`, `$`, or `.`), use a pair: `KC_USER_STORE_DB_KIND=mariadb` + `KCKEY_USER_STORE_DB_KIND=db-kind-user_store$marketing`.
- **Config file placeholders**: Use `${ENV_VAR}` syntax, e.g., `db-url-host=${MY_DB_HOST:mydb}` (with fallback).

## Modes

### Development Mode (`start-dev`)

```bash
bin/kc.sh start-dev
```

Defaults: HTTP enabled, strict hostname disabled, local cache, theme/template caching disabled.

### Production Mode (`start`)

```bash
bin/kc.sh start
```

**Secure by default**: HTTP disabled, hostname required, HTTPS/TLS required. Will NOT start without explicit hostname and TLS configuration.

## Startup Optimization

### Build Process

Keycloak distinguishes between **build options** (persisted into an optimized image) and **configuration options** (runtime only).

```bash
# Build step (optimizes startup time)
bin/kc.sh build --db=postgres

# Start using optimized image
bin/kc.sh start --optimized
```

Build options are marked with a tool icon in All configuration page. Configuration options (credentials, hostname, etc.) must NOT be persisted as build options for security reasons.

### Containerfile (Multi-stage Build)

```dockerfile
FROM quay.io/keycloak/keycloak:latest AS builder

ENV KC_HEALTH_ENABLED=true
ENV KC_METRICS_ENABLED=true
ENV KC_DB=postgres

RUN /opt/keycloak/bin/kc.sh build

FROM quay.io/keycloak/keycloak:latest
COPY --from=builder /opt/keycloak/ /opt/keycloak/

ENV KC_DB=postgres
ENV KC_DB_URL=<DBURL>
ENV KC_DB_USERNAME=<DBUSERNAME>
ENV KC_DB_PASSWORD=<DBPASSWORD>
ENV KC_HOSTNAME=localhost

ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
```

## Database Configuration

### Supported Databases

| Vendor | `db` value | Default JDBC URL |
|--------|-----------|------------------|
| PostgreSQL | `postgres` | `jdbc:postgresql://localhost/keycloak` |
| MySQL | `mysql` | `jdbc:mysql://localhost/keycloak` |
| MariaDB | `mariadb` | `jdbc:mariadb://localhost/keycloak` |
| Microsoft SQL Server | `mssql` | `jdbc:sqlserver://localhost:1433;databaseName=keycloak` |
| Oracle | `oracle` | `jdbc:oracle:thin:@//localhost:1521/keycloak` |
| TiDB | `tidb` | `jdbc:mysql://localhost:4000/keycloak` |
| Dev File (H2) | `dev-file` | (default, NOT for production) |
| Dev Mem (H2) | `dev-mem` | (NOT for production) |

### Minimum Database Configuration

```bash
# Via CLI
bin/kc.sh start --db postgres --db-url-host keycloak-postgres --db-username keycloak --db-password change_me

# Via conf/keycloak.conf (recommended for secrets)
db=postgres
db-username=keycloak
db-password=change_me
db-url-host=keycloak-postgres

# Via environment variables (Docker)
KC_DB=postgres
KC_DB_URL=jdbc:postgresql://keycloak-db:5432/keycloak
KC_DB_USERNAME=keycloak
KC_DB_PASSWORD=change_me
```

### Connection Pool

| Option | Default | Description |
|--------|---------|-------------|
| `db-pool-max-size` | `100` | Maximum pool size |
| `db-pool-min-size` | (auto) | Minimum pool size |
| `db-pool-initial-size` | (auto) | Initial pool size |
| `db-pool-max-lifetime` | (auto) | Max lifetime before replacing connection |

### Database TLS

```bash
# TLS with server certificate verification
bin/kc.sh start --db=postgres --db-tls-mode=verify-server --db-tls-trust-store-file=/path/to/cert.pem

# mTLS (mutual TLS)
bin/kc.sh start --db=postgres --db-tls-mode=verify-server \
  --db-tls-trust-store-file=/path/to/truststore.pem \
  --db-mtls-key-store-file=/path/to/keystore.p12 \
  --db-mtls-key-store-password=changeit
```

### Additional Datasources

For custom extensions needing a separate database, use named datasources:

```bash
# Build time: specify the DB kind
bin/kc.sh build --db-kind-user-store=postgres

# Runtime: configure connection
bin/kc.sh start --db-kind-user-store=postgres \
  --db-username-user-store=my-user \
  --db-password-user-store=my-pass \
  --db-url-host-user-store=my-db-host
```

## Hostname Configuration (v2)

### Basic Hostname

```bash
# Simple hostname (scheme/port resolved from request)
bin/kc.sh start --hostname my.keycloak.org

# Full URL (explicit scheme and port)
bin/kc.sh start --hostname https://my.keycloak.org

# With context path
bin/kc.sh start --hostname https://my.keycloak.org:123/auth
```

### Separate Admin Hostname

```bash
bin/kc.sh start --hostname https://my.keycloak.org --hostname-admin https://admin.my.keycloak.org:8443
```

### Backchannel Dynamic Resolution

```bash
# Clients access Keycloak via private network (backchannel dynamically resolved)
bin/kc.sh start --hostname https://my.keycloak.org --hostname-backchannel-dynamic true
```

### Hostname Options

| Option | Default | Description |
|--------|---------|-------------|
| `hostname` | (required in prod) | Server address (URL or hostname) |
| `hostname-admin` | (same as hostname) | Admin console address |
| `hostname-strict` | `true` | Disable dynamic hostname resolution from headers |
| `hostname-backchannel-dynamic` | `false` | Enable dynamic backchannel URL resolution |
| `hostname-debug` | `false` | Enable debug page at `/realms/master/hostname-debug` |

### Validations

- In production (`start`), either `--hostname` or `--hostname-strict false` must be configured.
- If `hostname-admin` is set, `hostname` must be a full URL.
- If `hostname-backchannel-dynamic` is `true`, `hostname` must be a full URL.

## Reverse Proxy Configuration

### Proxy Headers

```bash
# RFC 7239 Forwarded header
bin/kc.sh start --proxy-headers forwarded

# Legacy X-Forwarded-* headers
bin/kc.sh start --proxy-headers xforwarded
```

### Trusted Proxies

```bash
# Restrict which proxies can set headers
bin/kc.sh start --proxy-headers forwarded --proxy-trusted-addresses=192.168.0.32,127.0.0.0/8
```

### HA PROXY Protocol

```bash
# For HTTPS passthrough proxies (cannot use proxy-headers)
bin/kc.sh start --proxy-protocol-enabled true
```

### Sticky Sessions

```bash
# Disable attaching node to cookie (if proxy handles session affinity)
bin/kc.sh start --spi-sticky-session-encoder--infinispan--should-attach-route=false
```

### Exposed Path Recommendations

| Path | Expose | Reason |
|------|--------|--------|
| `/` | No | Exposes admin paths unnecessarily |
| `/admin/` | No | Unnecessary attack vector |
| `/realms/` | Yes | Required for OIDC/SAML endpoints |
| `/resources/` | Yes | Static assets (can be CDN-served) |
| `/.well-known/` | Yes | RFC 8414 metadata discovery |
| `/metrics` | No | Unnecessary attack vector |
| `/health` | No | Unnecessary attack vector |

### Graceful Shutdown

```bash
# For TLS passthrough (longer delay for connection draining)
bin/kc.sh start --shutdown-delay=30s --shutdown-timeout=1s

# For load balancer that polls readiness
bin/kc.sh start --shutdown-delay=16s --shutdown-timeout=1s
```

## TLS / HTTPS Configuration

### Using PEM Files

```bash
bin/kc.sh start \
  --https-certificate-file=/path/to/cert.pem \
  --https-certificate-key-file=/path/to/key.pem \
  --hostname=my.keycloak.org
```

### Using KeyStore

```bash
bin/kc.sh start \
  --https-key-store-file=/path/to/keystore.p12 \
  --https-key-store-password=password \
  --hostname=my.keycloak.org
```

### TLS Options

| Option | Default | Description |
|--------|---------|-------------|
| `https-certificate-file` | - | Server certificate PEM |
| `https-certificate-key-file` | - | Private key PEM |
| `https-key-store-file` | - | Keystore with certificate |
| `https-key-store-password` | `password` | Keystore password |
| `https-key-store-type` | (auto) | Keystore type (JKS, PKCS12, BCFKS) |
| `https-port` | `8443` | HTTPS port |
| `https-protocols` | `TLSv1.3,TLSv1.2` | Enabled TLS protocols |
| `https-cipher-suites` | (reasonable default) | Cipher suites |
| `https-client-auth` | `none` | Client auth (`none`, `request`, `required`) |
| `https-certificates-reload-period` | `1h` | Certificate reload interval |
| `http-enabled` | `false` | Enable HTTP listener (for edge TLS termination) |
| `http-port` | `8080` | HTTP port |

### Edge TLS Termination

```bash
# TLS terminated at reverse proxy, HTTP to Keycloak
bin/kc.sh start --hostname https://my.keycloak.org --http-enabled true
```

## Features

Enable/disable preview or optional features:

```bash
bin/kc.sh start --features="token-exchange,admin-fine-grained-authz:v1"

# Enable ALL preview features
bin/kc.sh start --features=preview

# Disable specific features
bin/kc.sh start --features-disabled="impersonation,docker"

# Enable specific version of a feature
bin/kc.sh start --feature-account=v3 --feature-admin=v2
```

### Notable Features

| Feature | Description | Status |
|---------|-------------|--------|
| `token-exchange[:v1]` | Token exchange (deprecated, use v2) | Preview |
| `token-exchange-standard[:v2]` | Standard token exchange (RFC 8693) | Preview |
| `token-exchange-external-internal[:v2]` | External-internal token exchange | Preview |
| `admin-fine-grained-authz[:v1,v2]` | Fine-grained admin permissions | Preview |
| `admin[:v2]` | New admin console | Preview |
| `account[:v3]` | New account console | Preview |
| `hostname[:v2]` | Hostname v2 (default in 26.x) | Stable |
| `declarative-ui[:v1]` | Declarative UI for login themes | Preview |
| `dynamic-scopes[:v1]` | Dynamic OAuth scopes | Preview |
| `fips[:v1]` | FIPS 140-2 support | Preview |
| `persistent-user-sessions[:v1]` | Persistent user sessions | Preview |
| `organization[:v1]` | Organization support | Preview |
| `scim-api[:v1]` | SCIM provisioning API | Preview |
| `step-up-authentication[:v1]` | Step-up authentication | Preview |
| `client-secret-rotation[:v1]` | Automatic client secret rotation | Preview |
| `update-email[:v1]` | Email update flow | Preview |
| `openapi[:v1]` | OpenAPI endpoint | Preview |
| `transient-users[:v1]` | Transient (non-persisted) users | Preview |
| `oid4vc-vci[:v1]` | OpenID for Verifiable Credentials | Preview |

## Management Interface

Keycloak 26.x uses a dedicated management interface (port 9000 by default) for health checks and metrics, separate from the main HTTP server.

```bash
bin/kc.sh start --health-enabled=true --metrics-enabled=true
```

| Option | Default | Description |
|--------|---------|-------------|
| `health-enabled` | `false` | Enable health endpoints (`/health`, `/health/ready`, `/health/live`) |
| `metrics-enabled` | `false` | Enable metrics endpoint (`/metrics`) |
| `http-management-port` | `9000` | Management interface port |
| `http-management-health-enabled` | `true` | Expose health on management interface |
| `http-management-scheme` | `inherited` | HTTP scheme for management (`http` or `inherited`) |

## Logging

### Handlers

```bash
# Enable multiple handlers
bin/kc.sh start --log="console,file,syslog"
```

| Handler | Default | Description |
|---------|---------|-------------|
| `console` | Enabled | Logs to stdout/stderr |
| `file` | Disabled | Logs to file (`data/log/keycloak.log`) |
| `syslog` | Disabled | Logs to syslog server |

### Log Levels

```bash
# Root level
bin/kc.sh start --log-level=info

# Category-specific
bin/kc.sh start --log-level="INFO,org.hibernate:debug,org.keycloak.events:trace"

# Individual category option (overrides --log-level)
bin/kc.sh start --log-level-org.keycloak=trace
```

### Per-handler Log Levels

```bash
bin/kc.sh start --log=console,file --log-level=debug --log-console-level=info
```
The root `--log-level` represents the maximum verbosity. Individual handlers can be less verbose but not more.

### JSON Output

```bash
# Default JSON format
bin/kc.sh start --log-console-output=json

# ECS (Elastic Common Schema) format
bin/kc.sh start --log-console-output=json --log-console-json-format=ecs
```

### HTTP Access Log

```bash
# Enable access logging
bin/kc.sh start --http-access-log-enabled=true

# Write to dedicated file with rotation
bin/kc.sh start --http-access-log-enabled=true --http-access-log-file-enabled=true
```

### MDC (Mapped Diagnostic Context)

```bash
# Enable realm/client context in logs
bin/kc.sh start --log-mdc-enabled=true

# Customize MDC keys
bin/kc.sh start --log-mdc-enabled=true --log-mdc-keys="realmName,clientId,ipAddress"
```

## Initial Admin User

```bash
# Via environment variables (required for containers)
KC_BOOTSTRAP_ADMIN_USERNAME=admin
KC_BOOTSTRAP_ADMIN_PASSWORD=change_me

# Temporary bootstrap admin (recovery)
KC_BOOTSTRAP_ADMIN_CLIENT_SECRET=<secret>
KC_BOOTSTRAP_ADMIN_PASSWORD=<password>
KC_BOOTSTRAP_ADMIN_USERNAME=temp-admin      # default
KC_BOOTSTRAP_ADMIN_CLIENT_ID=temp-admin      # default
```

## Container Deployment

### Key Images

- `quay.io/keycloak/keycloak:latest` - Official Keycloak container image
- Base path: `/opt/keycloak/`
- Provider JARs: `/opt/keycloak/providers/`
- Import directory: `/opt/keycloak/data/import`
- Data directory: `/opt/keycloak/data/`

### Running Containers

```bash
# Development mode
docker run --name mykeycloak -p 127.0.0.1:8080:8080 \
  -e KC_BOOTSTRAP_ADMIN_USERNAME=admin \
  -e KC_BOOTSTRAP_ADMIN_PASSWORD=change_me \
  quay.io/keycloak/keycloak:latest start-dev

# Production with optimized image
docker run --name mykeycloak -p 8443:8443 -p 9000:9000 \
  -e KC_BOOTSTRAP_ADMIN_USERNAME=admin \
  -e KC_BOOTSTRAP_ADMIN_PASSWORD=change_me \
  -e KC_DB=postgres \
  -e KC_DB_URL=jdbc:postgresql://postgres:5432/keycloak \
  -e KC_DB_USERNAME=keycloak \
  -e KC_DB_PASSWORD=change_me \
  -e KC_HOSTNAME=localhost \
  mykeycloak start --optimized
```

### Realm Import on Startup

```bash
docker run --name keycloak -p 8080:8080 \
  -e KC_BOOTSTRAP_ADMIN_USERNAME=admin \
  -e KC_BOOTSTRAP_ADMIN_PASSWORD=change_me \
  -v /path/to/realm/data:/opt/keycloak/data/import \
  quay.io/keycloak/keycloak:latest start-dev --import-realm
```

### Memory Settings

- Default heap: `-XX:MaxRAMPercentage=70`, `-XX:InitialRAMPercentage=50`
- Always set memory limit for containers (recommended minimum: 750 MB, production: 2 GB)
- Override via `JAVA_OPTS_KC_HEAP` environment variable

```bash
docker run --name mykeycloak -m 1g \
  -e JAVA_OPTS_KC_HEAP="-XX:MaxRAMPercentage=65" \
  ...
```

### Docker Compose (Production Pattern)

```yaml
keycloak-db:
  image: postgres:16-alpine
  environment:
    POSTGRES_DB: keycloak
    POSTGRES_USER: keycloak
    POSTGRES_PASSWORD: ${KEYCLOAK_DB_PASSWORD:-keycloak}
  volumes:
    - keycloak_db_data:/var/lib/postgresql/data
  healthcheck:
    test: ["CMD-SHELL", "pg_isready -U keycloak"]
    interval: 10s

keycloak:
  build:
    context: ./keycloak
    target: base
  command: start
  environment:
    KC_DB: postgres
    KC_DB_URL: jdbc:postgresql://keycloak-db:5432/keycloak
    KC_DB_USERNAME: keycloak
    KC_DB_PASSWORD: ${KEYCLOAK_DB_PASSWORD:-keycloak}
    KC_HOSTNAME: ${KEYCLOAK_HOSTNAME:-localhost}
    KC_HTTP_ENABLED: "true"
    KC_HOSTNAME_STRICT: "false"
  depends_on:
    keycloak-db:
      condition: service_healthy
  healthcheck:
    test: ["CMD", "curl", "-f", "http://localhost:8080/realms/master"]
    interval: 30s
    start_period: 60s
```

## OpenID Connect Client Configuration

### Confidential Client (Backend Service)

```bash
# Standard flow with client secret
# Client authentication: client_secret_basic or client_secret_post
```

Keycloak client configuration for backend service:
- **Client ID**: unique identifier for the service
- **Client authentication**: enabled (On)
- **Authentication flow**: Service accounts roles
- **Client secret**: generated or explicitly set
- **Access Type**: confidential

### Public Client (SPA / Frontend)

- **Client authentication**: disabled (Off)
- **Standard flow**: enabled
- **Valid redirect URIs**: `http://localhost:5173/*`, `https://app.example.com/*`
- **Valid post logout redirect URIs**: `http://localhost:5173/*`, `https://app.example.com/*`
- **Web origins**: `http://localhost:5173`, `https://app.example.com`
- **Access Type**: public

### Client Registration via CLI

```bash
# Using kcadm.sh (requires admin token)
kcadm.sh create clients \
  -r myrealm \
  -s clientId=my-service \
  -s secret=my-secret \
  -s serviceAccountsEnabled=true
```

## Caching

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `cache` | `ispn`, `local` | (auto) | `ispn` in production, `local` in dev |
| `cache-stack` | `jdbc-ping`, (any) | `jdbc-ping` | Cluster discovery |
| `cache-config-file` | File path | - | Custom cache config (relative to `conf/`) |
| `cache-remote-host` | String | - | External Infinispan host (multi-site/clusterless) |
| `cache-remote-port` | Integer | `11222` | External Infinispan port |
| `cache-embedded-mtls-enabled` | `true`, `false` | `true` | Encrypted cluster communication |

## Vault

```bash
# File-based vault
bin/kc.sh start --vault=file --vault-dir=/path/to/secrets

# Keystore-based vault
bin/kc.sh start --vault=keystore --vault-file=/path/to/vault.p12 --vault-pass=password
```

## Docker Registry Integration

Keycloak can authenticate Docker registries. Enable with:

```bash
bin/kc.sh build --features=docker[:v1]
```

## Common Pitfalls

1. **`dev-file` database in production**: Always explicitly set `--db` to a production database. The default `dev-file` (H2) is not suitable for production.
2. **Missing hostname in production mode**: Production mode requires `--hostname` or `--hostname-strict false`. The server will refuse to start.
3. **HTTP disabled in production**: By default, `--http-enabled=false`. Use `--http-enabled=true` only when behind a TLS-terminating reverse proxy.
4. **Password with `$` chars**: Use `KCRAW_DB_PASSWORD` environment variable instead of `KC_DB_PASSWORD` to prevent expression evaluation.
5. **MySQL 8.0.30+ invisible primary keys**: Disable `sql_generate_invisible_primary_key=OFF` on the MySQL server before installing/upgrading Keycloak.
6. **MS SQL Server deadlocks**: Set `READ_COMMITTED_SNAPSHOT ON` to prevent deadlocks under high load.
7. **Docker file timestamps**: Docker can truncate modification timestamps on provider JARs, causing `start --optimized` failures. Use `touch -m --date=@<timestamp>` before `RUN kc.sh build`.
8. **Wrong thread pool sizing**: Default max threads is `max(4 * processors, 50)`. For high-concurrency deployments, consider tuning `http-pool-max-threads`.
9. **Quarkus raw properties**: Avoid using `quarkus.*` properties directly. Use the Keycloak first-class configuration options (e.g., `kc.http-port` instead of `quarkus.http.port`).

## Best Practices

1. **Always use an optimized build in production**: Run `kc.sh build` during CI/CD or in a multi-stage container build, then use `start --optimized`.
2. **Use a dedicated PostgreSQL database**: With a separate PostgreSQL 16 instance in production. Set `READ_COMMITTED_SNAPSHOT ON` for MSSQL.
3. **Enable health checks and metrics**: `--health-enabled=true --metrics-enabled=true`. Use `/health/ready` for Kubernetes liveness/readiness probes.
4. **Set explicit hostname**: Always use `--hostname` with a full URL when behind a reverse proxy.
5. **Use `proxy-headers` option**: Configure `--proxy-headers forwarded` (RFC 7239) or `xforwarded` for correct client IP detection.
6. **Restrict exposed paths**: Only expose `/realms/`, `/resources/`, and `/.well-known/` through the reverse proxy.
7. **Use the Management Interface**: Health checks and metrics on port 9000 (separate from main traffic).
8. **Configure TLS for database connections**: `--db-tls-mode=verify-server` with proper certificate validation.
9. **Set container memory limits**: Always use `-m` (Docker) or `resources.limits.memory` (Kubernetes) to prevent excessive heap growth.
10. **Use environment variables for runtime config**: Database credentials, hostname. Use build options only for DB vendor, features, and provider configuration.
11. **Regularly rotate client secrets**: Use the `client-secret-rotation` feature (preview) for automated rotation.
12. **Enable async logging only when needed**: Adds memory overhead but improves throughput for I/O-heavy handlers.

## Version Notes

- Keycloak 26.x uses the Quarkus distribution (WildFly distribution is deprecated and removed).
- Hostname v2 (`hostname[:v2]`) is enabled by default in 26.x.
- Token Exchange v1 is deprecated; use `token-exchange-standard[:v2]` or `token-exchange-external-internal[:v2]`.
- The deprecated `proxy` option (which accepted `edge`, `reencrypt`, `passthrough`) is replaced by `proxy-headers`.
- The OpenTelemetry tracing (`tracing-enabled`) and metrics (`telemetry-metrics-enabled`) are separate features requiring `preview` or explicit feature flags.
- Starting from 26.x, health and metrics are exposed on a separate management interface (port 9000) by default.
