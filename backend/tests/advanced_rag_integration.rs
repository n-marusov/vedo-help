/// Integration tests for the advanced RAG pipeline.
///
/// Tests full pipeline with `debug: true`: verifies pipeline SSE events,
/// DebugData population, and extended SourceRef metadata.
///
/// Requires:
/// - Chroma (CHROMA_URL or default http://localhost:18000)
/// - Embedding service (EMBEDDING_SERVICE_URL or default http://localhost:18001)
/// - LLM API key (LLM_API_KEY)
/// - PostgreSQL (DATABASE_URL or test default)
///
/// Run: `cargo test --test advanced_rag_integration -- --ignored`
use std::env;
use std::time::Duration;

use futures::{pin_mut, StreamExt};
use sqlx::PgPool;
use uuid::Uuid;

use vedo_backend::modules::collections::models::Collection;
use vedo_backend::modules::collections::repository::CollectionRepository;
use vedo_backend::modules::conversations::repository::ConversationRepository;
use vedo_backend::modules::query::models::{QueryRequest, StreamEvent};
use vedo_backend::modules::query::repository::QueryRepository;
use vedo_backend::modules::query::service::QueryService;
use vedo_backend::shared::llm::LlmClient;
use vedo_backend::shared::ChromaClient;

mod common;

fn chroma_url() -> String {
    env::var("CHROMA_URL").unwrap_or_else(|_| "http://localhost:18000".to_string())
}

fn embedding_service_url() -> String {
    env::var("EMBEDDING_SERVICE_URL").unwrap_or_else(|_| "http://localhost:18001".to_string())
}

fn build_llm_client() -> Option<LlmClient> {
    let api_key = env::var("LLM_API_KEY").ok()?;
    if api_key.trim().is_empty() {
        return None;
    }
    let config = common::setup_test_config();
    Some(LlmClient::from_config(&config))
}

#[tokio::test]
#[ignore]
async fn test_advanced_rag_pipeline_with_debug() {
    let chroma_url = chroma_url();
    let embed_url = embedding_service_url();
    let llm_client = build_llm_client().expect("LLM_API_KEY must be set");

    // 1. Connect to PostgreSQL
    let config = common::setup_test_config();
    let db = PgPool::connect(&config.database_url)
        .await
        .expect("should connect to database");

    // 2. Create Chroma collection
    let chroma_client = ChromaClient::new(&chroma_url);
    let collection_id = Uuid::new_v4();
    let collection_name = collection_id.to_string();
    chroma_client
        .create_collection(&collection_name)
        .await
        .expect("should create Chroma collection");

    // 3. Create collection in SQLite
    let collection_repo = CollectionRepository::new(db.clone());
    let collection = Collection {
        id: collection_id,
        name: "advanced-rag-test".to_string(),
        description: None,
        created_at: chrono::Utc::now(),
        document_count: 0,
        user_id: "test-user".to_string(),
    };
    collection_repo
        .create_collection(&collection)
        .await
        .expect("should create collection in DB");

    // 4. Seed Chroma with test data
    chroma_client
        .add_embeddings(
            &collection_name,
            &[format!("{collection_id}-chunk-0")],
            &[vec![0.1f32; 384]],
            &[serde_json::json!({
                "text": "Rust is a systems programming language focused on safety and speed.",
                "document_id": collection_id.to_string(),
                "chunk_index": 0,
            })],
        )
        .await
        .expect("should add embeddings");

    // Wait for Chroma consistency
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 5. Setup QueryService
    let query_repo = QueryRepository::new(db.clone(), &chroma_url);
    let conversation_repo = ConversationRepository::new(db.clone());

    let query_service = QueryService::new(
        db.clone(),
        &chroma_url,
        llm_client,
        &embed_url,
        collection_repo.clone(),
        20,
        6000,
        true,                                      // advanced_rag_enabled
        5,                                         // rerank_top_k
        3,                                         // hybrid_top_k
        3,                                         // multi_query_count
        "anthropic/claude-sonnet-4.6".to_string(), // llm_rerank_model
    );

    // 6. Send query with debug=true
    let request = QueryRequest {
        collection_id,
        query: "What is Rust?".to_string(),
        session_id: None,
        debug: true,
    };

    let stream = query_service
        .process_query(request, "test-user", true)
        .await
        .expect("should process query");

    pin_mut!(stream);

    // 7. Collect events
    let mut events: Vec<StreamEvent> = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }

    // 8. Verify pipeline stages
    let pipeline_stages: Vec<&str> = events
        .iter()
        .filter(|e| e.event_type == "pipeline_stage")
        .filter_map(|e| e.data.get("stage").and_then(|s| s.as_str()))
        .collect();

    let expected_stages = [
        "expanded_questions",
        "hyde_docs",
        "keyword_matches",
        "merged_chunks",
        "reranked_chunks",
        "pipeline_metric",
    ];
    for stage in &expected_stages {
        assert!(
            pipeline_stages.contains(stage),
            "missing pipeline stage '{stage}'; found: {pipeline_stages:?}"
        );
    }

    // 9. Verify sources have stage metadata
    for event in &events {
        if event.event_type == "sources" {
            if let Some(sources) = event.data.as_array() {
                if let Some(first) = sources.first() {
                    assert!(
                        first.get("stage").is_some(),
                        "SourceRef should have stage field"
                    );
                }
            }
        }
    }

    // 10. Verify done event exists
    assert!(
        events.iter().any(|e| e.event_type == "done"),
        "should have a 'done' event"
    );

    // 11. Cleanup
    chroma_client
        .delete_collection(&collection_name)
        .await
        .expect("should clean up Chroma collection");
}
