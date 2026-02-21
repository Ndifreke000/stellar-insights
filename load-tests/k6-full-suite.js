import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import { htmlReport } from 'https://raw.githubusercontent.com/benc-uk/k6-reporter/main/dist/bundle.js';
import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.1/index.js';

// Custom metrics
const errorRate = new Rate('errors');
const apiDuration = new Trend('api_duration');
const requestCounter = new Counter('total_requests');
const cacheHitRate = new Rate('cache_hits');

// Comprehensive test configuration
export const options = {
  stages: [
    { duration: '2m', target: 50 },    // Warm up
    { duration: '5m', target: 100 },   // Normal load
    { duration: '2m', target: 200 },   // Peak load
    { duration: '3m', target: 200 },   // Sustained peak
    { duration: '2m', target: 300 },   // Stress test
    { duration: '2m', target: 100 },   // Recovery
    { duration: '2m', target: 0 },     // Cool down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<1000', 'p(99)<2000'],
    'http_req_failed': ['rate<0.05'],
    'errors': ['rate<0.05'],
    'api_duration': ['p(95)<1000'],
    'http_reqs': ['rate>50'], // Minimum 50 requests per second
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export default function () {
  requestCounter.add(1);

  // Simulate realistic user behavior with different endpoint weights
  const scenario = Math.random();

  if (scenario < 0.35) {
    // 35% - Corridor queries (most common)
    group('Corridor Operations', () => {
      testCorridorList();
      sleep(0.5);
      testCorridorListFiltered();
    });
  } else if (scenario < 0.60) {
    // 25% - Anchor queries
    group('Anchor Operations', () => {
      testAnchorList();
      sleep(0.3);
      testAnchorDetail();
    });
  } else if (scenario < 0.75) {
    // 15% - RPC queries
    group('RPC Operations', () => {
      testRpcHealth();
      sleep(0.5);
      testPayments();
    });
  } else if (scenario < 0.85) {
    // 10% - Liquidity pool queries
    group('Liquidity Pool Operations', () => {
      testLiquidityPools();
      sleep(0.4);
      testPoolStats();
    });
  } else if (scenario < 0.93) {
    // 8% - Fee bump queries
    group('Fee Bump Operations', () => {
      testFeeBumpStats();
      sleep(0.3);
      testRecentFeeBumps();
    });
  } else {
    // 7% - System monitoring
    group('System Monitoring', () => {
      testHealthCheck();
      testCacheStats();
      testMetrics();
    });
  }

  sleep(1);
}

// Corridor endpoints
function testCorridorList() {
  const res = http.get(`${BASE_URL}/api/corridors`, {
    tags: { name: 'CorridorList', endpoint: 'corridors' },
  });

  const success = check(res, {
    'corridor list: status 200': (r) => r.status === 200,
    'corridor list: fast response': (r) => r.timings.duration < 500,
    'corridor list: valid data': (r) => {
      try {
        const data = JSON.parse(r.body);
        return Array.isArray(data);
      } catch {
        return false;
      }
    },
  });

  apiDuration.add(res.timings.duration, { endpoint: 'corridors' });
  errorRate.add(!success);
  checkCacheHeader(res);
}

function testCorridorListFiltered() {
  const filters = [
    '?success_rate_min=80',
    '?success_rate_min=90&success_rate_max=100',
    '?volume_min=1000',
    '?asset_code=USDC',
  ];
  
  const filter = filters[Math.floor(Math.random() * filters.length)];
  const res = http.get(`${BASE_URL}/api/corridors${filter}`, {
    tags: { name: 'CorridorListFiltered', endpoint: 'corridors_filtered' },
  });

  const success = check(res, {
    'filtered corridors: status 200': (r) => r.status === 200,
    'filtered corridors: fast response': (r) => r.timings.duration < 600,
  });

  apiDuration.add(res.timings.duration, { endpoint: 'corridors_filtered' });
  errorRate.add(!success);
}

// Anchor endpoints
function testAnchorList() {
  const res = http.get(`${BASE_URL}/api/anchors`, {
    tags: { name: 'AnchorList', endpoint: 'anchors' },
  });

  const success = check(res, {
    'anchor list: status 200': (r) => r.status === 200,
    'anchor list: fast response': (r) => r.timings.duration < 400,
  });

  apiDuration.add(res.timings.duration, { endpoint: 'anchors' });
  errorRate.add(!success);
  checkCacheHeader(res);
}

function testAnchorDetail() {
  const anchorId = Math.floor(Math.random() * 10) + 1;
  const res = http.get(`${BASE_URL}/api/anchors/${anchorId}`, {
    tags: { name: 'AnchorDetail', endpoint: 'anchor_detail' },
  });

  const success = check(res, {
    'anchor detail: valid response': (r) => r.status === 200 || r.status === 404,
    'anchor detail: fast response': (r) => r.timings.duration < 500,
  });

  apiDuration.add(res.timings.duration, { endpoint: 'anchor_detail' });
  errorRate.add(!success);
}

// RPC endpoints
function testRpcHealth() {
  const res = http.get(`${BASE_URL}/api/rpc/health`, {
    tags: { name: 'RpcHealth', endpoint: 'rpc_health' },
    timeout: '5s',
  });

  const success = check(res, {
    'rpc health: status 200': (r) => r.status === 200,
    'rpc health: reasonable time': (r) => r.timings.duration < 1000,
  });

  apiDuration.add(res.timings.duration, { endpoint: 'rpc_health' });
  errorRate.add(!success);
}

function testPayments() {
  const res = http.get(`${BASE_URL}/api/rpc/payments?limit=50`, {
    tags: { name: 'RpcPayments', endpoint: 'rpc_payments' },
    timeout: '10s',
  });

  const success = check(res, {
    'rpc payments: status 200': (r) => r.status === 200,
    'rpc payments: reasonable time': (r) => r.timings.duration < 2000,
  });

  apiDuration.add(res.timings.duration, { endpoint: 'rpc_payments' });
  errorRate.add(!success);
}

// System endpoints
function testHealthCheck() {
  const res = http.get(`${BASE_URL}/health`, {
    tags: { name: 'HealthCheck', endpoint: 'health' },
  });

  const success = check(res, {
    'health: status 200': (r) => r.status === 200,
    'health: very fast': (r) => r.timings.duration < 100,
  });

  apiDuration.add(res.timings.duration, { endpoint: 'health' });
  errorRate.add(!success);
}

function testCacheStats() {
  const res = http.get(`${BASE_URL}/api/cache-stats`, {
    tags: { name: 'CacheStats', endpoint: 'cache_stats' },
  });

  const success = check(res, {
    'cache stats: valid response': (r) => r.status === 200,
  });

  errorRate.add(!success);
}

function testMetrics() {
  const res = http.get(`${BASE_URL}/api/metrics/overview`, {
    tags: { name: 'Metrics', endpoint: 'metrics' },
  });

  const success = check(res, {
    'metrics: valid response': (r) => r.status === 200,
  });

  errorRate.add(!success);
}

// Liquidity pool endpoints
function testLiquidityPools() {
  const res = http.get(`${BASE_URL}/api/liquidity-pools`, {
    tags: { name: 'LiquidityPools', endpoint: 'liquidity_pools' },
  });

  const success = check(res, {
    'liquidity pools: status 200': (r) => r.status === 200,
    'liquidity pools: fast response': (r) => r.timings.duration < 600,
  });

  apiDuration.add(res.timings.duration, { endpoint: 'liquidity_pools' });
  errorRate.add(!success);
}

function testPoolStats() {
  const res = http.get(`${BASE_URL}/api/liquidity-pools/stats`, {
    tags: { name: 'PoolStats', endpoint: 'pool_stats' },
  });

  const success = check(res, {
    'pool stats: status 200': (r) => r.status === 200,
    'pool stats: fast response': (r) => r.timings.duration < 500,
  });

  apiDuration.add(res.timings.duration, { endpoint: 'pool_stats' });
  errorRate.add(!success);
}

// Fee bump endpoints
function testFeeBumpStats() {
  const res = http.get(`${BASE_URL}/api/fee-bumps/stats`, {
    tags: { name: 'FeeBumpStats', endpoint: 'fee_bump_stats' },
  });

  const success = check(res, {
    'fee bump stats: status 200': (r) => r.status === 200,
    'fee bump stats: fast response': (r) => r.timings.duration < 500,
  });

  apiDuration.add(res.timings.duration, { endpoint: 'fee_bump_stats' });
  errorRate.add(!success);
}

function testRecentFeeBumps() {
  const res = http.get(`${BASE_URL}/api/fee-bumps/recent?limit=50`, {
    tags: { name: 'RecentFeeBumps', endpoint: 'recent_fee_bumps' },
  });

  const success = check(res, {
    'recent fee bumps: status 200': (r) => r.status === 200,
    'recent fee bumps: fast response': (r) => r.timings.duration < 700,
  });

  apiDuration.add(res.timings.duration, { endpoint: 'recent_fee_bumps' });
  errorRate.add(!success);
}

// Helper function to check cache headers
function checkCacheHeader(response) {
  const cacheHeader = response.headers['X-Cache'] || response.headers['x-cache'];
  if (cacheHeader) {
    cacheHitRate.add(cacheHeader.toLowerCase().includes('hit'));
  }
}

// Setup - runs once before test
export function setup() {
  console.log('='.repeat(60));
  console.log('Starting comprehensive load test');
  console.log(`Target: ${BASE_URL}`);
  console.log(`Test duration: ~18 minutes`);
  console.log(`Max concurrent users: 300`);
  console.log('='.repeat(60));
  
  // Verify server is accessible
  const res = http.get(`${BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`Server not accessible at ${BASE_URL}. Status: ${res.status}`);
  }
  
  console.log('✓ Server health check passed');
  console.log('✓ Starting load test...\n');
  
  return { 
    startTime: new Date().toISOString(),
    baseUrl: BASE_URL,
  };
}

// Teardown - runs once after test
export function teardown(data) {
  console.log('\n' + '='.repeat(60));
  console.log('Load test completed');
  console.log(`Started: ${data.startTime}`);
  console.log(`Ended: ${new Date().toISOString()}`);
  console.log('='.repeat(60));
}

// Generate HTML report
export function handleSummary(data) {
  return {
    'load-test-results.html': htmlReport(data),
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    'load-test-summary.json': JSON.stringify(data),
  };
}
