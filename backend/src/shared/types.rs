use serde::{Deserialize, Serialize};

/// A single chunk of text extracted from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    pub text: String,
    pub index: usize,
}

/// An embedding vector produced by the embedding service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
}

/// A search result from Chroma.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaResult {
    pub id: String,
    pub text: String,
    pub document_id: String,
    pub chunk_index: usize,
    pub score: f64,
}

/// Supported file types for upload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileType {
    Pdf,
    Markdown,
    Docx,
    Zip,
}

impl FileType {
    /// Returns the MIME type string for this file type.
    pub fn mime_type(&self) -> &'static str {
        match self {
            FileType::Pdf => "application/pdf",
            FileType::Markdown => "text/markdown",
            FileType::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            FileType::Zip => "application/zip",
        }
    }

    /// Detect file type from a filename extension.
    pub fn from_extension(filename: &str) -> Option<FileType> {
        let ext = filename.rsplit('.').next()?.to_lowercase();
        match ext.as_str() {
            "pdf" => Some(FileType::Pdf),
            "md" | "markdown" => Some(FileType::Markdown),
            "docx" => Some(FileType::Docx),
            "zip" => Some(FileType::Zip),
            _ => None,
        }
    }
}
