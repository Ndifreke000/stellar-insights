import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const poolListDuration = new Trend('pool_list_duration');
const poolStatsDuration = new Trend('pool_stats_duration');
const poolDetailDuration = new Trend('pool_detail_duration');
const poolRankingsDuration = new Trend('pool_rankings_duration');
const requestCounter = new Counter('total_requests');

// Test configuration
export const options = {
  stages: [
    { duration: '1m', target: 25 },   // Ramp up
    { duration: '3m', target: 50 },   // Steady state
    { duration: '1m', target: 80 },   // Spike
    { duration: '2m', target: 50 },   // Back to steady
    { duration: '1m', target: 0 },    // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<800', 'p(99)<1500'],
    'http_req_failed': ['rate<0.05'],
    'errors': ['rate<0.05'],
    'pool_list_duration': ['p(95)<600'],
    'pool_stats_duration': ['p(95)<500'],
    'pool_detail_duration': ['p(95)<900'],
    'pool_rankings_duration': ['p(95)<700'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Sample pool IDs for testing (will be populated from list endpoint)
let samplePoolIds = [];

export default function () {
  requestCounter.add(1);

  // Test pool list
  const poolIds = testPoolList();
  if (poolIds && poolIds.length > 0) {
    samplePoolIds = poolIds;
  }
  
  // Test pool stats
  testPoolStats();
  
  // Test pool rankings with different sort options
  testPoolRankings();
  
  // Test pool detail if we have IDs
  if (samplePoolIds.length > 0) {
    testPoolDetail();
    testPoolSnapshots();
  }

  sleep(1);
}

function testPoolList() {
  const res = http.get(`${BASE_URL}/api/liquidity-pools`, {
    tags: { name: 'PoolList' },
  });

  const success = check(res, {
    'pool list: status is 200': (r) => r.status === 200,
    'pool list: response time < 600ms': (r) => r.timings.duration < 600,
    'pool list: valid JSON array': (r) => {
      try {
        const data = JSON.parse(r.body);
        return Array.isArray(data);
      } catch {
        return false;
      }
    },
  });

  poolListDuration.add(res.timings.duration);
  errorRate.add(!success);

  // Extract pool IDs for detail tests
  if (res.status === 200) {
    try {
      const pools = JSON.parse(res.body);
      return pools.map(p => p.pool_id).filter(id => id);
    } catch {
      return [];
    }
  }
  return [];
}

function testPoolStats() {
  const res = http.get(`${BASE_URL}/api/liquidity-pools/stats`, {
    tags: { name: 'PoolStats' },
  });

  const success = check(res, {
    'pool stats: status is 200': (r) => r.status === 200,
    'pool stats: response time < 500ms': (r) => r.timings.duration < 500,
    'pool stats: valid JSON': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.total_pools !== undefined;
      } catch {
        return false;
      }
    },
    'pool stats: has required fields': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.total_value_locked_usd !== undefined &&
               data.total_volume_24h_usd !== undefined &&
               data.avg_apy !== undefined;
      } catch {
        return false;
      }
    },
  });

  poolStatsDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testPoolRankings() {
  const sortOptions = ['apy', 'volume', 'tvl', 'fees'];
  const sortBy = sortOptions[Math.floor(Math.random() * sortOptions.length)];
  const limits = [10, 20, 50];
  const limit = limits[Math.floor(Math.random() * limits.length)];
  
  const res = http.get(`${BASE_URL}/api/liquidity-pools/rankings?sort_by=${sortBy}&limit=${limit}`, {
    tags: { name: 'PoolRankings' },
  });

  const success = check(res, {
    'pool rankings: status is 200': (r) => r.status === 200,
    'pool rankings: response time < 700ms': (r) => r.timings.duration < 700,
    'pool rankings: valid JSON array': (r) => {
      try {
        const data = JSON.parse(r.body);
        return Array.isArray(data);
      } catch {
        return false;
      }
    },
    'pool rankings: respects limit': (r) => {
      try {
        const data = JSON.parse(r.body);
        return data.length <= limit;
      } catch {
        return false;
      }
    },
  });

  poolRankingsDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testPoolDetail() {
  const poolId = samplePoolIds[Math.floor(Math.random() * samplePoolIds.length)];
  
  const res = http.get(`${BASE_URL}/api/liquidity-pools/${encodeURIComponent(poolId)}`, {
    tags: { name: 'PoolDetail' },
  });

  const success = check(res, {
    'pool detail: valid response': (r) => r.status === 200 || r.status === 404,
    'pool detail: response time < 900ms': (r) => r.timings.duration < 900,
    'pool detail: valid structure if 200': (r) => {
      if (r.status !== 200) return true;
      try {
        const data = JSON.parse(r.body);
        return data.pool !== undefined && data.snapshots !== undefined;
      } catch {
        return false;
      }
    },
  });

  poolDetailDuration.add(res.timings.duration);
  errorRate.add(!success);
}

function testPoolSnapshots() {
  const poolId = samplePoolIds[Math.floor(Math.random() * samplePoolIds.length)];
  const limits = [50, 100, 200];
  const limit = limits[Math.floor(Math.random() * limits.length)];
  
  const res = http.get(`${BASE_URL}/api/liquidity-pools/${encodeURIComponent(poolId)}/snapshots?limit=${limit}`, {
    tags: { name: 'PoolSnapshots' },
  });

  const success = check(res, {
    'pool snapshots: valid response': (r) => r.status === 200 || r.status === 404,
    'pool snapshots: response time < 1000ms': (r) => r.timings.duration < 1000,
    'pool snapshots: valid array if 200': (r) => {
      if (r.status !== 200) return true;
      try {
        const data = JSON.parse(r.body);
        return Array.isArray(data) && data.length <= limit;
      } catch {
        return false;
      }
    },
  });

  errorRate.add(!success);
}

export function setup() {
  console.log(`Starting liquidity pools load test against ${BASE_URL}`);
  
  const res = http.get(`${BASE_URL}/health`);
  if (res.status !== 200) {
    throw new Error(`Server not accessible at ${BASE_URL}`);
  }
  
  return { startTime: new Date().toISOString() };
}

export function teardown(data) {
  console.log(`Liquidity pools load test completed. Started at: ${data.startTime}`);
}
