import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const anchorResponseTime = new Trend('anchor_response_time');
const anchorRequests = new Counter('anchor_requests');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const MAX_RESPONSE_TIME = parseInt(__ENV.MAX_RESPONSE_TIME || '500');

export const options = {
  stages: [
    { duration: '2m', target: 100 },
    { duration: '5m', target: 100 },
    { duration: '2m', target: 0 },
  ],
  thresholds: {
    'http_req_duration': ['p(95)<500', 'p(99)<1000'],
    'http_req_failed': ['rate<0.01'],
    'errors': ['rate<0.01'],
    'anchor_response_time': ['p(95)<500'],
  },
};

const testScenarios = [
  { path: '/api/anchors', weight: 50 },
  { path: '/api/anchors?limit=20&offset=0', weight: 20 },
  { path: '/api/anchors?limit=50&offset=0', weight: 15 },
  { path: '/api/anchors?limit=10&offset=10', weight: 10 },
  { path: '/api/anchors?limit=100&offset=0', weight: 5 },
];

function selectScenario() {
  const totalWeight = testScenarios.reduce((sum, s) => sum + s.weight, 0);
  let random = Math.random() * totalWeight;
  
  for (const scenario of testScenarios) {
    random -= scenario.weight;
    if (random <= 0) {
      return scenario;
    }
  }
  
  return testScenarios[0];
}

export default function () {
  const scenario = selectScenario();
  const url = `${BASE_URL}${scenario.path}`;
  
  const params = {
    headers: {
      'Accept': 'application/json',
      'User-Agent': 'k6-load-test/1.0',
    },
    tags: {
      name: 'AnchorsAPI',
      endpoint: scenario.path,
    },
  };
  
  const startTime = Date.now();
  const response = http.get(url, params);
  const duration = Date.now() - startTime;
  
  anchorResponseTime.add(duration);
  anchorRequests.add(1);
  
  const checkResult = check(response, {
    'status is 200': (r) => r.status === 200,
    'response time < threshold': (r) => r.timings.duration < MAX_RESPONSE_TIME,
    'response has body': (r) => r.body && r.body.length > 0,
    'content-type is JSON': (r) => r.headers['Content-Type'] && r.headers['Content-Type'].includes('application/json'),
    'no server errors': (r) => r.status < 500,
  });
  
  errorRate.add(!checkResult);
  
  if (response.status === 200) {
    try {
      const data = JSON.parse(response.body);
      
      check(data, {
        'has anchors array': (d) => d.anchors && Array.isArray(d.anchors),
        'has total count': (d) => typeof d.total === 'number',
      });
      
      if (data.anchors && data.anchors.length > 0) {
        const anchor = data.anchors[0];
        check(anchor, {
          'has id': (a) => a.id !== undefined,
          'has name': (a) => a.name !== undefined,
          'has stellar_account': (a) => a.stellar_account !== undefined,
          'has reliability_score': (a) => typeof a.reliability_score === 'number',
          'has status': (a) => ['green', 'yellow', 'red'].includes(a.status),
          'reliability_score valid': (a) => a.reliability_score >= 0 && a.reliability_score <= 100,
        });
      }
    } catch (e) {
      console.error(`Failed to parse response: ${e.message}`);
      errorRate.add(1);
    }
  }
  
  sleep(Math.random() * 2 + 1);
}

export function setup() {
  console.log('Starting Anchors Load Test');
  console.log(`Base URL: ${BASE_URL}`);
  
  const response = http.get(`${BASE_URL}/health`);
  if (response.status !== 200) {
    throw new Error(`Server health check failed: ${response.status}`);
  }
  
  return { startTime: Date.now() };
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log(`Test completed in ${duration.toFixed(2)} seconds`);
}
