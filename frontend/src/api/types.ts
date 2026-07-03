export type FileType = Record<string, never>;

export interface Document {
  id: string;
  name: string;
  file_type: string;
  file_size: number;
  uploaded_at: string;
  collection_id: string;
  source?: string;
}

export interface BatchDeleteResponse {
  deleted_count: number;
  ids: string[];
}

export interface UploadResponse {
  document_id: string;
  chunks_indexed: number;
  document_name: string;
}

export interface Collection {
  id: string;
  name: string;
  description?: string;
  created_at: string;
  document_count: number;
}

export interface CreateCollectionRequest {
  name: string;
  description?: string;
}

export interface Session {
  id: string;
  title: string;
  collection_id?: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  pinned?: boolean;
}

export interface SessionSummary {
  id: string;
  title: string;
  message_count: number;
  created_at: string;
  updated_at: string;
  pinned?: boolean;
  collection_id?: string;
}

export interface Message {
  id: string;
  session_id: string;
  role: 'user' | 'assistant';
  content: string;
  sources?: string;
  created_at: string;
  edited_at?: string;
  original_content?: string;
  debug_data?: string;
}

export interface EditMessageRequest {
  content: string;
}

export type ExportFormat = 'json' | 'md';

export interface SourceRef {
  document_id: string;
  document_name: string;
  chunk_index: number;
  text: string;
  relevance: number;
}

export interface StreamEvent {
  type: 'chunk' | 'sources' | 'debug' | 'error' | 'done';
  data?: {
    text?: string;
    sources?: SourceRef[];
    debug?: unknown;
    user_message_id?: string;
    assistant_message_id?: string;
  };
  // Legacy fallback — some events may have fields at top level
  text?: string;
  sources?: SourceRef[];
  user_message_id?: string;
  assistant_message_id?: string;
}

export interface QueryRequest {
  collection_id: string;
  query: string;
  session_id?: string;
}

export interface ZipUploadItem {
  filename: string;
  status: string;
  document_id: string | null;
  error: string | null;
}

export interface ZipUploadResponse {
  total_files: number;
  processed: number;
  failed: number;
  items: ZipUploadItem[];
}

export interface GitRepoSummary {
  id: string;
  url: string;
  branch: string;
  local_path: string;
  last_commit_hash?: string;
  last_synced_at?: string;
  collection_id: string;
  collection_name: string;
  status: 'idle' | 'syncing' | 'error';
  created_at: string;
  updated_at: string;
}

export interface CreateRepoRequest {
  url: string;
  branch?: string;
  access_token?: string;
  collection_id: string;
}

export interface SyncStatusResponse {
  repo_id: string;
  status: string;
  files_indexed: number;
  chunks_total: number;
  last_commit?: string;
  error?: string;
}

// ── Session Debug Types (Admin Panel) ──

export interface SearchResultItem {
  chunk_id: string;
  document_name: string;
  chunk_index: number;
  score: number;
  text_snippet: string;
}

export interface EmbeddingSearchStep {
  query_snippet: string;
  embedding_dimension: number;
  latency_ms: number;
  collection_name: string;
  top_k: number;
  result_count: number;
  retries: number;
  results: SearchResultItem[];
}

export interface FinalAnswerStep {
  model: string;
  max_retries: number;
  chunks_in_context: number;
  history_message_count: number;
  history_token_estimate: number;
  token_budget: number;
  total_tokens_estimate: number;
  latency_ms: number;
  prompt_preview: string;
}

export interface DebugData {
  query_text: string;
  multi_query: null | Record<string, unknown>;
  hyde: null | Record<string, unknown>;
  embedding_search: EmbeddingSearchStep | null;
  keyword_search: null | Record<string, unknown>;
  merge_dedup: null | Record<string, unknown>;
  reranking: null | Record<string, unknown>;
  final_answer: FinalAnswerStep | null;
}

export interface SessionSearchParams {
  search?: string;
  from?: string;
  to?: string;
}

// ── Health Check Types ──

export interface ServiceCheck {
  name: string;
  status: 'healthy' | 'unhealthy' | 'degraded';
  latency_ms: number;
  error?: string;
}

export interface HealthReport {
  status: 'healthy' | 'degraded' | 'unhealthy';
  checks: ServiceCheck[];
  timestamp: string;
}

// ── Document Summary (with source) ──

export interface DocumentSummary {
  id: string;
  name: string;
  file_type: string;
  file_size: number;
  uploaded_at: string;
  collection_id: string;
  is_active: boolean;
  source: string;
}

// ── Collection Stats & Chunk Search Types ──

export interface CollectionStats {
  total_documents: number;
  total_chunks: number;
  total_git_repos: number;
  upload_documents: number;
  git_documents: number;
  upload_chunks: number;
  git_chunks: number;
  total_file_size_bytes: number;
  document_types: Record<string, number>;
}

export interface ChunkSearchResult {
  chunk_id: string;
  document_id: string;
  document_name: string;
  chunk_index: number;
  text: string;
  source: string;
  score: number | null;
  file_path: string | null;
}

export interface ChunkSearchParams {
  q?: string;
  search_type?: 'text' | 'semantic';
  source?: 'upload' | 'git';
  limit?: number;
  offset?: number;
  top_k?: number;
}
