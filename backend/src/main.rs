use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, patch, post, put},
    Extension, Json, Router,
};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

use vedo_backend::config::AppConfig;
use vedo_backend::modules::audit::handlers as audit_handlers;
use vedo_backend::modules::audit::repository::AuditRepository;
use vedo_backend::modules::audit::service::AuditService;
use vedo_backend::modules::auth::handlers as auth_handlers;
use vedo_backend::modules::collections::repository::CollectionRepository;
use vedo_backend::modules::collections::{
    handlers as collections_handlers, service::CollectionService,
};
use vedo_backend::modules::conversations::repository::ConversationRepository;
use vedo_backend::modules::conversations::{
    handlers as conversations_handlers, service::ConversationService,
};
use vedo_backend::modules::documents::repository::DocumentRepository;
use vedo_backend::modules::documents::{handlers as documents_handlers, service::DocumentService};
use vedo_backend::modules::git_sync::repository::GitRepoRepository;
use vedo_backend::modules::git_sync::{handlers as git_sync_handlers, service::GitSyncService};
use vedo_backend::modules::query::repository::QueryRepository;
use vedo_backend::modules::query::{handlers as query_handlers, service::QueryService};
use vedo_backend::modules::settings::{
    handlers as settings_handlers, repository::SettingsRepository, service::SettingsService,
};
use vedo_backend::shared::{
    audit_middleware,
    auth::{authenticate_request, SharedJwtValidator},
    error::AppError,
    health::{HealthProbe, HealthReport, HealthService, HealthStatus},
    llm::LlmClient,
    pricing::PricingCache,
    rbac,
};

/// Initialize OpenTelemetry tracing pipeline with dual output:
/// 1. JSON-formatted stdout (for Docker journald/logging driver)
/// 2. OTLP gRPC export to OTel Collector
///
/// Returns the `TracerProvider` handle — keeping it alive until shutdown
/// ensures all spans are flushed before the process exits.
fn init_telemetry(config: &AppConfig) -> TracerProvider {
    let resource = Resource::new(vec![
        KeyValue::new("service.name", "vedo-backend"),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("deployment.environment", config.environment.clone()),
    ]);

    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&config.otel_endpoint)
        .build()
        .expect("Failed to build OTLP span exporter");

    let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(otlp_exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(resource)
        .build();

    let telemetry_layer =
        tracing_opentelemetry::layer().with_tracer(tracer_provider.tracer("vedo-backend"));

    // Build a registry that writes JSON to stdout AND exports via OTLP
    let subscriber = Registry::default()
        .with(EnvFilter::new(&config.rust_log))
        .with(tracing_subscriber::fmt::layer().json())
        .with(telemetry_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");

    tracing::info!(
        component = "main",
        otel_endpoint = %config.otel_endpoint,
        "telemetry.initialized"
    );

    tracer_provider
}

#[tokio::main]
async fn main() {
    // Load .env file for local development (silently ignore if not found)
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env();

    // Initialize telemetry (tracing + OTel)
    let _tracer_provider = init_telemetry(&config);

    tracing::info!(
        component = "main",
        service_name = %config.service_name,
        environment = %config.environment,
        otel_endpoint = %config.otel_endpoint,
        "server.starting",
    );

    // Connect to PostgreSQL with retry loop (database container may not be ready immediately in Docker).
    // Override retry count via DB_CONNECT_RETRIES env var (default: 30, 1 second between attempts).
    let max_retries: u32 = std::env::var("DB_CONNECT_RETRIES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let mut retries = 0;
    let db = loop {
        retries += 1;
        tracing::info!(
            component = "main",
            retry_attempt = retries,
            max_retries = max_retries,
            "db.connect.retry"
        );
        match PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.database_url)
            .await
        {
            Ok(pool) => {
                tracing::info!(component = "main", "db.connected");
                break pool;
            }
            Err(e) if retries < max_retries => {
                tracing::warn!(
                    component = "main",
                    error = %e,
                    retry_attempt = retries,
                    max_retries = max_retries,
                    "db.connect.retry_waiting"
                );
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                tracing::error!(
                    component = "main",
                    error = %e,
                    max_retries = max_retries,
                    "db.connect.exhausted"
                );
                std::process::exit(1);
            }
        }
    };

    // Ensure git clone root directory exists
    tracing::info!(
        component = "main",
        git_clone_root = %config.git_clone_root,
        "git_clone_root.configured"
    );
    std::fs::create_dir_all(&config.git_clone_root).unwrap_or_else(|e| {
        tracing::error!(
            component = "main",
            error = %e,
            git_clone_root = %config.git_clone_root,
            "git_clone_root.create_failed"
        );
        std::process::exit(1);
    });

    // Run migrations
    run_migrations(&db).await;

    // Initialize services
    let chroma_url = config.chroma_url.clone();

    // Chroma client
    let chroma_client = vedo_backend::shared::chroma_client::ChromaClient::new(&chroma_url);
    tracing::info!(
        component = "main",
        chroma_url = %chroma_url,
        "chroma_client.configured"
    );

    // Embedding client
    let embedding_client =
        vedo_backend::shared::embedding_client::EmbeddingClient::from_config(&config);
    tracing::info!(
        component = "main",
        embedding_model = %config.embedding_model,
        embedding_url = %config.embedding_base_url,
        "embedding_client.configured"
    );

    // Startup retry loops for downstream dependencies.
    // Non-fatal — the app continues in degraded mode if they are unavailable.
    let chroma_connect_retries: u32 = std::env::var("CHROMA_CONNECT_RETRIES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    // Embedding startup retry loop removed: embeddings are now served via RouterAI API,
    // which is covered by the LLM health probe since they share the same gateway.
    tracing::info!(
        component = "main",
        max_retries = chroma_connect_retries,
        "chroma.connect.start"
    );
    for attempt in 1..=chroma_connect_retries {
        match chroma_client.health().await {
            Ok(()) => {
                tracing::info!(component = "main", attempt = attempt, "chroma.connected");
                break;
            }
            Err(e) if attempt < chroma_connect_retries => {
                tracing::warn!(
                    component = "main",
                    attempt = attempt,
                    max_retries = chroma_connect_retries,
                    error = %e,
                    "chroma.connect.retry"
                );
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                tracing::warn!(
                    component = "main",
                    attempt = attempt,
                    max_retries = chroma_connect_retries,
                    error = %e,
                    "chroma.connect.exhausted"
                );
            }
        }
    }

    // LLM client
    let llm_client = LlmClient::from_config(&config);

    // Health service — register downstream dependency probes
    let mut health_service = HealthService::new();
    health_service.register(chroma_client.clone());
    health_service.register(embedding_client.clone());
    health_service.register(llm_client.clone());

    // Pricing cache — fetches model prices from RouterAI in the background.
    // First refresh happens after 3 seconds (non-blocking startup), then every 15 minutes.
    let pricing_cache = PricingCache::new(&config.llm_base_url);
    pricing_cache.clone().start_background_refresh(
        std::time::Duration::from_secs(3),
        std::time::Duration::from_secs(900),
    );
    tracing::info!(component = "main", "pricing_cache.initialized");

    // Wrap DB pool as a HealthProbe
    struct DbProbe {
        db: sqlx::PgPool,
    }

    #[async_trait::async_trait]
    impl HealthProbe for DbProbe {
        fn name(&self) -> &'static str {
            "PostgreSQL"
        }

        async fn probe(&self) -> Result<(), AppError> {
            tracing::debug!(component = "health", "db.probe_start");
            sqlx::query("SELECT 1")
                .execute(&self.db)
                .await
                .map(|_| {
                    tracing::debug!(component = "health", "db.probe_ok");
                })
                .map_err(|e| {
                    AppError::InternalError(format!("PostgreSQL health check failed: {e}"))
                })
        }
    }

    health_service.register(DbProbe { db: db.clone() });

    tracing::info!(
        component = "main",
        probe_count = 4,
        "health_service.configured"
    );

    // Repositories
    let doc_repo = DocumentRepository::new(db.clone());
    let _query_repo = QueryRepository::new(db.clone(), &chroma_url);
    let collection_repo = CollectionRepository::new(db.clone());
    let conversation_repo = ConversationRepository::new(db.clone());
    let git_repo_repo = GitRepoRepository::new(db.clone());
    let audit_repo = AuditRepository::new(db.clone());
    let settings_repo = SettingsRepository::new(db.clone());

    let settings_service =
        SettingsService::new(settings_repo, config.clone(), embedding_client.clone());

    // Services
    let doc_service = DocumentService::with_clients(
        doc_repo.clone(),
        collection_repo.clone(),
        chroma_client,
        embedding_client.clone(),
        Some(settings_service.clone()),
    );
    let collection_service = CollectionService::new(
        collection_repo.clone(),
        chroma_url.clone(),
        embedding_client.clone(),
        Some(settings_service.clone()),
    );
    let conversation_service = ConversationService::new(conversation_repo);
    let query_service = QueryService::new(
        db.clone(),
        &chroma_url,
        llm_client,
        embedding_client.clone(),
        collection_repo.clone(),
        config.llm_max_history_messages,
        config.llm_context_token_budget,
        config.clone(),
        Some(settings_service.clone()),
    );
    let audit_service = AuditService::new(audit_repo);

    let git_repo_repo_for_locks = git_repo_repo.clone();
    let git_sync_service = GitSyncService::new(
        git_repo_repo,
        doc_repo.clone(),
        chroma_url.clone(),
        embedding_client,
        std::path::PathBuf::from(&config.git_clone_root),
        Some(settings_service.clone()),
    );

    // Reset stale sync locks left from a previous crash or restart.
    // Must happen before the scheduler or any handler can start new syncs.
    if let Err(e) = git_repo_repo_for_locks.reset_stale_sync_locks().await {
        tracing::error!(
            component = "main",
            error = %e,
            "reset_stale_sync_locks.failed"
        );
    }

    // Create broadcast channel for shutdown signal
    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

    // Start the polling scheduler background task
    let scheduler_svc = Arc::new(git_sync_service.clone());
    tokio::spawn(async move {
        scheduler_svc
            .start_scheduler(config.git_sync_interval_secs, shutdown_rx)
            .await;
    });

    // JWT validator — thread-safe, shared across middleware invocations
    let jwt_validator = vedo_backend::shared::auth::JwtValidator::shared(&config);

    // Admin sub-router — all endpoints require the "admin" realm role.
    // Must be nested into the main /api router BEFORE the auth route_layer
    // so auth middleware covers it, and placed outside the public-route
    // guard so admin endpoints are not exposed without auth.
    let admin_router = Router::new()
        .route(
            "/api/admin/collections",
            get(collections_handlers::admin_list),
        )
        .route(
            "/api/admin/collections/:id",
            delete(collections_handlers::admin_delete),
        )
        .route("/api/admin/audit-log", get(audit_handlers::list_audit_log))
        .route(
            "/api/admin/sessions/users",
            get(conversations_handlers::admin_list_session_users),
        )
        .route(
            "/api/admin/sessions",
            get(conversations_handlers::admin_list_sessions),
        )
        .route("/api/admin/models", get(settings_handlers::get_models))
        .route("/api/admin/settings", get(settings_handlers::get_settings))
        .route(
            "/api/admin/settings",
            put(settings_handlers::update_settings),
        );

    // Build router
    let app = Router::new()
        // Admin routes (behind auth + admin role middleware via route_layer)
        .merge(admin_router.route_layer(middleware::from_fn_with_state(
            "admin".to_string(),
            rbac::require_role,
        )))
        // Auth routes (behind auth middleware via route_layer)
        .route("/api/auth/me", get(auth_handlers::me))
        .route("/api/auth/logout", post(auth_handlers::logout))
        // Document routes
        .route("/api/documents/upload", post(documents_handlers::upload))
        .route(
            "/api/documents/upload-zip",
            post(documents_handlers::upload_zip)
                .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024)),
        )
        .route("/api/documents", get(documents_handlers::list))
        .route(
            "/api/documents/batch",
            delete(documents_handlers::delete_batch),
        )
        .route("/api/documents/:id", delete(documents_handlers::delete))
        .route(
            "/api/documents/reload/:id",
            post(documents_handlers::reload),
        )
        // Collection routes
        .route("/api/collections", post(collections_handlers::create))
        .route("/api/collections", get(collections_handlers::list))
        .route("/api/collections/:id", get(collections_handlers::get))
        .route("/api/collections/:id", delete(collections_handlers::delete))
        .route(
            "/api/collections/:id/stats",
            get(collections_handlers::get_collection_stats),
        )
        .route(
            "/api/collections/:id/chunks",
            get(collections_handlers::search_collection_chunks),
        )
        // Git sync routes
        .route("/api/git-sync/repos", post(git_sync_handlers::create_repo))
        .route("/api/git-sync/repos", get(git_sync_handlers::list_repos))
        .route("/api/git-sync/repos/:id", get(git_sync_handlers::get_repo))
        .route(
            "/api/git-sync/repos/:id/sync",
            post(git_sync_handlers::trigger_sync),
        )
        .route(
            "/api/git-sync/repos/:id/status",
            get(git_sync_handlers::get_sync_status),
        )
        .route(
            "/api/git-sync/repos/:id",
            delete(git_sync_handlers::delete_repo),
        )
        // Query routes
        .route("/api/query", post(query_handlers::query_handler))
        // Session routes
        .route("/api/sessions", get(conversations_handlers::list_sessions))
        .route(
            "/api/sessions",
            post(conversations_handlers::create_session),
        )
        .route(
            "/api/sessions",
            delete(conversations_handlers::delete_all_sessions),
        )
        .route(
            "/api/sessions/:id",
            get(conversations_handlers::get_session).patch(conversations_handlers::patch_session),
        )
        .route(
            "/api/sessions/:id",
            delete(conversations_handlers::delete_session),
        )
        .route(
            "/api/sessions/:id/export",
            get(conversations_handlers::export_session),
        )
        // Message edit/delete routes (v0.3.1)
        .route(
            "/api/sessions/:sid/messages/:mid",
            patch(conversations_handlers::patch_message)
                .delete(conversations_handlers::delete_message),
        )
        // Audit middleware for all /api/* routes (inner — runs after auth).
        .route_layer(middleware::from_fn(audit_middleware::audit_middleware))
        // Auth middleware for all /api/* routes (outer — runs first, inserts AuthInfo).
        .route_layer(middleware::from_fn(auth_middleware))
        // Public routes registered AFTER route_layer so auth middleware does not apply.
        .route("/health", get(health_check))
        .route("/api/health/deep", get(deep_health_check))
        // Webhook endpoint — public, registered AFTER route_layer so auth middleware
        // does not apply. Auth is handled via HMAC signature or webhook token.
        .route("/api/git-sync/webhook", post(git_sync_handlers::webhook))
        // JWT validator shared across middleware
        .layer(Extension(jwt_validator))
        // Audit service for middleware
        .layer(Extension(audit_service.clone()))
        // Security headers — outer layer covering all routes
        .layer(middleware::from_fn_with_state(
            config.environment.clone(),
            vedo_backend::shared::security_headers::middleware,
        ))
        // CORS — explicit origin from config
        .layer(
            CorsLayer::new()
                .allow_origin(
                    config
                        .frontend_url
                        .parse::<axum::http::HeaderValue>()
                        .expect("Invalid frontend_url for CORS origin"),
                )
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::PATCH,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::HeaderName::from_static("x-requested-with"),
                ])
                .allow_credentials(true)
                .max_age(std::time::Duration::from_secs(86400)),
        )
        // Shared state
        .with_state(AppState {
            doc_service,
            collection_service,
            conversation_service,
            query_service,
            git_sync_service,
            audit_service,
            settings_service,
            health_service,
            pricing_cache,
        });

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("Invalid host:port address");

    tracing::info!(component = "main", addr = %addr, "server.starting");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(component = "main", error = %e, addr = %addr, "server.bind_failed");
            std::process::exit(1);
        });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal_with_tx(shutdown_tx))
        .await
        .unwrap_or_else(|e| {
            tracing::error!(component = "main", error = %e, "server.error");
        });

    tracing::info!(component = "main", "server.shutdown");
}

/// Shared application state accessible from all handlers.
#[derive(Clone)]
pub struct AppState {
    pub doc_service: DocumentService,
    pub collection_service: CollectionService,
    pub conversation_service: ConversationService,
    pub query_service: QueryService,
    pub git_sync_service: GitSyncService,
    pub audit_service: AuditService,
    pub settings_service: SettingsService,
    pub health_service: HealthService,
    pub pricing_cache: PricingCache,
}

impl FromRef<AppState> for DocumentService {
    fn from_ref(state: &AppState) -> Self {
        state.doc_service.clone()
    }
}

impl FromRef<AppState> for CollectionService {
    fn from_ref(state: &AppState) -> Self {
        state.collection_service.clone()
    }
}

impl FromRef<AppState> for ConversationService {
    fn from_ref(state: &AppState) -> Self {
        state.conversation_service.clone()
    }
}

impl FromRef<AppState> for QueryService {
    fn from_ref(state: &AppState) -> Self {
        state.query_service.clone()
    }
}

impl FromRef<AppState> for GitSyncService {
    fn from_ref(state: &AppState) -> Self {
        state.git_sync_service.clone()
    }
}

impl FromRef<AppState> for AuditService {
    fn from_ref(state: &AppState) -> Self {
        state.audit_service.clone()
    }
}

impl FromRef<AppState> for SettingsService {
    fn from_ref(state: &AppState) -> Self {
        state.settings_service.clone()
    }
}

impl FromRef<AppState> for HealthService {
    fn from_ref(state: &AppState) -> Self {
        state.health_service.clone()
    }
}

impl FromRef<AppState> for PricingCache {
    fn from_ref(state: &AppState) -> Self {
        state.pricing_cache.clone()
    }
}

/// Auth middleware — validates Bearer JWT token for all /api/* routes.
async fn auth_middleware(
    axum::extract::Extension(jwt_validator): axum::extract::Extension<SharedJwtValidator>,
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Result<axum::response::Response, axum::response::Response> {
    match authenticate_request(req.headers(), Some(&jwt_validator)).await {
        Ok(auth_info) => {
            // Store auth info in request extensions for downstream handlers.
            let mut req = req;
            req.extensions_mut().insert(auth_info);
            Ok(next.run(req).await)
        }
        Err(response) => Err(response),
    }
}

/// Health check endpoint — returns 200 OK.
async fn health_check() -> &'static str {
    "OK"
}

/// Deep health check endpoint — probes all downstream dependencies.
///
/// Public (registered after `route_layer`), no auth required.
async fn deep_health_check(
    State(health_service): State<HealthService>,
) -> (StatusCode, Json<HealthReport>) {
    tracing::debug!(component = "health", "deep_health_check.request");

    let report = health_service.check_all().await;
    let status = match report.status {
        HealthStatus::Healthy | HealthStatus::Degraded => StatusCode::OK,
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    tracing::info!(
        component = "health",
        status = %report.status,
        checks_total = report.checks.len(),
        checks_unhealthy = report.checks.iter().filter(|c| matches!(c.status, vedo_backend::shared::health::CheckStatus::Unhealthy)).count(),
        "deep_health_check.response"
    );

    (status, Json(report))
}

/// Wait for SIGINT or SIGTERM to initiate graceful shutdown.
/// Sends a signal on the broadcast channel to notify background tasks.
async fn shutdown_signal_with_tx(tx: broadcast::Sender<()>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            let _ = tx.send(());
        },
        _ = terminate => {
            let _ = tx.send(());
        },
    }
}

/// Run PostgreSQL schema migrations using sqlx::migrate!().
async fn run_migrations(db: &sqlx::PgPool) {
    tracing::info!(component = "main", "db.migrations.start");
    sqlx::migrate!().run(db).await.unwrap_or_else(|e| {
        tracing::error!(component = "main", error = %e, "db.migrations.failed");
        std::process::exit(1);
    });
    tracing::info!(component = "main", "db.migrations.complete");
}
