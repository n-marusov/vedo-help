//! Integration tests for the v0.3.1 chat-rework conversation contract.
//!
//! Covers:
//!   - `PATCH /api/sessions/:sid/messages/:mid` (edit user message)
//!   - `DELETE /api/sessions/:sid/messages/:mid` (soft-delete any message)
//!   - `GET  /api/sessions/:id/export?format={json|markdown}` (new markdown shape)
//!   - soft-delete exclusion from `load_conversation_history` used by `/api/query`
//!
//! These tests are written RED (Phase 1, executable specification). The handler
//! functions `conv::patch_message` / `conv::delete_message`, the markdown export
//! path, and the soft-delete columns do not exist yet — so each behavioral case is
//! marked `#[ignore]` until the Phase 2 backend implementation lands (T6–T8).
//! Neutering the `#[ignore]` is the green signal.
//!
//! Test preconditions (per `.ai-factory/skill-context` rule on exact preconditions):
//!   - A PostgreSQL test database is required. `common::setup_test_db` connects to
//!     `DATABASE_URL` (default `postgres://vedo:test-vedo-password@localhost:15432/vedo`)
//!     and runs all migrations, then truncates tables for a fresh state per test.
//!   - No Chroma / embedding / LLM dependency is needed for these endpoints.
//!   - Auth middleware is intentionally NOT mounted in `TestApp` so tests can hit
//!     the conversation routes directly without a real JWT, matching the existing
//!     `git_sync_integration` precedent (handler-level integration, not edge auth).
//!
//! ```bash
//! cargo test --test conversations_integration -- --ignored
//! ```

mod common;

use axum::body::{to_bytes, Bytes};
use axum::http::{Request, StatusCode};
use axum::routing::{get, patch};
use axum::Router;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use vedo_backend::modules::conversations::handlers as conv;
use vedo_backend::modules::conversations::models::{Message, Session};
use vedo_backend::modules::conversations::repository::ConversationRepository;
use vedo_backend::modules::conversations::service::ConversationService;

/// Test harness with a real PostgreSQL pool and a conversation-only router.
///
/// Auth middleware is deliberately omitted so tests exercise handlers directly.
struct TestApp {
    router: Router,
    db: PgPool,
}

/// Build the test app: fresh DB (migrations + truncate) and a minimal conversation router.
async fn spawn_app() -> TestApp {
    let db = common::setup_test_db().await;
    let svc = ConversationService::new(ConversationRepository::new(db.clone()));

    let router = Router::new()
        // Existing session routes (already implemented).
        .route(
            "/api/sessions",
            get(conv::list_sessions)
                .post(conv::create_session)
                .delete(conv::delete_all_sessions),
        )
        .route(
            "/api/sessions/:id",
            get(conv::get_session).delete(conv::delete_session),
        )
        .route("/api/sessions/:id/export", get(conv::export_session))
        // New v0.3.1 message edit/delete routes (T6 — not yet implemented => compile-time
        // reference; this is expected RED state per plan).
        .route(
            "/api/sessions/:sid/messages/:mid",
            patch(conv::patch_message).delete(conv::delete_message),
        )
        .with_state(svc);

    TestApp { router, db }
}

/// Insert a fresh session row via the repository (no HTTP roundtrip).
async fn seed_session(db: &PgPool, title: &str) -> Session {
    let repo = ConversationRepository::new(db.clone());
    let now = chrono::Utc::now();
    let session = Session {
        id: Uuid::new_v4(),
        title: title.to_string(),
        pinned: false,
        collection_id: None,
        user_id: "test-user".to_string(),
        created_at: now,
        updated_at: now,
        message_count: 0,
    };
    repo.create_session(&session).await.expect("seed session");
    session
}

/// Insert a message row via the repository.
async fn seed_message(db: &PgPool, session_id: Uuid, role: &str, content: &str) -> Message {
    let repo = ConversationRepository::new(db.clone());
    let msg = Message {
        id: Uuid::new_v4(),
        session_id,
        role: role.to_string(),
        content: content.to_string(),
        sources: None,
        created_at: chrono::Utc::now(),
        edited_at: None,
        original_content: None,
        deleted_at: None,
    };
    repo.add_message(&msg).await.expect("seed message");
    msg
}

/// Read the response body as bytes (helper; bounded to 1 MiB for tests).
async fn body_bytes(resp: axum::response::Response) -> Bytes {
    to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("read body")
}

/// PATCH a message with new content.
async fn patch_message(router: Router, sid: Uuid, mid: Uuid, content: &str) -> (StatusCode, Value) {
    let resp = router
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/sessions/{sid}/messages/{mid}"))
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_vec(&json!({ "content": content })).expect("serialize"),
                ))
                .expect("build request"),
        )
        .await
        .expect("router");
    let status = resp.status();
    let bytes = body_bytes(resp).await;
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

/// DELETE a message.
async fn delete_message(router: Router, sid: Uuid, mid: Uuid) -> StatusCode {
    let resp = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/sessions/{sid}/messages/{mid}"))
                .body(axum::body::Body::empty())
                .expect("build request"),
        )
        .await
        .expect("router");
    resp.status()
}

/// GET messages history for a session.
async fn get_messages(router: Router, sid: Uuid) -> Value {
    let resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/sessions/{sid}"))
                .body(axum::body::Body::empty())
                .expect("build request"),
        )
        .await
        .expect("router");
    let bytes = body_bytes(resp).await;
    serde_json::from_slice(&bytes).expect("json")
}

/// GET export with optional ?format= query.
async fn get_export(router: Router, sid: Uuid, format: Option<&str>) -> (StatusCode, String) {
    let uri = match format {
        Some(f) => format!("/api/sessions/{sid}/export?format={f}"),
        None => format!("/api/sessions/{sid}/export"),
    };
    let resp = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(axum::body::Body::empty())
                .expect("build request"),
        )
        .await
        .expect("router");
    let status = resp.status();
    let bytes = body_bytes(resp).await;
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

// ---------------------------------------------------------------------------
// T6/T8 target contract: message edit / soft-delete / markdown export
// ---------------------------------------------------------------------------

/// PATCH a user message → 200, content replaced, `original_content` retains the
/// prior content, `edited_at` is set.
#[tokio::test]
async fn test_patch_message_updates_content() {
    let app = spawn_app().await;
    let session = seed_session(&app.db, "Edit contract").await;
    let user_msg = seed_message(&app.db, session.id, "user", "initial question").await;

    let (status, body) =
        patch_message(app.router.clone(), session.id, user_msg.id, "updated").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["content"], "updated");
    assert_eq!(body["original_content"], "initial question");
    assert!(body["edited_at"].is_string(), "edited_at must be populated");
    assert_eq!(body["role"], "user");
}

/// DELETE a message → 204; subsequent GET excludes it; `get_message_count`
/// reflects the drop.
#[tokio::test]
async fn test_delete_message_soft_delete_then_excluded_from_history() {
    let app = spawn_app().await;
    let session = seed_session(&app.db, "Delete contract").await;
    let m1 = seed_message(&app.db, session.id, "user", "q1").await;
    let _m2 = seed_message(&app.db, session.id, "assistant", "a1").await;

    let status = delete_message(app.router.clone(), session.id, m1.id).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let body = get_messages(app.router.clone(), session.id).await;
    let messages = body["messages"].as_array().expect("messages array");
    assert_eq!(
        messages.len(),
        1,
        "soft-deleted message must be excluded from history"
    );
    assert_eq!(messages[0]["role"], "assistant");
    assert!(messages
        .iter()
        .all(|m| m["id"].as_str().unwrap() != m1.id.to_string()));
}

/// PATCH on an assistant-role message → 422.
#[tokio::test]
async fn test_cannot_edit_assistant_message() {
    let app = spawn_app().await;
    let session = seed_session(&app.db, "Reject assistant edit").await;
    let a = seed_message(&app.db, session.id, "assistant", "ai answer").await;

    let (status, _body) = patch_message(app.router.clone(), session.id, a.id, "tampered").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// After editing a user message, export reflects the new content; soft-deleted
/// messages remain excluded from export.
#[tokio::test]
async fn test_edit_user_message_reappears_in_export() {
    let app = spawn_app().await;
    let session = seed_session(&app.db, "Export after edit").await;
    let user = seed_message(&app.db, session.id, "user", "old question").await;
    let _asst = seed_message(&app.db, session.id, "assistant", "old answer").await;

    let (ok, _) = patch_message(app.router.clone(), session.id, user.id, "new question").await;
    assert_eq!(ok, StatusCode::OK);

    let (status, body) = get_export(app.router.clone(), session.id, Some("json")).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json export");
    let messages = v["messages"].as_array().expect("messages");
    assert!(messages.iter().any(|m| m["content"] == "new question"));
    assert!(messages.iter().any(|m| m["content"] == "old answer"));

    // Soft-delete the assistant message → must disappear from export.
    let _ = delete_message(app.router.clone(), session.id, _asst.id).await;
    let (_s2, body2) = get_export(app.router.clone(), session.id, Some("json")).await;
    let v2: Value = serde_json::from_str(&body2).expect("json export 2");
    let msgs2 = v2["messages"].as_array().expect("messages 2");
    assert_eq!(
        msgs2.len(),
        1,
        "soft-deleted assistant excluded from export"
    );
    assert_eq!(msgs2[0]["content"], "new question");
}

/// `?format=markdown` → `Content-Type: text/markdown`, H1 session title,
/// `## user` / `## assistant` headers per message.
#[tokio::test]
async fn test_export_markdown_format() {
    let app = spawn_app().await;
    let session = seed_session(&app.db, "Markdown Title").await;
    let _u = seed_message(&app.db, session.id, "user", "hello").await;
    let _a = seed_message(&app.db, session.id, "assistant", "world").await;

    let (status, body) = get_export(app.router.clone(), session.id, Some("markdown")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.starts_with("# Markdown Title"), "H1 session title");
    assert!(body.contains("## user"), "user message header");
    assert!(body.contains("## assistant"), "assistant message header");
    assert!(body.contains("hello"));
    assert!(body.contains("world"));
}

/// Default (no `format`) export stays JSON with the prior shape — backward compatible.
#[tokio::test]
async fn test_export_default_json_unchanged() {
    let app = spawn_app().await;
    let session = seed_session(&app.db, "Backward Compat").await;
    let _u = seed_message(&app.db, session.id, "user", "q").await;

    let (status, body) = get_export(app.router.clone(), session.id, None).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json");
    assert!(v["session"].is_object(), "session present");
    assert!(v["messages"].is_array(), "messages array present");
}

/// After soft-deleting a user message, the history handed to the LLM via
/// `load_conversation_history` excludes it (asserted at the repository level,
/// mirroring `/api/query`'s internal use).
#[tokio::test]
async fn test_conversation_history_filters_soft_deleted() {
    let app = spawn_app().await;
    let session = seed_session(&app.db, "History filtering").await;
    let u = seed_message(&app.db, session.id, "user", "kept question").await;
    let deleted = seed_message(&app.db, session.id, "user", "removed question").await;
    let _a = seed_message(&app.db, session.id, "assistant", "answer").await;

    // Soft-delete via the repository method that T6 will add.
    let repo = ConversationRepository::new(app.db.clone());
    repo.soft_delete_message(deleted.id)
        .await
        .expect("soft_delete_message (T6)");

    let live = repo.get_messages(session.id).await.expect("get_messages");
    let contents: Vec<&str> = live.iter().map(|m| m.content.as_str()).collect();
    assert!(live.iter().any(|m| m.id == u.id));
    assert!(
        contents.iter().all(|c| *c != "removed question"),
        "soft-deleted excluded"
    );
    assert_eq!(live.len(), 2);
}
