//! Conversation context-window trimming utilities (v0.3.1).
//!
//! Word-count-based tokenizer (`count_tokens`) and sliding-window history
//! trimmer (`trim_history`) that drops oldest user+assistant message pairs
//! until both `max_messages` and `token_budget` constraints are satisfied.
//!
//! ## Design Decision
//!
//! A word-count heuristic (`content.split_whitespace().count()`) is used as
//! a cheap token-count proxy. This is documented as a known limitation of
//! the v0.3.1 implementation — revisit with `tiktoken-rs` or similar in
//! v0.5 Advanced RAG if quality suffers.

use crate::shared::llm::Message;

/// Count tokens (approximation) in a text string using a word-count heuristic.
///
/// Returns the number of whitespace-delimited words. An empty string returns 0.
/// Multiple consecutive spaces are handled correctly by `split_whitespace`.
pub fn count_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    text.split_whitespace().count()
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

    #[test]
    fn test_count_tokens_empty() {
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn test_count_tokens_words() {
        assert_eq!(count_tokens("one two three"), 3);
    }

    #[test]
    fn test_count_tokens_spaced() {
        assert_eq!(count_tokens("  spaced  words  "), 2);
    }

    #[test]
    fn test_trim_history_under_budget_noop() {
        let hist = vec![llm_msg("user", "hi"), llm_msg("assistant", "hello")];
        let (trimmed, dropped) = trim_history(&hist, 20, 1000);
        assert_eq!(trimmed.len(), 2);
        assert_eq!(dropped, 0);
    }

    #[test]
    fn test_trim_history_drops_oldest_until_under_budget() {
        let hist = vec![
            llm_msg("user", "alpha beta gamma delta"),
            llm_msg("assistant", "epsilon zeta eta theta"),
            llm_msg("user", "iota kappa"),
            llm_msg("assistant", "lambda mu"),
        ];
        let (trimmed, dropped) = trim_history(&hist, 20, 6);
        assert_eq!(dropped, 2, "drops oldest user+assistant pair");
        assert_eq!(trimmed.len(), 2);
        assert!(trimmed.iter().any(|m| m.content == "iota kappa"));
        let total: usize = trimmed.iter().map(|m| count_tokens(&m.content)).sum();
        assert!(total <= 6, "remaining history within token budget");
    }

    #[test]
    fn test_trim_history_preserves_recent_turn() {
        let hist = vec![
            llm_msg("user", "alpha beta gamma"),
            llm_msg("assistant", "delta epsilon zeta"),
            llm_msg("user", "iota kappa lambda"),
            llm_msg("assistant", "mu nu xi"),
        ];
        let (trimmed, _) = trim_history(&hist, 20, 1);
        assert!(trimmed.len() >= 2, "preserve at least one recent turn");
        assert_eq!(trimmed[trimmed.len() - 2].content, "iota kappa lambda");
        assert_eq!(trimmed.last().unwrap().content, "mu nu xi");
    }

    #[test]
    fn test_count_tokens_long_text() {
        // Longer text should produce proportionally more tokens
        let short = "hello world";
        let long = "hello world ".repeat(100).trim_end().to_string();
        assert!(count_tokens(&long) > count_tokens(short));
        assert_eq!(count_tokens(&long), 200);
    }

    #[test]
    fn test_count_tokens_special_chars() {
        // Word-count heuristic splits on whitespace
        assert_eq!(count_tokens("hello-world"), 1); // hyphenated = 1 word
        assert_eq!(count_tokens("hello world"), 2); // space-separated = 2 words
        assert_eq!(count_tokens("don't"), 1); // contraction = 1 word
    }

    #[test]
    fn test_count_tokens_code_snippet() {
        // Code with special characters is counted per whitespace-delimited token
        let code = "fn main() { println!(\"hello\"); }";
        assert_eq!(count_tokens(code), 5); // "fn", "main()", "{", "println!(\"hello\");", "}"
    }

    #[test]
    fn test_trim_history_max_messages_cap() {
        let mut hist = Vec::new();
        for i in 0..10 {
            hist.push(llm_msg("user", &format!("q{i}")));
            hist.push(llm_msg("assistant", &format!("a{i}")));
        }
        let (trimmed, dropped) = trim_history(&hist, 4, 1000);
        assert_eq!(trimmed.len(), 4, "caps to max_messages");
        assert_eq!(dropped, 16);
        assert_eq!(trimmed[0].content, "q8");
    }
}
