# Load Testing Suite for Stellar Insights Backend

Comprehensive load testing suite using k6 to verify backend performance under realistic traffic conditions.

## Prerequisites

### Install k6

**macOS:**
```bash
brew install k6
```

**Windows:**
```powershell
choco install k6
# or
winget install k6
```

**Linux:**
```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Docker:**
```bash
docker pull grafana/k6:latest
```

## Test Suites

### 1. Corridor Endpoints Test (`k6-corridors.js`)
Tests the most critical endpoints for payment corridor analytics.

**Load Profile:**
- Ramp up: 50 users over 2 minutes
- Steady: 100 users for 5 minutes
- Spike: 200 users for 3 minutes
- Ramp down: 2 minutes

**Endpoints Tested:**
- `GET /api/corridors` - List all corridors
- `GET /api/corridors?filters` - Filtered corridor queries
- `GET /api/corridors/:key` - Corridor detail
- `GET /health` - Health check

**Performance Targets:**
- 95th percentile: < 500ms
- 99th percentile: < 1000ms
- Error rate: < 5%

### 2. Anchor Endpoints Test (`k6-anchors.js`)
Tests anchor-related endpoints with moderate load.

**Load Profile:**
- Ramp up: 30 users over 1 minute
- Steady: 50 users for 3 minutes
- Spike: 100 users for 1 minute
- Recovery: 50 users for 2 minutes

**Endpoints Tested:**
- `GET /api/anchors` - List anchors
- `GET /api/anchors/:id` - Anchor details
- `GET /api/anchors/:id/assets` - Anchor assets

**Performance Targets:**
- 95th percentile: < 400ms
- 99th percentile: < 800ms
- Error rate: < 3%

### 3. RPC Endpoints Test (`k6-rpc.js`)
Tests Stellar RPC proxy endpoints (higher latency expected).

**Load Profile:**
- Gentle ramp: 20 users over 1 minute
- Steady: 40 users for 3 minutes
- Spike: 60 users for 1 minute

**Endpoints Tested:**
- `GET /api/rpc/health` - RPC health check
- `GET /api/rpc/payments` - Recent payments
- `GET /api/rpc/payments/account/:id` - Account payments
- `GET /api/rpc/trades` - Recent trades
- `GET /api/rpc/ledger/latest` - Latest ledger

**Performance Targets:**
- 95th percentile: < 2000ms (RPC calls are slower)
- 99th percentile: < 3000ms
- Error rate: < 5%

### 4. Liquidity Pools Test (`k6-liquidity-pools.js`)
Tests liquidity pool analysis endpoints.

**Load Profile:**
- Ramp up: 25 users over 1 minute
- Steady: 50 users for 3 minutes
- Spike: 80 users for 1 minute
- Recovery: 50 users for 2 minutes

**Endpoints Tested:**
- `GET /api/liquidity-pools` - List all pools
- `GET /api/liquidity-pools/stats` - Pool statistics
- `GET /api/liquidity-pools/rankings` - Pool rankings
- `GET /api/liquidity-pools/:id` - Pool detail
- `GET /api/liquidity-pools/:id/snapshots` - Historical snapshots

**Performance Targets:**
- 95th percentile: < 800ms
- 99th percentile: < 1500ms
- Error rate: < 5%

### 5. Fee Bump Tracking Test (`k6-fee-bumps.js`)
Tests fee bump transaction tracking endpoints.

**Load Profile:**
- Ramp up: 20 users over 1 minute
- Steady: 40 users for 3 minutes
- Spike: 60 users for 1 minute
- Recovery: 40 users for 2 minutes

**Endpoints Tested:**
- `GET /api/fee-bumps/stats` - Fee bump statistics
- `GET /api/fee-bumps/recent` - Recent fee bump transactions

**Performance Targets:**
- 95th percentile: < 600ms
- 99th percentile: < 1000ms
- Error rate: < 5%

### 6. Cache & Metrics Test (`k6-cache-metrics.js`)
Tests system monitoring and cache performance endpoints.

**Load Profile:**
- Lighter load for monitoring endpoints
- Peak: 40 concurrent users
- Duration: ~5 minutes

**Endpoints Tested:**
- `GET /api/cache/stats` - Cache hit/miss statistics
- `GET /api/metrics/overview` - System metrics overview

**Performance Targets:**
- 95th percentile: < 300ms
- 99th percentile: < 500ms
- Error rate: < 2%

### 7. Full Suite Test (`k6-full-suite.js`)
Comprehensive test simulating realistic user behavior across all endpoints.

**Load Profile:**
- 18-minute test with multiple stages
- Peak load: 300 concurrent users
- Stress testing included

**Features:**
- Weighted endpoint distribution:
  - 35% corridors
  - 25% anchors
  - 15% RPC
  - 10% liquidity pools
  - 8% fee bumps
  - 7% system monitoring
- Cache hit rate monitoring
- HTML report generation
- JSON summary export

## Running Tests

### Start the Backend Server
```bash
cd backend
cargo run --release
```

The server should be running on `http://localhost:8080`

### Run Individual Test Suites

**Corridor Test:**
```bash
k6 run load-tests/k6-corridors.js
```

**Anchor Test:**
```bash
k6 run load-tests/k6-anchors.js
```

**RPC Test:**
```bash
k6 run load-tests/k6-rpc.js
```

**Liquidity Pools Test:**
```bash
k6 run load-tests/k6-liquidity-pools.js
```

**Fee Bumps Test:**
```bash
k6 run load-tests/k6-fee-bumps.js
```

**Cache & Metrics Test:**
```bash
k6 run load-tests/k6-cache-metrics.js
```

**Full Suite:**
```bash
k6 run load-tests/k6-full-suite.js
```

### Run All Tests Sequentially

**Unix/Linux/macOS:**
```bash
chmod +x load-tests/run-all-tests.sh
./load-tests/run-all-tests.sh
```

**Windows PowerShell:**
```powershell
.\load-tests\run-all-tests.ps1
```

### Custom Configuration

**Change target URL:**
```bash
k6 run -e BASE_URL=http://production-server:8080 load-tests/k6-corridors.js
```

**Adjust load:**
```bash
# Quick smoke test
k6 run --vus 10 --duration 30s load-tests/k6-corridors.js

# Extended soak test
k6 run --vus 50 --duration 30m load-tests/k6-corridors.js
```

**Output to file:**
```bash
k6 run --out json=results.json load-tests/k6-corridors.js
```

### Using Docker

```bash
docker run --rm -i --network=host \
  -v $(pwd)/load-tests:/scripts \
  grafana/k6:latest run /scripts/k6-corridors.js
```

## Interpreting Results

### Key Metrics

**http_req_duration**: Response time distribution
- p(95): 95% of requests completed within this time
- p(99): 99% of requests completed within this time
- avg: Average response time
- max: Maximum response time

**http_req_failed**: Percentage of failed requests
- Should be < 5% for passing tests

**http_reqs**: Request rate
- Requests per second throughput

**Custom Metrics:**
- `corridor_list_duration`: Specific timing for corridor list endpoint
- `error_rate`: Overall error rate across all requests
- `cache_hits`: Cache hit rate (full suite only)

### Success Criteria

✅ **Pass:**
- All thresholds met
- Error rate < 5%
- 95th percentile within targets
- No server crashes or timeouts

⚠️ **Warning:**
- Some thresholds exceeded
- Error rate 5-10%
- Degraded performance under peak load

❌ **Fail:**
- Multiple threshold violations
- Error rate > 10%
- Server crashes or becomes unresponsive

### Example Output

```
✓ list corridors: status is 200
✓ list corridors: response time < 500ms
✓ list corridors: has valid JSON

checks.........................: 98.50% ✓ 5910  ✗ 90
data_received..................: 12 MB  40 kB/s
data_sent......................: 1.2 MB 4.0 kB/s
http_req_duration..............: avg=245ms min=45ms med=198ms max=1.2s p(95)=456ms p(99)=789ms
http_req_failed................: 1.50%  ✓ 90    ✗ 5910
http_reqs......................: 6000   20/s
```

## Performance Bottleneck Identification

### High Response Times
- Check database query performance
- Review cache hit rates
- Examine RPC call latency
- Monitor CPU/memory usage

### High Error Rates
- Check server logs for errors
- Verify database connections
- Review rate limiting configuration
- Check external API availability (Stellar RPC)

### Low Throughput
- Check connection pool sizes
- Review async task handling
- Examine database connection limits
- Monitor system resources

## Continuous Integration

### GitHub Actions Example

```yaml
name: Load Tests

on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM
  workflow_dispatch:

jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Start backend
        run: |
          cd backend
          cargo build --release
          cargo run --release &
          sleep 10
      
      - name: Install k6
        run: |
          sudo gpg -k
          sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
          echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
          sudo apt-get update
          sudo apt-get install k6
      
      - name: Run load tests
        run: k6 run load-tests/k6-full-suite.js
      
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: load-test-results
          path: |
            load-test-results.html
            load-test-summary.json
```

## Apache Bench Alternative

For quick smoke tests, you can also use Apache Bench:

```bash
# Install
sudo apt-get install apache2-utils  # Linux
brew install httpd                   # macOS

# Test corridor endpoint
ab -n 1000 -c 10 http://localhost:8080/api/corridors

# With keep-alive
ab -n 1000 -c 10 -k http://localhost:8080/api/corridors
```

## Monitoring During Tests

### Server Metrics
```bash
# CPU and memory
htop

# Network connections
netstat -an | grep 8080 | wc -l

# Database connections
sqlite3 stellar_insights.db "PRAGMA database_list;"
```

### Application Logs
```bash
# Follow logs
tail -f backend.log

# Filter errors
grep ERROR backend.log
```

## Troubleshooting

### "Server not accessible" Error
- Ensure backend is running: `curl http://localhost:8080/health`
- Check firewall settings
- Verify port 8080 is not in use

### High Error Rates
- Check server logs for specific errors
- Verify database is accessible
- Ensure Redis is running (if configured)
- Check RPC_MOCK_MODE setting

### Timeouts
- Increase timeout in test scripts
- Check external API availability (Stellar Horizon/RPC)
- Review database query performance
- Monitor system resources

## Performance Optimization Tips

1. **Enable caching**: Ensure Redis is configured and running
2. **Database indexing**: Review query plans and add indexes
3. **Connection pooling**: Adjust SQLx pool size
4. **Rate limiting**: Configure appropriate limits
5. **Async optimization**: Review tokio runtime configuration
6. **RPC caching**: Cache RPC responses when possible

## Next Steps

After running load tests:

1. Document baseline performance metrics
2. Identify bottlenecks from test results
3. Optimize critical paths
4. Re-run tests to verify improvements
5. Set up continuous monitoring
6. Configure alerts for performance degradation

## Resources

- [k6 Documentation](https://k6.io/docs/)
- [k6 Best Practices](https://k6.io/docs/testing-guides/test-types/)
- [Performance Testing Guide](https://k6.io/docs/testing-guides/)
