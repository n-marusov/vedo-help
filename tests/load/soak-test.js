// Soak test — extended run at moderate load
// Runs for 30 minutes at 5 concurrent users to detect memory leaks,
// connection pool exhaustion, and gradual performance degradation.

import { check, group, sleep } from 'k6';
import http from 'k6/http';
import { BASE_URL, THRESHOLDS, getHeaders } from './options.js';

const MODERATE_VUS = 5;

export const options = {
  stages: [
    { duration: '2m', target: MODERATE_VUS }, // Ramp-up
    { duration: '30m', target: MODERATE_VUS }, // Soak (30 min)
    { duration: '1m', target: 0 }, // Ramp-down
  ],
  thresholds: {
    ...THRESHOLDS,
    // Stricter error threshold for soak — no errors expected
    http_req_failed: ['rate<0.001'],
    // Track iteration duration to detect slowdowns over time
    iteration_duration: ['p(95)<60000'],
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(50)', 'p(95)', 'p(99)', 'count'],
};

const QUERIES = [
  'How do I upload documents?',
  'What collections are available?',
  'How does the RAG pipeline work?',
  'What is Hybrid search?',
  'How do I configure the LLM?',
  'Explain the chunking strategy',
  'What authentication methods are supported?',
  'How do I delete a collection?',
  'How does the system handle errors?',
  'What is the architecture?',
];

export default function () {
  group('query', () => {
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
      'query returns 200': (r) => r.status === 200,
      'response has content': (r) => r.body.length > 0,
    });
  });

  // Normal think time (3-5 seconds)
  sleep(3 + Math.random() * 2);
}
