import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const responseTime = new Trend('response_time');

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Stress test configuration - push system to its limits
export const options = {
  stages: [
    { duration: '2m', target: 100 },   // Warm up
    { duration: '3m', target: 200 },   // Ramp to stress level
    { duration: '5m', target: 200 },   // Maintain stress
    { duration: '3m', target: 300 },   // Push harder
    { duration: '5m', target: 300 },   // Sustained high stress
    { duration: '2m', target: 400 },   // Peak stress
    { duration: '3m', target: 400 },   // Hold at peak
    { duration: '3m', target: 0 },     // Ramp down
  ],
  thresholds: {
    // More lenient thresholds for stress test
    'http_req_duration': ['p(95)<2000', 'p(99)<5000'],
    'http_req_failed': ['rate<0.05'],  // Allow up to 5% failure
    'errors': ['rate<0.05'],
  },
};

const endpoints = [
  '/api/corridors',
  '/api/anchors',
  '/api/rpc/payments?limit=20',
  '/api/rpc/trades?limit=20',
  '/health',
];

export default function () {
  const endpoint = endpoints[Math.floor(Math.random() * endpoints.length)];
  const url = `${BASE_URL}${endpoint}`;
  
  const params = {
    headers: {
      'Accept': 'application/json',
      'User-Agent': 'k6-stress-test/1.0',
    },
  };
  
  const startTime = Date.now();
  const response = http.get(url, params);
  const duration = Date.now() - startTime;
  
  responseTime.add(duration);
  
  const checkResult = check(response, {
    'status is 2xx or 429': (r) => (r.status >= 200 && r.status < 300) || r.status === 429,
    'no 5xx errors': (r) => r.status < 500,
  });
  
  errorRate.add(!checkResult);
  
  // Minimal sleep to maximize stress
  sleep(0.1);
}

export function setup() {
  console.log('='.repeat(60));
  console.log('STRESS TEST - Finding System Limits');
  console.log('='.repeat(60));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Max VUs: 400`);
  console.log(`Duration: ~26 minutes`);
  console.log('');
  console.log('This test will push the system to its limits.');
  console.log('Monitor system resources (CPU, memory, connections).');
  console.log('');
  
  return { startTime: Date.now() };
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('');
  console.log('='.repeat(60));
  console.log(`Stress test completed in ${duration.toFixed(2)} seconds`);
  console.log('Review metrics to identify bottlenecks and breaking points.');
  console.log('='.repeat(60));
}
