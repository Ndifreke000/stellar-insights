import http from 'k6/http';
import { check } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Preflight endpoint load test (issue #2652): sustains a fixed 200 requests
// per second against GET /api/v1/preflight — the self-serve funnel's
// highest-traffic read endpoint (see docs/DEPLOY.md's smoke-test curl) —
// and records p50/p95/p99 latency.
//
// Uses `constant-arrival-rate` rather than a VU ramp (like the other
// scripts in this directory) because the acceptance criterion is a request
// RATE, not a user count: k6 auto-scales VUs as needed to actually sustain
// 200 iterations/sec regardless of how slow or fast each one responds.

const errorRate = new Rate('errors');
const preflightResponseTime = new Trend('preflight_response_time');

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const TARGET_RPS = parseInt(__ENV.TARGET_RPS || '200', 10);
const DURATION = __ENV.DURATION || '3m';

export const options = {
  scenarios: {
    preflight_200rps: {
      executor: 'constant-arrival-rate',
      rate: TARGET_RPS,
      timeUnit: '1s',
      duration: DURATION,
      // Generous headroom over the target rate/duration so k6 can keep up
      // even if the backend is briefly slow, rather than dropping iterations.
      preAllocatedVUs: Math.max(50, TARGET_RPS),
      maxVUs: TARGET_RPS * 3,
    },
  },
  thresholds: {
    http_req_duration: ['p(50)<200', 'p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.01'],
    errors: ['rate<0.01'],
  },
};

// Representative corridors — same style as corridors-load-test.js. Mixing a
// few avoids every request hitting (and only measuring) one cache entry.
const scenarios = [
  { source_asset: 'USDC', destination_asset: 'NGN', amount_usd: 2500 },
  { source_asset: 'USDC', destination_asset: 'KES', amount_usd: 500 },
  { source_asset: 'XLM', destination_asset: 'NGN', amount_usd: 100 },
  { source_asset: 'USDC', destination_asset: 'XLM', amount_usd: 1000 },
];

export default function () {
  const s = scenarios[Math.floor(Math.random() * scenarios.length)];
  const qs = `source_asset=${s.source_asset}&destination_asset=${s.destination_asset}&amount_usd=${s.amount_usd}`;
  const res = http.get(`${BASE_URL}/api/v1/preflight?${qs}`);

  const ok = check(res, {
    'status is 200': (r) => r.status === 200,
    'has a decision': (r) => {
      try {
        return ['proceed', 'caution', 'hold', 'unknown'].includes(JSON.parse(r.body).decision);
      } catch {
        return false;
      }
    },
  });

  errorRate.add(!ok);
  preflightResponseTime.add(res.timings.duration);
}
