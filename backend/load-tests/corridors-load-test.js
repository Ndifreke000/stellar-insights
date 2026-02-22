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

// Load test configuration
export const options = {
  stages: [
    { duration: '2m', target: 100 },  // Ramp up to 100 users over 2 minutes
    { duration: '5m', target: 100 },  // Stay at 100 users for 5 minutes
    { duration: '2m', target: 0 },    // Ramp down to 0 users over 2 minutes
  ],
  thresholds: {
    'http_req_duration': ['p(95)<500', 'p(99)<1000'],  // 95% of requests under 500ms, 99% under 1s
    'http_req_failed': ['rate<0.01'],                   // Error rate under 1%
    'errors': ['rate<0.01'],                            // Custom error rate under 1%
    'corridor_response_time': ['p(95)<500'],            // 95% of corridor requests under 500ms
  },
  ext: {
    loadimpact: {
      projectID: 3596969,
      name: 'Corridors Load Test'
    }
  }
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
        'response is array': (d) => Array.isArray(d),
        'has corridor data': (d) => Array.isArray(d) && d.length >= 0,
      });
      
      // Validate first corridor structure if present
      if (Array.isArray(data) && data.length > 0) {
        const corridor = data[0];
        check(corridor, {
          'has id': (c) => c.id !== undefined,
          'has source_asset': (c) => c.source_asset !== undefined,
          'has destination_asset': (c) => c.destination_asset !== undefined,
          'has success_rate': (c) => typeof c.success_rate === 'number',
          'has total_attempts': (c) => typeof c.total_attempts === 'number',
          'has health_score': (c) => typeof c.health_score === 'number',
          'success_rate valid range': (c) => c.success_rate >= 0 && c.success_rate <= 100,
          'health_score valid range': (c) => c.health_score >= 0 && c.health_score <= 100,
        });
      }
    } catch (e) {
      console.error(`Failed to parse response: ${e.message}`);
      errorRate.add(1);
    }
  }
  
  // Think time - simulate real user behavior
  sleep(Math.random() * 2 + 1); // Random sleep between 1-3 seconds
}

// Setup function - runs once before test
export function setup() {
  console.log('Starting Corridors Load Test');
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Max Response Time: ${MAX_RESPONSE_TIME}ms`);
  
  // Verify server is accessible
  const response = http.get(`${BASE_URL}/health`);
  if (response.status !== 200) {
    throw new Error(`Server health check failed: ${response.status}`);
  }
  
  console.log('Server health check passed');
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
  const enableColors = options.enableColors || false;
  
  let summary = '\n' + indent + '='.repeat(60) + '\n';
  summary += indent + 'CORRIDORS LOAD TEST SUMMARY\n';
  summary += indent + '='.repeat(60) + '\n\n';
  
  // Add key metrics
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
