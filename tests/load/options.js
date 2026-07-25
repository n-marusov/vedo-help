import http from 'k6/http';

// Shared options and configuration for VEDO hub k6 load tests
// Import in each test: import { BASE_URL, AUTH_TOKEN, COLLECTION_ID } from './options.js';

export const BASE_URL = __ENV.BASE_URL || 'http://localhost:3000';
export const AUTH_TOKEN = __ENV.AUTH_TOKEN || '';
export const COLLECTION_ID = __ENV.COLLECTION_ID || '';
export const LLM_MODEL = __ENV.LLM_MODEL || 'test/mock-model';

// Default threshold targets (from v1.0 milestone)
export const THRESHOLDS = {
  http_req_failed: ['rate<0.01'], // Less than 1% errors
  http_req_duration: ['p(95)<5000', 'p(99)<10000'], // P95 < 5s, P99 < 10s
};

// Common headers for API requests
export function getHeaders() {
  const headers = { 'Content-Type': 'application/json' };
  if (AUTH_TOKEN) {
    headers.Authorization = `Bearer ${AUTH_TOKEN}`;
  }
  return headers;
}

// Helper: perform a query against the RAG API
export function sendQuery(payload) {
  const url = `${BASE_URL}/api/query`;
  const response = http.post(url, JSON.stringify(payload), {
    headers: getHeaders(),
    timeout: '60s',
  });
  return response;
}

// Helper: check health endpoint
export function checkHealth() {
  const response = http.get(`${BASE_URL}/health`);
  return response.status === 200;
}
