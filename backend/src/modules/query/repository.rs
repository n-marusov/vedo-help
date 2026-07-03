use sqlx::PgPool;

use crate::shared::ChromaClient;

/// Repository for query-related data access: Chroma vector search and
/// PostgreSQL chunk / document lookups.
#[derive(Clone, Debug)]
pub struct QueryRepository {
    db: PgPool,
    chroma: ChromaClient,
}

impl QueryRepository {
    /// Create a new QueryRepository with the given database pool and Chroma URL.
    pub fn new(db: PgPool, chroma_url: &str) -> Self {
        let chroma = ChromaClient::new(chroma_url);
        tracing::debug!(component = "query/repository", "repository.initialized");
        Self { db, chroma }
    }

    /// Access the underlying PostgreSQL pool (for conversation history, etc.).
    pub fn db(&self) -> &PgPool {
        &self.db
    }

    /// Access the underlying ChromaClient.
    pub fn chroma(&self) -> &ChromaClient {
        &self.chroma
    }
}
