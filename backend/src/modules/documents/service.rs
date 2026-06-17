use uuid::Uuid;

use crate::modules::documents::models::{Document, DocumentSummary, UploadResponse};
use crate::modules::documents::repository::DocumentRepository;
use crate::shared::chunking::chunk_document;
use crate::shared::error::AppError;
use crate::shared::file_validation::validate_file;
use crate::shared::types::FileType;

/// Service for document management operations.
#[derive(Clone, Debug)]
pub struct DocumentService {
    repo: DocumentRepository,
}

impl DocumentService {
    /// Create a new DocumentService.
    pub fn new(repo: DocumentRepository) -> Self {
        Self { repo }
    }

    /// Process a document upload: validate, parse, chunk, save.
    pub async fn process_upload(
        &self,
        data: &[u8],
        filename: &str,
        _content_type: String,
    ) -> Result<UploadResponse, AppError> {
        // 1. Validate file
        let file_type = validate_file(data, filename)?;
        tracing::info!(
            "Document uploaded: {filename} ({file_type:?}, {size} bytes)",
            size = data.len()
        );

        // 2. Parse file into text
        let text = parse_file_content(data, filename, &file_type)?;
        tracing::info!("Document parsed: {filename} -> {} chars", text.len());

        // 3. Chunk text
        let chunks = chunk_document(&text);
        tracing::debug!(
            "Chunk {idx}: {preview}...",
            idx = if chunks.is_empty() { 0 } else { 1 },
            preview = chunks
                .first()
                .map(|c| &c.text[..c.text.len().min(80)])
                .unwrap_or("")
        );

        // 4. Create document record
        let doc_id = Uuid::new_v4();
        let doc = Document {
            id: doc_id,
            name: filename.to_string(),
            file_type: format!("{:?}", file_type),
            file_size: data.len() as i64,
            uploaded_at: chrono::Utc::now(),
            collection_id: Uuid::default(), // Will be set by client later
        };

        self.repo.save_document(&doc).await?;

        // 5. Save chunks
        for chunk in &chunks {
            let chunk_record = crate::modules::documents::models::Chunk {
                id: Uuid::new_v4(),
                document_id: doc_id,
                index: chunk.index,
                text: chunk.text.clone(),
            };
            self.repo.save_chunk(&chunk_record).await?;
        }

        Ok(UploadResponse {
            document_id: doc_id,
            chunks_indexed: chunks.len(),
            document_name: filename.to_string(),
        })
    }

    /// List documents in a collection.
    pub async fn list_documents(
        &self,
        collection_id: Uuid,
    ) -> Result<Vec<DocumentSummary>, AppError> {
        let documents = self.repo.list_documents(collection_id).await?;
        let summaries = documents
            .into_iter()
            .map(|d| DocumentSummary {
                id: d.id,
                name: d.name,
                file_type: d.file_type,
                file_size: d.file_size,
                uploaded_at: d.uploaded_at,
                collection_id: d.collection_id,
            })
            .collect();
        Ok(summaries)
    }

    /// Delete a document and its chunks.
    pub async fn delete_document(&self, id: Uuid) -> Result<(), AppError> {
        self.repo.delete_document(id).await
    }
}

/// Parse file content into plain text based on file type.
fn parse_file_content(
    data: &[u8],
    _filename: &str,
    file_type: &FileType,
) -> Result<String, AppError> {
    match file_type {
        FileType::Markdown => String::from_utf8(data.to_vec())
            .map_err(|e| AppError::FileError(format!("Invalid UTF-8 in markdown file: {e}"))),
        FileType::Pdf => {
            // pdf-extract extraction
            let text = pdf_extract::extract_text_from_mem(data)
                .map_err(|e| AppError::FileError(format!("PDF parse error: {e}")))?;
            Ok(text)
        }
        FileType::Docx => {
            // docx-rs extraction
            let text = extract_docx_text(data)?;
            Ok(text)
        }
        FileType::Zip => Err(AppError::FileError(
            "ZIP files cannot be parsed directly — use the batch upload endpoint".to_string(),
        )),
    }
}

/// Extract text from a DOCX file using docx-rs.
fn extract_docx_text(data: &[u8]) -> Result<String, AppError> {
    use docx_rs::*;
    let doc = read_docx(data).map_err(|e| AppError::FileError(format!("DOCX parse error: {e}")))?;
    let mut text = String::new();
    for paragraph in &doc.document.children {
        if let DocumentChild::Paragraph(p) = paragraph {
            for p_child in &p.children {
                if let ParagraphChild::Run(run) = p_child {
                    for r_child in &run.children {
                        if let RunChild::Text(t) = r_child {
                            text.push_str(&t.text);
                        }
                    }
                }
            }
            text.push('\n');
        }
    }
    Ok(text)
}
