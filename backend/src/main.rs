use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::FromRef,
    middleware,
    routing::{delete, get, post},
    Extension, Router,
};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::signal;
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
use vedo_backend::modules::query::repository::QueryRepository;
use vedo_backend::modules::query::{handlers as query_handlers, service::QueryService};
use vedo_backend::shared::{
    auth::{authenticate_request, SharedJwtValidator},
    llm::OpenRouterClient,
};

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

    // Ensure the database parent directory exists (for SQLite file-based URLs)
    // Strip URL scheme prefix to get the filesystem path
    // Handles:
    //   - sqlite:data/vedo.db?mode=rwc  (relative with query params)
    //   - sqlite:///data/vedo.db        (absolute path)
    //   - sqlite://data/vedo.db         (relative, legacy format — for backward compat)
    let db_path = config
        .database_url
        .strip_prefix("sqlite:///")
        .or_else(|| config.database_url.strip_prefix("sqlite://"))
        .or_else(|| config.database_url.strip_prefix("sqlite:"))
        .unwrap_or(&config.database_url);
    // Strip any query parameters (e.g., `?mode=rwc`) for directory creation
    let db_path = db_path.split('?').next().unwrap_or(db_path);
    if let Some(dir) = std::path::Path::new(db_path).parent() {
        if !dir.as_os_str().is_empty() {
            tracing::debug!("Ensuring database directory exists: {}", dir.display());
            std::fs::create_dir_all(dir).unwrap_or_else(|e| {
                tracing::error!("Failed to create database directory {}: {e}", dir.display());
                std::process::exit(1);
            });
        }
    }

    // Initialize SQLite pool
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(
                "Failed to connect to database at {}: {e}",
                config.database_url
            );
            std::process::exit(1);
        });
    tracing::info!("Database connected: {}", config.database_url);

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
    let _chroma_client = vedo_backend::shared::chroma_client::ChromaClient::new(&chroma_url);
    tracing::info!("Chroma client configured: {chroma_url}");

    // Embedding client
    let _embedding_client =
        vedo_backend::shared::embedding_client::EmbeddingClient::new(&embedding_service_url);
    tracing::info!("Embedding service configured: {embedding_service_url}");

    // LLM client
    let llm_client = OpenRouterClient::from_config(&config);

    // Repositories
    let doc_repo = DocumentRepository::new(db.clone());
    let _query_repo = QueryRepository::new(db.clone(), &chroma_url);
    let collection_repo = CollectionRepository::new(db.clone());
    let conversation_repo = ConversationRepository::new(db.clone());

    // Services
    let doc_service = DocumentService::new(doc_repo);
    let collection_service = CollectionService::new(collection_repo, chroma_url.clone());
    let conversation_service = ConversationService::new(conversation_repo);
    let query_service =
        QueryService::new(db.clone(), &chroma_url, llm_client, &embedding_service_url);

    // Auth middleware config
    let auth_config = Arc::new(config.clone());

    // JWT validator — thread-safe, shared across middleware invocations
    let jwt_validator = vedo_backend::shared::auth::JwtValidator::shared(&config);

    // Build router
    let app = Router::new()
        // Public routes
        .route("/health", get(health_check))
        // Auth routes (behind auth middleware via /api/* route_layer)
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
        .route("/api/documents/{id}", delete(documents_handlers::delete))
        // Collection routes
        .route("/api/collections", post(collections_handlers::create))
        .route("/api/collections", get(collections_handlers::list))
        .route("/api/collections/{id}", get(collections_handlers::get))
        .route(
            "/api/collections/{id}",
            delete(collections_handlers::delete),
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
            "/api/sessions/{id}",
            get(conversations_handlers::get_session),
        )
        .route(
            "/api/sessions/{id}",
            delete(conversations_handlers::delete_session),
        )
        .route(
            "/api/sessions/{id}/export",
            get(conversations_handlers::export_session),
        )
        // Auth middleware for all /api/* routes
        .route_layer(middleware::from_fn_with_state(
            auth_config.clone(),
            auth_middleware,
        ))
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
        .with_graceful_shutdown(shutdown_signal())
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

/// Auth middleware — validates Bearer token for all /api/* routes.
///
/// Supports both legacy admin API key and KeyCloak JWT tokens.
async fn auth_middleware(
    axum::extract::State(config): axum::extract::State<Arc<AppConfig>>,
    axum::extract::Extension(jwt_validator): axum::extract::Extension<SharedJwtValidator>,
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Result<axum::response::Response, axum::response::Response> {
    match authenticate_request(req.headers(), &config, Some(&jwt_validator)).await {
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
async fn shutdown_signal() {
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
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Run SQLite schema migrations.
async fn run_migrations(db: &sqlx::SqlitePool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS collections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(db)
    .await
    .expect("Failed to create collections table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS documents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            file_type TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            uploaded_at TEXT NOT NULL,
            collection_id TEXT NOT NULL,
            FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(db)
    .await
    .expect("Failed to create documents table");

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS chunks (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            "index" INTEGER NOT NULL,
            text TEXT NOT NULL,
            FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
        )"#,
    )
    .execute(db)
    .await
    .expect("Failed to create chunks table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT 'New Chat',
            collection_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE SET NULL
        )
        "#,
    )
    .execute(db)
    .await
    .expect("Failed to create sessions table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
            content TEXT NOT NULL,
            sources TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(db)
    .await
    .expect("Failed to create messages table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS git_repositories (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL,
            branch TEXT NOT NULL DEFAULT 'main',
            access_token TEXT,
            local_path TEXT NOT NULL,
            last_commit_hash TEXT,
            last_synced_at TEXT,
            collection_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'idle' CHECK(status IN ('idle','syncing','error')),
            webhook_secret TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(db)
    .await
    .expect("Failed to create git_repositories table");

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_git_repos_collection
            ON git_repositories(collection_id)
        "#,
    )
    .execute(db)
    .await
    .expect("Failed to create git_repositories index");

    tracing::info!("Git repositories table migration applied");
    tracing::info!("Database migrations completed");
}
