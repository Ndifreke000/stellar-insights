import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const rpcHealthDuration = new Trend('rpc_health_duration');
const paymentsQueryDuration = new Trend('payments_query_duration');
const tradesQueryDuration = new Trend('trades_query_duration');
const requestCounter = new Counter('total_requests');

// Test configuration - RPC endpoints may be slower
export const options = {
  stages: [
    { duration: '1m', target: 20 },   // Gentle ramp up
    { duration: '3m', target: 40 },   // Steady load
    { duration: '1m', target: 60 },   // Spike
    { duration: '2m', target: 40 },   // Back down
    { duration: '1m', target: 0 },    // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<2000', 'p(99)<3000'], // RPC can be slower
    'http_req_failed': ['rate<0.05'],
    'errors': ['rate<0.05'],
    'rpc_health_duration': ['p(95)<1000'],
    'payments_query_duration': ['p(95)<2000'],
    'trades_query_duration': ['p(95)<2000'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Sample Stellar account IDs for testing
const SAMPLE_ACCOUNTS = [
  'GAHK7EEG2WWHVKDNT4CEQFZGKF2LGDSW2IVM4S5DP42RBW3K6BTODB4A',
  'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN7',
  'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
];

export default function () {
  requestCounter.add(1);

  // Test RPC health
  testRpcHealth();
  
  // Test payments endpoint
  testPayments();
  
  // Test account payments
  testAccountPayments();
  
  // Test trades
  testTrades();
  
  // Test latest ledger
  testLatestLedger();

  sleep(2); // Longer sleep for RPC endpoints
}

function testRpcHealth() {
  const res = http.get(`${BASE_URL}/api/rpc/health`, {
    tags: { name: 'RpcHealth' },
    timeout: '5s',
  });

  const success = check(res, {
    'rpc health: status is 200': (r) => r.status === 200,
    'rpc health: response time < 1000ms': (r) => r.timings.duration < 1000,
    'rpc health: has valid response': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.status !== undefined;
      } catch {
        return false;
      }
    },
  });

  rpcHealthDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testPayments() {
  const res = http.get(`${BASE_URL}/api/rpc/payments?limit=50`, {
    tags: { name: 'RpcPayments' },
    timeout: '10s',
  });

  const success = check(res, {
    'rpc payments: status is 200': (r) => r.status === 200,
    'rpc payments: response time < 2000ms': (r) => r.timings.duration < 2000,
    'rpc payments: valid JSON': (r) => {
      try {
        JSON.parse(r.body);
        return true;
      } catch {
        return false;
      }
    },
  });

  paymentsQueryDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testAccountPayments() {
  const account = SAMPLE_ACCOUNTS[Math.floor(Math.random() * SAMPLE_ACCOUNTS.length)];
  const res = http.get(`${BASE_URL}/api/rpc/payments/account/${account}?limit=20`, {
    tags: { name: 'RpcAccountPayments' },
    timeout: '10s',
  });

  const success = check(res, {
    'account payments: response received': (r) => r.status === 200 || r.status === 404,
    'account payments: response time < 2500ms': (r) => r.timings.duration < 2500,
  });

  errorRate.add(!success);
}

function testTrades() {
  const res = http.get(`${BASE_URL}/api/rpc/trades?limit=50`, {
    tags: { name: 'RpcTrades' },
    timeout: '10s',
  });

  const success = check(res, {
    'rpc trades: status is 200': (r) => r.status === 200,
    'rpc trades: response time < 2000ms': (r) => r.timings.duration < 2000,
  });

  tradesQueryDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testLatestLedger() {
  const res = http.get(`${BASE_URL}/api/rpc/ledger/latest`, {
    tags: { name: 'RpcLatestLedger' },
    timeout: '5s',
  });

  const success = check(res, {
    'latest ledger: status is 200': (r) => r.status === 200,
    'latest ledger: response time < 1500ms': (r) => r.timings.duration < 1500,
    'latest ledger: has sequence': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.sequence !== undefined;
      } catch {
        return false;
      }
    },
  });

  errorRate.add(!success);
}

export function setup() {
  console.log(`Starting RPC load test against ${BASE_URL}`);
  console.log('Note: RPC endpoints may have higher latency due to external API calls');
  
  const res = http.get(`${BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`Server not accessible at ${BASE_URL}`);
  }
  
  return { startTime: new Date().toISOString() };
}

export function teardown(data) {
  console.log(`RPC load test completed. Started at: ${data.startTime}`);
}
