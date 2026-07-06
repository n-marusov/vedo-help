use crate::shared::types::ChunkData;

/// Maximum size of a single chunk in characters (default fallback).
pub const CHUNK_SIZE: usize = 1000;

/// Overlap between consecutive chunks in characters (default fallback).
pub const CHUNK_OVERLAP: usize = 200;

/// Supported chunking strategies.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ChunkMode {
    /// Paragraph-aware splitting on double newlines (default).
    #[default]
    Paragraph,
    /// Fixed-size character-based split with overlap.
    Fixed,
}

impl std::str::FromStr for ChunkMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "paragraph" => Ok(Self::Paragraph),
            "fixed" => Ok(Self::Fixed),
            other => Err(format!("Unknown chunk mode: {other}")),
        }
    }
}

impl std::fmt::Display for ChunkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paragraph => write!(f, "paragraph"),
            Self::Fixed => write!(f, "fixed"),
        }
    }
}

/// Split a document text into overlapping chunks with configurable parameters.
///
/// # Parameters
///
/// * `text` - The document text to split
/// * `chunk_size` - Maximum size of a single chunk in characters
/// * `chunk_overlap` - Overlap between consecutive chunks in characters
/// * `method` - The chunking strategy to use
///
/// ## Paragraph method
/// 1. Split on double newlines (paragraphs)
/// 2. Merge paragraphs into chunks of up to `chunk_size` characters
/// 3. Overlap between chunks is `chunk_overlap` characters
///
/// ## Fixed method
/// 1. Split into fixed-size character chunks
/// 2. Each chunk (except the first) starts `chunk_size - chunk_overlap` chars
///    from the start of the previous chunk
/// 3. Safe UTF-8 boundaries are respected
pub fn chunk_document(
    text: &str,
    chunk_size: usize,
    chunk_overlap: usize,
    method: ChunkMode,
) -> Vec<ChunkData> {
    match method {
        ChunkMode::Paragraph => chunk_by_paragraph(text, chunk_size, chunk_overlap),
        ChunkMode::Fixed => chunk_fixed(text, chunk_size, chunk_overlap),
    }
}

/// Paragraph-aware chunking: split on double newlines, merge into chunks.
fn chunk_by_paragraph(text: &str, chunk_size: usize, chunk_overlap: usize) -> Vec<ChunkData> {
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut index = 0;

    for paragraph in paragraphs {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        // If adding this paragraph would exceed chunk size, finalize current chunk
        if current.len() + trimmed.len() + 2 > chunk_size && !current.is_empty() {
            chunks.push(ChunkData {
                text: current.clone(),
                index,
            });
            index += 1;

            // Start new chunk with overlap from previous
            // Use char_indices to find a safe UTF-8 boundary
            let overlap_start = current.len().saturating_sub(chunk_overlap);
            let safe_start = current
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(current.len()))
                .find(|&i| i >= overlap_start)
                .unwrap_or(current.len());
            current = current[safe_start..].to_string();
            current.push('\n');
            current.push_str(trimmed);
        } else {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(trimmed);
        }
    }

    // Don't forget the last chunk
    if !current.is_empty() {
        chunks.push(ChunkData {
            text: current,
            index,
        });
    }

    tracing::debug!(
        component = "chunking",
        method = "paragraph",
        chunk_count = chunks.len(),
        chunk_size = chunk_size,
        chunk_overlap = chunk_overlap,
        "document.chunked"
    );

    chunks
}

/// Fixed-size chunking: split text into chunks of exactly `chunk_size` chars
/// with `chunk_overlap` overlap, respecting UTF-8 boundaries.
fn chunk_fixed(text: &str, chunk_size: usize, chunk_overlap: usize) -> Vec<ChunkData> {
    if text.is_empty() {
        return Vec::new();
    }

    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();
    let step = chunk_size.saturating_sub(chunk_overlap);
    let mut chunks = Vec::new();
    let mut index = 0;

    // If chunk_size is 0 or step is 0, prevent infinite loop — fall back to single chunk
    if chunk_size == 0 || step == 0 {
        chunks.push(ChunkData {
            text: text.to_string(),
            index: 0,
        });
        tracing::warn!(
            component = "chunking",
            method = "fixed",
            chunk_size = chunk_size,
            chunk_overlap = chunk_overlap,
            "chunking.fixed.invalid_params"
        );
        return chunks;
    }

    let mut start = 0;
    while start < total_chars {
        let end = (start + chunk_size).min(total_chars);
        let chunk_text: String = chars[start..end].iter().collect();
        chunks.push(ChunkData {
            text: chunk_text,
            index,
        });
        index += 1;
        start += step;
    }

    tracing::debug!(
        component = "chunking",
        method = "fixed",
        chunk_count = chunks.len(),
        chunk_size = chunk_size,
        chunk_overlap = chunk_overlap,
        input_length = total_chars,
        "document.chunked"
    );

    chunks
}

/// Convenience wrapper using default constants (backward compatible).
pub fn chunk_document_default(text: &str) -> Vec<ChunkData> {
    chunk_document(text, CHUNK_SIZE, CHUNK_OVERLAP, ChunkMode::Paragraph)
}

// ---------------------------------------------------------------------------
// ChunkData helpers
// ---------------------------------------------------------------------------

/// Calculate the total number of bytes consumed by chunking.
pub fn total_chunk_bytes(chunks: &[ChunkData]) -> usize {
    chunks.iter().map(|c| c.text.len()).sum()
}

/// Calculate the total number of characters consumed by chunking.
pub fn total_chunk_chars(chunks: &[ChunkData]) -> usize {
    chunks.iter().map(|c| c.text.chars().count()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Paragraph method tests ──

    #[test]
    fn test_chunk_empty_text() {
        let chunks = chunk_document("", 1000, 200, ChunkMode::Paragraph);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_small_text() {
        let text = "Hello, world!";
        let chunks = chunk_document(text, 1000, 200, ChunkMode::Paragraph);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello, world!");
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_chunk_no_data_loss() {
        let text = "Paragraph one.\n\nParagraph two.\n\nParagraph three.";
        let chunks = chunk_document(text, 1000, 200, ChunkMode::Paragraph);
        let reconstructed: String = chunks
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join("");
        // The reconstructed string should be longer because of overlaps
        assert!(reconstructed.len() >= text.replace("\n\n", "").len());
    }

    #[test]
    fn test_chunk_overlap_non_ascii() {
        // Regression: overlap slicing with Cyrillic multi-byte chars
        let long_para = "Привет мир".repeat(100);
        let text = format!(
            "{long_para}\n\nДополнительный параграф с кириллицей для форсирования overlap.\n\nЕщё один параграф."
        );
        let chunks = chunk_document(&text, 1000, 200, ChunkMode::Paragraph);
        assert!(!chunks.is_empty(), "Expected at least one chunk");
        for chunk in &chunks {
            assert!(
                std::str::from_utf8(chunk.text.as_bytes()).is_ok(),
                "Chunk {} contains invalid UTF-8",
                chunk.index
            );
        }
        let total_chars: usize = chunks.iter().map(|c| c.text.len()).sum();
        assert!(total_chars >= text.len(), "Data loss detected");
    }

    #[test]
    fn test_chunk_overlap_emoji() {
        let emoji_para = "😀🚀🌈🧪🔥".repeat(200);
        let text = format!("{emoji_para}\n\nMore emoji: 🎉🎊🎈🎁\n\nFinal paragraph with text.");
        let chunks = chunk_document(&text, 1000, 200, ChunkMode::Paragraph);
        assert!(!chunks.is_empty(), "Expected at least one chunk");
        for chunk in &chunks {
            assert!(
                std::str::from_utf8(chunk.text.as_bytes()).is_ok(),
                "Chunk {} contains invalid UTF-8 (emoji)",
                chunk.index
            );
        }
    }

    #[test]
    fn test_chunk_respects_size() {
        let text = (0..5)
            .map(|_i| format!("Paragraph {} with some content.\n\n", "A".repeat(200)))
            .collect::<String>();
        let chunks = chunk_document(&text, 1000, 200, ChunkMode::Paragraph);
        assert!(
            chunks.len() >= 2,
            "Expected at least 2 chunks, got {}",
            chunks.len()
        );
        for chunk in &chunks {
            assert!(chunk.text.len() <= 1000 + 200);
        }
    }

    // ── Fixed method tests ──

    #[test]
    fn test_fixed_empty_text() {
        let chunks = chunk_document("", 100, 20, ChunkMode::Fixed);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_fixed_small_text() {
        let text = "Hello, world!";
        let chunks = chunk_document(text, 100, 20, ChunkMode::Fixed);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello, world!");
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_fixed_multiple_chunks() {
        let text = "A".repeat(300);
        let chunks = chunk_document(&text, 100, 20, ChunkMode::Fixed);
        assert_eq!(chunks.len(), 4, "300 chars / 80 step = 4 chunks");
        assert_eq!(chunks[0].text.len(), 100);
        assert_eq!(chunks[1].text.len(), 100);
        assert_eq!(chunks[2].text.len(), 100);
    }

    #[test]
    fn test_fixed_utf8_safe() {
        let text = "😀".repeat(50); // 50 emoji = 50 chars
        let chunks = chunk_document(&text, 10, 2, ChunkMode::Fixed);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(
                std::str::from_utf8(chunk.text.as_bytes()).is_ok(),
                "Chunk {} contains invalid UTF-8",
                chunk.index
            );
        }
    }

    #[test]
    fn test_fixed_zero_chunk_size_fallback() {
        let text = "Some text";
        let chunks = chunk_document(text, 0, 0, ChunkMode::Fixed);
        assert_eq!(
            chunks.len(),
            1,
            "Zero chunk_size should fall back to single chunk"
        );
        assert_eq!(chunks[0].text, "Some text");
    }

    // ── ChunkMode parsing tests ──

    #[test]
    fn test_chunk_mode_from_str() {
        assert_eq!(
            "paragraph".parse::<ChunkMode>().ok(),
            Some(ChunkMode::Paragraph),
        );
        assert_eq!("fixed".parse::<ChunkMode>().ok(), Some(ChunkMode::Fixed),);
        assert_eq!(
            "Paragraph".parse::<ChunkMode>().ok(),
            Some(ChunkMode::Paragraph),
        );
        assert_eq!("FIXED".parse::<ChunkMode>().ok(), Some(ChunkMode::Fixed),);
        assert!("recursive".parse::<ChunkMode>().is_err());
        assert!("".parse::<ChunkMode>().is_err());
    }

    #[test]
    fn test_chunk_mode_default() {
        assert_eq!(ChunkMode::default(), ChunkMode::Paragraph);
    }

    #[test]
    fn test_chunk_mode_display() {
        assert_eq!(ChunkMode::Paragraph.to_string(), "paragraph");
        assert_eq!(ChunkMode::Fixed.to_string(), "fixed");
    }

    // ── Default wrapper test ──

    #[test]
    fn test_chunk_document_default() {
        let text = "Paragraph one.\n\nParagraph two.";
        let chunks = chunk_document_default(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].index, 0);
    }
}
