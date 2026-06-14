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
            let overlap_start = current.len().saturating_sub(CHUNK_OVERLAP);
            current = current[overlap_start..].to_string();
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
    fn test_chunk_respects_size() {
        // Create text with multiple paragraphs that exceeds chunk size
        let text = (0..5)
            .map(|i| format!("Paragraph {} with some content.\n\n", "A".repeat(200)))
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
