//! Unit / db-round-trip red tests for the v0.3.1 chat-rework (T3).
//!
//! These cover the repository soft-delete/edit round-trip, the service markdown
//! export, and the pure context-window trimming logic. They are RED
//! (executable specification): the repository methods `update_message` /
//! `soft_delete_message` / `get_message` / public `get_message_count`, the
//! `build_markdown_export` service method, and the context-window functions
//! `count_tokens` / `trim_history` do not exist yet (T6–T8). Each case is
//! `#[ignore]` until the matching Phase 2 implementation lands; flipping the
//! `#[ignore]` is the green signal.
//!
//! This binary is intentionally separate from `conversations_integration.rs`
//! (HTTP contract) and is not built by the project's CI `cargo test --lib`
//! / `cargo test --test integration` gates, so the red phase does not break CI.
//!
//! Preconditions (per skill-context rule):
//!   - Repository/service tests require the PostgreSQL test database
//!     (`common::setup_test_db` → `DATABASE_URL`, default
//!     `postgres://vedo:test-vedo-password@localhost:15432/vedo`).
//!   - Context-window tests are pure (no DB).
//!
//! ```bash
//! cargo test --test conversations_unit -- --ignored
//! ```

mod common;

use sqlx::PgPool;
use uuid::Uuid;

use vedo_backend::modules::conversations::models::{Message, Session};
use vedo_backend::modules::conversations::repository::ConversationRepository;
use vedo_backend::modules::conversations::service::ConversationService;
use vedo_backend::modules::query::context_window;
use vedo_backend::shared::llm::Message as LlmMessage;

/// Insert a fresh session row via the repository.
async fn seed_session(db: &PgPool, title: &str) -> Session {
    let repo = ConversationRepository::new(db.clone());
    let now = chrono::Utc::now();
    let session = Session {
        id: Uuid::new_v4(),
        title: title.to_string(),
        collection_id: None,
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
    };
    repo.add_message(&msg).await.expect("seed message");
    msg
}

// ===========================================================================
// Repository round-trip (soft-delete + edit) — targets T6
// ===========================================================================

/// update_message sets edited_at and preserves original_content across edits.
#[tokio::test]
#[ignore = "RED: update_message round-trip (T6)"]
async fn test_update_message_sets_edited_at_and_original_content() {
    let db = common::setup_test_db().await;
    let session = seed_session(&db, "Repo update").await;
    let m = seed_message(&db, session.id, "user", "first").await;
    let repo = ConversationRepository::new(db.clone());

    let updated = repo
        .update_message(m.id, "second")
        .await
        .expect("update_message (T6)");
    assert_eq!(updated.content, "second");
    assert_eq!(updated.original_content.as_deref(), Some("first"));
    assert!(updated.edited_at.is_some(), "edited_at populated");

    // Second edit must NOT overwrite original_content (preserves audit trail).
    let again = repo
        .update_message(m.id, "third")
        .await
        .expect("update_message second");
    assert_eq!(again.content, "third");
    assert_eq!(
        again.original_content.as_deref(),
        Some("first"),
        "original preserved across edits"
    );
}

/// soft_delete_message sets deleted_at and is excluded from get_messages / get_message.
#[tokio::test]
#[ignore = "RED: soft_delete_message + filters (T6)"]
async fn test_soft_delete_sets_deleted_at_and_filters() {
    let db = common::setup_test_db().await;
    let session = seed_session(&db, "Repo soft del").await;
    let m1 = seed_message(&db, session.id, "user", "a").await;
    let _m2 = seed_message(&db, session.id, "assistant", "b").await;
    let _m3 = seed_message(&db, session.id, "user", "c").await;
    let repo = ConversationRepository::new(db.clone());

    repo.soft_delete_message(m1.id)
        .await
        .expect("soft_delete_message (T6)");
    let live = repo.get_messages(session.id).await.expect("get_messages");
    assert_eq!(live.len(), 2, "soft-deleted excluded from get_messages");
    assert!(live.iter().all(|m| m.id != m1.id));

    // get_message returns an error for a soft-deleted id (NotFound).
    let fetched = repo.get_message(m1.id).await;
    assert!(fetched.is_err(), "get_message rejects soft-deleted");
}

/// get_message_count reflects soft-deleted exclusion.
#[tokio::test]
#[ignore = "RED: get_message_count excludes soft-deleted (T6)"]
async fn test_get_message_count_excludes_soft_deleted() {
    let db = common::setup_test_db().await;
    let session = seed_session(&db, "Repo count").await;
    let m1 = seed_message(&db, session.id, "user", "a").await;
    let _m2 = seed_message(&db, session.id, "assistant", "b").await;
    let _m3 = seed_message(&db, session.id, "user", "c").await;
    let repo = ConversationRepository::new(db.clone());

    repo.soft_delete_message(m1.id)
        .await
        .expect("soft_delete_message (T6)");
    let count = repo
        .get_message_count(session.id)
        .await
        .expect("get_message_count (T6)");
    assert_eq!(count, 2, "count excludes soft-deleted");
}

// ===========================================================================
// Service markdown export — targets T6/T8
// ===========================================================================

/// build_markdown_export includes live messages only; excludes soft-deleted.
#[tokio::test]
#[ignore = "RED: build_markdown_export + soft-delete filter (T6/T8)"]
async fn test_export_markdown_includes_all_live_messages_only() {
    let db = common::setup_test_db().await;
    let session = seed_session(&db, "Md live only").await;
    let _m1 = seed_message(&db, session.id, "user", "q1").await;
    let _m2 = seed_message(&db, session.id, "assistant", "a1").await;
    let deleted = seed_message(&db, session.id, "user", "gone").await;

    let repo = ConversationRepository::new(db.clone());
    repo.soft_delete_message(deleted.id)
        .await
        .expect("soft_delete_message (T6)");

    let live = repo.get_messages(session.id).await.expect("get_messages");
    assert_eq!(live.len(), 2, "soft-deleted excluded");

    let svc = ConversationService::new(repo);
    let md = svc
        .build_markdown_export(&session, &live)
        .expect("build_markdown_export (T8)");
    assert!(md.contains("# Md live only"), "H1 session title");
    assert!(md.contains("q1"));
    assert!(md.contains("a1"));
    assert!(!md.contains("gone"), "soft-deleted omitted from markdown");
}

// ===========================================================================
// Context window trimming — targets T7 (pure unit, no DB)
// ===========================================================================

/// Build an LlmMessage helper.
fn llm_msg(role: &str, content: &str) -> LlmMessage {
    LlmMessage {
        role: role.to_string(),
        content: content.to_string(),
    }
}

/// count_tokens approximates via word count.
#[test]
#[ignore = "RED: count_tokens word heuristic (T7)"]
fn test_count_tokens_word_approach_approximates_size() {
    assert_eq!(context_window::count_tokens(""), 0);
    assert_eq!(context_window::count_tokens("one two three"), 3);
    assert_eq!(context_window::count_tokens("  spaced  words  "), 2);
}

/// trim_history drops oldest user+assistant pair until under budget.
#[test]
#[ignore = "RED: trim_history drops oldest until under budget (T7)"]
fn test_trim_history_drops_oldest_until_under_budget() {
    let hist = vec![
        llm_msg("user", "alpha beta gamma delta"),
        llm_msg("assistant", "epsilon zeta eta theta"),
        llm_msg("user", "iota kappa"),
        llm_msg("assistant", "lambda mu"),
    ];
    // budget=6; pair1 tokens=8 (over), pair2 tokens=4 (fits). Must drop pair1.
    let (trimmed, dropped) = context_window::trim_history(&hist, 20, 6);
    assert_eq!(dropped, 2, "drops oldest user+assistant pair");
    assert_eq!(trimmed.len(), 2);
    assert!(trimmed.iter().any(|m| m.content == "iota kappa"));
    assert!(trimmed.iter().any(|m| m.content == "lambda mu"));
    let total: usize = trimmed
        .iter()
        .map(|m| context_window::count_tokens(&m.content))
        .sum();
    assert!(total <= 6, "remaining history is within token budget");
}

/// trim_history preserves at least the most recent turn (2 messages) regardless of budget.
#[test]
#[ignore = "RED: trim_history preserves recent turn (T7)"]
fn test_trim_history_preserves_at_least_one_recent_turn() {
    let hist = vec![
        llm_msg("user", "alpha beta gamma"),
        llm_msg("assistant", "delta epsilon zeta"),
        llm_msg("user", "iota kappa lambda"),
        llm_msg("assistant", "mu nu xi"),
    ];
    // Absurdly tight budget — must still keep the last turn (2 messages).
    let (trimmed, _dropped) = context_window::trim_history(&hist, 20, 1);
    assert!(trimmed.len() >= 2, "preserve at least one recent turn");
    assert_eq!(trimmed[trimmed.len() - 2].content, "iota kappa lambda");
    assert_eq!(trimmed.last().unwrap().content, "mu nu xi");
}

/// trim_history caps by max_messages (drops oldest pairs).
#[test]
#[ignore = "RED: trim_history max_messages cap (T7)"]
fn test_trim_history_max_history_messages_cap() {
    let mut hist = Vec::new();
    for i in 0..10 {
        hist.push(llm_msg("user", &format!("q{i}")));
        hist.push(llm_msg("assistant", &format!("a{i}")));
    }
    let (trimmed, dropped) = context_window::trim_history(&hist, 4, 1000);
    assert_eq!(trimmed.len(), 4, "caps to max_messages");
    assert_eq!(dropped, 16);
    assert_eq!(trimmed[0].content, "q8");
    assert_eq!(trimmed.last().unwrap().content, "a9");
}

/// trim_history over-budget / under-cap history is a no-op.
#[test]
#[ignore = "RED: trim_history under budget is noop (T7)"]
fn test_trim_history_under_budget_is_noop() {
    let hist = vec![llm_msg("user", "hello"), llm_msg("assistant", "world")];
    let (trimmed, dropped) = context_window::trim_history(&hist, 20, 1000);
    assert_eq!(trimmed.len(), 2);
    assert_eq!(dropped, 0);
}
