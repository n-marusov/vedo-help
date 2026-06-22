use std::collections::HashMap;

use uuid::Uuid;

use crate::modules::documents::models::{
    BatchDeleteResponse, Chunk, Document, DocumentSummary, UploadResponse, ZipUploadItem,
    ZipUploadResponse,
};
use crate::modules::documents::repository::DocumentRepository;
use crate::shared::chroma_client::ChromaClient;
use crate::shared::chunking::chunk_document;
use crate::shared::embedding_client::EmbeddingClient;
use crate::shared::error::AppError;
use crate::shared::file_validation::{validate_file, MAX_FILE_SIZE};
use crate::shared::types::FileType;

/// Service for document management operations.
#[derive(Clone, Debug)]
pub struct DocumentService {
    pub(crate) repo: DocumentRepository,
    pub(crate) chroma_client: Option<ChromaClient>,
    pub(crate) embedding_client: Option<EmbeddingClient>,
}

impl DocumentService {
    /// Create a new DocumentService with only a repository (no Chroma/embedding).
    pub fn new(repo: DocumentRepository) -> Self {
        Self {
            repo,
            chroma_client: None,
            embedding_client: None,
        }
    }

    /// Create a DocumentService with Chroma and embedding clients.
    pub fn with_clients(
        repo: DocumentRepository,
        chroma_client: ChromaClient,
        embedding_client: EmbeddingClient,
    ) -> Self {
        Self {
            repo,
            chroma_client: Some(chroma_client),
            embedding_client: Some(embedding_client),
        }
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
            is_active: true,
        };

        self.repo.save_document(&doc).await?;

        // 5. Save chunks
        let mut chunk_records = Vec::new();
        for chunk in &chunks {
            let chunk_record = crate::modules::documents::models::Chunk {
                id: Uuid::new_v4(),
                document_id: doc_id,
                index: chunk.index,
                text: chunk.text.clone(),
                is_active: true,
            };
            self.repo.save_chunk(&chunk_record).await?;
            chunk_records.push(chunk_record);
        }

        // 6. Index into Chroma if clients are available. If indexing fails, roll
        // back the active database rows so users do not see unsearchable documents.
        if let Err(e) = self
            .index_chunks_in_chroma(collection_id, doc_id, filename, &chunk_records)
            .await
        {
            tracing::error!(
                "Upload indexing failed; deactivating document and chunks: document={doc_id}, error={e}"
            );
            self.repo.deactivate_chunks(doc_id).await?;
            self.repo.deactivate_document(doc_id).await?;
            return Err(e);
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
                is_active: d.is_active,
            })
            .collect();
        Ok(summaries)
    }

    /// Soft delete a document: deactivate it and its chunks in the database.
    /// Rows remain in the database but are excluded from active queries.
    /// Also removes the document's embeddings from Chroma if clients are configured.
    pub async fn delete_document(&self, id: Uuid) -> Result<(), AppError> {
        tracing::info!("Soft deleting document: {id}");

        // Fetch the document to get collection_id for Chroma cleanup
        let doc = self.repo.get_document(id).await?;

        // Deactivate chunks first
        self.repo.deactivate_chunks(id).await?;

        // Deactivate the document itself
        self.repo.deactivate_document(id).await?;

        // Clean up Chroma embeddings if clients are available
        if let Some(ref chroma) = self.chroma_client {
            let collection_name = doc.collection_id.to_string();
            let filter = serde_json::json!({"document_id": id.to_string()});

            tracing::debug!(
                "Deleting Chroma embeddings for document {id} in collection {collection_name}"
            );

            if let Err(e) = chroma.delete_where(&collection_name, &filter).await {
                // Log but don't fail — database soft delete already succeeded
                tracing::warn!("Failed to delete Chroma embeddings for document {id}: {e}");
            } else {
                tracing::info!(
                    "Deleted Chroma embeddings for document {id} in collection {collection_name}"
                );
            }
        }

        Ok(())
    }

    /// Soft delete multiple documents and their chunks in the database.
    /// Rows remain in the database but are excluded from active queries.
    /// Also removes each document's embeddings from Chroma if clients are configured.
    pub async fn delete_documents_batch(
        &self,
        ids: Vec<Uuid>,
    ) -> Result<BatchDeleteResponse, AppError> {
        if ids.is_empty() {
            tracing::warn!("[DocumentService.delete_documents_batch] empty document id list");
            return Err(AppError::BadRequest("No document IDs provided".to_string()));
        }

        tracing::info!(
            "[DocumentService.delete_documents_batch] soft deleting documents: count={count}",
            count = ids.len()
        );
        tracing::debug!("[DocumentService.delete_documents_batch] requested document ids: {ids:?}");

        let documents = self.repo.get_documents_by_ids(&ids).await?;
        if documents.is_empty() {
            tracing::warn!(
                "[DocumentService.delete_documents_batch] no matching documents found: requested={count}",
                count = ids.len()
            );
            return Err(AppError::NotFound(
                "No matching documents found".to_string(),
            ));
        }

        let document_ids: Vec<Uuid> = documents.iter().map(|doc| doc.id).collect();
        tracing::debug!(
            "[DocumentService.delete_documents_batch] matched active/inactive documents: requested={requested}, matched={matched}",
            requested = ids.len(),
            matched = document_ids.len()
        );

        self.repo.deactivate_chunks_batch(&document_ids).await?;
        let deleted_count = self.repo.deactivate_documents_batch(&document_ids).await? as usize;

        if let Some(ref chroma) = self.chroma_client {
            let mut by_collection: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
            for doc in &documents {
                by_collection
                    .entry(doc.collection_id)
                    .or_default()
                    .push(doc.id);
            }

            for (collection_id, collection_doc_ids) in by_collection {
                let collection_name = collection_id.to_string();
                tracing::debug!(
                    "[DocumentService.delete_documents_batch] deleting Chroma embeddings: collection={collection_name}, count={count}",
                    count = collection_doc_ids.len()
                );

                for document_id in collection_doc_ids {
                    let filter = serde_json::json!({ "document_id": document_id.to_string() });
                    if let Err(e) = chroma.delete_where(&collection_name, &filter).await {
                        tracing::warn!(
                            "[DocumentService.delete_documents_batch] failed to delete Chroma embeddings: document={document_id}, collection={collection_name}, error={e}"
                        );
                    } else {
                        tracing::debug!(
                            "[DocumentService.delete_documents_batch] deleted Chroma embeddings: document={document_id}, collection={collection_name}"
                        );
                    }
                }
            }
        } else {
            tracing::debug!(
                "[DocumentService.delete_documents_batch] skipping Chroma cleanup: client not configured"
            );
        }

        tracing::info!(
            "[DocumentService.delete_documents_batch] completed soft delete: deleted_count={deleted_count}, requested={requested}",
            requested = ids.len()
        );

        Ok(BatchDeleteResponse {
            deleted_count,
            ids: document_ids,
        })
    }

    /// Reload/re-index a document: deactivate old chunks, parse new content,
    /// chunk it, and save new active chunks while preserving the document identity.
    /// Also updates Chroma embeddings if clients are configured.
    pub async fn reload_document(
        &self,
        data: &[u8],
        filename: &str,
        document_id: Uuid,
    ) -> Result<UploadResponse, AppError> {
        tracing::info!("Reloading document: {document_id}, filename: {filename}");

        // 1. Verify document exists (will error if not found)
        let doc = self.repo.get_document(document_id).await?;

        // 2. Validate and parse new file before changing active state.
        let file_type = validate_file(data, filename)?;
        let content = parse_file_content(data, filename, &file_type)
            .map_err(|e| AppError::FileError(format!("Failed to parse reloaded file: {e}")))?;

        // 3. Chunk the new content
        let chunks = crate::shared::chunking::chunk_document(&content);

        tracing::debug!(
            "Parsed {new_count} new chunks for document {document_id}",
            new_count = chunks.len()
        );

        // 4. Save new chunks as active, but keep old chunks active until Chroma indexing succeeds.
        let old_chunks = self.repo.get_active_chunks(document_id).await?;
        let old_count = old_chunks.len();
        let mut chunk_records = Vec::new();
        for chunk in &chunks {
            let chunk_record = crate::modules::documents::models::Chunk {
                id: Uuid::new_v4(),
                document_id,
                index: chunk.index,
                text: chunk.text.clone(),
                is_active: true,
            };
            self.repo.save_chunk(&chunk_record).await?;
            chunk_records.push(chunk_record);
        }

        // 5. Index new embeddings first. On failure, deactivate only the newly saved chunks.
        if let Err(e) = self
            .index_chunks_in_chroma(doc.collection_id, document_id, filename, &chunk_records)
            .await
        {
            let new_chunk_ids: Vec<Uuid> = chunk_records.iter().map(|c| c.id).collect();
            tracing::error!(
                "Reload indexing failed; preserving old active chunks and deactivating new chunks: document={document_id}, error={e}"
            );
            self.repo.deactivate_chunks_by_ids(&new_chunk_ids).await?;
            return Err(e);
        }

        // 6. Switch active version after new Chroma data is available.
        let old_chunk_ids: Vec<Uuid> = old_chunks.iter().map(|c| c.id).collect();
        self.repo.deactivate_chunks_by_ids(&old_chunk_ids).await?;
        self.repo
            .update_document_metadata(
                document_id,
                filename,
                &format!("{:?}", file_type),
                data.len() as i64,
            )
            .await?;

        // 7. Delete old Chroma embeddings if clients are available. This happens after
        // successful re-indexing so temporary Chroma failures do not remove the last searchable version.
        if let Some(ref chroma) = self.chroma_client {
            let collection_name = doc.collection_id.to_string();
            let old_ids: Vec<String> = old_chunk_ids.iter().map(Uuid::to_string).collect();
            if let Err(e) = chroma.delete_document(&collection_name, &old_ids).await {
                tracing::warn!(
                    "Failed to delete old Chroma embeddings for document {document_id}: {e}"
                );
            } else {
                tracing::debug!("Deleted old Chroma embeddings for document {document_id}");
            }
        }

        let new_count = chunks.len();
        tracing::info!(
            "Reload complete: document={document_id}, old_chunks={old_count}, new_chunks={new_count}"
        );

        Ok(UploadResponse {
            document_id,
            chunks_indexed: new_count,
            document_name: filename.to_string(),
        })
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

            // Enforce 10-file limit BEFORE extraction to avoid validating
            // malformed entries when Playwright or other clients send oversized ZIPs.
            if archive.len() > 10 {
                tracing::info!(
                    "ZIP has {} entries — exceeds limit of 10, rejecting early",
                    archive.len()
                );
                return Err(AppError::PayloadTooLarge(format!(
                    "ZIP contains more than 10 files (found {})",
                    archive.len()
                )));
            }

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

        // Process each extracted file asynchronously
        let mut processed = 0usize;
        let mut failed = 0usize;
        let mut items = Vec::new();
        // Accumulators for Chroma batch indexing
        let mut chroma_ids: Vec<String> = Vec::new();
        let mut chroma_texts: Vec<String> = Vec::new();
        let mut chroma_metadatas: Vec<serde_json::Value> = Vec::new();

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
                is_active: true,
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
            let mut file_chunk_records: Vec<Chunk> = Vec::new();
            let chunk_doc_name = name.clone();
            for chunk in &chunks {
                let chunk_record = Chunk {
                    id: Uuid::new_v4(),
                    document_id: doc_id,
                    index: chunk.index,
                    text: chunk.text.clone(),
                    is_active: true,
                };
                if let Err(e) = self.repo.save_chunk(&chunk_record).await {
                    tracing::warn!("Failed to save chunk {} for {name}: {e}", chunk.index);
                    save_ok = false;
                    break;
                }
                file_chunk_records.push(chunk_record);
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

            // Accumulate chunk data for Chroma batch indexing
            if self.chroma_client.is_some() && self.embedding_client.is_some() {
                for chunk_record in &file_chunk_records {
                    chroma_ids.push(chunk_record.id.to_string());
                    chroma_texts.push(chunk_record.text.clone());
                    chroma_metadatas.push(serde_json::json!({
                        "document_id": doc_id.to_string(),
                        "document_name": chunk_doc_name.clone(),
                        "chunk_id": chunk_record.id.to_string(),
                        "chunk_index": chunk_record.index,
                        "is_active": true,
                        "source": "upload",
                    }));
                }
            }
        }

        // Batch index all accumulated chunks into Chroma. If this fails, deactivate
        // the documents/chunks that were reported as processed so they are not visible
        // without searchable embeddings.
        if !chroma_ids.is_empty() {
            if let (Some(ref chroma), Some(ref embed)) =
                (&self.chroma_client, &self.embedding_client)
            {
                let collection_name = collection_id.to_string();

                let index_result = async {
                    let embeddings = embed.embed(chroma_texts).await?;
                    chroma
                        .add_embeddings(
                            &collection_name,
                            &chroma_ids,
                            &embeddings,
                            &chroma_metadatas,
                        )
                        .await
                }
                .await;

                if let Err(e) = index_result {
                    tracing::error!(
                        "ZIP indexing failed; deactivating processed batch rows: collection={collection_id}, error={e}"
                    );
                    for item in &items {
                        if let Some(document_id) = item.document_id {
                            self.repo.deactivate_chunks(document_id).await?;
                            self.repo.deactivate_document(document_id).await?;
                        }
                    }
                    return Err(e);
                }

                tracing::info!(
                    "Indexed {n} ZIP chunks into Chroma collection {col}",
                    n = chroma_ids.len(),
                    col = collection_name
                );
            }
        }

        tracing::info!(
            "ZIP upload complete: {processed}/{total} files processed",
            total = processed + failed
        );

        Ok(ZipUploadResponse {
            total_files: processed + failed,
            processed,
            failed,
            items,
        })
    }
    async fn index_chunks_in_chroma(
        &self,
        collection_id: Uuid,
        document_id: Uuid,
        document_name: &str,
        chunks: &[Chunk],
    ) -> Result<(), AppError> {
        if chunks.is_empty() {
            tracing::debug!("No chunks to index for document {document_id}");
            return Ok(());
        }

        let (Some(chroma), Some(embed)) = (&self.chroma_client, &self.embedding_client) else {
            tracing::debug!(
                "Skipping Chroma indexing for document {document_id}: clients not configured"
            );
            return Ok(());
        };

        let collection_name = collection_id.to_string();
        let chunk_texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        tracing::debug!(
            "Embedding {count} chunks before Chroma indexing: document={document_id}, collection={collection_name}",
            count = chunk_texts.len()
        );
        let embeddings = embed.embed(chunk_texts).await?;

        let ids: Vec<String> = chunks.iter().map(|c| c.id.to_string()).collect();
        let metadatas: Vec<serde_json::Value> = chunks
            .iter()
            .map(|c| {
                serde_json::json!({
                    "document_id": document_id.to_string(),
                    "document_name": document_name,
                    "chunk_id": c.id.to_string(),
                    "chunk_index": c.index,
                    "is_active": true,
                    "source": "upload",
                })
            })
            .collect();

        chroma
            .add_embeddings(&collection_name, &ids, &embeddings, &metadatas)
            .await?;

        tracing::info!(
            "Indexed {n} chunks into Chroma collection {col} for document {doc}",
            n = chunks.len(),
            col = collection_name,
            doc = document_id
        );

        Ok(())
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
    use serial_test::serial;
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
    #[serial]
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

        // Insert parent collection to satisfy FK constraint
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(collection_id)
            .bind(format!("test-collection-non-ascii-{collection_id}"))
            .bind("")
            .execute(svc.repo.db_pool())
            .await
            .expect("Failed to insert collection");

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
    #[serial]
    async fn test_process_upload_emoji_content() {
        // Regression: process_upload with 4-byte UTF-8 (emoji) must not panic
        let emoji_line = "😀🚀🌈🧪🔥🎉🎊🎈🎁\n".repeat(30);
        let content = format!("{emoji_line}\nMore emoji text 🎯🎲🎮🕹️🎰.\n");
        let data = content.as_bytes();
        let filename = "test-emoji.md";
        let collection_id = Uuid::new_v4();

        let svc = make_service().await;

        // Insert parent collection to satisfy FK constraint
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(collection_id)
            .bind(format!("test-collection-emoji-{collection_id}"))
            .bind("")
            .execute(svc.repo.db_pool())
            .await
            .expect("Failed to insert collection");

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
    #[serial]
    async fn test_process_upload_mixed_encoding() {
        // Regression: process_upload with mixed CJK + Cyrillic + emoji must not panic
        let mixed = "English text. Привет-мир-你好世界😀🚀\n".repeat(40);
        let content = format!("{mixed}\n\nEND");
        let data = content.as_bytes();
        let filename = "test-mixed.md";
        let collection_id = Uuid::new_v4();

        let svc = make_service().await;

        // Insert parent collection to satisfy FK constraint
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(collection_id)
            .bind(format!("test-collection-mixed-{collection_id}"))
            .bind("")
            .execute(svc.repo.db_pool())
            .await
            .expect("Failed to insert collection");

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
    #[serial]
    async fn test_process_upload_ascii_regression() {
        // Regression: ASCII-only upload must still work after UTF-8 fixes
        let content =
            "Hello, world!\n\nThis is a test document with ASCII text only.\n\nParagraph three.\n";
        let data = content.as_bytes();
        let filename = "test-ascii.md";
        let collection_id = Uuid::new_v4();

        let svc = make_service().await;

        // Insert parent collection to satisfy FK constraint
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(collection_id)
            .bind(format!("test-collection-ascii-{collection_id}"))
            .bind("")
            .execute(svc.repo.db_pool())
            .await
            .expect("Failed to insert collection");

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
    #[serial]
    async fn test_process_zip_with_5_md_files() {
        let data = make_zip(&[
            ("file1.md", "# File 1"),
            ("file2.md", "# File 2"),
            ("file3.md", "# File 3"),
            ("file4.md", "# File 4"),
            ("file5.md", "# File 5"),
        ]);
        let collection_id = Uuid::new_v4();
        let svc = make_service().await;

        // Insert parent collection to satisfy FK constraint
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(collection_id)
            .bind(format!("test-collection-zip-5-{collection_id}"))
            .bind("")
            .execute(svc.repo.db_pool())
            .await
            .expect("Failed to insert collection");

        let result = svc.process_zip_upload(&data, collection_id).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.processed, 5);
        assert_eq!(response.failed, 0);
        assert_eq!(response.total_files, 5);
    }

    #[tokio::test]
    #[serial]
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
    #[serial]
    async fn test_process_zip_mixed_valid_invalid() {
        let data = make_zip(&[
            ("valid.md", "# Valid"),
            ("script.exe", "fake exe"),
            ("notes.txt", "Plain text"),
        ]);
        let collection_id = Uuid::new_v4();
        let svc = make_service().await;

        // Insert parent collection to satisfy FK constraint
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(collection_id)
            .bind(format!("test-collection-zip-mixed-{collection_id}"))
            .bind("")
            .execute(svc.repo.db_pool())
            .await
            .expect("Failed to insert collection");

        let result = svc.process_zip_upload(&data, collection_id).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.processed > 0);
        assert!(response.processed < response.total_files);
    }

    #[tokio::test]
    #[serial]
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
    #[serial]
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
    #[serial]
    async fn test_process_zip_unsupported_types_skipped() {
        let data = make_zip(&[
            ("valid.md", "# Valid"),
            ("readme.txt", "Plain text"),
            ("app.exe", "binary"),
        ]);
        let collection_id = Uuid::new_v4();
        let svc = make_service().await;

        // Insert parent collection to satisfy FK constraint
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(collection_id)
            .bind(format!("test-collection-zip-unsupported-{collection_id}"))
            .bind("")
            .execute(svc.repo.db_pool())
            .await
            .expect("Failed to insert collection");

        let result = svc.process_zip_upload(&data, collection_id).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.processed >= 1);
        // .txt and .exe should be skipped, only .md processed
        assert!(response.failed > 0 || response.processed < response.total_files);
    }

    #[tokio::test]
    #[serial]
    async fn test_reload_document_deactivates_old_chunks_and_saves_new_active_chunks() {
        let svc = make_service().await;
        let collection_id = Uuid::new_v4();

        // Insert a collection for FK
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(collection_id)
            .bind(format!("test-collection-{collection_id}"))
            .bind("")
            .execute(svc.repo.db_pool())
            .await
            .expect("Failed to insert collection");

        // First upload: create document with initial content
        let initial_content = b"# Initial version\n\nThis is the first version of the document.";
        let upload_result = svc
            .process_upload(
                initial_content,
                "test.md",
                collection_id,
                "text/markdown".into(),
            )
            .await
            .expect("first upload should succeed");
        let doc_id = upload_result.document_id;

        // Reload with new content
        let reload_content =
            b"# Updated version\n\nThis is the reloaded version with different text.";
        svc.reload_document(reload_content, "test.md", doc_id)
            .await
            .expect("reload should succeed");

        // Assert: old chunks are inactive
        let old_chunks = svc
            .repo
            .get_chunks(doc_id)
            .await
            .expect("should fetch all chunks");
        assert!(
            !old_chunks.is_empty(),
            "there should be some chunks in the database"
        );

        // Check is_active via direct SQL for all chunks
        let rows: Vec<(Uuid, bool)> = sqlx::query_as(
            r#"SELECT id, is_active FROM chunks WHERE document_id = $1 ORDER BY "index""#,
        )
        .bind(doc_id)
        .fetch_all(svc.repo.db_pool())
        .await
        .expect("should query chunks");

        // Count active vs inactive
        let active_count = rows.iter().filter(|(_, active)| *active).count();
        let inactive_count = rows.iter().filter(|(_, active)| !*active).count();

        assert!(
            inactive_count > 0,
            "old chunks should be deactivated (found {inactive_count} inactive)"
        );
        assert!(
            active_count > 0,
            "new chunks should be active (found {active_count} active)"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_soft_delete_keeps_rows_but_removes_from_active_results() {
        let svc = make_service().await;
        let collection_id = Uuid::new_v4();

        // Insert a collection for FK
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(collection_id)
            .bind(format!("test-collection-del-{collection_id}"))
            .bind("")
            .execute(svc.repo.db_pool())
            .await
            .expect("Failed to insert collection");

        // Upload a document
        let content = b"# Test document\n\nThis document will be deleted.";
        let upload_result = svc
            .process_upload(
                content,
                "delete-me.md",
                collection_id,
                "text/markdown".into(),
            )
            .await
            .expect("upload should succeed");
        let doc_id = upload_result.document_id;

        // Confirm document is visible in active results
        let docs_before = svc
            .list_documents(collection_id)
            .await
            .expect("should list documents");
        assert!(
            docs_before.iter().any(|d| d.id == doc_id),
            "document should be listed before delete"
        );

        // Delete the document (soft delete)
        svc.delete_document(doc_id)
            .await
            .expect("soft delete should succeed");

        // Assert: document row still exists in the database
        let doc_row: Option<(Uuid, bool)> =
            sqlx::query_as("SELECT id, is_active FROM documents WHERE id = $1")
                .bind(doc_id)
                .fetch_optional(svc.repo.db_pool())
                .await
                .expect("should query document");
        assert!(
            doc_row.is_some(),
            "document row should still exist after soft delete"
        );
        let (_, is_active) = doc_row.unwrap();
        assert!(!is_active, "document should be marked inactive");

        // Assert: chunks remain but inactive
        let chunk_rows: Vec<(Uuid, bool)> =
            sqlx::query_as(r#"SELECT id, is_active FROM chunks WHERE document_id = $1"#)
                .bind(doc_id)
                .fetch_all(svc.repo.db_pool())
                .await
                .expect("should query chunks");
        assert!(
            !chunk_rows.is_empty(),
            "chunks should still exist after soft delete"
        );
        for (chunk_id, active) in &chunk_rows {
            assert!(
                !*active,
                "chunk {chunk_id} should be inactive after soft delete"
            );
        }

        // Assert: document does not appear in active listing
        let docs_after = svc
            .list_documents(collection_id)
            .await
            .expect("should list documents after delete");
        assert!(
            !docs_after.iter().any(|d| d.id == doc_id),
            "document should not appear in active listing after soft delete"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_batch_delete_keeps_rows_but_removes_from_active_results() {
        let svc = make_service().await;
        let collection_id = Uuid::new_v4();

        // Insert a collection
        sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
            .bind(collection_id)
            .bind(format!("test-collection-batch-{collection_id}"))
            .bind("")
            .execute(svc.repo.db_pool())
            .await
            .expect("Failed to insert collection");

        // Upload 3 documents
        let mut doc_ids = Vec::new();
        for i in 0..3 {
            let content = format!("# Document {i}\n\nThis is document number {i}.");
            let result = svc
                .process_upload(
                    content.as_bytes(),
                    &format!("doc-{i}.md"),
                    collection_id,
                    "text/markdown".into(),
                )
                .await
                .expect("upload should succeed");
            doc_ids.push(result.document_id);
        }

        // Confirm all 3 are visible
        let docs_before = svc
            .list_documents(collection_id)
            .await
            .expect("should list documents");
        assert_eq!(docs_before.len(), 3, "all 3 documents should be visible");

        // Delete 2 via batch
        let to_delete = vec![doc_ids[0], doc_ids[1]];
        let batch_result = svc
            .delete_documents_batch(to_delete)
            .await
            .expect("batch delete should succeed");
        assert_eq!(batch_result.deleted_count, 2);

        // Assert: remaining 1 is visible, 2 are invisible
        let docs_after = svc
            .list_documents(collection_id)
            .await
            .expect("should list documents after batch delete");
        assert_eq!(docs_after.len(), 1, "only 1 document should remain visible");
        assert!(
            docs_after.iter().any(|d| d.id == doc_ids[2]),
            "the third document should still be visible"
        );

        // Assert: rows still exist but are inactive
        for deleted_id in &[doc_ids[0], doc_ids[1]] {
            let doc_row: Option<(Uuid, bool)> =
                sqlx::query_as("SELECT id, is_active FROM documents WHERE id = $1")
                    .bind(*deleted_id)
                    .fetch_optional(svc.repo.db_pool())
                    .await
                    .expect("should query document");
            assert!(doc_row.is_some(), "deleted document row should still exist");
            assert!(!doc_row.unwrap().1, "document should be marked inactive");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_batch_delete_with_mixed_collections() {
        let svc = make_service().await;
        let collection_a = Uuid::new_v4();
        let collection_b = Uuid::new_v4();

        // Insert both collections
        for col_id in &[collection_a, collection_b] {
            sqlx::query("INSERT INTO collections (id, name, description) VALUES ($1, $2, $3)")
                .bind(*col_id)
                .bind(format!("col-{col_id}"))
                .bind("")
                .execute(svc.repo.db_pool())
                .await
                .expect("Failed to insert collection");
        }

        // Upload 2 docs to col A, 2 docs to col B
        let mut col_a_ids = Vec::new();
        let mut col_b_ids = Vec::new();

        for i in 0..2 {
            let content = format!("# Doc A{i}");
            let result = svc
                .process_upload(
                    content.as_bytes(),
                    &format!("a-{i}.md"),
                    collection_a,
                    "text/markdown".into(),
                )
                .await
                .expect("upload to col A should succeed");
            col_a_ids.push(result.document_id);
        }
        for i in 0..2 {
            let content = format!("# Doc B{i}");
            let result = svc
                .process_upload(
                    content.as_bytes(),
                    &format!("b-{i}.md"),
                    collection_b,
                    "text/markdown".into(),
                )
                .await
                .expect("upload to col B should succeed");
            col_b_ids.push(result.document_id);
        }

        // Delete 1 doc from col A + 1 doc from col B in one batch
        let to_delete = vec![col_a_ids[0], col_b_ids[0]];
        let result = svc
            .delete_documents_batch(to_delete)
            .await
            .expect("batch delete across collections should succeed");
        assert_eq!(result.deleted_count, 2);

        // Assert correct per-collection active state
        let docs_a = svc
            .list_documents(collection_a)
            .await
            .expect("should list col A");
        assert_eq!(docs_a.len(), 1, "col A should have 1 doc remaining");
        assert_eq!(
            docs_a[0].id, col_a_ids[1],
            "col A should keep the second doc"
        );

        let docs_b = svc
            .list_documents(collection_b)
            .await
            .expect("should list col B");
        assert_eq!(docs_b.len(), 1, "col B should have 1 doc remaining");
        assert_eq!(
            docs_b[0].id, col_b_ids[1],
            "col B should keep the second doc"
        );
    }

    /// Create a DocumentService with a PostgreSQL pool for testing.
    /// Uses the DATABASE_URL env var (default: postgres://vedo:vedo@localhost:5432/vedo_test).
    async fn make_service() -> DocumentService {
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://vedo:vedo@localhost:5432/vedo_test".to_string());
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .expect("Failed to connect to test database");

        // Migrations are already applied by the Docker test container.
        // Just truncate tables for a fresh state.
        sqlx::query(
            "TRUNCATE TABLE git_repositories, messages, sessions, chunks, documents, collections CASCADE",
        )
        .execute(&pool)
        .await
        .expect("Failed to truncate tables");

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
