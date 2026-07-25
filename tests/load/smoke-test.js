// Smoke test — single-user, single-query to verify test setup
// 1 VU, 1 iteration — validates that the system responds correctly
// under minimal load before running heavier scenarios.

import { check, sleep } from 'k6';
import http from 'k6/http';
import { BASE_URL, THRESHOLDS, getHeaders } from './options.js';

export const options = {
  vus: 1,
  iterations: 1,
  thresholds: THRESHOLDS,
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(95)', 'p(99)'],
};

export default function () {
  // Health check
  const healthResp = http.get(`${BASE_URL}/health`);
  check(healthResp, {
    'health endpoint returns 200': (r) => r.status === 200,
  });

  // Simple query
  const payload = {
    message: 'What is this project about?',
    collection_id: __ENV.COLLECTION_ID || '',
    stream: false,
  };

  const queryResp = http.post(`${BASE_URL}/api/query`, JSON.stringify(payload), {
    headers: getHeaders(),
    timeout: '120s',
  });

  check(queryResp, {
    'query returns 200': (r) => r.status === 200,
    'query response has content': (r) => r.body.length > 0,
  });

  sleep(1);
}
