use std::collections::HashMap;

use uuid::Uuid;

use crate::modules::collections::repository::CollectionRepository;
use crate::modules::documents::models::{
    BatchDeleteResponse, Chunk, Document, DocumentSummary, UploadResponse, ZipUploadItem,
    ZipUploadResponse,
};
use crate::modules::documents::repository::DocumentRepository;
use crate::shared::chroma_client::ChromaClient;
use crate::shared::chunking::chunk_document_default;
use crate::shared::embedding_client::EmbeddingClient;
use crate::shared::error::AppError;
use crate::shared::file_validation::{validate_file, MAX_FILE_SIZE};
use crate::shared::types::FileType;

/// Service for document management operations.
#[derive(Clone, Debug)]
pub struct DocumentService {
    pub(crate) repo: DocumentRepository,
    pub(crate) collection_repo: CollectionRepository,
    pub(crate) chroma_client: Option<ChromaClient>,
    pub(crate) embedding_client: Option<EmbeddingClient>,
    pub(crate) settings_service: Option<crate::modules::settings::service::SettingsService>,
}

impl DocumentService {
    /// Create a new DocumentService with only a repository (no Chroma/embedding).
    pub fn new(repo: DocumentRepository, collection_repo: CollectionRepository) -> Self {
        Self {
            repo,
            collection_repo,
            chroma_client: None,
            embedding_client: None,
            settings_service: None,
        }
    }

    /// Create a DocumentService with Chroma and embedding clients.
    pub fn with_clients(
        repo: DocumentRepository,
        collection_repo: CollectionRepository,
        chroma_client: ChromaClient,
        embedding_client: EmbeddingClient,
        settings_service: Option<crate::modules::settings::service::SettingsService>,
    ) -> Self {
        Self {
            repo,
            collection_repo,
            chroma_client: Some(chroma_client),
            embedding_client: Some(embedding_client),
            settings_service,
        }
    }

    /// Process a document upload: validate, parse, chunk, save.
    pub async fn process_upload(
        &self,
        data: &[u8],
        filename: &str,
        collection_id: Uuid,
        _content_type: String,
        user_id: &str,
        is_admin: bool,
    ) -> Result<UploadResponse, AppError> {
        // 0. Verify collection ownership
        self.collection_repo
            .get_collection_for_user(collection_id, user_id, is_admin)
            .await?;

        // 1. Validate file
        let file_type = validate_file(data, filename)?;
        tracing::info!(
            component = "documents/service",
            file_name = %filename,
            file_type = ?file_type,
            file_size = data.len(),
            collection_id = %collection_id,
            user_id = %user_id,
            "document.upload.started"
        );

        // 2. Parse file into text
        let text = parse_file_content(data, filename, &file_type)?;
        tracing::info!(component = "documents/service", file_name = %filename, text_length = text.len(), "document.parsed");

        // 3. Chunk text
        let chunks = chunk_document_default(&text);
        tracing::debug!(
            component = "documents/service",
            chunk_index = if chunks.is_empty() { 0 } else { 1 },
            chunk_preview = chunks
                .first()
                .map(|c| {
                    let end = c
                        .text
                        .char_indices()
                        .map(|(i, _)| i)
                        .chain(std::iter::once(c.text.len()))
                        .rfind(|&i| i <= 80)
                        .unwrap_or(0);
                    &c.text[..end]
                })
                .unwrap_or(""),
            "document.chunk_start"
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
            source: "upload".to_string(),
            user_id: user_id.to_string(),
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
                component = "documents/service",
                document_id = %doc_id,
                error = %e,
                "document.upload.indexing_failed"
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

    /// List documents in a collection with user ownership scoping.
    pub async fn list_documents(
        &self,
        collection_id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<DocumentSummary>, AppError> {
        // Verify collection ownership first
        self.collection_repo
            .get_collection_for_user(collection_id, user_id, is_admin)
            .await?;

        let documents = self
            .repo
            .list_documents_for_user(collection_id, user_id, is_admin)
            .await?;
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
                source: d.source,
            })
            .collect();
        Ok(summaries)
    }

    /// Soft delete a document with ownership verification.
    /// Non-admin users can only delete their own documents.
    pub async fn delete_document(
        &self,
        id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<(), AppError> {
        tracing::info!(component = "documents/service", document_id = %id, user_id = %user_id, "document.delete.soft_start");

        // Fetch the document with ownership check
        let doc = self
            .repo
            .get_document_for_user(id, user_id, is_admin)
            .await?;

        // Deactivate chunks first
        self.repo.deactivate_chunks(id).await?;

        // Deactivate the document itself
        self.repo
            .deactivate_document_for_user(id, user_id, is_admin)
            .await?;

        // Clean up Chroma embeddings if clients are available
        if let Some(ref chroma) = self.chroma_client {
            let collection_name = doc.collection_id.to_string();
            let filter = serde_json::json!({"document_id": id.to_string()});

            tracing::debug!(
                component = "documents/service",
                document_id = %id,
                collection_name = %collection_name,
                "document.delete.chroma_delete_start"
            );

            if let Err(e) = chroma.delete_where(&collection_name, &filter).await {
                // Log but don't fail — database soft delete already succeeded
                tracing::warn!(
                    component = "documents/service",
                    document_id = %id,
                    error = %e,
                    "document.delete.chroma_delete_failed"
                );
            } else {
                tracing::info!(
                    component = "documents/service",
                    document_id = %id,
                    collection_name = %collection_name,
                    "document.delete.chroma_deleted"
                );
            }
        }

        Ok(())
    }

    /// Soft delete multiple documents and their chunks with ownership verification.
    /// Non-admin users can only delete their own documents.
    pub async fn delete_documents_batch(
        &self,
        ids: Vec<Uuid>,
        user_id: &str,
        is_admin: bool,
    ) -> Result<BatchDeleteResponse, AppError> {
        if ids.is_empty() {
            tracing::warn!(
                component = "documents/service",
                "documents.batch_delete.empty_ids"
            );
            return Err(AppError::BadRequest("No document IDs provided".to_string()));
        }

        tracing::info!(
            component = "documents/service",
            request_count = ids.len(),
            user_id = %user_id,
            "documents.batch_delete.start"
        );
        tracing::debug!(
            component = "documents/service",
            document_ids = ?ids,
            "documents.batch_delete.ids"
        );

        let documents = self.repo.get_documents_by_ids(&ids).await?;
        if documents.is_empty() {
            tracing::warn!(
                component = "documents/service",
                request_count = ids.len(),
                "documents.batch_delete.no_matches"
            );
            return Err(AppError::NotFound(
                "No matching documents found".to_string(),
            ));
        }

        // Filter by ownership: non-admin users can only delete their own documents
        let documents: Vec<Document> = if is_admin {
            documents
        } else {
            documents
                .into_iter()
                .filter(|d| d.user_id == user_id)
                .collect()
        };

        if documents.is_empty() {
            return Err(AppError::NotFound(
                "No matching documents found".to_string(),
            ));
        }

        let document_ids: Vec<Uuid> = documents.iter().map(|doc| doc.id).collect();
        tracing::debug!(
            component = "documents/service",
            request_count = ids.len(),
            matched_count = document_ids.len(),
            "documents.batch_delete.matched"
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
                    component = "documents/service",
                    collection_name = %collection_name,
                    count = collection_doc_ids.len(),
                    "documents.batch_delete.chroma_delete_start"
                );

                for document_id in collection_doc_ids {
                    let filter = serde_json::json!({ "document_id": document_id.to_string() });
                    if let Err(e) = chroma.delete_where(&collection_name, &filter).await {
                        tracing::warn!(
                            component = "documents/service",
                            document_id = %document_id,
                            collection_name = %collection_name,
                            error = %e,
                            "documents.batch_delete.chroma_delete_failed"
                        );
                    } else {
                        tracing::debug!(
                            component = "documents/service",
                            document_id = %document_id,
                            collection_name = %collection_name,
                            "documents.batch_delete.chroma_deleted"
                        );
                    }
                }
            }
        } else {
            tracing::debug!(
                component = "documents/service",
                "documents.batch_delete.chroma_skipped"
            );
        }

        tracing::info!(
            component = "documents/service",
            deleted_count = deleted_count,
            request_count = ids.len(),
            "documents.batch_delete.complete"
        );

        Ok(BatchDeleteResponse {
            deleted_count,
            ids: document_ids,
        })
    }

    /// Reload/re-index a document with ownership verification.
    pub async fn reload_document(
        &self,
        data: &[u8],
        filename: &str,
        document_id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<UploadResponse, AppError> {
        tracing::info!(
            component = "documents/service",
            document_id = %document_id,
            file_name = %filename,
            "document.reload.started"
        );

        // 1. Verify document exists and user owns it
        let doc = self
            .repo
            .get_document_for_user(document_id, user_id, is_admin)
            .await?;

        // 2. Validate and parse new file before changing active state.
        let file_type = validate_file(data, filename)?;
        let content = parse_file_content(data, filename, &file_type)
            .map_err(|e| AppError::FileError(format!("Failed to parse reloaded file: {e}")))?;

        // 3. Chunk the new content
        let chunks = crate::shared::chunking::chunk_document_default(&content);

        tracing::debug!(
            component = "documents/service",
            document_id = %document_id,
            new_chunk_count = chunks.len(),
            "document.reload.parsed"
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
                component = "documents/service",
                document_id = %document_id,
                error = %e,
                "document.reload.indexing_failed"
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
                    component = "documents/service",
                    document_id = %document_id,
                    error = %e,
                    "document.reload.old_embeddings_delete_failed"
                );
            } else {
                tracing::debug!(
                    component = "documents/service",
                    document_id = %document_id,
                    "document.reload.old_embeddings_deleted"
                );
            }
        }

        let new_count = chunks.len();
        tracing::info!(
            component = "documents/service",
            document_id = %document_id,
            old_chunk_count = old_count,
            new_chunk_count = new_count,
            "document.reload.complete"
        );

        Ok(UploadResponse {
            document_id,
            chunks_indexed: new_count,
            document_name: filename.to_string(),
        })
    }

    /// Process a ZIP batch upload with ownership verification.
    pub async fn process_zip_upload(
        &self,
        data: &[u8],
        collection_id: Uuid,
        user_id: &str,
        is_admin: bool,
    ) -> Result<ZipUploadResponse, AppError> {
        use std::io::Cursor;
        use std::io::Read;

        // Verify collection ownership
        self.collection_repo
            .get_collection_for_user(collection_id, user_id, is_admin)
            .await?;

        tracing::info!(
            component = "documents/service",
            collection_id = %collection_id,
            file_size = data.len(),
            user_id = %user_id,
            "zip.upload.started"
        );

        // Read all entries into memory synchronously (ZipFile is not Send)
        // Returns: Vec<(filename, file_data)>
        let extracted = {
            let reader = Cursor::new(data);
            let mut archive = zip::ZipArchive::new(reader)
                .map_err(|e| AppError::FileError(format!("Invalid ZIP file: {e}")))?;

            tracing::debug!(
                component = "documents/service",
                entry_count = archive.len(),
                "zip.open"
            );

            // Enforce 10-file limit BEFORE extraction to avoid validating
            // malformed entries when Playwright or other clients send oversized ZIPs.
            if archive.len() > 10 {
                tracing::info!(
                    component = "documents/service",
                    entry_count = archive.len(),
                    "zip.too_many_entries"
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
                        tracing::warn!(
                            component = "documents/service",
                            entry_index = idx,
                            error = %e,
                            "zip.entry.read_failed"
                        );
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
                        tracing::warn!(
                            component = "documents/service",
                            raw_name = %raw,
                            "zip.entry.no_filename"
                        );
                        raw
                    });
                let entry_size = entry.size();
                tracing::debug!(
                    component = "documents/service",
                    file_name = %name,
                    entry_size = entry_size,
                    "zip.entry.extracting"
                );

                // Guard against zip bombs: reject entries whose declared uncompressed size
                // exceeds the limit before allocating (prevents Vec::with_capacity OOM).
                if entry_size > MAX_FILE_SIZE {
                    tracing::warn!(
                        component = "documents/service",
                        file_name = %name,
                        entry_size = entry_size,
                        "zip.entry.size_exceeds_limit"
                    );
                    continue;
                }

                let mut entry_data = Vec::with_capacity(entry_size as usize);
                // Use a size-bounded reader as defense-in-depth against decompression bombs
                // (a ZIP can advertise a small uncompressed size while expanding to gigabytes).
                let mut limited = entry.take(MAX_FILE_SIZE + 1);
                if limited.read_to_end(&mut entry_data).is_err() {
                    tracing::warn!(
                        component = "documents/service",
                        file_name = %name,
                        "zip.entry.read_failed"
                    );
                    continue;
                }

                // Post-check: if the bounded reader filled past MAX_FILE_SIZE, the decompressed
                // data exceeded the limit and was truncated — reject it.
                if entry_data.len() as u64 > MAX_FILE_SIZE {
                    tracing::warn!(
                        component = "documents/service",
                        file_name = %name,
                        max_size_mb = MAX_FILE_SIZE / (1024 * 1024),
                        "zip.entry.decompressed_exceeds_limit"
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
            tracing::debug!(
                component = "documents/service",
                file_name = %name,
                file_size = entry_data.len(),
                "zip.entry.processing"
            );

            // Detect inner file type by extension
            let inner_file_type = match crate::shared::types::FileType::from_extension(&name) {
                Some(ft) => ft,
                None => {
                    tracing::warn!(
                        component = "documents/service",
                        file_name = %name,
                        "zip.entry.unsupported_extension"
                    );
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
                tracing::warn!(
                    component = "documents/service",
                    file_name = %name,
                    error = %e,
                    "zip.entry.validation_failed"
                );
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
                    tracing::warn!(
                        component = "documents/service",
                        file_name = %name,
                        error = %e,
                        "zip.entry.parse_failed"
                    );
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
            let chunks = chunk_document_default(&text);
            tracing::debug!(
                component = "documents/service",
                file_name = %name,
                chunk_count = chunks.len(),
                "zip.entry.chunked"
            );

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
                source: "upload".to_string(),
                user_id: user_id.to_string(),
            };

            // Save document (async, no ZipFile borrow)
            if let Err(e) = self.repo.save_document(&doc).await {
                tracing::warn!(
                    component = "documents/service",
                    file_name = %name,
                    error = %e,
                    "zip.entry.save_failed"
                );
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
                    tracing::warn!(
                        component = "documents/service",
                        chunk_index = chunk.index,
                        file_name = %name,
                        error = %e,
                        "zip.entry.chunk_save_failed"
                    );
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

                let mut embedding_model =
                    crate::shared::embedding_client::DEFAULT_EMBEDDING_MODEL.to_string();
                if let Some(ref settings) = self.settings_service {
                    if let Ok(rag_settings) = settings.get_rag_settings().await {
                        embedding_model = rag_settings.embedding_model;
                    }
                }

                let index_result = async {
                    let embeddings = embed.embed(&embedding_model, chroma_texts.clone()).await?;
                    chroma
                        .add_embeddings(
                            &collection_name,
                            &chroma_ids,
                            &embeddings,
                            &chroma_metadatas,
                            &chroma_texts,
                        )
                        .await
                }
                .await;

                if let Err(e) = index_result {
                    tracing::error!(
                        component = "documents/service",
                        collection_id = %collection_id,
                        error = %e,
                        "zip.upload.indexing_failed"
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
                    component = "documents/service",
                    chunk_count = chroma_ids.len(),
                    collection_name = %collection_name,
                    "zip.upload.indexed"
                );
            }
        }

        tracing::info!(
            component = "documents/service",
            processed_count = processed,
            total_count = processed + failed,
            "zip.upload.complete"
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
            tracing::debug!(component = "documents/service", document_id = %document_id, "chunks.index.skipped_empty");
            return Ok(());
        }

        let (Some(chroma), Some(embed)) = (&self.chroma_client, &self.embedding_client) else {
            tracing::debug!(
                component = "documents/service",
                document_id = %document_id,
                "chunks.index.skipped_no_clients"
            );
            return Ok(());
        };

        let collection_name = collection_id.to_string();
        let chunk_texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        tracing::debug!(
            component = "documents/service",
            chunk_count = chunk_texts.len(),
            document_id = %document_id,
            collection_name = %collection_name,
            "chunks.index.embedding_start"
        );
        let mut embedding_model =
            crate::shared::embedding_client::DEFAULT_EMBEDDING_MODEL.to_string();
        if let Some(ref settings) = self.settings_service {
            if let Ok(rag_settings) = settings.get_rag_settings().await {
                embedding_model = rag_settings.embedding_model;
            }
        }

        let embeddings = embed.embed(&embedding_model, chunk_texts.clone()).await?;

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
            .add_embeddings(
                &collection_name,
                &ids,
                &embeddings,
                &metadatas,
                &chunk_texts,
            )
            .await?;

        tracing::info!(
            component = "documents/service",
            chunk_count = chunks.len(),
            collection_name = %collection_name,
            document_id = %document_id,
            "chunks.indexed"
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
