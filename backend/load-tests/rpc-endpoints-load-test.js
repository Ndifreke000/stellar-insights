import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const rpcResponseTime = new Trend('rpc_response_time');
const rpcRequests = new Counter('rpc_requests');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const MAX_RESPONSE_TIME = parseInt(__ENV.MAX_RESPONSE_TIME || '1000');

export const options = {
  stages: [
    { duration: '2m', target: 50 },   // RPC endpoints are heavier
    { duration: '5m', target: 50 },
    { duration: '2m', target: 0 },
  ],
  thresholds: {
    'http_req_duration': ['p(95)<1000', 'p(99)<2000'],
    'http_req_failed': ['rate<0.02'],
    'errors': ['rate<0.02'],
    'rpc_response_time': ['p(95)<1000'],
  },
};

const testScenarios = [
  { path: '/api/rpc/health', weight: 10 },
  { path: '/api/rpc/ledger/latest', weight: 15 },
  { path: '/api/rpc/payments?limit=20', weight: 30 },
  { path: '/api/rpc/payments?limit=50', weight: 20 },
  { path: '/api/rpc/trades?limit=20', weight: 15 },
  { path: '/api/rpc/trades?limit=50', weight: 10 },
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
      name: 'RPC_API',
      endpoint: scenario.path,
    },
  };
  
  const startTime = Date.now();
  const response = http.get(url, params);
  const duration = Date.now() - startTime;
  
  rpcResponseTime.add(duration);
  rpcRequests.add(1);
  
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
      
      // Validate based on endpoint
      if (scenario.path.includes('/payments')) {
        check(data, {
          'payments is array': (d) => Array.isArray(d),
        });
        
        if (Array.isArray(data) && data.length > 0) {
          const payment = data[0];
          check(payment, {
            'has id': (p) => p.id !== undefined,
            'has amount': (p) => p.amount !== undefined,
            'has asset_type': (p) => p.asset_type !== undefined,
          });
        }
      } else if (scenario.path.includes('/trades')) {
        check(data, {
          'trades is array': (d) => Array.isArray(d),
        });
      } else if (scenario.path.includes('/ledger')) {
        check(data, {
          'has sequence': (d) => d.sequence !== undefined,
        });
      }
    } catch (e) {
      console.error(`Failed to parse response: ${e.message}`);
      errorRate.add(1);
    }
  }
  
  sleep(Math.random() * 3 + 2); // Longer sleep for RPC endpoints
}

export function setup() {
  console.log('Starting RPC Endpoints Load Test');
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
