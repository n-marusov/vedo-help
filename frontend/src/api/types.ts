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
  user_name?: string | null;
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
  stage?: string;
  rerank_score?: number;
  rerank_verdict?: string;
}

export interface StreamEvent {
  type: 'chunk' | 'sources' | 'debug' | 'error' | 'done' | 'pipeline_stage';
  data?: {
    text?: string;
    sources?: SourceRef[];
    debug?: unknown;
    user_message_id?: string;
    assistant_message_id?: string;
    stage_name?: string;
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

export interface SyncProgress {
  total_files: number;
  indexed_files: number;
  current_file: string;
  phase: string;
}

export interface SyncStatusResponse {
  repo_id: string;
  status: string;
  files_indexed: number;
  chunks_total: number;
  last_commit?: string;
  error?: string;
  progress?: SyncProgress;
}

// ── Web Crawl Types ──

export interface CrawlJobSummary {
  id: string;
  entry_url: string;
  config: Record<string, unknown>;
  status: 'idle' | 'crawling' | 'completed' | 'cancelled' | 'error';
  pages_found: number;
  pages_indexed: number;
  collection_id: string;
  collection_name: string;
  created_at: string;
  updated_at: string;
}

export interface CreateCrawlJobRequest {
  entry_url: string;
  collection_id: string;
  config?: {
    max_depth?: number;
    max_pages?: number;
    delay_ms?: number;
    path_prefix?: string;
  };
}

export interface CrawlPage {
  id: string;
  job_id: string;
  url: string;
  depth: number;
  status: string;
  http_status?: number;
  title?: string;
  created_at: string;
}

export interface CrawlJobDetailResponse {
  id: string;
  entry_url: string;
  config: Record<string, unknown>;
  status: string;
  pages_found: number;
  pages_indexed: number;
  collection_id: string;
  collection_name: string;
  error_message?: string;
  created_at: string;
  updated_at: string;
  pages: CrawlPage[];
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

export interface MultiQueryStep {
  original_query: string;
  variants: string[];
  latency_ms: number;
}

export interface HydeResult {
  query: string;
  hypothetical_doc: string;
  latency_ms: number;
}

export interface HydeStep {
  per_query: HydeResult[];
}

export interface KeywordSearchStep {
  query_tokens: string[];
  total_matches: number;
  results: SearchResultItem[];
  latency_ms: number;
}

export interface MergeSourceBreakdown {
  vector_chunks: number;
  keyword_chunks: number;
}

export interface MergeDedupStep {
  input_chunks: number;
  after_dedup: number;
  source_breakdown: MergeSourceBreakdown;
  results: SearchResultItem[];
  deduped_ids: string[];
}

export interface RerankResult {
  chunk_id: string;
  score: number;
  verdict: string;
  comment: string;
}

export interface RerankingStep {
  input_count: number;
  accepted: number;
  rejected: number;
  results: RerankResult[];
}

export interface DebugData {
  query_text: string;
  multi_query: MultiQueryStep | null;
  hyde: HydeStep | null;
  embedding_search: EmbeddingSearchStep | null;
  keyword_search: KeywordSearchStep | null;
  merge_dedup: MergeDedupStep | null;
  reranking: RerankingStep | null;
  final_answer: FinalAnswerStep | null;
}

export interface SessionSearchParams {
  search?: string;
  from?: string;
  to?: string;
  user_name?: string;
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

// ── Model Lists (from backend /api/admin/models) ──

export interface ModelOption {
  value: string;
  label: string;
  pricing?: string;
}

export interface ModelsResponse {
  llm_models: ModelOption[];
  embedding_models: ModelOption[];
  rerank_models: ModelOption[];
}

export interface ChunkSearchParams {
  q?: string;
  search_type?: 'text' | 'semantic';
  source?: 'upload' | 'git' | 'web';
  limit?: number;
  offset?: number;
  top_k?: number;
}
