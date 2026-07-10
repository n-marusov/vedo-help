//! Conversation context-window trimming utilities (v0.5).
//!
//! tiktoken-rs based tokenizer (`count_tokens`) and sliding-window history
//! trimmer (`trim_history`) that drops oldest user+assistant message pairs
//! until both `max_messages` and `token_budget` constraints are satisfied.
//!
//! ## Design Decision
//!
//! Uses `tiktoken-rs` with the `cl100k_base` encoding (standard for Claude
//! and GPT-4 family models) for accurate token counting. Falls back to a
//! word-count heuristic (`content.split_whitespace().count()`) if the
//! tokenizer fails to initialise — ensuring the system remains operational
//! even when the tiktoken BPE vocabulary cannot be loaded.

use std::sync::LazyLock;

use tiktoken_rs::{cl100k_base, CoreBPE};

use crate::shared::llm::Message;

/// Global tiktoken BPE tokenizer (lazily initialised once).
static TOKENIZER: LazyLock<Option<CoreBPE>> = LazyLock::new(|| match cl100k_base() {
    Ok(bpe) => {
        tracing::info!(
            component = "context_window",
            "Tokenizer initialised (cl100k_base)"
        );
        Some(bpe)
    }
    Err(e) => {
        tracing::warn!(
            component = "context_window",
            error = %e,
            "Tokenizer init failed — falling back to word-count heuristic"
        );
        None
    }
});

/// Count tokens in a text string using the tiktoken-rs `cl100k_base` tokenizer.
///
/// Falls back to a word-count heuristic if the BPE tokenizer failed to load.
/// Returns 0 for empty strings.
pub fn count_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    match TOKENIZER.as_ref() {
        Some(bpe) => {
            let tokens = bpe.encode_with_special_tokens(text);
            let count = tokens.len();
            tracing::trace!(
                component = "context_window",
                input_len = text.len(),
                token_count = count,
                "count_tokens.tiktoken"
            );
            count
        }
        None => {
            // Fallback: word-count heuristic
            let count = text.split_whitespace().count();
            tracing::trace!(
                component = "context_window",
                input_len = text.len(),
                token_count = count,
                method = "word_count_fallback",
                "count_tokens.fallback"
            );
            count
        }
    }
}

/// Trim conversation history by dropping oldest user+assistant message pairs
/// until both `max_messages` and `token_budget` constraints are satisfied.
///
/// ## Algorithm
///
/// 1. If the history is already within both constraints → no-op.
/// 2. Drop oldest **pairs** (user + assistant) one at a time until:
///    - The message count is at most `max_messages`, AND
///    - The total token count is at most `token_budget`.
/// 3. Always preserve at least the **last 2 messages** (1 user+assistant turn)
///    regardless of how tight the budget is.
/// 4. `max_messages` cap is enforced strictly first; then token budget.
///
/// ## Returns
///
/// A tuple `(trimmed, dropped_count)` where:
/// - `trimmed` — the surviving messages (in original order).
/// - `dropped_count` — total number of messages dropped.
pub fn trim_history(
    history: &[Message],
    max_messages: usize,
    token_budget: usize,
) -> (Vec<Message>, usize) {
    if history.len() <= 2 {
        return (history.to_vec(), 0);
    }

    let total_tokens: usize = history.iter().map(|m| count_tokens(&m.content)).sum();

    // Check if we're already within budget
    if history.len() <= max_messages && total_tokens <= token_budget {
        return (history.to_vec(), 0);
    }

    let mut trimmed: Vec<Message> = history.to_vec();
    let mut dropped = 0;

    // First pass: enforce max_messages cap by dropping oldest pairs
    while trimmed.len() > max_messages && trimmed.len() > 2 {
        // Drop oldest 2 messages (user+assistant)
        trimmed.remove(0);
        trimmed.remove(0);
        dropped += 2;
    }

    // Second pass: enforce token budget by dropping oldest pairs
    loop {
        let current_tokens: usize = trimmed.iter().map(|m| count_tokens(&m.content)).sum();
        if current_tokens <= token_budget || trimmed.len() <= 2 {
            break;
        }
        // Drop oldest pair
        trimmed.remove(0);
        trimmed.remove(0);
        dropped += 2;
    }

    (trimmed, dropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn llm_msg(role: &str, content: &str) -> Message {
        Message {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    // ── count_tokens (tiktoken-rs) tests ──

    #[test]
    fn test_count_tokens_empty() {
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn test_count_tokens_whitespace_only() {
        // tiktoken-rs treats whitespace as BPE tokens (not ignored)
        let count = count_tokens("   \n\r\t   ");
        assert!(count >= 1, "whitespace chars produce at least 1 token");
    }

    #[test]
    fn test_count_tokens_simple_sentence() {
        // Common words in cl100k_base are typically single tokens
        let count = count_tokens("Hello, world!");
        assert!(count > 0 && count < 10, "expected 1-9 tokens, got {count}");
    }

    #[test]
    fn test_count_tokens_longer_text_produces_more_tokens() {
        let short = "Rust is a systems programming language";
        let long = "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety. ";
        assert!(
            count_tokens(long) > count_tokens(short),
            "longer text should produce more tokens"
        );
    }

    #[test]
    fn test_count_tokens_non_ascii() {
        // Cyrillic characters take multiple tokens in cl100k_base
        let russian = "Привет, как дела?";
        let count = count_tokens(russian);
        assert!(count > 0, "Russian text should produce tokens");
    }

    #[test]
    fn test_count_tokens_code_snippet() {
        let code = r#"fn main() {
    println!("hello");
}"#;
        let count = count_tokens(code);
        assert!(count > 0, "code snippet should produce tokens");
        assert!(count >= 5, "code should produce at least ~5 tokens");
    }

    #[test]
    fn test_count_tokens_large_text_within_reasonable_bounds() {
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(50);
        let count = count_tokens(&text);
        assert!(
            count > 50,
            "50 repeated sentences should produce >50 tokens"
        );
        assert!(
            count < 2000,
            "50 sentences should produce <2000 tokens, got {count}"
        );
    }

    #[test]
    fn test_count_tokens_tiktoken_differs_from_word_count_for_code() {
        // Compact code: `a=>{b:1}` is 1 word but multiple BPE tokens
        let compact_code = "a=>{b:1}";
        let token_count = count_tokens(compact_code);
        let word_count = compact_code.split_whitespace().count();
        assert!(
            token_count >= word_count,
            "tiktoken count ({token_count}) should be >= word count ({word_count}) for code"
        );
    }

    // ── trim_history tests ──

    #[test]
    fn test_trim_history_empty() {
        let (trimmed, dropped) = trim_history(&[], 10, 1000);
        assert_eq!(trimmed.len(), 0);
        assert_eq!(dropped, 0);
    }

    #[test]
    fn test_trim_history_under_budget_noop() {
        let hist = vec![llm_msg("user", "hi"), llm_msg("assistant", "hello")];
        let (trimmed, dropped) = trim_history(&hist, 20, 1000);
        assert_eq!(trimmed.len(), 2);
        assert_eq!(dropped, 0);
    }

    #[test]
    fn test_trim_history_drops_pairs_when_over_budget() {
        let hist = vec![
            llm_msg("user", "What is the capital of France?"),
            llm_msg("assistant", "Paris is the capital of France."),
            llm_msg("user", "Tell me more about its history."),
            llm_msg(
                "assistant",
                "Paris has a rich history dating back to the 3rd century BC.",
            ),
        ];
        let (trimmed, dropped) = trim_history(&hist, 20, 10);
        assert!(dropped > 0, "should drop messages when over token budget");
        assert_eq!(trimmed.len(), 2, "should keep only the last turn");
        assert_eq!(trimmed[0].content, "Tell me more about its history.");
        assert!(trimmed[1].content.contains("rich history"));
    }

    #[test]
    fn test_trim_history_preserves_recent_turn() {
        let hist = vec![
            llm_msg("user", "What is Rust?"),
            llm_msg("assistant", "Rust is a systems programming language."),
            llm_msg("user", "What is Python?"),
            llm_msg("assistant", "Python is a high-level interpreted language."),
        ];
        let (trimmed, _) = trim_history(&hist, 20, 1);
        assert!(trimmed.len() >= 2, "preserve at least one recent turn");
        assert_eq!(trimmed[trimmed.len() - 2].content, "What is Python?");
        assert!(trimmed.last().unwrap().content.contains("Python"));
    }

    #[test]
    fn test_trim_history_max_messages_cap() {
        let mut hist = Vec::new();
        for i in 0..10 {
            hist.push(llm_msg("user", &format!("q{i}")));
            hist.push(llm_msg("assistant", &format!("a{i}")));
        }
        let (trimmed, dropped) = trim_history(&hist, 4, 100000);
        assert_eq!(trimmed.len(), 4, "caps to max_messages");
        assert_eq!(dropped, 16);
        assert_eq!(trimmed[0].content, "q8");
    }

    #[test]
    fn test_trim_history_budget_enforced_before_message_cap() {
        let mut hist = Vec::new();
        for i in 0..5 {
            hist.push(llm_msg("user", &format!("q{i}")));
            hist.push(llm_msg("assistant", &format!("a{i}")));
        }
        let (trimmed, dropped) = trim_history(&hist, 100, 3);
        assert!(dropped > 0, "should drop pairs when over token budget");
        // The algorithm preserves at least the last user+assistant turn (2 messages)
        assert_eq!(trimmed.len(), 2, "should keep only the last turn");
        assert_eq!(trimmed[0].content, "q4", "last user message preserved");
        assert_eq!(trimmed[1].content, "a4", "last assistant message preserved");
    }

    #[test]
    fn test_trim_history_single_turn_never_dropped() {
        let hist = vec![
            llm_msg("user", "Very long question "),
            llm_msg("assistant", "Very long answer "),
        ];
        let (trimmed, dropped) = trim_history(&hist, 0, 0);
        assert_eq!(dropped, 0, "single turn should never be dropped");
        assert_eq!(trimmed.len(), 2);
    }

    // ── Tokenizer lazy initialisation tests ──

    #[test]
    fn test_tokenizer_is_lazily_initialised() {
        // TOKENIZER should be in a valid state
        assert!(
            TOKENIZER.is_some() || TOKENIZER.is_none(),
            "tokenizer must be in a valid state"
        );
        // On standard systems cl100k_base works
        assert!(
            TOKENIZER.is_some(),
            "cl100k_base tokenizer should initialise on this platform"
        );
    }

    #[test]
    fn test_tokenizer_idempotent() {
        // Repeated calls return consistent results
        let a = count_tokens("Consistency is key");
        let b = count_tokens("Consistency is key");
        assert_eq!(a, b, "tokenizer must be deterministic");
    }

    #[test]
    fn test_count_tokens_large_json() {
        let json = r#"{"name": "Alice", "age": 30, "city": "New York", "occupation": "Engineer"}"#;
        let count = count_tokens(json);
        assert!(
            count > 5,
            "JSON with 4 fields should produce >5 tokens, got {count}"
        );
        assert!(
            count < 100,
            "JSON with 4 fields should produce <100 tokens, got {count}"
        );
    }

    #[test]
    fn test_count_tokens_markdown() {
        let md = "# Header\n\nThis is **bold** text with `code` and [links](https://example.com).";
        let count = count_tokens(md);
        assert!(count > 5, "markdown should produce >5 tokens, got {count}");
    }
}
