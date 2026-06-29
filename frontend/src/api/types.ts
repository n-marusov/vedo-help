export type FileType = Record<string, never>;

export interface Document {
  id: string;
  name: string;
  file_type: string;
  file_size: number;
  uploaded_at: string;
  collection_id: string;
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
  stage?: string;
  rerank_score?: number;
  rerank_verdict?: string;
  rerank_comment?: string;
  keyword_matches?: string[];
}

export interface PipelineStageEvent {
  stage: string;
  data: Record<string, unknown>;
  latency_ms: number;
}

export interface PipelineMetric {
  total_latency_ms: number;
  step_timings: Record<string, number>;
}

export interface StreamEvent {
  type: 'chunk' | 'sources' | 'pipeline_stage' | 'debug' | 'error' | 'done';
  data?: {
    text?: string;
    sources?: SourceRef[];
    debug?: unknown;
    stage?: PipelineStageEvent;
    user_message_id?: string;
    assistant_message_id?: string;
  };
  // Legacy fallback — some events may have fields at top level
  text?: string;
  sources?: SourceRef[];
  stage?: PipelineStageEvent;
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

export interface MultiQueryData {
  original_query: string;
  variants: string[];
  latency_ms: number;
}

export interface HydeResult {
  query: string;
  hypothetical_doc: string;
  latency_ms: number;
}

export interface HydeData {
  per_query: HydeResult[];
}

export interface KeywordSearchData {
  query_tokens: string[];
  total_matches: number;
  results: SearchResultItem[];
  latency_ms: number;
}

export interface MergeSourceBreakdown {
  vector_chunks: number;
  keyword_chunks: number;
}

export interface MergeDedupData {
  input_chunks: number;
  after_dedup: number;
  source_breakdown: MergeSourceBreakdown;
}

export interface RerankResult {
  chunk_id: string;
  score: number;
  verdict: string;
  comment: string;
}

export interface RerankingData {
  input_count: number;
  accepted: number;
  rejected: number;
  results: RerankResult[];
}

export interface DebugData {
  query_text: string;
  multi_query: MultiQueryData | null;
  hyde: HydeData | null;
  embedding_search: EmbeddingSearchStep | null;
  keyword_search: KeywordSearchData | null;
  merge_dedup: MergeDedupData | null;
  reranking: RerankingData | null;
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
