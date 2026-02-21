import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const responseTime = new Trend('response_time');

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Spike test configuration - sudden traffic spikes
export const options = {
  stages: [
    { duration: '1m', target: 10 },    // Normal load
    { duration: '0s', target: 500 },   // Instant spike!
    { duration: '1m', target: 500 },   // Hold spike
    { duration: '0s', target: 10 },    // Drop back
    { duration: '1m', target: 10 },    // Recover
    { duration: '0s', target: 500 },   // Second spike
    { duration: '1m', target: 500 },   // Hold spike
    { duration: '1m', target: 0 },     // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<3000'],  // Lenient during spikes
    'http_req_failed': ['rate<0.1'],      // Allow 10% failure during spikes
  },
};

const endpoints = [
  '/api/corridors',
  '/api/anchors',
  '/health',
];

export default function () {
  const endpoint = endpoints[Math.floor(Math.random() * endpoints.length)];
  const url = `${BASE_URL}${endpoint}`;
  
  const params = {
    headers: {
      'Accept': 'application/json',
      'User-Agent': 'k6-spike-test/1.0',
    },
  };
  
  const startTime = Date.now();
  const response = http.get(url, params);
  const duration = Date.now() - startTime;
  
  responseTime.add(duration);
  
  const checkResult = check(response, {
    'status is 2xx or 429 or 503': (r) => 
      (r.status >= 200 && r.status < 300) || 
      r.status === 429 || 
      r.status === 503,
  });
  
  errorRate.add(!checkResult);
  
  sleep(0.5);
}

export function setup() {
  console.log('='.repeat(60));
  console.log('SPIKE TEST - Testing Sudden Traffic Bursts');
  console.log('='.repeat(60));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Spike to: 500 VUs`);
  console.log('');
  console.log('This test simulates sudden traffic spikes.');
  console.log('Monitor how the system handles rapid load changes.');
  console.log('');
  
  return { startTime: Date.now() };
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('');
  console.log('='.repeat(60));
  console.log(`Spike test completed in ${duration.toFixed(2)} seconds`);
  console.log('Check if system recovered gracefully after spikes.');
  console.log('='.repeat(60));
}
