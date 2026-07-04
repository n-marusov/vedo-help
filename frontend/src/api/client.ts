import { ref } from 'vue';
import type {
  BatchDeleteResponse,
  ChunkSearchParams,
  ChunkSearchResult,
  CollectionStats,
  CreateRepoRequest,
  EditMessageRequest,
  ExportFormat,
  GitRepoSummary,
  HealthReport,
  Message,
  Session,
  SessionSearchParams,
  SessionSummary,
  SyncStatusResponse,
} from './types';

const API_BASE = '/api';

/**
 * Reactive access token that Vue can track for computed dependencies.
 * Using ref() ensures that components re-evaluate when the token changes.
 */
const accessToken = ref<string | null>(null);

export function setAccessToken(token: string | null) {
  accessToken.value = token;
}

export function getAccessToken(): string | null {
  return accessToken.value;
}

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  if (accessToken.value) {
    headers.Authorization = `Bearer ${accessToken.value}`;
  }
  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: { message: res.statusText } }));
    throw new ApiError(res.status, body?.error?.message || res.statusText);
  }
  return res.json();
}

export const api = {
  get: <T>(path: string) => request<T>(path),
  post: <T>(path: string, body?: unknown) =>
    request<T>(path, {
      method: 'POST',
      body: body ? JSON.stringify(body) : undefined,
    }),
  del: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
  batchDeleteDocuments: (ids: string[]) =>
    request<BatchDeleteResponse>('/documents/batch', {
      method: 'DELETE',
      body: JSON.stringify({ ids }),
    }),

  // ── Git Sync ──
  getGitRepos: () => api.get<GitRepoSummary[]>('/git-sync/repos'),
  createGitRepo: (req: CreateRepoRequest) => api.post<GitRepoSummary>('/git-sync/repos', req),
  triggerSync: (id: string) => api.post<SyncStatusResponse>(`/git-sync/repos/${id}/sync`),
  deleteGitRepo: (id: string) => api.del<{ status: string; id: string }>(`/git-sync/repos/${id}`),
  upload: <T>(path: string, formData: FormData) => {
    const headers: Record<string, string> = {};
    if (accessToken.value) {
      headers.Authorization = `Bearer ${accessToken.value}`;
    }
    return fetch(`${API_BASE}${path}`, {
      method: 'POST',
      headers,
      body: formData,
    }).then(async (res) => {
      if (!res.ok) {
        throw new ApiError(
          res.status,
          (await res.json().catch(() => ({ error: { message: 'Upload failed' } }))).error?.message,
        );
      }
      return res.json() as Promise<T>;
    });
  },

  // ── Admin Session Debug ──
  adminSearchSessions: (params: SessionSearchParams) => {
    const query = new URLSearchParams();
    if (params.search) query.set('search', params.search);
    if (params.from) query.set('from', params.from);
    if (params.to) query.set('to', params.to);
    if (params.user_id) query.set('user_id', params.user_id);
    const qs = query.toString();
    return api.get<SessionSummary[]>(`/admin/sessions${qs ? `?${qs}` : ''}`);
  },
  getSessionWithMessages: (id: string) =>
    api.get<{ session: Session; messages: Message[] }>(`/sessions/${id}`),
  patch: <T>(path: string, body: unknown) =>
    request<T>(path, {
      method: 'PATCH',
      body: JSON.stringify(body),
    }),
  editMessage: (sessionId: string, messageId: string, req: EditMessageRequest) =>
    api.patch<Message>(`/sessions/${sessionId}/messages/${messageId}`, req),
  deleteMessage: (sessionId: string, messageId: string) =>
    api.del<Record<string, never>>(`/sessions/${sessionId}/messages/${messageId}`),
  exportSession: (sessionId: string, format: ExportFormat) => {
    const headers: Record<string, string> = {};
    if (accessToken.value) {
      headers.Authorization = `Bearer ${accessToken.value}`;
    }
    return fetch(`${API_BASE}/sessions/${sessionId}/export?format=${format}`, {
      headers,
    }).then(async (res) => {
      if (!res.ok) {
        throw new ApiError(res.status, 'Export failed');
      }
      return res.blob();
    });
  },

  // ── Health Check ──
  getHealthStatus: () => api.get<HealthReport>('/health/deep'),

  // ── Collection Stats ──
  getCollectionStats: (id: string) => api.get<CollectionStats>(`/collections/${id}/stats`),

  // ── Chunk Search ──
  searchChunks: (collectionId: string, params: ChunkSearchParams) => {
    const query = new URLSearchParams();
    if (params.q) query.set('q', params.q);
    if (params.search_type) query.set('search_type', params.search_type);
    if (params.source) query.set('source', params.source);
    if (params.limit !== undefined) query.set('limit', String(params.limit));
    if (params.offset !== undefined) query.set('offset', String(params.offset));
    if (params.top_k !== undefined) query.set('top_k', String(params.top_k));
    return api.get<ChunkSearchResult[]>(`/collections/${collectionId}/chunks?${query.toString()}`);
  },
};
