// Stress test — find breaking point
// Step ramp-up: add users every 2 minutes until error rate exceeds 5%.
// Identifies the maximum concurrent users the system can handle.

import { check, sleep } from 'k6';
import http from 'k6/http';
import { BASE_URL, getHeaders } from './options.js';

// Step ramp: 5 users every 2 minutes, up to 50
const STEPS = 10;
const USERS_PER_STEP = 5;

function generateStages() {
  const stages = [];
  for (let i = 1; i <= STEPS; i++) {
    stages.push({ duration: '2m', target: i * USERS_PER_STEP });
  }
  // Stay at max for a moment before letting k6 finish
  stages.push({ duration: '1m', target: STEPS * USERS_PER_STEP });
  return stages;
}

export const options = {
  stages: generateStages(),
  thresholds: {
    http_req_failed: ['rate<0.05'], // Abort criterion: >5% errors
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(95)', 'p(99)'],
  // No explicit abort — the test tracks failures per stage
  // and the operator stops early if error rate exceeds 5%.
};

const QUERIES = [
  'How do I upload documents?',
  'What is RAG?',
  'How does authentication work?',
  'Explain collections',
  'What models are supported?',
  'How do I configure the system?',
  'What is Hybrid search?',
  'How do I delete documents?',
];

export default function () {
  // Health check (every 10th iteration)
  if (__ITER % 10 === 0) {
    const hc = http.get(`${BASE_URL}/health`);
    check(hc, { 'health ok': (r) => r.status === 200 });
  }

  // Query
  const query = QUERIES[Math.floor(Math.random() * QUERIES.length)];
  const payload = {
    message: query,
    collection_id: __ENV.COLLECTION_ID || '',
    stream: false,
  };

  const resp = http.post(`${BASE_URL}/api/query`, JSON.stringify(payload), {
    headers: getHeaders(),
    timeout: '120s',
  });

  check(resp, {
    'status 200': (r) => r.status === 200,
    'has body': (r) => r.body.length > 0,
  });

  // Minimal think time for stress test (1-2 seconds)
  sleep(1 + Math.random() * 1);
}
