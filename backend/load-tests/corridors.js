import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const corridorResponseTime = new Trend('corridor_response_time');
const corridorRequests = new Counter('corridor_requests');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const MAX_RESPONSE_TIME = parseInt(__ENV.MAX_RESPONSE_TIME || '500');
const TARGET_VUS = parseInt(__ENV.VUS || '0');

// Load test stages - graduated scenarios for 100, 500, and 1000 concurrent users
export const options = TARGET_VUS > 0
  ? {
      stages: [
        { duration: '30s', target: TARGET_VUS },
        { duration: '1m', target: TARGET_VUS },
        { duration: '30s', target: 0 },
      ],
      thresholds: {
        'http_req_duration': ['p(95)<500', 'p(99)<1000'],
        'http_req_failed': ['rate<0.01'],
        'errors': ['rate<0.01'],
        'corridor_response_time': ['p(95)<500'],
      },
    }
  : {
      stages: [
        // Scenario 1: Baseline 100 concurrent users
        { duration: '30s', target: 100 },
        { duration: '1m', target: 100 },
        // Scenario 2: Moderate load 500 concurrent users
        { duration: '30s', target: 500 },
        { duration: '1m', target: 500 },
        // Scenario 3: High load / stress 1000 concurrent users
        { duration: '30s', target: 1000 },
        { duration: '1m', target: 1000 },
        // Ramp-down
        { duration: '30s', target: 0 },
      ],
      thresholds: {
        'http_req_duration': ['p(95)<500', 'p(99)<1000'],
        'http_req_failed': ['rate<0.01'],
        'errors': ['rate<0.01'],
        'corridor_response_time': ['p(95)<500'],
      },
      ext: {
        loadimpact: {
          projectID: 3596969,
          name: 'Corridors Load Test (100, 500, 1000 VUs)',
        },
      },
    };

// Test scenarios for different query patterns
const testScenarios = [
  // Basic list request
  { path: '/api/corridors', weight: 40 },
  
  // With pagination
  { path: '/api/corridors?limit=20&offset=0', weight: 20 },
  { path: '/api/corridors?limit=50&offset=50', weight: 10 },
  
  // With filters
  { path: '/api/corridors?success_rate_min=95', weight: 10 },
  { path: '/api/corridors?volume_min=100000', weight: 5 },
  { path: '/api/corridors?asset_code=USDC', weight: 5 },
  { path: '/api/corridors?asset_code=XLM', weight: 5 },
  
  // Combined filters
  { path: '/api/corridors?success_rate_min=95&volume_min=100000', weight: 3 },
  { path: '/api/corridors?asset_code=USDC&success_rate_min=99', weight: 2 },
];

// Weighted random selection
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
      name: 'CorridorsAPI',
      endpoint: scenario.path,
    },
  };
  
  const startTime = Date.now();
  const response = http.get(url, params);
  const duration = Date.now() - startTime;
  
  // Record custom metrics
  corridorResponseTime.add(duration);
  corridorRequests.add(1);
  
  // Comprehensive checks
  const checkResult = check(response, {
    'status is 200': (r) => r.status === 200,
    'response time < threshold': (r) => r.timings.duration < MAX_RESPONSE_TIME,
    'response has body': (r) => r.body && r.body.length > 0,
    'content-type is JSON': (r) => r.headers['Content-Type'] && r.headers['Content-Type'].includes('application/json'),
    'no server errors': (r) => r.status < 500,
  });
  
  // Record errors
  errorRate.add(!checkResult);
  
  // Validate response structure
  if (response.status === 200) {
    try {
      const data = JSON.parse(response.body);
      
      check(data, {
        'response is array or object': (d) => Array.isArray(d) || typeof d === 'object',
      });
      
      const items = Array.isArray(data) ? data : (data.corridors || data.data || []);
      if (items.length > 0) {
        const corridor = items[0];
        check(corridor, {
          'has id or identifier': (c) => c.id !== undefined || c.corridor_id !== undefined,
          'has success_rate': (c) => c.success_rate !== undefined,
        });
      }
    } catch (e) {
      console.error(`Failed to parse response: ${e.message}`);
      errorRate.add(1);
    }
  }
  
  // Think time - simulate real user behavior
  sleep(Math.random() * 2 + 1);
}

// Setup function - runs once before test
export function setup() {
  console.log('Starting Corridors Load Test');
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Max Response Time: ${MAX_RESPONSE_TIME}ms`);
  
  const response = http.get(`${BASE_URL}/health`);
  if (response.status !== 200) {
    console.warn(`Health check returned status ${response.status}; proceeding anyway`);
  } else {
    console.log('Server health check passed');
  }
  return { startTime: Date.now() };
}

// Teardown function - runs once after test
export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log(`Test completed in ${duration.toFixed(2)} seconds`);
}

// Handle summary for custom reporting
export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    'load-test-results.json': JSON.stringify(data, null, 2),
  };
}

function textSummary(data, options) {
  const indent = options.indent || '';
  let summary = '\n' + indent + '='.repeat(60) + '\n';
  summary += indent + 'CORRIDORS LOAD TEST SUMMARY\n';
  summary += indent + '='.repeat(60) + '\n\n';
  
  const metrics = data.metrics;
  if (metrics.http_req_duration) {
    summary += indent + 'Response Times:\n';
    summary += indent + `  Average: ${metrics.http_req_duration.values.avg.toFixed(2)}ms\n`;
    summary += indent + `  Median:  ${metrics.http_req_duration.values.med.toFixed(2)}ms\n`;
    summary += indent + `  p95:     ${metrics.http_req_duration.values['p(95)'].toFixed(2)}ms\n`;
    summary += indent + `  p99:     ${metrics.http_req_duration.values['p(99)'].toFixed(2)}ms\n`;
    summary += indent + `  Max:     ${metrics.http_req_duration.values.max.toFixed(2)}ms\n\n`;
  }
  
  if (metrics.http_reqs) {
    summary += indent + `Total Requests: ${metrics.http_reqs.values.count}\n`;
    summary += indent + `Requests/sec:   ${metrics.http_reqs.values.rate.toFixed(2)}\n\n`;
  }
  
  if (metrics.http_req_failed) {
    const failRate = (metrics.http_req_failed.values.rate * 100).toFixed(2);
    summary += indent + `Failed Requests: ${failRate}%\n\n`;
  }
  
  summary += indent + '='.repeat(60) + '\n';
  return summary;
}
