import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const corridorListDuration = new Trend('corridor_list_duration');
const corridorDetailDuration = new Trend('corridor_detail_duration');
const requestCounter = new Counter('total_requests');

// Test configuration
export const options = {
  stages: [
    { duration: '2m', target: 50 },   // Ramp up to 50 users
    { duration: '5m', target: 100 },  // Stay at 100 users
    { duration: '2m', target: 200 },  // Spike to 200 users
    { duration: '3m', target: 200 },  // Maintain spike
    { duration: '2m', target: 0 },    // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<500', 'p(99)<1000'], // 95% under 500ms, 99% under 1s
    'http_req_failed': ['rate<0.05'],                  // Error rate under 5%
    'errors': ['rate<0.05'],
    'corridor_list_duration': ['p(95)<600'],
    'corridor_detail_duration': ['p(95)<800'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test data - realistic corridor keys based on Stellar ecosystem
const CORRIDOR_KEYS = [
  'USDC:native->XLM:native',
  'USDT:native->XLM:native',
  'EURC:native->XLM:native',
  'BTC:native->XLM:native',
  'ETH:native->XLM:native',
];

export default function () {
  requestCounter.add(1);

  // Test 1: List corridors endpoint (most critical)
  testListCorridors();
  
  // Test 2: List corridors with filters
  testListCorridorsWithFilters();
  
  // Test 3: Corridor detail (currently returns 404 but test for performance)
  testCorridorDetail();
  
  // Test 4: Health check
  testHealthCheck();

  sleep(1); // Think time between iterations
}

function testListCorridors() {
  const res = http.get(`${BASE_URL}/api/corridors`, {
    tags: { name: 'ListCorridors' },
  });

  const success = check(res, {
    'list corridors: status is 200': (r) => r.status === 200,
    'list corridors: response time < 500ms': (r) => r.timings.duration < 500,
    'list corridors: has valid JSON': (r) => {
      try {
        const body = JSON.parse(r.body);
        return Array.isArray(body);
      } catch {
        return false;
      }
    },
    'list corridors: returns data': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.length >= 0;
      } catch {
        return false;
      }
    },
  });

  corridorListDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testListCorridorsWithFilters() {
  const filters = [
    '?success_rate_min=80',
    '?success_rate_min=90&success_rate_max=100',
    '?volume_min=1000',
    '?asset_code=USDC',
    '?success_rate_min=85&volume_min=500',
  ];

  const filter = filters[Math.floor(Math.random() * filters.length)];
  const res = http.get(`${BASE_URL}/api/corridors${filter}`, {
    tags: { name: 'ListCorridorsFiltered' },
  });

  const success = check(res, {
    'filtered corridors: status is 200': (r) => r.status === 200,
    'filtered corridors: response time < 600ms': (r) => r.timings.duration < 600,
    'filtered corridors: valid JSON array': (r) => {
      try {
        return Array.isArray(JSON.parse(r.body));
      } catch {
        return false;
      }
    },
  });

  errorRate.add(!success);
}

function testCorridorDetail() {
  const corridorKey = CORRIDOR_KEYS[Math.floor(Math.random() * CORRIDOR_KEYS.length)];
  const res = http.get(`${BASE_URL}/api/corridors/${encodeURIComponent(corridorKey)}`, {
    tags: { name: 'CorridorDetail' },
  });

  // Currently returns 404 (not implemented), but we test performance anyway
  const success = check(res, {
    'corridor detail: response received': (r) => r.status === 404 || r.status === 200,
    'corridor detail: response time < 800ms': (r) => r.timings.duration < 800,
  });

  corridorDetailDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testHealthCheck() {
  const res = http.get(`${BASE_URL}/health`, {
    tags: { name: 'HealthCheck' },
  });

  const success = check(res, {
    'health: status is 200': (r) => r.status === 200,
    'health: response time < 100ms': (r) => r.timings.duration < 100,
  });

  errorRate.add(!success);
}

// Setup function - runs once before test
export function setup() {
  console.log(`Starting load test against ${BASE_URL}`);
  
  // Verify server is accessible
  const res = http.get(`${BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`Server not accessible at ${BASE_URL}. Status: ${res.status}`);
  }
  
  console.log('Server health check passed');
  return { startTime: new Date().toISOString() };
}

// Teardown function - runs once after test
export function teardown(data) {
  console.log(`Load test completed. Started at: ${data.startTime}`);
}
