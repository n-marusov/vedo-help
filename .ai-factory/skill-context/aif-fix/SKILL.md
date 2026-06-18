# Project-specific context for aif-fix

## Chroma collection naming constraints

When fixing collection CRUD, document indexing, or query logic that calls `ChromaClient::create_collection()`, `delete_collection()`, `add_embeddings()`, or `query()`:

- Chroma collection names must satisfy: 3-63 chars, start/end with alphanumeric, contain only `[a-zA-Z0-9_-]`, no consecutive periods, and not a valid IPv4 address.
- **Never pass user-supplied display names directly to Chroma.** User names can contain any Unicode characters (Cyrillic, CJK, accented Latin, spaces, etc.), which Chroma rejects with HTTP 400.
- Use a deterministic safe identifier instead: the collection UUID (`id.to_string()`) or a slug derived from it. The `QueryService` already passes `collection_id.to_string()` as the Chroma collection name — keep this pattern consistent across all services.
- Store the display name only in SQLite; use UUID for Chroma operations.
- Retry logic in `ChromaClient` does not distinguish HTTP 4xx (client error, unrecoverable) from 5xx (server error, retryable). A 400 from naming validation will be retried 3 times and then fail — catch naming errors early with validation or by using a safe identifier upfront.

## Docker service discovery rule

When fixing backend, embedding, frontend proxy, or Docker Compose connectivity issues, distinguish host-facing URLs from container-internal URLs.

- Inside Docker Compose, containers must call sibling services by Compose service name and container port, for example `http://chroma:8000`, `http://embedding:8001`, `http://backend:3000`, and `http://keycloak:8080`.
- Do not use `localhost` for backend-to-service calls from inside a container. In a container, `localhost` refers to that same container, not the host and not another Compose service.
- `localhost` is acceptable only for browser/public issuer URLs, host-published ports, container self-healthchecks, or code/tests that intentionally run on the host.
- For Compose-managed services, set internal URLs explicitly in the service `environment` block even if application defaults work for local host execution.
- During review, check every env var ending in `_URL`, `_URI`, `_HOST`, or proxy target for the correct network scope: browser/public, host-local, or Docker-internal.

## Regression checks

For Docker connectivity fixes, verify with:

- `docker compose config` to ensure rendered environment values are correct.
- A container-side connectivity check such as `docker compose exec backend curl -f http://chroma:8000/api/v1/heartbeat` when services are running.
- An affected endpoint smoke test after restart, for example creating a collection through `/api/collections`.
