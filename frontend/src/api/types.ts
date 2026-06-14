export type FileType = Record<string, never>;

export interface Document {
  id: string;
  name: string;
  file_type: string;
  file_size: number;
  uploaded_at: string;
  collection_id: string;
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
}

export interface SessionSummary {
  id: string;
  title: string;
  message_count: number;
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: string;
  session_id: string;
  role: 'user' | 'assistant';
  content: string;
  sources?: string;
  created_at: string;
}

export interface SourceRef {
  document_id: string;
  document_name: string;
  chunk_index: number;
  text: string;
  relevance: number;
}

export interface StreamEvent {
  type: 'chunk' | 'sources' | 'error' | 'done';
  text?: string;
  sources?: SourceRef[];
}

export interface QueryRequest {
  collection_id: string;
  query: string;
  session_id?: string;
}
