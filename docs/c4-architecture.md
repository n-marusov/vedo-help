# C4 Architecture Diagrams

> C4 model visualizations for the VEDO hub RAG Assistant.

## System Context Diagram

Shows the system as a whole, its users, and external dependencies.

```mermaid
C4Context
    title System Context — VEDO hub RAG Assistant

    Person(user, "User", "End user asking questions about technical documentation")
    Person(admin, "Admin", "System administrator managing the deployment")

    System(vedo, "VEDO hub RAG Assistant", "Personal Q&A system with RAG pipeline")

    Rel(user, vedo, "Asks questions, uploads documents, manages collections")
    Rel(admin, vedo, "Deploys, monitors, and configures the system")

    System_Ext(routerai, "RouterAI API", "LLM gateway for embeddings and chat completions")
    System_Ext(github, "GitHub", "Source control, CI/CD, container registry (GHCR)")
    System_Ext(email, "Let's Encrypt", "TLS certificate provisioning")

    Rel(vedo, routerai, "Generates embeddings and LLM responses")
    Rel(vedo, github, "CI/CD pipeline, image registry")
    Rel(user, email, "TLS certificates via Caddy")
```

## Container Diagram

Shows the high-level technical architecture — services, data stores, and communication.

```mermaid
C4Container
    title Container Diagram — VEDO hub RAG Assistant

    Person(user, "User")
    Person(admin, "Admin")

    System_Boundary(vedo, "VEDO hub RAG Assistant") {
        Container(caddy, "Caddy", "Go", "Reverse proxy with TLS termination and rate limiting")

        Container(frontend, "Frontend", "Vue 3 + TypeScript", "SPA served by nginx on port 80")
        Container(backend, "Backend", "Rust/axum", "REST API on port 3000 with SSE streaming")

        ContainerDb(chroma, "Chroma", "Vector Database", "Semantic search on document embeddings")
        ContainerDb(postgres, "PostgreSQL 16", "Relational Database", "Application and KeyCloak data")

        Container(keycloak, "KeyCloak", "Java", "OIDC/OAuth2 authentication server")

        Container(otel, "OTel Collector", "OpenTelemetry", "OTLP receiver for logs, traces, metrics")
        Container(cadvisor, "cAdvisor", "Go", "Container resource metrics exporter")
        Container(prometheus, "Prometheus", "Go", "Metrics storage (15-day retention)")
        Container(grafana, "Grafana", "Go", "Monitoring dashboards with Prometheus datasource")

        Rel(user, caddy, "HTTPS (443)")
        Rel(caddy, frontend, "Reverse proxy / to port 80")
        Rel(caddy, backend, "Reverse proxy /api/* to port 3000")
        Rel(caddy, keycloak, "Reverse proxy /auth/* to port 8080")

        Rel(frontend, backend, "API calls via Caddy")
        Rel(backend, chroma, "Vector search queries")
        Rel(backend, postgres, "Metadata and conversation storage")
        Rel(backend, keycloak, "Token validation (JWKS)")

        Rel(backend, otel, "OTLP export (traces, logs, metrics)")
        Rel(cadvisor, prometheus, "Container metrics scrape on port 8080")
        Rel(otel, prometheus, "Application metrics scrape on port 9090")
        Rel(prometheus, grafana, "Datasource queries")

        Rel(admin, caddy, "SSH tunnel for monitoring access")
        Rel(admin, grafana, "Dashboard access via tunnel")
    }

    Rel(backend, routerai, "Embeddings and LLM completions")
    Rel(vedo, github, "CI/CD and image registry")
```

## Component Diagram

Shows the internal structure of the backend application.

```mermaid
C4Component
    title Component Diagram — Backend (Rust/axum)

    Container_Boundary(backend, "Backend") {
        System_Ext(chroma_ext, "Chroma", "Vector DB")
        System_Ext(postgres_ext, "PostgreSQL", "Relational DB")
        System_Ext(routerai_ext, "RouterAI API", "LLM Gateway")
        System_Ext(keycloak_ext, "KeyCloak", "Auth Server")

        Boundary(modules, "Feature Modules") {
            Component(query, "Query Module", "Rust", "RAG pipeline: multi-query, HyDE, BM25, reranking")
            Component(documents, "Documents Module", "Rust", "Upload, parse, chunk, index")
            Component(collections, "Collections Module", "Rust", "Collection CRUD with Chroma")
            Component(conversations, "Conversations Module", "Rust", "Chat sessions and message history")
            Component(auth, "Auth Module", "Rust", "OIDC/OAuth2 token validation, user context")
            Component(git_sync, "Git Sync Module", "Rust", "Git repo clone, pull, parse, index")
        }

        Boundary(shared, "Shared Services") {
            Component(chroma_client, "Chroma Client", "Rust", "HTTP client for Chroma REST API")
            Component(embedding_client, "Embedding Client", "Rust", "RouterAI embedding API client")
            Component(llm_client, "LLM Client", "Rust", "RouterAI chat completions client")
            Component(chunking, "Chunking Service", "Rust", "Text splitting strategies")
            Component(file_val, "File Validation", "Rust", "MIME + magic bytes validation")
            Component(rate_limit, "Rate Limiter", "Rust", "Body size limiting")
            Component(error, "Error Handling", "Rust", "Unified AppError enum")
        }

        Rel(query, chroma_client, "Vector search queries")
        Rel(query, llm_client, "LLM reranking and generation")
        Rel(query, embedding_client, "Query embedding")

        Rel(documents, chunking, "Text splitting")
        Rel(documents, embedding_client, "Document embedding")
        Rel(documents, chroma_client, "Index storage")

        Rel(collections, chroma_client, "Collection management")
        Rel(conversations, postgres_ext, "Session and message storage")

        Rel(auth, keycloak_ext, "JWKS endpoint fetch")
        Rel(git_sync, documents, "File indexing pipeline")

        Rel(chroma_client, chroma_ext, "HTTP API calls")
        Rel(embedding_client, routerai_ext, "HTTP API calls")
        Rel(llm_client, routerai_ext, "HTTP API calls")
    }
```

## Deployment Diagram

Shows the physical deployment on a single VPS.

```mermaid
C4Deployment
    title Deployment Diagram — VEDO hub RAG Assistant

    Deployment_Node(vps, "VPS (Single Node)", "Ubuntu 22.04 LTS, Docker + Compose") {
        Deployment_Node(docker, "Docker Compose Platform", "Docker Engine 24+") {
            Deployment_Node(network, "internal (bridge network)", "172.x.x.x/16") {
                Container(caddy, "Caddy", "caddy:2.8-alpine", "Ports 80/443 to host")
                Container(frontend, "Frontend", "nginx", "Port 80 (internal)")
                Container(backend, "Backend", "Rust/axum", "Port 3000 (internal)")
                Container(chroma, "Chroma", "chromadb/chroma:0.6.2", "Port 8000 (internal)")
                Container(postgres, "PostgreSQL", "postgres:16-alpine", "Port 5432 (internal)")
                Container(keycloak, "KeyCloak", "quay.io/keycloak:26.1", "Port 8080 (internal)")
                Container(otel, "OTel Collector", "otel/opentelemetry-collector-contrib:0.120.0", "Ports 4317/4318/9090 (internal)")
                Container(cadvisor, "cAdvisor", "gcr.io/cadvisor/cadvisor:latest", "Port 8080 (internal)")
                Container(prometheus, "Prometheus", "prom/prometheus:latest", "Port 9090 (internal)")
                Container(grafana, "Grafana", "grafana/grafana:latest", "Port 3000 (internal)")
            }
        }

        Deployment_Node(volumes, "Persistent Volumes") {
            ContainerDb(chroma_data, "chroma_data", "Vector index files")
            ContainerDb(db_data, "db_data", "PostgreSQL data files")
            ContainerDb(prometheus_data, "prometheus_data", "Time-series metrics (15d)")
            ContainerDb(grafana_data, "grafana_data", "Dashboard configuration")
            ContainerDb(caddy_data, "caddy_data", "TLS certificates")
        }
    }

    Rel(caddy, frontend, "Reverse proxy /")
    Rel(caddy, backend, "Reverse proxy /api/*")
    Rel(caddy, keycloak, "Reverse proxy /auth/*")
    Rel(backend, chroma, "Vector search")
    Rel(backend, postgres, "SQL queries")
    Rel(backend, keycloak, "JWKS fetch")
    Rel(cadvisor, prometheus, "Metrics scrape")
    Rel(otel, prometheus, "Metrics scrape")
    Rel(prometheus, grafana, "Queries")
```

## See Also

- [Runbook](runbook.md) — operational procedures
- [Monitoring](monitoring.md) — dashboards and alerts
- [Deployment](deployment.md) — setup and configuration
- [Architecture](architecture.md) — service interaction overview
