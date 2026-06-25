# OpenTelemetry Logging Unification Reference

> Source:
> - https://opentelemetry.io/docs/reference/specification/logs/data-model/
> - https://opentelemetry.io/docs/specs/semconv/general/logs/
> - https://opentelemetry.io/docs/specs/semconv/general/events/
> - https://opentelemetry.io/docs/specs/otel/logs/bridge-api/
> - https://opentelemetry.io/docs/specs/semconv/resource/
>
> Created: 2026-06-25
> Updated: 2026-06-25

## Overview

This reference describes how to unify logging across all VEDO hub services (Rust backend, Python embedding, TypeScript/Vue frontend) into a single structured format aligned with the OpenTelemetry Logs Data Model. The goal is to replace the current fragmented logging with a coherent system where every log record contains consistent fields (`trace_id`, `span_id`, `severity`, `service.name`, structured `attributes`) and can be exported via OTLP to any backend.

**Current state in this project:**

| Service | Library | Format | Structured? | OTel-ready? |
|---------|---------|--------|-------------|-------------|
| Backend (Rust) | `tracing` + `tracing-subscriber` | JSON via `.json()` | Partial | No OTLP export |
| Embedding (Python) | `logging` stdlib | `asctime [level] name: message` | No | No |
| Frontend (TS/Vue) | `console.debug()` | Raw text | No | No |
| Docker logging | `json-file` driver | Container stdout | Only Docker meta | No |

## Core Concepts

### Logs Data Model (OTel)

Every log record in OpenTelemetry consists of the following fields. Fields marked **critical** should be present in every log record in this project.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| **`Timestamp`** | uint64 (ns) | Recommended | Time when the event occurred |
| **`ObservedTimestamp`** | uint64 (ns) | Recommended | Time when the event was observed by collector |
| **`TraceId`** | byte[16] | Optional | W3C Trace Context trace ID |
| **`SpanId`** | byte[8] | Optional | W3C Trace Context span ID |
| `TraceFlags` | byte | Optional | W3C trace flags (e.g. SAMPLED) |
| **`SeverityText`** | string | Optional | Original severity string (e.g. "INFO", "ERROR") |
| **`SeverityNumber`** | int | Optional | Normalized severity (1-24 scale) |
| **`Body`** | AnyValue | Optional | Log message or structured body |
| **`Resource`** | Resource | Optional | Describes the source (service name, host, etc.) |
| **`InstrumentationScope`** | Scope | Optional | Library/component that emitted the log |
| **`Attributes`** | map | Optional | Key-value pairs with additional context |
| `EventName` | string | Optional | Identifies event class (for Event-type logs) |

### Severity Number Scale

| Range | Name | Usage |
|-------|------|-------|
| 1-4 | TRACE | Fine-grained debugging, disabled by default |
| 5-8 | DEBUG | Debugging events |
| 9-12 | INFO | Informational events |
| 13-16 | WARN | Warning events |
| 17-20 | ERROR | Error events |
| 21-24 | FATAL | Fatal errors / crashes |

Recommended mapping: `SeverityNumber=9` for INFO, `13` for WARN, `17` for ERROR.

### Semantic Conventions for Attributes

All attribute keys should follow the `domain.name` pattern (e.g. `http.request.method`, `service.name`, `log.file.path`). Reuse standard OTel attributes whenever possible:

| Attribute Key | Description | Example |
|---------------|-------------|---------|
| `service.name` | Logical service name | `vedo-backend` |
| `service.version` | Service version | `0.1.0` |
| `service.instance.id` | Unique instance ID | UUID per process |
| `log.file.name` | Basename of log file | `embedding.log` |
| `log.file.path` | Full path to log file | `/var/log/embedding.log` |
| `log.iostream` | stdout or stderr | `stdout` |
| `log.record.original` | Complete original raw log record | Raw syslog line |
| `log.record.uid` | Unique log record ID | ULID or UUID |
| `error.type` | Error type / class | `RuntimeError`, `DbConnectionError` |
| `code.filepath` | Source file path | `src/modules/query/service.rs` |
| `code.lineno` | Source line number | `142` |
| `code.function` | Function name | `query_handler` |

### Events vs Logs

- **Events** are LogRecords with a non-empty `EventName`. They describe named occurrences at a meaningful point in time (e.g. `user.login`, `document.uploaded`, `sync.completed`).
- **Logs** are generic LogRecords without `EventName`. Use for unstructured diagnostic messages.
- Events should be preferred for domain-specific occurrences (checkpoints, state changes, lifecycle moments) that need to be queryable by name.

## Resource Attributes

Every service should set these Resource attributes:

| Attribute | Backend | Embedding | Frontend |
|-----------|---------|-----------|----------|
| `service.name` | `vedo-backend` | `vedo-embedding` | `vedo-frontend` |
| `service.version` | `0.1.0` | `0.1.0` | `0.1.0` |
| `telemetry.sdk.language` | `rust` | `python` | `webjs` |
| `telemetry.sdk.name` | `opentelemetry` | `opentelemetry` | `opentelemetry` |
| `deployment.environment` | `production` / `development` (via env) | same | same |

## Usage Patterns

### Rust Backend — Current (needs migration)

Current setup in `backend/src/main.rs`:
```rust
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::new(&config.rust_log))
    .json()
    .init();
```

**Target setup with OTel OTLP export:**

```toml
# Cargo.toml additions
tracing-opentelemetry = "0.28"
opentelemetry = { version = "0.27", features = ["trace"] }
opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic"] }
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
```

```rust
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use tracing_subscriber::prelude::*;

fn init_telemetry(config: &AppConfig) {
    let resource = Resource::new(vec![
        KeyValue::new("service.name", "vedo-backend"),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("deployment.environment", config.environment.clone()),
    ]);

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.otel_endpoint),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default().with_resource(resource),
        )
        .install_batch()
        .expect("Failed to install OTLP tracer");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Keep JSON stdout for Docker logs, add OTel
    tracing_subscriber::registry()
        .with(EnvFilter::new(&config.rust_log))
        .with(tracing_subscriber::fmt::layer().json())
        .with(telemetry)
        .init();
}

// Logging pattern with structured fields:
tracing::info!(
    user_id = %auth_info.sub,
    collection_id = %collection_id,
    "Document uploaded successfully"
);
```

### Python Embedding — Complete migration needed

Current (text-based):
```python
logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
```

**Target with structlog + OTel:**

```toml
# pyproject.toml additions
opentelemetry-api = "^1.29"
opentelemetry-sdk = "^1.29"
opentelemetry-exporter-otlp-proto-grpc = "^1.29"
opentelemetry-instrumentation-logging = "^0.50b0"
structlog = "^24.4"
```

```python
import structlog
import logging
from opentelemetry import _logs
from opentelemetry.sdk._logs import LoggerProvider, LoggingHandler
from opentelemetry.sdk._logs.export import BatchLogRecordProcessor
from opentelemetry.exporter.otlp.proto.grpc._log_exporter import OTLPLogExporter
from opentelemetry.sdk.resources import Resource
from opentelemetry.semconv.resource import ResourceAttributes

resource = Resource.create({
    ResourceAttributes.SERVICE_NAME: "vedo-embedding",
    ResourceAttributes.SERVICE_VERSION: "0.1.0",
    ResourceAttributes.DEPLOYMENT_ENVIRONMENT: os.environ.get("ENVIRONMENT", "development"),
})

logger_provider = LoggerProvider(resource=resource)
log_exporter = OTLPLogExporter(endpoint=os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT", "http://otel-collector:4317"))
logger_provider.add_log_record_processor(BatchLogRecordProcessor(log_exporter))

# stdlib handler that sends to OTel
handler = LoggingHandler(level=logging.NOTSET, logger_provider=logger_provider)

# Configure structlog as the application logger
structlog.configure(
    processors=[
        structlog.contextvars.merge_contextvars,
        structlog.processors.add_log_level,
        structlog.processors.StackInfoRenderer(),
        structlog.dev.ConsoleRenderer() if os.environ.get("DEV") else structlog.processors.JSONRenderer(),
    ],
    wrapper_class=structlog.make_filtering_bound_logger(logging.DEBUG),
    context_class=dict,
    logger_factory=structlog.PrintLoggerFactory(),
    cache_logger_on_first_use=True,
)

logger = structlog.get_logger()

# Usage:
logger.info("embedding.computed", text_count=len(texts), model=model_name)
logger.error("embedding.failed", error=str(e), text_count=len(texts))
```

### Frontend (TypeScript/Vue) — Add structured logging

Current: `console.debug('[App] mounted: root layout initialized with persistent header')`

**Target with @opentelemetry/instrumentation:**

```json
// package.json additions
"@opentelemetry/api": "^1.9",
"@opentelemetry/sdk-logs": "^0.56",
"@opentelemetry/exporter-logs-otlp-grpc": "^0.56",
"@opentelemetry/resources": "^1.29",
"@opentelemetry/semantic-conventions": "^1.28",
"@opentelemetry/instrumentation-document-load": "^0.44"
```

```typescript
// src/telemetry.ts
import { logs } from '@opentelemetry/api-logs';
import {
  LoggerProvider,
  BatchLogRecordProcessor,
  SimpleLogRecordProcessor,
} from '@opentelemetry/sdk-logs';
import { OTLPLogExporter } from '@opentelemetry/exporter-logs-otlp-http';
import { Resource } from '@opentelemetry/resources';
import { SemanticResourceAttributes } from '@opentelemetry/semantic-conventions';

const resource = Resource.default().merge(
  new Resource({
    [SemanticResourceAttributes.SERVICE_NAME]: 'vedo-frontend',
    [SemanticResourceAttributes.SERVICE_VERSION]: '0.1.0',
    [SemanticResourceAttributes.DEPLOYMENT_ENVIRONMENT]:
      import.meta.env.PROD ? 'production' : 'development',
  }),
);

const loggerProvider = new LoggerProvider({ resource });
loggerProvider.addLogRecordProcessor(
  new BatchLogRecordProcessor(
    new OTLPLogExporter({
      url: import.meta.env.VITE_OTEL_COLLECTOR_URL || '/v1/logs',
    }),
  ),
);

// Also keep a console exporter in dev
if (import.meta.env.DEV) {
  loggerProvider.addLogRecordProcessor(
    new SimpleLogRecordProcessor({
      export: (records) => {
        for (const r of records) {
          const body = r.body;
          const sev = r.severityText || 'INFO';
          console.log(`[${sev}] ${r.attributes?.['component'] || '-'}: ${body}`);
        }
      },
      shutdown: async () => {},
    }),
  );
}

logs.setGlobalLoggerProvider(loggerProvider);

// Usage in components:
import { logs } from '@opentelemetry/api-logs';
const logger = logs.getLogger('vedo-frontend', '0.1.0');

// Instead of console.debug('[App] mounted...'):
logger.emit({
  body: 'Root layout initialized with persistent header',
  severityText: 'DEBUG',
  attributes: {
    component: 'App',
    route: window.location.pathname,
  },
});
```

### OTel Collector Configuration

Add to `docker-compose.yml` as a new service:

```yaml
otel-collector:
  image: otel/opentelemetry-collector-contrib:0.114.0
  command: ["--config=/etc/otel-collector-config.yaml"]
  volumes:
    - ./otel-collector-config.yaml:/etc/otel-collector-config.yaml
  networks:
    - internal
  restart: unless-stopped
  logging: *default-logging
  depends_on:
    - backend  # minimal dep to avoid circular waits
```

With config file `otel-collector-config.yaml`:

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 1s
    send_batch_size: 1024
  attributes:
    actions:
      - key: deployment.environment
        value: "${ENVIRONMENT}"
        action: upsert
  resourcedetection:
    detectors: [docker, env, hostname]
    timeout: 2s

exporters:
  # Example: stdout for debugging
  debug:
    verbosity: detailed
  # Example: Loki (common choice for log storage)
  loki:
    endpoint: http://loki:3100/loki/api/v1/push
  # Example: file export
  file:
    path: /var/log/otel/logs.json

service:
  pipelines:
    logs:
      receivers: [otlp]
      processors: [resourcedetection, attributes, batch]
      exporters: [debug]  # Replace with real backend (Loki, Datadog, etc.)
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [debug]
```

## Configuration

### Environment Variables

| Variable | Default | Services | Description |
|----------|---------|----------|-------------|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://otel-collector:4317` | backend, embedding | OTLP gRPC endpoint |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | `grpc` | backend, embedding | OTLP protocol (grpc/http/protobuf) |
| `OTEL_SERVICE_NAME` | _(per service)_ | all | Override default service name |
| `OTEL_RESOURCE_ATTRIBUTES` | — | all | Extra resource attributes as comma-separated key=value |
| `ENVIRONMENT` | `development` | all | Deployment environment for resource attribute |
| `RUST_LOG` | `vedo_backend=debug,tower_http=debug` | backend | Tracing filter (kept for local/JSON output) |
| `LOG_LEVEL` | `debug` | embedding | Fallback log level for structlog/stdlib |

### Service-Specific Configuration

**Backend** — no separate config needed; `tracing-opentelemetry` layer shares the same `EnvFilter`.
**Embedding** — replace `LOG_LEVEL` usage with structured equivalent via structlog configuration.
**Frontend** — `VITE_OTEL_COLLECTOR_URL` env var for the OTLP HTTP endpoint path.

## Best Practices

1. **Every log record must have `service.name`.** This is the foundation for filtering across services in any centralized log system. Set via Resource, not per-record.

2. **Always pass `trace_id` and `span_id` context.** The OTel SDK automatically extracts these from the current span context. If a log is emitted within a traced request, the IDs will be present without extra work.

3. **Use structured attributes, not string interpolation.** Instead of `format!("Failed to connect: {e}")`, use `tracing::error!(error = %e, "Failed to connect")`. This preserves the error as a queryable field.

4. **Prefer Event semantics for domain operations.** Name events like `document.uploaded`, `sync.completed`, `embedding.computed`. Events are structurally searchable by name.

5. **Use semantic conventions for attribute keys.** Standard keys (`http.request.method`, `error.type`, `service.name`) ensure compatibility with OTel ecosystem tools and backends.

6. **Maintain a local stdout fallback.** When OTel collector is unavailable, services should still output structured logs (JSON) to stdout for Docker's `json-file` driver.

7. **Keep the `EnvFilter` / level control.** The OTel layer should respect the same severity filtering as the local output. Set `OTEL_LOG_LEVEL` independently only when needed.

8. **Instrument at the library/SDK bridge, not by wrapping `console.log`.** Use the OTel Logs Bridge API / LoggingHandler pattern (Python `LoggingHandler`, Rust `tracing-opentelemetry` layer) so existing log statements are automatically captured.

## Common Pitfalls

- **Blocking the main thread on OTLP export.** Always use batch processors (`BatchLogRecordProcessor`, `BatchSpanProcessor`) with async exporters. The Rust `tracing-opentelemetry` layer is async-safe when paired with `opentelemetry-otlp`'s tonic transport.

- **Missing trace context in async Python.** FastAPI async handlers lose thread-local context. Use `OpenTelemetryMiddleware` (from `opentelemetry-instrumentation-fastapi`) and ensure `structlog.contextvars.merge_contextvars` is in the processor chain.

- **Logging sensitive data in attributes.** OTel attributes are exported as-is. Never put passwords, tokens, or PII in attribute values. If needed, configure the Collector's `attributes` processor with `action: hash` or `action: redact`.

- **Too many unique attribute keys.** Each unique attribute key increases indexing/memory costs. Prefer a bounded set of well-known keys (from semantic conventions) and use `Body` for free-form details.

- **Forgetting to set `service.name`.** Without a service name, logs from all services look identical. Always set via Resource at initialization.

- **Frontend OTLP export CORS issues.** If exporting directly from the browser, the Collector must allow CORS. Prefer proxying through Caddy (`reverse_proxy /v1/logs otel-collector:4318`) instead of exposing the Collector directly.

## Version Notes

- OpenTelemetry Logs API/ SDK status: **Stable** (as of 2025)
- `tracing-opentelemetry` 0.27+ works with `opentelemetry` 0.27+
- Python `opentelemetry-exporter-otlp-proto-grpc` 1.29+ required for log export
- `@opentelemetry/sdk-logs` 0.56+ is compatible with `@opentelemetry/api-logs` 0.56+
- The Logs Bridge API (for library authors) has status **Stable**
- **Resource semantic conventions** are **Mixed** status — some attributes (e.g. Container, K8s) are still in Development
- **General logs attributes** (`log.record.original`, `log.file.name`, `log.iostream`) have status **Development** — may change
- **Event semantic conventions** have status **Development** — `event.name` field is stable but specific event conventions may evolve

## Migration Plan for This Project

### Phase 1: Instrumentation Layer (no behavioral change)

1. **Backend**: Add `tracing-opentelemetry`, `opentelemetry-otlp` to `Cargo.toml`. Initialize OTel pipeline alongside existing `tracing_subscriber::fmt()`. Log output stays JSON to stdout; OTLP export starts in parallel.
2. **Embedding**: Add `structlog`, `opentelemetry-*` to `pyproject.toml`. Replace `logging.basicConfig` with structlog configuration. Add `LoggingHandler` for OTel export.
3. **Frontend**: Add `@opentelemetry/sdk-logs` and OTLP exporter. Replace `console.*` calls with logger.emit() in all .vue/.ts files.
4. **Infrastructure**: Add `otel-collector` service to `docker-compose.yml`. Add `otel-collector-config.yaml` to project root.

### Phase 2: Context Propagation

5. Add trace context propagation across HTTP calls (backend → embedding). The `reqwest` calls from backend to embedding service should carry `traceparent` headers.
6. Instrument FastAPI handlers with `opentelemetry-instrumentation-fastapi`.

### Phase 3: Backend Integration

7. Add OTel collector exporters (Loki, file, or cloud backend).
8. Remove `LOG_LEVEL` and `RUST_LOG` env vars — consolidate under `OTEL_LOG_LEVEL`.
9. Update `docs/configuration.md` with new env vars.
10. Replace all unstructured `console.*` calls in frontend with structured OTel log records.
