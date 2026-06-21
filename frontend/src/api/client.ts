import type {
  BatchDeleteResponse,
  CreateRepoRequest,
  GitRepoSummary,
  SyncStatusResponse,
} from './types';

const API_BASE = '/api';

let accessToken: string | null = null;

export function setAccessToken(token: string | null) {
  accessToken = token;
}

export function getAccessToken(): string | null {
  return accessToken;
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
  if (accessToken) {
    headers.Authorization = `Bearer ${accessToken}`;
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
    if (accessToken) {
      headers.Authorization = `Bearer ${accessToken}`;
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
};
