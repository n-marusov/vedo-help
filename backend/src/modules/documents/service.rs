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
