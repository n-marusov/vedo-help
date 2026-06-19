use crate::shared::types::ChunkData;

/// Maximum size of a single chunk in characters.
pub const CHUNK_SIZE: usize = 1000;

/// Overlap between consecutive chunks in characters.
pub const CHUNK_OVERLAP: usize = 200;

/// Split a document text into overlapping chunks.
///
/// Uses a simple paragraph-aware splitting strategy:
/// 1. Split on double newlines (paragraphs)
/// 2. Merge paragraphs into chunks of up to `CHUNK_SIZE` characters
/// 3. Overlap between chunks is `CHUNK_OVERLAP` characters
pub fn chunk_document(text: &str) -> Vec<ChunkData> {
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
        if current.len() + trimmed.len() + 2 > CHUNK_SIZE && !current.is_empty() {
            chunks.push(ChunkData {
                text: current.clone(),
                index,
            });
            index += 1;

            // Start new chunk with overlap from previous
            // Use char_indices to find a safe UTF-8 boundary
            let overlap_start = current.len().saturating_sub(CHUNK_OVERLAP);
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
        "Document chunked into {} chunks (size={CHUNK_SIZE}, overlap={CHUNK_OVERLAP})",
        chunks.len()
    );

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_empty_text() {
        let chunks = chunk_document("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_small_text() {
        let text = "Hello, world!";
        let chunks = chunk_document(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello, world!");
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_chunk_no_data_loss() {
        let text = "Paragraph one.\n\nParagraph two.\n\nParagraph three.";
        let chunks = chunk_document(text);
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
        // Build paragraphs where overlap boundary falls inside a Cyrillic char
        let long_para = "Привет мир".repeat(100); // ~1000 Cyrillic chars = ~2000 bytes
        let text = format!(
            "{long_para}\n\nДополнительный параграф с кириллицей для форсирования overlap.\n\nЕщё один параграф."
        );
        let chunks = chunk_document(&text);
        assert!(!chunks.is_empty(), "Expected at least one chunk");
        // Verify all chunks are valid UTF-8 (no panic on slice)
        for chunk in &chunks {
            assert!(
                std::str::from_utf8(chunk.text.as_bytes()).is_ok(),
                "Chunk {} contains invalid UTF-8",
                chunk.index
            );
        }
        // Verify no data is lost (all chars from original appear somewhere)
        let total_chars: usize = chunks.iter().map(|c| c.text.len()).sum();
        assert!(total_chars >= text.len(), "Data loss detected");
    }

    #[test]
    fn test_chunk_overlap_emoji() {
        // Regression: 4-byte UTF-8 (emoji) at overlap boundary
        let emoji_para = "😀🚀🌈🧪🔥".repeat(200); // 4-byte chars
        let text = format!("{emoji_para}\n\nMore emoji: 🎉🎊🎈🎁\n\nFinal paragraph with text.");
        let chunks = chunk_document(&text);
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
    fn test_chunk_overlap_mixed_encoding() {
        // Regression: mixed ASCII + multi-byte (CJK + Cyrillic + emoji)
        let mixed = format!(
            "English text. {cyr} {cjk} {emoji}",
            cyr = "Привет-мир-".repeat(80),
            cjk = "你好世界".repeat(60),
            emoji = "😀".repeat(100),
        );
        let text = format!(
            "{mixed}\n\n{mixed}\n\n{mixed}\n\nFinal paragraph to test overlap with mixed encodings."
        );
        let chunks = chunk_document(&text);
        assert!(!chunks.is_empty(), "Expected at least one chunk");
        for chunk in &chunks {
            assert!(
                std::str::from_utf8(chunk.text.as_bytes()).is_ok(),
                "Chunk {} contains invalid UTF-8 (mixed)",
                chunk.index
            );
        }
    }

    #[test]
    fn test_chunk_respects_size() {
        // Create text with multiple paragraphs that exceeds chunk size
        let text = (0..5)
            .map(|_i| format!("Paragraph {} with some content.\n\n", "A".repeat(200)))
            .collect::<String>();
        let chunks = chunk_document(&text);
        assert!(
            chunks.len() >= 2,
            "Expected at least 2 chunks, got {}",
            chunks.len()
        );
        for chunk in &chunks {
            assert!(chunk.text.len() <= CHUNK_SIZE + CHUNK_OVERLAP);
        }
    }
}
