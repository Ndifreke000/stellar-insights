import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';
import ws from 'k6/ws';
import { randomString } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';

// Custom metrics for mainnet-scale load testing
const transactionErrors = new Rate('transaction_errors');
const analyticsErrors = new Rate('analytics_errors');
const websocketErrors = new Rate('websocket_errors');

const transactionLatency = new Trend('transaction_latency');
const analyticsLatency = new Trend('analytics_latency');
const websocketLatency = new Trend('websocket_latency');

const transactionCount = new Counter('transactions_submitted');
const analyticsQueries = new Counter('analytics_queries');
const websocketConnections = new Counter('websocket_connections');
const activeConnections = new Gauge('active_websocket_connections');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const WS_URL = __ENV.WS_URL || 'ws://localhost:8080/ws';
const HORIZON_URL = __ENV.HORIZON_URL || 'https://horizon.stellar.org';
const TARGET_TPS = 10000;
const DURATION = '60s';
const RAMP_UP = '10s';
const MAX_CONNECTIONS = 1000;

// Load test configuration targeting 10,000 TPS with three concurrent scenarios
export const options = {
  stages: [
    { duration: RAMP_UP, target: Math.ceil(TARGET_TPS / 10) },      // Ramp up: 1000 VUs
    { duration: DURATION, target: Math.ceil(TARGET_TPS / 10) },     // Sustained: 1000 VUs for 10 req/s each
    { duration: '5s', target: 0 },                                   // Ramp down
  ],
  thresholds: {
    'transaction_latency': ['p(50)<100', 'p(95)<500', 'p(99)<1000'],
    'analytics_latency': ['p(95)<300', 'p(99)<800'],
    'websocket_latency': ['p(95)<200', 'p(99)<500'],
    'transaction_errors': ['rate<0.01'],
    'analytics_errors': ['rate<0.01'],
    'websocket_errors': ['rate<0.05'],
    'http_req_failed': ['rate<0.01'],
  },
};

// Scenario 1: Transaction ingestion at 10,000 TPS
// Simulates high-volume transaction submission to the ingestion endpoint
function transactionIngestionScenario() {
  group('Transaction Ingestion', () => {
    // Generate a realistic Stellar XDR transaction payload
    // Format: base64-encoded Stellar transaction envelope
    const xdr = generateStellarXdr();
    const sourceAccount = `GBRPYHIL2CI3C7OHIXBBYLCA3VU2MXXIIVU2KHV25QVKNUKE6FSKIZB${Math.floor(Math.random() * 100000)}`;

    const payload = {
      source_account: sourceAccount,
      xdr: xdr,
      required_signatures: 1,
    };

    const startTime = Date.now();
    const response = http.post(`${BASE_URL}/api/transactions`, JSON.stringify(payload), {
      headers: {
        'Content-Type': 'application/json',
      },
      tags: {
        name: 'TransactionIngestion',
        scenario: 'mainnet_10k_tps',
      },
    });
    const duration = Date.now() - startTime;

    transactionLatency.add(duration);
    transactionCount.add(1);

    const success = check(response, {
      'status is 200': (r) => r.status === 200,
      'response time < 1s': (r) => r.timings.duration < 1000,
      'transaction created': (r) => r.body && r.body.includes('id'),
      'no server errors': (r) => r.status < 500,
    });

    if (!success) {
      transactionErrors.add(1);
    }
  });

  // Slight delay to avoid overwhelming the system
  sleep(0.001);
}

// Scenario 2: Analytics dashboard queries
// Simulates concurrent users polling for updated dashboard data
function analyticsDashboardScenario() {
  group('Analytics Dashboard', () => {
    const endpoints = [
      '/api/anchors?limit=20',
      '/api/corridors?limit=20',
      '/api/analytics/summary',
      '/api/analytics/volume-24h',
    ];

    const endpoint = endpoints[Math.floor(Math.random() * endpoints.length)];
    const startTime = Date.now();

    const response = http.get(`${BASE_URL}${endpoint}`, {
      headers: {
        'Accept': 'application/json',
      },
      tags: {
        name: 'AnalyticsDashboard',
        scenario: 'mainnet_10k_tps',
      },
    });
    const duration = Date.now() - startTime;

    analyticsLatency.add(duration);
    analyticsQueries.add(1);

    const success = check(response, {
      'status is 200': (r) => r.status === 200,
      'response time < 500ms': (r) => r.timings.duration < 500,
      'response is JSON': (r) => r.headers['Content-Type'] && r.headers['Content-Type'].includes('application/json'),
      'no server errors': (r) => r.status < 500,
    });

    if (!success) {
      analyticsErrors.add(1);
    }
  });

  sleep(0.005);
}

// Scenario 3: WebSocket fan-out and message delivery latency
// Establishes multiple concurrent WebSocket connections and measures message delivery latency
function websocketFanOutScenario() {
  group('WebSocket Fan-Out', () => {
    let connectionLatency = 0;
    let messageReceived = false;

    try {
      const startConnectTime = Date.now();

      const res = ws.connect(WS_URL, function (socket) {
        connectionLatency = Date.now() - startConnectTime;
        websocketLatency.add(connectionLatency);
        websocketConnections.add(1);

        socket.on('open', () => {
          activeConnections.add(1);
        });

        socket.on('message', (msg) => {
          messageReceived = true;
        });

        socket.on('close', () => {
          activeConnections.add(-1);
        });

        // Keep connection open for measurement window
        socket.on('error', (err) => {
          websocketErrors.add(1);
          tracing.error(`WebSocket error: ${err}`);
        });

        // Subscribe to updates
        socket.send(JSON.stringify({ type: 'subscribe', channel: 'anchors' }));

        // Hold connection for 5 seconds to measure latency and delivery
        const startTime = Date.now();
        while (Date.now() - startTime < 5000) {
          sleep(0.1);
        }
        socket.close();
      });

      const success = check(res, {
        'WebSocket connection established': (r) => r.status === 101,
        'connection latency < 200ms': () => connectionLatency < 200,
        'no connection errors': (r) => r.status === 101,
      });

      if (!success) {
        websocketErrors.add(1);
      }
    } catch (err) {
      websocketErrors.add(1);
    }
  });

  sleep(0.01);
}

// Export default test function
export default function () {
  // Distribute load across three scenarios based on realistic traffic patterns
  // 60% transaction ingestion, 30% analytics reads, 10% WebSocket connections
  const rand = Math.random();

  if (rand < 0.6) {
    transactionIngestionScenario();
  } else if (rand < 0.9) {
    analyticsDashboardScenario();
  } else {
    websocketFanOutScenario();
  }
}

// Health check setup
export function setup() {
  // Verify backend is healthy before test
  const res = http.get(`${BASE_URL}/health`);
  check(res, {
    'backend is healthy': (r) => r.status === 200,
  });
}

// Helper function to generate realistic Stellar XDR
// This is a simplified base64-encoded transaction; in production,
// use the Stellar SDK to generate actual transaction envelopes
function generateStellarXdr() {
  const xdrPrefix = 'AAAAAP';
  const randomPart = randomString(100);
  return xdrPrefix + randomPart.replace(/[^A-Za-z0-9+/=]/g, '').substring(0, 100);
}
