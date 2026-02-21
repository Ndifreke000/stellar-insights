import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const anchorListDuration = new Trend('anchor_list_duration');
const anchorDetailDuration = new Trend('anchor_detail_duration');
const requestCounter = new Counter('total_requests');

// Test configuration - moderate load for anchor endpoints
export const options = {
  stages: [
    { duration: '1m', target: 30 },   // Ramp up
    { duration: '3m', target: 50 },   // Steady state
    { duration: '1m', target: 100 },  // Spike test
    { duration: '2m', target: 50 },   // Back to steady
    { duration: '1m', target: 0 },    // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<400', 'p(99)<800'],
    'http_req_failed': ['rate<0.03'],
    'errors': ['rate<0.03'],
    'anchor_list_duration': ['p(95)<500'],
    'anchor_detail_duration': ['p(95)<600'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export default function () {
  requestCounter.add(1);

  // Test anchor list endpoint
  testListAnchors();
  
  // Test anchor detail (if we have IDs from list)
  testAnchorDetail();
  
  // Test anchor assets
  testAnchorAssets();

  sleep(1);
}

function testListAnchors() {
  const res = http.get(`${BASE_URL}/api/anchors`, {
    tags: { name: 'ListAnchors' },
  });

  const success = check(res, {
    'list anchors: status is 200': (r) => r.status === 200,
    'list anchors: response time < 400ms': (r) => r.timings.duration < 400,
    'list anchors: valid JSON array': (r) => {
      try {
        return Array.isArray(JSON.parse(r.body));
      } catch {
        return false;
      }
    },
  });

  anchorListDuration.add(res.timings.duration);
  errorRate.add(!success);

  // Store anchor IDs for detail tests
  if (res.status === 200) {
    try {
      const anchors = JSON.parse(res.body);
      if (anchors.length > 0) {
        return anchors[0].id;
      }
    } catch (e) {
      // Ignore parsing errors
    }
  }
  return null;
}

function testAnchorDetail() {
  // Use a mock ID since we may not have real data
  const anchorId = 1;
  const res = http.get(`${BASE_URL}/api/anchors/${anchorId}`, {
    tags: { name: 'AnchorDetail' },
  });

  const success = check(res, {
    'anchor detail: response received': (r) => r.status === 200 || r.status === 404,
    'anchor detail: response time < 600ms': (r) => r.timings.duration < 600,
  });

  anchorDetailDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testAnchorAssets() {
  const anchorId = 1;
  const res = http.get(`${BASE_URL}/api/anchors/${anchorId}/assets`, {
    tags: { name: 'AnchorAssets' },
  });

  const success = check(res, {
    'anchor assets: response received': (r) => r.status === 200 || r.status === 404,
    'anchor assets: response time < 500ms': (r) => r.timings.duration < 500,
  });

  errorRate.add(!success);
}

export function setup() {
  console.log(`Starting anchor load test against ${BASE_URL}`);
  
  const res = http.get(`${BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`Server not accessible at ${BASE_URL}`);
  }
  
  return { startTime: new Date().toISOString() };
}

export function teardown(data) {
  console.log(`Anchor load test completed. Started at: ${data.startTime}`);
}
