// Load test — sustained load with a realistic query pattern
// Ramp-up to target VUs over 2 minutes, hold for 5 minutes, ramp-down over 1 minute.
// Measures response time (P50, P95, P99), error rate, and throughput (RPS).

import { check, group, sleep } from 'k6';
import http from 'k6/http';
import { BASE_URL, THRESHOLDS, getHeaders } from './options.js';

// Target: 10 concurrent users making queries
const TARGET_VUS = 10;

export const options = {
  stages: [
    { duration: '2m', target: TARGET_VUS }, // Ramp-up
    { duration: '5m', target: TARGET_VUS }, // Hold
    { duration: '1m', target: 0 }, // Ramp-down
  ],
  thresholds: {
    ...THRESHOLDS,
    http_req_duration: ['p(50)<3000'], // Median response under 3s
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(50)', 'p(95)', 'p(99)', 'count'],
};

// Realistic query templates
const QUERIES = [
  'How do I upload documents?',
  'What collections are available?',
  'How does the RAG pipeline work?',
  'What is Hybrid search?',
  'How do I configure the LLM?',
  'Explain the chunking strategy',
  'What authentication methods are supported?',
  'How do I delete a collection?',
];

export default function () {
  group('health check', () => {
    const resp = http.get(`${BASE_URL}/health`);
    check(resp, { 'health is ok': (r) => r.status === 200 });
  });

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

  // Think time: simulate real user reading the response (2-5 seconds)
  sleep(2 + Math.random() * 3);
}
