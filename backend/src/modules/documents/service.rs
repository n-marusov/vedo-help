use uuid::Uuid;

use crate::modules::documents::models::{
    Chunk, Document, DocumentSummary, UploadResponse, ZipUploadItem, ZipUploadResponse,
};
use crate::modules::documents::repository::DocumentRepository;
use crate::shared::chunking::chunk_document;
use crate::shared::error::AppError;
use crate::shared::file_validation::{validate_file, MAX_FILE_SIZE};
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
        collection_id: Uuid,
        _content_type: String,
    ) -> Result<UploadResponse, AppError> {
        // 1. Validate file
        let file_type = validate_file(data, filename)?;
        tracing::info!(
            "Document uploaded: {filename} ({file_type:?}, {size} bytes) -> collection {collection_id}",
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
                .map(|c| {
                    // Find the last char boundary at or before byte 80
                    let end = c
                        .text
                        .char_indices()
                        .map(|(i, _)| i)
                        .chain(std::iter::once(c.text.len()))
                        .rfind(|&i| i <= 80)
                        .unwrap_or(0);
                    &c.text[..end]
                })
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
            collection_id,
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

    /// Process a ZIP batch upload: extract, validate, chunk, and save each file.
    pub async fn process_zip_upload(
        &self,
        data: &[u8],
        collection_id: Uuid,
    ) -> Result<ZipUploadResponse, AppError> {
        use std::io::Cursor;
        use std::io::Read;

        tracing::info!(
            "Processing ZIP upload for collection {collection_id}: {} bytes",
            data.len()
        );

        // Read all entries into memory synchronously (ZipFile is not Send)
        // Returns: Vec<(filename, file_data)>
        let extracted = {
            let reader = Cursor::new(data);
            let mut archive = zip::ZipArchive::new(reader)
                .map_err(|e| AppError::FileError(format!("Invalid ZIP file: {e}")))?;

            tracing::debug!("ZIP opened: {} entries found", archive.len());

            let total_count = archive.len();
            let mut extracted: Vec<(String, Vec<u8>)> = Vec::new();

            for idx in 0..total_count {
                let entry = match archive.by_index(idx) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!("Failed to read entry at index {idx}: {e}");
                        continue;
                    }
                };

                if entry.is_dir() {
                    continue;
                }

                let name = entry
                    .mangled_name()
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| {
                        let raw = entry.mangled_name().to_string_lossy().to_string();
                        tracing::warn!("ZIP entry has no valid filename, using raw name: {raw}");
                        raw
                    });
                let entry_size = entry.size();
                tracing::debug!("Extracting: {name} ({entry_size} bytes)");

                // Guard against zip bombs: reject entries whose declared uncompressed size
                // exceeds the limit before allocating (prevents Vec::with_capacity OOM).
                if entry_size > MAX_FILE_SIZE {
                    tracing::warn!(
                        "File skipped: {name} - declared uncompressed size {entry_size} exceeds limit",
                    );
                    continue;
                }

                let mut entry_data = Vec::with_capacity(entry_size as usize);
                // Use a size-bounded reader as defense-in-depth against decompression bombs
                // (a ZIP can advertise a small uncompressed size while expanding to gigabytes).
                let mut limited = entry.take(MAX_FILE_SIZE + 1);
                if limited.read_to_end(&mut entry_data).is_err() {
                    tracing::warn!("File skipped: {name} - failed to read data");
                    continue;
                }

                // Post-check: if the bounded reader filled past MAX_FILE_SIZE, the decompressed
                // data exceeded the limit and was truncated — reject it.
                if entry_data.len() as u64 > MAX_FILE_SIZE {
                    tracing::warn!(
                        "File skipped: {name} - decompressed data exceeds maximum size of {} MB",
                        MAX_FILE_SIZE / (1024 * 1024),
                    );
                    continue;
                }

                extracted.push((name, entry_data));
            }

            extracted
        };

        let file_count = extracted.len();

        // Enforce 10-file limit
        if file_count > 10 {
            tracing::warn!("ZIP contains {file_count} files, exceeds limit of 10");
            return Err(AppError::PayloadTooLarge(format!(
                "ZIP contains more than 10 files (found {file_count})",
            )));
        }

        // Process each extracted file asynchronously
        let mut processed = 0usize;
        let mut failed = 0usize;
        let mut items = Vec::new();

        for (name, entry_data) in extracted {
            tracing::debug!("Processing: {name} ({} bytes)", entry_data.len());

            // Detect inner file type by extension
            let inner_file_type = match crate::shared::types::FileType::from_extension(&name) {
                Some(ft) => ft,
                None => {
                    tracing::warn!("File skipped: {name} - unsupported file extension");
                    items.push(ZipUploadItem {
                        filename: name.clone(),
                        status: "skipped".to_string(),
                        document_id: None,
                        error: Some("Unsupported file extension".to_string()),
                    });
                    failed += 1;
                    continue;
                }
            };

            // Validate via validate_file
            if let Err(e) = validate_file(&entry_data, &name) {
                tracing::warn!("File skipped: {name} - {e}");
                items.push(ZipUploadItem {
                    filename: name.clone(),
                    status: "skipped".to_string(),
                    document_id: None,
                    error: Some(format!("Validation failed: {e}")),
                });
                failed += 1;
                continue;
            }

            // Parse file content
            let text = match parse_file_content(&entry_data, &name, &inner_file_type) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("File skipped: {name} - parse error: {e}");
                    items.push(ZipUploadItem {
                        filename: name.clone(),
                        status: "failed".to_string(),
                        document_id: None,
                        error: Some(format!("Parse error: {e}")),
                    });
                    failed += 1;
                    continue;
                }
            };

            // Chunk text
            let chunks = chunk_document(&text);
            tracing::debug!("File processed: {name} -> {} chunks", chunks.len());

            // Create document record
            let doc_id = Uuid::new_v4();
            let doc = Document {
                id: doc_id,
                name: name.clone(),
                file_type: format!("{:?}", inner_file_type),
                file_size: entry_data.len() as i64,
                uploaded_at: chrono::Utc::now(),
                collection_id,
            };

            // Save document (async, no ZipFile borrow)
            if let Err(e) = self.repo.save_document(&doc).await {
                tracing::warn!("File skipped: {name} - failed to save: {e}");
                items.push(ZipUploadItem {
                    filename: name.clone(),
                    status: "failed".to_string(),
                    document_id: None,
                    error: Some(format!("Database error: {e}")),
                });
                failed += 1;
                continue;
            }

            // Save chunks
            let mut save_ok = true;
            for chunk in &chunks {
                let chunk_record = Chunk {
                    id: Uuid::new_v4(),
                    document_id: doc_id,
                    index: chunk.index,
                    text: chunk.text.clone(),
                };
                if let Err(e) = self.repo.save_chunk(&chunk_record).await {
                    tracing::warn!("Failed to save chunk {} for {name}: {e}", chunk.index);
                    save_ok = false;
                    break;
                }
            }

            if !save_ok {
                items.push(ZipUploadItem {
                    filename: name.clone(),
                    status: "failed".to_string(),
                    document_id: None,
                    error: Some("Failed to save chunks".to_string()),
                });
                failed += 1;
                continue;
            }

            processed += 1;
            items.push(ZipUploadItem {
                filename: name,
                status: "success".to_string(),
                document_id: Some(doc_id),
                error: None,
            });
        }

        tracing::info!("ZIP upload complete: {processed}/{file_count} files processed");

        Ok(ZipUploadResponse {
            total_files: file_count,
            processed,
            failed,
            items,
        })
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

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: create an in-memory ZIP with given (filename, content) pairs.
    fn make_zip(files: &[(&str, &str)]) -> Vec<u8> {
        let buf = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(buf);
        for &(name, content) in files {
            let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file(name, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[tokio::test]
    async fn test_process_upload_non_ascii_text() {
        // Regression: process_upload with Cyrillic text must not panic
        // on debug preview slicing (service.rs:50)
        let cyrillic_content = "Привет, мир! Это тестовый документ с кириллицей.

"
        .repeat(50);
        let content = format!(
            "{cyrillic_content}

Дополнительный параграф для проверки корректной обработки многобайтовых символов на границе чанков.

И ещё один параграф с русским текстом для верности."
        );
        let data = content.as_bytes();
        let filename = "test-cyrillic.md";
        let collection_id = Uuid::new_v4();

        let svc = make_service().await;
        let result = svc
            .process_upload(data, filename, collection_id, "text/markdown".to_string())
            .await;

        assert!(result.is_ok(), "process_upload failed: {:?}", result.err());
        let response = result.unwrap();
        assert!(response.chunks_indexed > 0, "Expected at least 1 chunk");
        assert_eq!(response.document_name, filename);

        // Verify document appears in list
        let documents = svc.list_documents(collection_id).await.unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].name, filename);
    }

    #[tokio::test]
    async fn test_process_upload_emoji_content() {
        // Regression: process_upload with 4-byte UTF-8 (emoji) must not panic
        let emoji_line = "😀🚀🌈🧪🔥🎉🎊🎈🎁\n".repeat(30);
        let content = format!("{emoji_line}\nMore emoji text 🎯🎲🎮🕹️🎰.\n");
        let data = content.as_bytes();
        let filename = "test-emoji.md";
        let collection_id = Uuid::new_v4();

        let svc = make_service().await;
        let result = svc
            .process_upload(data, filename, collection_id, "text/markdown".to_string())
            .await;

        assert!(
            result.is_ok(),
            "process_upload with emoji failed: {:?}",
            result.err()
        );
        let response = result.unwrap();
        assert!(response.chunks_indexed > 0);
        assert_eq!(response.document_name, filename);
    }

    #[tokio::test]
    async fn test_process_upload_mixed_encoding() {
        // Regression: process_upload with mixed CJK + Cyrillic + emoji must not panic
        let mixed = "English text. Привет-мир-你好世界😀🚀\n".repeat(40);
        let content = format!("{mixed}\n\nEND");
        let data = content.as_bytes();
        let filename = "test-mixed.md";
        let collection_id = Uuid::new_v4();

        let svc = make_service().await;
        let result = svc
            .process_upload(data, filename, collection_id, "text/markdown".to_string())
            .await;

        assert!(
            result.is_ok(),
            "process_upload with mixed encoding failed: {:?}",
            result.err()
        );
        let response = result.unwrap();
        assert!(response.chunks_indexed > 0);
        assert_eq!(response.document_name, filename);
    }

    #[tokio::test]
    async fn test_process_upload_ascii_regression() {
        // Regression: ASCII-only upload must still work after UTF-8 fixes
        let content =
            "Hello, world!\n\nThis is a test document with ASCII text only.\n\nParagraph three.\n";
        let data = content.as_bytes();
        let filename = "test-ascii.md";
        let collection_id = Uuid::new_v4();

        let svc = make_service().await;
        let result = svc
            .process_upload(data, filename, collection_id, "text/markdown".to_string())
            .await;

        assert!(
            result.is_ok(),
            "process_upload with ASCII failed: {:?}",
            result.err()
        );
        let response = result.unwrap();
        assert_eq!(response.document_name, filename);
    }

    #[tokio::test]
    async fn test_process_zip_with_5_md_files() {
        let data = make_zip(&[
            ("file1.md", "# File 1"),
            ("file2.md", "# File 2"),
            ("file3.md", "# File 3"),
            ("file4.md", "# File 4"),
            ("file5.md", "# File 5"),
        ]);
        let result = make_service()
            .await
            .process_zip_upload(&data, Uuid::new_v4())
            .await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.processed, 5);
        assert_eq!(response.failed, 0);
        assert_eq!(response.total_files, 5);
    }

    #[tokio::test]
    async fn test_process_zip_with_11_files_returns_413() {
        let names: Vec<String> = (0..11).map(|i| format!("doc-{i}.md")).collect();
        let refs: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "# content")).collect();
        let data = make_zip(&refs);
        let result = make_service()
            .await
            .process_zip_upload(&data, Uuid::new_v4())
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::PayloadTooLarge(_) => {}
            _ => panic!("Expected PayloadTooLarge error"),
        }
    }

    #[tokio::test]
    async fn test_process_zip_mixed_valid_invalid() {
        let data = make_zip(&[
            ("valid.md", "# Valid"),
            ("script.exe", "fake exe"),
            ("notes.txt", "Plain text"),
        ]);
        let result = make_service()
            .await
            .process_zip_upload(&data, Uuid::new_v4())
            .await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.processed > 0);
        assert!(response.processed < response.total_files);
    }

    #[tokio::test]
    async fn test_process_zip_empty() {
        let data = make_zip(&[]);
        let result = make_service()
            .await
            .process_zip_upload(&data, Uuid::new_v4())
            .await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.total_files, 0);
        assert_eq!(response.processed, 0);
    }

    #[tokio::test]
    async fn test_process_zip_corrupted() {
        let data = vec![0x00, 0x01, 0x02, 0x03];
        let result = make_service()
            .await
            .process_zip_upload(&data, Uuid::new_v4())
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::FileError(_) => {}
            _ => panic!("Expected FileError for corrupted ZIP"),
        }
    }

    #[tokio::test]
    async fn test_process_zip_unsupported_types_skipped() {
        let data = make_zip(&[
            ("valid.md", "# Valid"),
            ("readme.txt", "Plain text"),
            ("app.exe", "binary"),
        ]);
        let result = make_service()
            .await
            .process_zip_upload(&data, Uuid::new_v4())
            .await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.processed >= 1);
        // .txt and .exe should be skipped, only .md processed
        assert!(response.failed > 0 || response.processed < response.total_files);
    }

    /// Create a DocumentService with an in-memory repository for testing.
    async fn make_service() -> DocumentService {
        // Use shared cache mode so all pool connections see the same in-memory DB
        let pool = sqlx::SqlitePool::connect("sqlite:file::memory:?cache=shared")
            .await
            .expect("Failed to create in-memory SQLite pool");

        // Create schema in the in-memory database
        // Disable FK enforcement for test isolation
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&pool)
            .await
            .ok();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                created_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .ok();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                file_type TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                uploaded_at TEXT NOT NULL,
                collection_id TEXT NOT NULL,
                FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .ok();
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                "index" INTEGER NOT NULL,
                text TEXT NOT NULL,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )"#,
        )
        .execute(&pool)
        .await
        .ok();

        let repo = DocumentRepository::new(pool);
        DocumentService::new(repo)
    }
}

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
