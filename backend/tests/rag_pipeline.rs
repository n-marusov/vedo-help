/// Integration test for the full RAG pipeline.
///
/// Tests the 7-step advanced RAG pipeline end-to-end:
/// 1. Creates a collection with documents and chunks in PostgreSQL + Chroma
/// 2. Gets real embeddings from the embedding service
/// 3. Runs QueryService.process_query with debug=true
/// 4. Verifies chunks flow through all pipeline stages via debug_data
///
/// Requires the test Docker Compose environment:
/// ```bash
/// docker compose --env-file .env.test -f docker-compose.test.yml down -v
/// docker compose --env-file .env.test -f docker-compose.test.yml up -d
/// LLM_API_KEY=test-key LLM_BASE_URL=http://localhost:18002 LLM_MODEL=mock-model \
/// EMBEDDING_SERVICE_URL=http://localhost:18081 \
/// cargo test --test rag_pipeline -- --nocapture
/// ```
use std::env;
use std::time::Duration;

use futures::StreamExt;
use sqlx::Row;
use uuid::Uuid;

use vedo_backend::config::AppConfig;
use vedo_backend::modules::collections::models::Collection;
use vedo_backend::modules::collections::repository::CollectionRepository;
use vedo_backend::modules::query::models::QueryRequest;
use vedo_backend::modules::query::service::QueryService;
use vedo_backend::shared::chroma_client::ChromaClient;
use vedo_backend::shared::embedding_client::EmbeddingClient;
use vedo_backend::shared::llm::LlmClient;

mod common;

/// Configuration for test service URLs with env var overrides.
struct TestUrls {
    chroma: String,
    embedding: String,
    llm_base: String,
    llm_api_key: String,
    llm_model: String,
}

fn test_urls() -> TestUrls {
    TestUrls {
        chroma: env::var("CHROMA_URL").unwrap_or_else(|_| "http://localhost:18000".to_string()),
        embedding: env::var("EMBEDDING_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:18081".to_string()),
        llm_base: env::var("LLM_BASE_URL").unwrap_or_else(|_| "http://localhost:18002".to_string()),
        llm_api_key: env::var("LLM_API_KEY")
            .expect("LLM_API_KEY must be set for RAG pipeline test"),
        llm_model: env::var("LLM_MODEL").unwrap_or_else(|_| "mock-model".to_string()),
    }
}

/// Check if Chroma and LLM mock services are reachable.
async fn check_services_healthy() -> Result<(), String> {
    let urls = test_urls();

    let chroma_ok = reqwest::get(&format!("{}/api/v1/heartbeat", urls.chroma))
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    if !chroma_ok {
        return Err(format!("Chroma not reachable at {}", urls.chroma));
    }

    // Embeddings are now served via RouterAI API (same gateway as LLM).
    // The LLM mock health check covers RouterAI connectivity.
    let llm_ok = reqwest::get(&format!("{}/health", urls.llm_base))
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    if !llm_ok {
        return Err(format!("LLM mock not reachable at {}", urls.llm_base));
    }

    Ok(())
}

/// Create a test collection, document, and chunks in both PostgreSQL and Chroma.
async fn create_test_data(
    pool: &sqlx::PgPool,
    chroma: &ChromaClient,
    collection_id: Uuid,
    collection_repo: &CollectionRepository,
    embedding_client: &EmbeddingClient,
    chroma_collection_name: &str,
) -> (Uuid, Vec<Uuid>) {
    let document_id = Uuid::new_v4();
    let chunk_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    let chunk_texts = vec![
        "VEDO hub is a RAG assistant that helps users find answers in technical documentation. \
         It supports multiple file formats and uses vector search for semantic retrieval.",
        "The system supports uploading PDF, Markdown, and DOCX files for indexing. \
         Documents are parsed, chunked, and embedded using a dedicated embedding service.",
        "Questions are answered using vector search and large language models with citations. \
         The RAG pipeline includes query expansion, BM25 keyword search, and LLM reranking.",
    ];

    // Create collection in Chroma
    chroma
        .create_collection(chroma_collection_name)
        .await
        .expect("should create collection in Chroma");

    // Create collection in PostgreSQL
    let collection = Collection {
        id: collection_id,
        name: "RAG Pipeline Test".to_string(),
        description: Some("Collection for RAG pipeline integration test".to_string()),
        created_at: chrono::Utc::now(),
        document_count: 0,
        user_id: "test-user".to_string(),
    };
    collection_repo
        .create_collection(&collection)
        .await
        .expect("should create collection in PG");

    // Insert document
    sqlx::query(
        "INSERT INTO documents (id, name, file_type, file_size, source, collection_id, user_id, is_active, uploaded_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(document_id)
    .bind("vedo-hub-docs.md")
    .bind("text/markdown")
    .bind(2048i64)
    .bind("upload")
    .bind(collection_id)
    .bind("test-user")
    .bind(true)
    .bind(chrono::Utc::now())
    .execute(pool)
    .await
    .expect("should insert document");

    // Insert chunks
    for (i, (chunk_id, text)) in chunk_ids.iter().zip(&chunk_texts).enumerate() {
        sqlx::query(
            "INSERT INTO chunks (id, document_id, \"index\", text, is_active) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(chunk_id)
        .bind(document_id)
        .bind(i as i32)
        .bind(text)
        .bind(true)
        .execute(pool)
        .await
        .expect("should insert chunk");
    }

    // Get real embeddings from the embedding service
    let texts: Vec<String> = chunk_texts.iter().map(|t| t.to_string()).collect();
    let embeddings = embedding_client
        .embed(texts)
        .await
        .expect("should get embeddings from embedding service");

    assert_eq!(embeddings.len(), 3, "should receive 3 embeddings");

    // Add embeddings to Chroma with metadata
    let ids: Vec<String> = chunk_ids.iter().map(|id| id.to_string()).collect();
    let metadatas: Vec<serde_json::Value> = chunk_ids
        .iter()
        .enumerate()
        .map(|(i, _chunk_id)| {
            serde_json::json!({
                "text": chunk_texts[i],
                "document_id": document_id.to_string(),
                "chunk_index": i,
                "is_active": true,
                "source": "upload",
            })
        })
        .collect();

    chroma
        .add_embeddings(chroma_collection_name, &ids, &embeddings, &metadatas)
        .await
        .expect("should add embeddings to Chroma");

    // Small delay for Chroma to index
    tokio::time::sleep(Duration::from_millis(500)).await;

    (document_id, chunk_ids)
}

/// Build an AppConfig for testing with the advanced RAG pipeline enabled.
fn make_test_config(urls: &TestUrls, advanced_rag: bool) -> AppConfig {
    AppConfig {
        database_url: String::new(),
        chroma_url: urls.chroma.clone(),
        llm_api_key: urls.llm_api_key.clone(),
        llm_base_url: urls.llm_base.clone(),
        llm_model: urls.llm_model.clone(),
        embedding_api_key: urls.llm_api_key.clone(),
        embedding_base_url: urls.llm_base.clone(),
        embedding_model: "sentence-transformers/all-minilm-l6-v2".to_string(),
        embedding_cache_size: 1000,
        host: "127.0.0.1".to_string(),
        port: 0,
        rust_log: "off".to_string(),
        frontend_url: "http://localhost:5173".to_string(),
        keycloak_url: "http://localhost:8080".to_string(),
        keycloak_jwks_url: "http://localhost:8080".to_string(),
        keycloak_realm: "vedo-hub".to_string(),
        keycloak_client_id: "vedo-backend".to_string(),
        git_clone_root: "/tmp/test-git-repos".to_string(),
        git_sync_interval_secs: 0,
        otel_endpoint: String::new(),
        service_name: "vedo-backend-test".to_string(),
        environment: "test".to_string(),
        llm_max_history_messages: 20,
        llm_context_token_budget: 6000,
        advanced_rag_enabled: advanced_rag,
        rerank_top_k: 5,
        hybrid_top_k: 20,
        multi_query_count: 3,
        llm_rerank_model: urls.llm_model.clone(),
    }
}

/// Collect all events from the SSE stream, unwrapping Infallible Results.
async fn collect_events(
    stream: impl futures::Stream<
        Item = Result<vedo_backend::modules::query::models::StreamEvent, std::convert::Infallible>,
    >,
) -> Vec<vedo_backend::modules::query::models::StreamEvent> {
    stream
        .filter_map(|r| futures::future::ready(r.ok()))
        .collect()
        .await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_rag_pipeline_full_flow() {
    // End-to-end RAG pipeline: verifies chunks are retrieved, kept through
    // reranking, and produce a grounded LLM response.
    let urls = test_urls();

    eprintln!("=== RAG Pipeline Integration Test ===");
    eprintln!("Chroma:      {}", urls.chroma);
    eprintln!("Embedding:   {}", urls.embedding);
    eprintln!("LLM mock:    {}", urls.llm_base);
    eprintln!("LLM model:   {}", urls.llm_model);

    if let Err(msg) = check_services_healthy().await {
        eprintln!("SKIP: {msg}");
        eprintln!("      Ensure test Docker Compose is running:");
        eprintln!("      docker compose --env-file .env.test -f docker-compose.test.yml up -d");
        return;
    }

    let pool = common::setup_test_db().await;
    let chroma = ChromaClient::new(&urls.chroma);
    let config = make_test_config(&urls, true);
    let embedding_client = EmbeddingClient::from_config(&config);
    let collection_repo = CollectionRepository::new(pool.clone());

    let collection_id = Uuid::new_v4();
    let chroma_collection_name = collection_id.to_string();

    create_test_data(
        &pool,
        &chroma,
        collection_id,
        &collection_repo,
        &embedding_client,
        &chroma_collection_name,
    )
    .await;

    // Set up QueryService
    let llm_client = LlmClient::from_config(&config);

    let query_service = QueryService::new(
        pool.clone(),
        &urls.chroma,
        llm_client,
        embedding_client,
        collection_repo,
        20,
        6000,
        config,
        None,
    );

    // Execute query
    let request = QueryRequest {
        collection_id,
        query: "What is VEDO hub RAG assistant?".to_string(),
        session_id: None,
        debug: true,
    };

    eprintln!("\n=== Executing RAG query ===");
    let stream = query_service
        .process_query(request, "test-user", true)
        .await
        .expect("process_query should succeed");

    let events = collect_events(stream).await;

    assert!(!events.is_empty(), "should receive at least one event");

    // Verify event sequence
    let chunk_count = events.iter().filter(|e| e.event_type == "chunk").count();
    let sources_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "sources")
        .collect();
    let done_count = events.iter().filter(|e| e.event_type == "done").count();

    assert!(
        chunk_count > 0,
        "should have at least one chunk event — LLM responded"
    );
    assert_eq!(
        sources_events.len(),
        1,
        "should have exactly one sources event"
    );
    assert_eq!(done_count, 1, "should have exactly one done event");

    // Verify sources contain results
    let sources_event = &sources_events[0];
    let sources_array = sources_event.data["sources"]
        .as_array()
        .expect("sources event should contain a sources array");

    assert!(
        !sources_array.is_empty(),
        "sources should NOT be empty — chunks should have been retrieved and kept through reranking"
    );

    eprintln!(
        "\n=== Results: {} chunks retrieved, {} sources ===",
        chunk_count,
        sources_array.len()
    );
    for source in sources_array {
        eprintln!(
            "  Source: document='{}', relevance={:.4}, chunk_index={}",
            source["document_name"].as_str().unwrap_or("?"),
            source["relevance"].as_f64().unwrap_or(0.0),
            source["chunk_index"].as_i64().unwrap_or(-1),
        );
    }

    // Verify LLM response text
    let full_response: String = events
        .iter()
        .filter(|e| e.event_type == "chunk")
        .filter_map(|e| e.data["text"].as_str())
        .collect();
    eprintln!("\n=== LLM response: ===");
    eprintln!("{}", &full_response);
    assert!(
        !full_response.is_empty(),
        "LLM response should not be empty"
    );

    // Cleanup
    eprintln!("\n=== Cleaning up ===");
    let _ = chroma.delete_collection(&chroma_collection_name).await;
    eprintln!("=== Test complete ===");
}

#[tokio::test]
async fn test_rag_pipeline_debug_data_flow() {
    // Verify chunk counts through each pipeline stage using debug_data.
    // This is THE critical test for the user's problem — it shows exactly
    // where chunks are lost in the pipeline.
    let urls = test_urls();

    if let Err(msg) = check_services_healthy().await {
        eprintln!("SKIP: {msg}");
        return;
    }

    let pool = common::setup_test_db().await;
    let chroma = ChromaClient::new(&urls.chroma);
    let config = make_test_config(&urls, true);
    let embedding_client = EmbeddingClient::from_config(&config);
    let collection_repo = CollectionRepository::new(pool.clone());

    let collection_id = Uuid::new_v4();
    let chroma_collection_name = collection_id.to_string();

    create_test_data(
        &pool,
        &chroma,
        collection_id,
        &collection_repo,
        &embedding_client,
        &chroma_collection_name,
    )
    .await;

    let llm_client = LlmClient::from_config(&config);

    let query_service = QueryService::new(
        pool.clone(),
        &urls.chroma,
        llm_client,
        embedding_client,
        collection_repo,
        20,
        6000,
        config,
        None,
    );

    // Insert a session so FK constraint on messages is satisfied
    let session_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO sessions (id, title, collection_id, created_at, updated_at, user_id) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(session_id)
    .bind("Test Session")
    .bind(collection_id)
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .bind("test-user")
    .execute(&pool)
    .await
    .expect("should insert test session");

    // Query with session so debug data is persisted
    let request = QueryRequest {
        collection_id,
        query: "What is VEDO hub?".to_string(),
        session_id: Some(session_id),
        debug: true,
    };

    let stream = query_service
        .process_query(request, "test-user", true)
        .await
        .expect("process_query should succeed");

    let events = collect_events(stream).await;

    // Find the done event which contains message IDs
    let done_event = events
        .iter()
        .find(|e| e.event_type == "done")
        .expect("should have done event");

    let assistant_message_id = done_event.data["assistant_message_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok());

    // Fetch debug data from persisted message
    if let Some(asst_id) = assistant_message_id {
        let row = sqlx::query("SELECT debug_data FROM messages WHERE id = $1")
            .bind(asst_id)
            .fetch_optional(&pool)
            .await
            .expect("should fetch assistant message");

        if let Some(row) = row {
            let debug_json_str: String = row.get("debug_data");
            let debug_data: serde_json::Value =
                serde_json::from_str(&debug_json_str).expect("debug_data should be valid JSON");

            eprintln!("\n=== Pipeline Debug Data ===");
            eprintln!("Query text: {}", debug_data["query_text"]);

            // Multi-Query step
            if let Some(mq) = debug_data["multi_query"].as_object() {
                let variants = mq["variants"].as_array().map(|a| a.len()).unwrap_or(0);
                let latency = mq["latency_ms"].as_u64().unwrap_or(0);
                eprintln!("Multi-Query: {} variants in {}ms", variants, latency);
                assert!(variants > 0, "Multi-Query should generate variants");
            }

            // HyDE step
            if let Some(hyde) = debug_data["hyde"].as_object() {
                let per_query = hyde["per_query"].as_array().map(|a| a.len()).unwrap_or(0);
                eprintln!("HyDE: {} hypothetical documents", per_query);
                assert!(per_query > 0, "HyDE should generate hypothetical documents");
            }

            // Embedding search step
            if let Some(es) = debug_data["embedding_search"].as_object() {
                let result_count = es["result_count"].as_u64().unwrap_or(0);
                let latency = es["latency_ms"].as_u64().unwrap_or(0);
                eprintln!(
                    "Embedding search: {} results in {}ms",
                    result_count, latency
                );
                assert!(
                    result_count > 0,
                    "Embedding search should return results for a matching query"
                );
            }

            // Keyword search step
            if let Some(ks) = debug_data["keyword_search"].as_object() {
                let total = ks["total_matches"].as_u64().unwrap_or(0);
                let latency = ks["latency_ms"].as_u64().unwrap_or(0);
                eprintln!("Keyword search (BM25): {} results in {}ms", total, latency);
            }

            // Merge & Dedup step
            if let Some(md) = debug_data["merge_dedup"].as_object() {
                let input = md["input_chunks"].as_u64().unwrap_or(0);
                let after = md["after_dedup"].as_u64().unwrap_or(0);
                let vector = md["source_breakdown"]["vector_chunks"]
                    .as_u64()
                    .unwrap_or(0);
                let keyword = md["source_breakdown"]["keyword_chunks"]
                    .as_u64()
                    .unwrap_or(0);
                eprintln!(
                    "Merge & Dedup: {} input (vector={}, keyword={}) -> {} after dedup",
                    input, vector, keyword, after
                );
                assert!(after <= input, "after_dedup should be <= input_chunks");
                assert!(
                    vector > 0 || keyword > 0,
                    "at least one search method should return results"
                );
            }

            // Reranking step (THE CRITICAL ONE — this is where chunks may be lost)
            if let Some(rr) = debug_data["reranking"].as_object() {
                let input = rr["input_count"].as_u64().unwrap_or(0);
                let accepted = rr["accepted"].as_u64().unwrap_or(0);
                let rejected = rr["rejected"].as_u64().unwrap_or(0);
                eprintln!(
                    "Reranking: {} input -> {} accepted, {} rejected",
                    input, accepted, rejected
                );
                assert!(
                    accepted > 0,
                    "Reranking should accept at least some chunks — if accepted=0, \
                     the LLM is rejecting all chunks (the root cause of 'no information')"
                );
                if accepted == 0 {
                    if let Some(results) = rr["results"].as_array() {
                        for r in results.iter().take(5) {
                            eprintln!(
                                "  chunk={}: verdict='{}', comment='{}'",
                                r["chunk_id"].as_str().unwrap_or("?"),
                                r["verdict"].as_str().unwrap_or("?"),
                                r["comment"].as_str().unwrap_or("?"),
                            );
                        }
                    }
                }
            }

            // Final answer step
            if let Some(fa) = debug_data["final_answer"].as_object() {
                let chunks = fa["chunks_in_context"].as_u64().unwrap_or(0);
                let latency = fa["latency_ms"].as_u64().unwrap_or(0);
                let model = fa["model"].as_str().unwrap_or("?");
                eprintln!(
                    "Final answer: {} chunks in context, {}ms, model={}",
                    chunks, latency, model
                );
                assert!(
                    chunks > 0,
                    "chunks_in_context should be > 0 — the LLM needs context"
                );
            }

            eprintln!("=== Debug data verified successfully ===");
        } else {
            eprintln!("WARN: assistant message not found in DB");
        }
    } else {
        eprintln!("WARN: assistant_message_id is null");
    }

    // Cleanup
    let _ = chroma.delete_collection(&chroma_collection_name).await;
}

#[tokio::test]
async fn test_rag_pipeline_advanced_disabled() {
    // Verify the standard (non-advanced) pipeline works as fallback.
    let urls = test_urls();

    if let Err(msg) = check_services_healthy().await {
        eprintln!("SKIP: {msg}");
        return;
    }

    let pool = common::setup_test_db().await;
    let chroma = ChromaClient::new(&urls.chroma);
    // Disable advanced RAG
    let config = make_test_config(&urls, false);
    let embedding_client = EmbeddingClient::from_config(&config);
    let collection_repo = CollectionRepository::new(pool.clone());

    let collection_id = Uuid::new_v4();
    let chroma_collection_name = collection_id.to_string();

    create_test_data(
        &pool,
        &chroma,
        collection_id,
        &collection_repo,
        &embedding_client,
        &chroma_collection_name,
    )
    .await;

    let llm_client = LlmClient::from_config(&config);

    let query_service = QueryService::new(
        pool.clone(),
        &urls.chroma,
        llm_client,
        embedding_client,
        collection_repo,
        20,
        6000,
        config,
        None,
    );

    let request = QueryRequest {
        collection_id,
        query: "What is VEDO hub?".to_string(),
        session_id: None,
        debug: true,
    };

    let stream = query_service
        .process_query(request, "test-user", true)
        .await
        .expect("process_query with advanced_rag_enabled=false should succeed");

    let events = collect_events(stream).await;

    let chunk_count = events.iter().filter(|e| e.event_type == "chunk").count();
    let sources_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "sources")
        .collect();

    assert!(
        chunk_count > 0,
        "standard pipeline should also produce LLM response chunks"
    );

    if let Some(sources) = sources_events.first() {
        let sources_array = sources.data["sources"]
            .as_array()
            .expect("sources should be an array");
        assert!(
            !sources_array.is_empty(),
            "standard pipeline should retrieve chunks too"
        );
        eprintln!(
            "Standard pipeline: {} sources retrieved",
            sources_array.len()
        );
    }

    let _ = chroma.delete_collection(&chroma_collection_name).await;
    eprintln!("=== Standard pipeline test complete ===");
}
