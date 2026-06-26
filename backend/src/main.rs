use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::FromRef,
    middleware,
    routing::{delete, get, patch, post},
    Extension, Router,
};
use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tracing_subscriber::EnvFilter;

use vedo_backend::config::AppConfig;
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
use vedo_backend::shared::{
    auth::{authenticate_request, SharedJwtValidator},
    llm::LlmClient,
};

/// Redact the password from a PostgreSQL URL for safe logging.
/// `postgres://vedo:s3cret@db:5432/vedo` → `postgres://vedo:***@db:5432/vedo`
fn redacted_db_url(url: &str) -> String {
    // Simple redaction: replace password between :// and @
    if let Some(after_scheme) = url.split_once("://") {
        let scheme = after_scheme.0;
        let rest = after_scheme.1;
        if let Some((before_at, after_at)) = rest.split_once('@') {
            // before_at could be "user" or "user:password"
            if let Some((user, _password)) = before_at.split_once(':') {
                return format!("{}://{}:***@{}", scheme, user, after_at);
            }
        }
    }
    url.to_string()
}

#[tokio::main]
async fn main() {
    // Load .env file for local development (silently ignore if not found)
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.rust_log))
        .json()
        .init();

    tracing::info!("Starting VEDO hub RAG Assistant backend");

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
            "[main] connecting to PostgreSQL (attempt {}/{max_retries})",
            retries
        );
        match PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.database_url)
            .await
        {
            Ok(pool) => {
                tracing::info!(
                    "[main] database connected: postgresql://{}",
                    redacted_db_url(&config.database_url)
                );
                break pool;
            }
            Err(e) if retries < max_retries => {
                tracing::warn!(
                    "[main] PostgreSQL not ready (attempt {}/{max_retries}): {e}",
                    retries
                );
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                tracing::error!(
                    "[main] Failed to connect to database after {max_retries} retries: {e}"
                );
                std::process::exit(1);
            }
        }
    };

    // Ensure git clone root directory exists
    tracing::info!("Git clone root: {}", config.git_clone_root);
    std::fs::create_dir_all(&config.git_clone_root).unwrap_or_else(|e| {
        tracing::error!(
            "Failed to create git clone root directory {}: {e}",
            config.git_clone_root
        );
        std::process::exit(1);
    });

    // Run migrations
    run_migrations(&db).await;

    // Initialize services
    let chroma_url = config.chroma_url.clone();
    let embedding_service_url = config.embedding_service_url.clone();

    // Chroma client
    let chroma_client = vedo_backend::shared::chroma_client::ChromaClient::new(&chroma_url);
    tracing::info!("Chroma client configured: {chroma_url}");

    // Embedding client
    let embedding_client =
        vedo_backend::shared::embedding_client::EmbeddingClient::new(&embedding_service_url);
    tracing::info!("Embedding service configured: {embedding_service_url}");

    // LLM client
    let llm_client = LlmClient::from_config(&config);

    // Repositories
    let doc_repo = DocumentRepository::new(db.clone());
    let _query_repo = QueryRepository::new(db.clone(), &chroma_url);
    let collection_repo = CollectionRepository::new(db.clone());
    let conversation_repo = ConversationRepository::new(db.clone());
    let git_repo_repo = GitRepoRepository::new(db.clone());

    // Services
    let doc_service = DocumentService::with_clients(doc_repo, chroma_client, embedding_client);
    let collection_service = CollectionService::new(collection_repo, chroma_url.clone());
    let conversation_service = ConversationService::new(conversation_repo);
    let query_service = QueryService::new(
        db.clone(),
        &chroma_url,
        llm_client,
        &embedding_service_url,
        config.llm_max_history_messages,
        config.llm_context_token_budget,
    );
    let git_sync_service = GitSyncService::new(
        git_repo_repo,
        chroma_url.clone(),
        embedding_service_url.clone(),
        std::path::PathBuf::from(&config.git_clone_root),
    );

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

    // Build router
    let app = Router::new()
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
        // Admin routes (behind auth middleware)
        .route(
            "/api/admin/sessions",
            get(conversations_handlers::admin_list_sessions),
        )
        // Auth middleware for all /api/* routes (applies to routes defined above)
        .route_layer(middleware::from_fn(auth_middleware))
        // Public routes registered AFTER route_layer so auth middleware does not apply.
        .route("/health", get(health_check))
        // Webhook endpoint — public, registered AFTER route_layer so auth middleware
        // does not apply. Auth is handled via HMAC signature or webhook token.
        .route("/api/git-sync/webhook", post(git_sync_handlers::webhook))
        // JWT validator shared across middleware
        .layer(Extension(jwt_validator))
        // CORS
        .layer(CorsLayer::permissive())
        // Shared state
        .with_state(AppState {
            doc_service,
            collection_service,
            conversation_service,
            query_service,
            git_sync_service,
        });

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("Invalid host:port address");

    tracing::info!("Starting server on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Failed to bind to {addr}: {e}");
            std::process::exit(1);
        });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal_with_tx(shutdown_tx))
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Server error: {e}");
        });

    tracing::info!("Server shut down gracefully");
}

/// Shared application state accessible from all handlers.
#[derive(Clone)]
pub struct AppState {
    pub doc_service: DocumentService,
    pub collection_service: CollectionService,
    pub conversation_service: ConversationService,
    pub query_service: QueryService,
    pub git_sync_service: GitSyncService,
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
    tracing::info!("[main] running database migrations...");
    sqlx::migrate!().run(db).await.unwrap_or_else(|e| {
        tracing::error!("[main] Failed to run migrations: {e}");
        std::process::exit(1);
    });
    tracing::info!("[main] database migrations completed successfully");
}
