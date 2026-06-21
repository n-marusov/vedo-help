//! Conversation context-window trimming utilities (v0.3.1).
//!
//! RED-phase scaffold: the public functions `count_tokens` and `trim_history`
//! are added in T7 (Phase 2). Behavioral tests for them live in the external
//! integration test binary (`backend/tests/conversations_integration.rs`,
//! section "context window") so the lib test build (`cargo test --lib`) stays
//! green during the red phase — those tests reference the symbols that land here
//! only after T7.
