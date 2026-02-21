import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const feeBumpStatsDuration = new Trend('fee_bump_stats_duration');
const feeBumpRecentDuration = new Trend('fee_bump_recent_duration');
const requestCounter = new Counter('total_requests');

// Test configuration
export const options = {
  stages: [
    { duration: '1m', target: 20 },   // Ramp up
    { duration: '3m', target: 40 },   // Steady state
    { duration: '1m', target: 60 },   // Spike
    { duration: '2m', target: 40 },   // Back to steady
    { duration: '1m', target: 0 },    // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<600', 'p(99)<1000'],
    'http_req_failed': ['rate<0.05'],
    'errors': ['rate<0.05'],
    'fee_bump_stats_duration': ['p(95)<500'],
    'fee_bump_recent_duration': ['p(95)<700'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export default function () {
  requestCounter.add(1);

  // Test fee bump stats
  testFeeBumpStats();
  
  // Test recent fee bumps with various limits
  testRecentFeeBumps();

  sleep(1);
}

function testFeeBumpStats() {
  const res = http.get(`${BASE_URL}/api/fee-bumps/stats`, {
    tags: { name: 'FeeBumpStats' },
  });

  const success = check(res, {
    'fee bump stats: status is 200': (r) => r.status === 200,
    'fee bump stats: response time < 500ms': (r) => r.timings.duration < 500,
    'fee bump stats: valid JSON': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.total_fee_bumps !== undefined;
      } catch {
        return false;
      }
    },
    'fee bump stats: has required fields': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.avg_fee_charged !== undefined &&
               data.max_fee_charged !== undefined &&
               data.min_fee_charged !== undefined &&
               data.unique_fee_sources !== undefined;
      } catch {
        return false;
      }
    },
  });

  feeBumpStatsDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testRecentFeeBumps() {
  const limits = [10, 25, 50, 75, 100];
  const limit = limits[Math.floor(Math.random() * limits.length)];
  
  const res = http.get(`${BASE_URL}/api/fee-bumps/recent?limit=${limit}`, {
    tags: { name: 'RecentFeeBumps' },
  });

  const success = check(res, {
    'recent fee bumps: status is 200': (r) => r.status === 200,
    'recent fee bumps: response time < 700ms': (r) => r.timings.duration < 700,
    'recent fee bumps: valid JSON array': (r) => {
      try {
        const data = JSON.parse(r.body);
        return Array.isArray(data);
      } catch {
        return false;
      }
    },
    'recent fee bumps: respects limit': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.length <= limit;
      } catch {
        return false;
      }
    },
  });

  feeBumpRecentDuration.add(res.timings.duration);
  errorRate.add(!success);
}

export function setup() {
  console.log(`Starting fee bump load test against ${BASE_URL}`);
  
  const res = http.get(`${BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`Server not accessible at ${BASE_URL}`);
  }
  
  return { startTime: new Date().toISOString() };
}

export function teardown(data) {
  console.log(`Fee bump load test completed. Started at: ${data.startTime}`);
}
