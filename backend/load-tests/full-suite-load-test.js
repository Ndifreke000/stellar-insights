import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const responseTime = new Trend('response_time');
const requestCounter = new Counter('total_requests');
const endpointMetrics = {};

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export const options = {
  stages: [
    { duration: '1m', target: 50 },   // Warm up
    { duration: '3m', target: 100 },  // Ramp up
    { duration: '5m', target: 100 },  // Sustained load
    { duration: '2m', target: 150 },  // Peak load
    { duration: '3m', target: 150 },  // Sustained peak
    { duration: '2m', target: 0 },    // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<800', 'p(99)<1500'],
    'http_req_failed': ['rate<0.01'],
    'errors': ['rate<0.01'],
    'group_duration{group:::Corridors}': ['p(95)<600'],
    'group_duration{group:::Anchors}': ['p(95)<600'],
    'group_duration{group:::RPC}': ['p(95)<1200'],
  },
};

// Endpoint definitions with realistic weights
const endpoints = {
  corridors: [
    { path: '/api/corridors', weight: 30 },
    { path: '/api/corridors?limit=20', weight: 15 },
    { path: '/api/corridors?success_rate_min=95', weight: 10 },
    { path: '/api/corridors?asset_code=USDC', weight: 5 },
  ],
  anchors: [
    { path: '/api/anchors', weight: 25 },
    { path: '/api/anchors?limit=20', weight: 15 },
    { path: '/api/anchors?limit=50', weight: 10 },
  ],
  rpc: [
    { path: '/api/rpc/health', weight: 5 },
    { path: '/api/rpc/ledger/latest', weight: 10 },
    { path: '/api/rpc/payments?limit=20', weight: 15 },
    { path: '/api/rpc/trades?limit=20', weight: 10 },
  ],
  other: [
    { path: '/health', weight: 10 },
    { path: '/api/cache/stats', weight: 5 },
  ],
};

function selectEndpoint(category) {
  const categoryEndpoints = endpoints[category];
  const totalWeight = categoryEndpoints.reduce((sum, e) => sum + e.weight, 0);
  let random = Math.random() * totalWeight;
  
  for (const endpoint of categoryEndpoints) {
    random -= endpoint.weight;
    if (random <= 0) {
      return endpoint;
    }
  }
  
  return categoryEndpoints[0];
}

function makeRequest(url, tags) {
  const params = {
    headers: {
      'Accept': 'application/json',
      'User-Agent': 'k6-full-suite-test/1.0',
    },
    tags: tags,
  };
  
  const startTime = Date.now();
  const response = http.get(url, params);
  const duration = Date.now() - startTime;
  
  responseTime.add(duration);
  requestCounter.add(1);
  
  // Track per-endpoint metrics
  const metricKey = `${tags.category}_${tags.endpoint}`;
  if (!endpointMetrics[metricKey]) {
    endpointMetrics[metricKey] = new Trend(metricKey);
  }
  endpointMetrics[metricKey].add(duration);
  
  const checkResult = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
    'content-type is JSON': (r) => r.headers['Content-Type'] && r.headers['Content-Type'].includes('application/json'),
    'no server errors': (r) => r.status < 500,
  });
  
  errorRate.add(!checkResult);
  
  return response;
}

export default function () {
  // Simulate realistic user behavior with multiple endpoint calls
  
  group('Corridors', function () {
    const endpoint = selectEndpoint('corridors');
    const url = `${BASE_URL}${endpoint.path}`;
    const response = makeRequest(url, {
      category: 'corridors',
      endpoint: endpoint.path,
    });
    
    if (response.status === 200) {
      try {
        const data = JSON.parse(response.body);
        check(data, {
          'corridors: is array': (d) => Array.isArray(d),
          'corridors: has data': (d) => Array.isArray(d) && d.length >= 0,
        });
      } catch (e) {
        errorRate.add(1);
      }
    }
    
    sleep(1);
  });
  
  group('Anchors', function () {
    const endpoint = selectEndpoint('anchors');
    const url = `${BASE_URL}${endpoint.path}`;
    const response = makeRequest(url, {
      category: 'anchors',
      endpoint: endpoint.path,
    });
    
    if (response.status === 200) {
      try {
        const data = JSON.parse(response.body);
        check(data, {
          'anchors: has anchors array': (d) => d.anchors && Array.isArray(d.anchors),
          'anchors: has total': (d) => typeof d.total === 'number',
        });
      } catch (e) {
        errorRate.add(1);
      }
    }
    
    sleep(1);
  });
  
  // RPC calls less frequently (heavier operations)
  if (Math.random() < 0.5) {
    group('RPC', function () {
      const endpoint = selectEndpoint('rpc');
      const url = `${BASE_URL}${endpoint.path}`;
      const response = makeRequest(url, {
        category: 'rpc',
        endpoint: endpoint.path,
      });
      
      if (response.status === 200) {
        try {
          JSON.parse(response.body);
        } catch (e) {
          errorRate.add(1);
        }
      }
      
      sleep(2);
    });
  }
  
  // Health check occasionally
  if (Math.random() < 0.2) {
    group('Health', function () {
      const endpoint = selectEndpoint('other');
      const url = `${BASE_URL}${endpoint.path}`;
      makeRequest(url, {
        category: 'health',
        endpoint: endpoint.path,
      });
      
      sleep(0.5);
    });
  }
  
  // Random think time between user sessions
  sleep(Math.random() * 3 + 1);
}

export function setup() {
  console.log('='.repeat(60));
  console.log('FULL SUITE LOAD TEST');
  console.log('='.repeat(60));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Test Duration: ~16 minutes`);
  console.log(`Max VUs: 150`);
  console.log('');
  
  // Verify server is accessible
  const response = http.get(`${BASE_URL}/health`);
  if (response.status !== 200) {
    throw new Error(`Server health check failed: ${response.status}`);
  }
  
  console.log('✓ Server health check passed');
  console.log('✓ Starting load test...');
  console.log('');
  
  return { startTime: Date.now() };
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('');
  console.log('='.repeat(60));
  console.log(`Test completed in ${duration.toFixed(2)} seconds`);
  console.log('='.repeat(60));
}

export function handleSummary(data) {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  
  return {
    'stdout': generateTextSummary(data),
    [`load-test-results-${timestamp}.json`]: JSON.stringify(data, null, 2),
    [`load-test-summary-${timestamp}.html`]: generateHTMLSummary(data),
  };
}

function generateTextSummary(data) {
  let summary = '\n' + '='.repeat(70) + '\n';
  summary += 'FULL SUITE LOAD TEST - SUMMARY REPORT\n';
  summary += '='.repeat(70) + '\n\n';
  
  const metrics = data.metrics;
  
  // Overall Performance
  summary += 'OVERALL PERFORMANCE:\n';
  summary += '-'.repeat(70) + '\n';
  if (metrics.http_req_duration) {
    summary += `  Response Time (avg):  ${metrics.http_req_duration.values.avg.toFixed(2)}ms\n`;
    summary += `  Response Time (med):  ${metrics.http_req_duration.values.med.toFixed(2)}ms\n`;
    summary += `  Response Time (p95):  ${metrics.http_req_duration.values['p(95)'].toFixed(2)}ms\n`;
    summary += `  Response Time (p99):  ${metrics.http_req_duration.values['p(99)'].toFixed(2)}ms\n`;
    summary += `  Response Time (max):  ${metrics.http_req_duration.values.max.toFixed(2)}ms\n`;
  }
  summary += '\n';
  
  // Throughput
  summary += 'THROUGHPUT:\n';
  summary += '-'.repeat(70) + '\n';
  if (metrics.http_reqs) {
    summary += `  Total Requests:       ${metrics.http_reqs.values.count}\n`;
    summary += `  Requests/sec (avg):   ${metrics.http_reqs.values.rate.toFixed(2)}\n`;
  }
  if (metrics.iterations) {
    summary += `  Iterations:           ${metrics.iterations.values.count}\n`;
    summary += `  Iterations/sec:       ${metrics.iterations.values.rate.toFixed(2)}\n`;
  }
  summary += '\n';
  
  // Error Rates
  summary += 'ERROR RATES:\n';
  summary += '-'.repeat(70) + '\n';
  if (metrics.http_req_failed) {
    const failRate = (metrics.http_req_failed.values.rate * 100).toFixed(3);
    const status = failRate < 1 ? '✓' : '✗';
    summary += `  ${status} Failed Requests:     ${failRate}%\n`;
  }
  if (metrics.errors) {
    const errorRate = (metrics.errors.values.rate * 100).toFixed(3);
    const status = errorRate < 1 ? '✓' : '✗';
    summary += `  ${status} Error Rate:          ${errorRate}%\n`;
  }
  summary += '\n';
  
  // Group Performance
  summary += 'ENDPOINT GROUP PERFORMANCE:\n';
  summary += '-'.repeat(70) + '\n';
  const groups = ['Corridors', 'Anchors', 'RPC', 'Health'];
  for (const groupName of groups) {
    const groupKey = `group_duration{group:::${groupName}}`;
    if (metrics[groupKey]) {
      summary += `  ${groupName}:\n`;
      summary += `    Average: ${metrics[groupKey].values.avg.toFixed(2)}ms\n`;
      summary += `    p95:     ${metrics[groupKey].values['p(95)'].toFixed(2)}ms\n`;
    }
  }
  summary += '\n';
  
  // Thresholds
  summary += 'THRESHOLD RESULTS:\n';
  summary += '-'.repeat(70) + '\n';
  for (const [name, threshold] of Object.entries(data.thresholds || {})) {
    const status = threshold.ok ? '✓' : '✗';
    summary += `  ${status} ${name}\n`;
  }
  summary += '\n';
  
  summary += '='.repeat(70) + '\n';
  
  return summary;
}

function generateHTMLSummary(data) {
  const metrics = data.metrics;
  
  return `
<!DOCTYPE html>
<html>
<head>
  <title>Load Test Results</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }
    .container { max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
    h1 { color: #333; border-bottom: 3px solid #4CAF50; padding-bottom: 10px; }
    h2 { color: #555; margin-top: 30px; border-bottom: 2px solid #ddd; padding-bottom: 8px; }
    .metric { display: flex; justify-content: space-between; padding: 10px; border-bottom: 1px solid #eee; }
    .metric-name { font-weight: bold; color: #666; }
    .metric-value { color: #333; }
    .success { color: #4CAF50; }
    .warning { color: #FF9800; }
    .error { color: #F44336; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; margin: 20px 0; }
    .card { background: #f9f9f9; padding: 20px; border-radius: 4px; border-left: 4px solid #4CAF50; }
    .card h3 { margin-top: 0; color: #333; }
  </style>
</head>
<body>
  <div class="container">
    <h1>Full Suite Load Test Results</h1>
    <p><strong>Test Date:</strong> ${new Date().toLocaleString()}</p>
    
    <h2>Overall Performance</h2>
    <div class="grid">
      <div class="card">
        <h3>Response Times</h3>
        <div class="metric">
          <span class="metric-name">Average:</span>
          <span class="metric-value">${metrics.http_req_duration?.values.avg.toFixed(2)}ms</span>
        </div>
        <div class="metric">
          <span class="metric-name">Median:</span>
          <span class="metric-value">${metrics.http_req_duration?.values.med.toFixed(2)}ms</span>
        </div>
        <div class="metric">
          <span class="metric-name">p95:</span>
          <span class="metric-value">${metrics.http_req_duration?.values['p(95)'].toFixed(2)}ms</span>
        </div>
        <div class="metric">
          <span class="metric-name">p99:</span>
          <span class="metric-value">${metrics.http_req_duration?.values['p(99)'].toFixed(2)}ms</span>
        </div>
      </div>
      
      <div class="card">
        <h3>Throughput</h3>
        <div class="metric">
          <span class="metric-name">Total Requests:</span>
          <span class="metric-value">${metrics.http_reqs?.values.count}</span>
        </div>
        <div class="metric">
          <span class="metric-name">Requests/sec:</span>
          <span class="metric-value">${metrics.http_reqs?.values.rate.toFixed(2)}</span>
        </div>
        <div class="metric">
          <span class="metric-name">Iterations:</span>
          <span class="metric-value">${metrics.iterations?.values.count}</span>
        </div>
      </div>
      
      <div class="card">
        <h3>Error Rates</h3>
        <div class="metric">
          <span class="metric-name">Failed Requests:</span>
          <span class="metric-value ${(metrics.http_req_failed?.values.rate * 100) < 1 ? 'success' : 'error'}">
            ${(metrics.http_req_failed?.values.rate * 100).toFixed(3)}%
          </span>
        </div>
        <div class="metric">
          <span class="metric-name">Error Rate:</span>
          <span class="metric-value ${(metrics.errors?.values.rate * 100) < 1 ? 'success' : 'error'}">
            ${(metrics.errors?.values.rate * 100).toFixed(3)}%
          </span>
        </div>
      </div>
    </div>
    
    <h2>Threshold Results</h2>
    ${Object.entries(data.thresholds || {}).map(([name, threshold]) => `
      <div class="metric">
        <span class="metric-name">${name}</span>
        <span class="metric-value ${threshold.ok ? 'success' : 'error'}">
          ${threshold.ok ? '✓ PASS' : '✗ FAIL'}
        </span>
      </div>
    `).join('')}
  </div>
</body>
</html>
  `;
}
