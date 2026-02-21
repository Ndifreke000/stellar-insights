import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheStatsDuration = new Trend('cache_stats_duration');
const metricsOverviewDuration = new Trend('metrics_overview_duration');
const requestCounter = new Counter('total_requests');

// Test configuration - lighter load for monitoring endpoints
export const options = {
  stages: [
    { duration: '30s', target: 10 },  // Ramp up
    { duration: '2m', target: 20 },   // Steady state
    { duration: '30s', target: 40 },  // Spike
    { duration: '1m', target: 20 },   // Back to steady
    { duration: '30s', target: 0 },   // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<300', 'p(99)<500'],
    'http_req_failed': ['rate<0.02'],
    'errors': ['rate<0.02'],
    'cache_stats_duration': ['p(95)<200'],
    'metrics_overview_duration': ['p(95)<400'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export default function () {
  requestCounter.add(1);

  // Test cache stats endpoint
  testCacheStats();
  
  // Test metrics overview endpoint
  testMetricsOverview();

  sleep(2); // Longer sleep for monitoring endpoints
}

function testCacheStats() {
  const res = http.get(`${BASE_URL}/api/cache/stats`, {
    tags: { name: 'CacheStats' },
  });

  const success = check(res, {
    'cache stats: status is 200': (r) => r.status === 200,
    'cache stats: response time < 200ms': (r) => r.timings.duration < 200,
    'cache stats: valid JSON': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.hits !== undefined;
      } catch {
        return false;
      }
    },
    'cache stats: has required fields': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.hits !== undefined &&
               data.misses !== undefined &&
               data.invalidations !== undefined &&
               data.hit_rate_percent !== undefined &&
               data.total_requests !== undefined;
      } catch {
        return false;
      }
    },
    'cache stats: hit rate is valid': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.hit_rate_percent >= 0 && data.hit_rate_percent <= 100;
      } catch {
        return false;
      }
    },
  });

  cacheStatsDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testMetricsOverview() {
  const res = http.get(`${BASE_URL}/api/metrics/overview`, {
    tags: { name: 'MetricsOverview' },
  });

  const success = check(res, {
    'metrics overview: status is 200': (r) => r.status === 200,
    'metrics overview: response time < 400ms': (r) => r.timings.duration < 400,
    'metrics overview: valid JSON': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.total_volume !== undefined;
      } catch {
        return false;
      }
    },
    'metrics overview: has required fields': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.total_volume !== undefined &&
               data.total_transactions !== undefined &&
               data.active_users !== undefined &&
               data.average_transaction_value !== undefined &&
               data.corridor_count !== undefined;
      } catch {
        return false;
      }
    },
  });

  metricsOverviewDuration.add(res.timings.duration);
  errorRate.add(!success);
}

export function setup() {
  console.log(`Starting cache & metrics load test against ${BASE_URL}`);
  
  const res = http.get(`${BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`Server not accessible at ${BASE_URL}`);
  }
  
  return { startTime: new Date().toISOString() };
}

export function teardown(data) {
  console.log(`Cache & metrics load test completed. Started at: ${data.startTime}`);
}
