# Mainnet-Scale 10k TPS Load Test Report

## Overview
This document describes how to run the mainnet-scale load test, capture metrics, and interpret results to identify performance bottlenecks and database connection exhaustion.

## Running the Test

### Prerequisites
- k6 installed (https://k6.io/docs/getting-started/installation/)
- Backend running at `http://localhost:8080` (or set `BASE_URL` env var)
- WebSocket endpoint available at `ws://localhost:8080/ws`
- Sufficient system resources (1000 concurrent users targeting 10,000 TPS)

### Basic Execution
```bash
k6 run backend/load-tests/mainnet_10k_tps.js
```

### Custom Configuration
```bash
# Run against staging environment
BASE_URL=https://api-staging.stellar-insights.com k6 run backend/load-tests/mainnet_10k_tps.js

# With custom WebSocket endpoint
WS_URL=wss://api-staging.stellar-insights.com/ws k6 run backend/load-tests/mainnet_10k_tps.js

# With Horizon endpoint override
HORIZON_URL=https://horizon-testnet.stellar.org k6 run backend/load-tests/mainnet_10k_tps.js

# Stream results to InfluxDB/Grafana (if configured)
k6 run -o influxdb=http://localhost:8086/mydb backend/load-tests/mainnet_10k_tps.js
```

## Metrics to Capture

### Latency Percentiles
- **p50 (median)**: Expected behavior; track for regression
- **p95**: User-facing threshold; should be < 500ms for transactions, < 300ms for analytics
- **p99**: Rare outliers; identify resource contention at the tail

**Key metrics file:**
```bash
# Export raw k6 output
k6 run --out json backend/load-tests/mainnet_10k_tps.js > results.json

# Parse with jq for specific percentiles
cat results.json | jq '.metrics.transaction_latency'
```

### Error Rate
- **transaction_errors**: Percentage of failed transaction submissions
- **analytics_errors**: Percentage of failed analytics queries
- **websocket_errors**: Percentage of WebSocket connection failures
- **http_req_failed**: Overall HTTP failure rate

**Threshold**: Error rate should stay < 1% under sustained load (< 5% for WebSocket connections).

### Throughput
- **transactions_submitted**: Total transaction count; should approach TARGET_TPS (10,000 TPS) at peak
- **analytics_queries**: Total analytics API calls
- **websocket_connections**: Total WebSocket connections established

## Identifying Database Connection Exhaustion

### Symptoms
1. **Error rate spike**: When connection pool is exhausted, new requests fail with connection timeout errors
2. **Backend logs show**: `too many connections` or `connection pool timeout` messages
3. **Latency spike**: Requests queue while waiting for available connections
4. **WebSocket drops**: New connections rejected before handshake completes

### How to Detect

#### Method 1: Monitor Backend Logs
```bash
# Watch for connection pool errors in real time
docker logs -f stellar-insights-backend | grep -i "connection\|pool\|exhausted"
```

Expected log patterns when connection pool nears limits:
```
WARN: connection pool timeout waiting for available connection
ERROR: failed to acquire connection from pool: resource exhausted
ERROR: too many connections to database
```

#### Method 2: Query Database Connection Stats
```sql
-- For PostgreSQL
SELECT 
  datname,
  count(*) as connections,
  max_conn
FROM pg_stat_activity
CROSS JOIN (SELECT setting::int as max_conn FROM pg_settings WHERE name = 'max_connections') s
GROUP BY datname, max_conn;

-- Watch for: connections >= max_conn (indicates exhaustion)
```

#### Method 3: Monitor Metrics in k6 Output
```bash
# Connection exhaustion is indicated by:
# 1. Sustained high error rate (> 1%)
# 2. Latency suddenly increasing to 10+ seconds
# 3. WebSocket error rate jumping to 50%+
```

### Finding the Exhaustion Point

1. Run the test and observe logs
2. Record the **error rate at which connections are exhausted**:
   - Identify the time (T) when error rate exceeds 1%
   - Note the active connection count at that time (from database query above)
   - Calculate: `connections_at_exhaustion = DB_POOL_MAX_CONNECTIONS`

3. Document in test report:
   ```
   Connection exhaustion observed at:
   - Concurrent users: 750
   - Error rate: 1.2%
   - Time: 45 seconds into test
   - Database connections used: 50/50 (pool full)
   - Recommendation: Increase DB_POOL_MAX_CONNECTIONS or optimize connection re-use
   ```

## Performance Baselines

### Expected Results for 10k TPS Target

| Metric | Target | Alert Threshold |
|--------|--------|-----------------|
| Transaction p95 latency | < 500ms | > 800ms |
| Analytics p95 latency | < 300ms | > 500ms |
| WebSocket p95 latency | < 200ms | > 400ms |
| Error rate (HTTP) | < 1% | > 2% |
| Error rate (WebSocket) | < 5% | > 10% |
| DB connections used | < 80% of max | > 90% of max |

### Scaling Considerations

- **10k TPS**: ~1000 concurrent users at 10 req/s each
- **20k TPS**: Requires database read replicas and caching layers
- **100k TPS**: Requires sharding and multi-region deployment

## Troubleshooting

### High Latency
1. Check backend CPU/memory usage
2. Verify database query performance (enable slow query logging)
3. Check network latency between k6 and backend

### High Error Rate
1. Review backend error logs for specific failures
2. Check database connection pool status
3. Verify WebSocket upgrade protocol compatibility

### WebSocket Failures
1. Confirm WebSocket endpoint is accessible: `curl -i -N -H "Connection: Upgrade" $WS_URL`
2. Check firewall rules for WebSocket ports
3. Verify backend WebSocket handler is running

## Cleanup After Test
```bash
# Stop any lingering connections
pkill -f "k6 run"

# Check for database connection leaks
docker logs stellar-insights-backend | tail -20
```
