# Load Testing Suite for Stellar Insights Backend

This directory contains comprehensive load testing scripts using k6 to verify the backend can handle expected traffic levels.

## Prerequisites

### Install k6

**macOS:**
```bash
brew install k6
```

**Windows:**
```powershell
winget install k6 --source winget
```

**Linux:**
```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Or use Docker:**
```bash
docker pull grafana/k6:latest
```

## Test Scripts

### 1. `corridors-load-test.js`
Tests the `/api/corridors` endpoint with various load patterns.

### 2. `anchors-load-test.js`
Tests the `/api/anchors` endpoint with pagination and filtering.

### 3. `rpc-endpoints-load-test.js`
Tests RPC endpoints (payments, trades, ledger data).

### 4. `full-suite-load-test.js`
Comprehensive test covering all critical endpoints with realistic traffic patterns.

### 5. `stress-test.js`
Stress test to identify breaking points and bottlenecks.

### 6. `spike-test.js`
Tests system behavior under sudden traffic spikes.

## Running Tests

### Quick Start
```bash
# Run individual test
k6 run corridors-load-test.js

# Run with custom VUs and duration
k6 run --vus 50 --duration 2m corridors-load-test.js

# Run full suite
k6 run full-suite-load-test.js

# Run with HTML report
k6 run --out json=results.json full-suite-load-test.js
```

### Using Docker
```bash
docker run --rm -i --network=host grafana/k6:latest run - <corridors-load-test.js
```

### Environment Variables
```bash
# Set custom base URL
export BASE_URL=http://localhost:8080
k6 run corridors-load-test.js

# Set custom thresholds
export MAX_RESPONSE_TIME=1000
k6 run corridors-load-test.js
```

## Test Scenarios

### Load Test (Default)
- Ramp up to 100 VUs over 2 minutes
- Maintain 100 VUs for 5 minutes
- Ramp down to 0 over 2 minutes
- **Purpose:** Verify normal operation under expected load

### Stress Test
- Ramp up to 200 VUs over 3 minutes
- Maintain 200 VUs for 10 minutes
- Ramp down to 0 over 2 minutes
- **Purpose:** Identify system limits and bottlenecks

### Spike Test
- Start with 10 VUs
- Spike to 500 VUs instantly
- Maintain for 1 minute
- Drop back to 10 VUs
- **Purpose:** Test system resilience to sudden traffic spikes

## Performance Targets

### Response Times
- **p95 < 500ms** - 95% of requests complete within 500ms
- **p99 < 1000ms** - 99% of requests complete within 1 second
- **Average < 300ms** - Average response time under 300ms

### Success Rate
- **> 99.5%** - Less than 0.5% error rate
- **No 5xx errors** - No server errors under normal load

### Throughput
- **> 100 req/s** - Minimum 100 requests per second
- **> 500 req/s** - Target 500 requests per second

## Analyzing Results

### Key Metrics
- **http_req_duration**: Request duration (response time)
- **http_req_failed**: Failed request rate
- **http_reqs**: Total requests per second
- **vus**: Virtual users (concurrent connections)
- **iterations**: Completed test iterations

### Example Output
```
     ✓ status is 200
     ✓ response time < 500ms

     checks.........................: 100.00% ✓ 10000      ✗ 0
     data_received..................: 50 MB   1.7 MB/s
     data_sent......................: 2.5 MB  83 kB/s
     http_req_blocked...............: avg=1.2ms    min=0s       med=0s       max=50ms     p(95)=5ms
     http_req_duration..............: avg=250ms    min=100ms    med=230ms    max=800ms    p(95)=450ms
     http_req_failed................: 0.00%   ✓ 0          ✗ 10000
     http_reqs......................: 10000   333.33/s
     iterations.....................: 10000   333.33/s
     vus............................: 100     min=0        max=100
```

## Troubleshooting

### High Error Rates
- Check server logs for errors
- Verify database connection pool size
- Check Redis connection limits
- Monitor system resources (CPU, memory, disk I/O)

### Slow Response Times
- Check database query performance
- Verify cache hit rates
- Monitor network latency
- Check for N+1 query problems

### Connection Errors
- Increase server connection limits
- Check firewall settings
- Verify network configuration
- Increase database pool size

## Integration with CI/CD

### GitHub Actions Example
```yaml
name: Load Tests
on: [push, pull_request]
jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install k6
        run: |
          sudo gpg -k
          sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
          echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
          sudo apt-get update
          sudo apt-get install k6
      - name: Run load tests
        run: k6 run backend/load-tests/full-suite-load-test.js
```

## Best Practices

1. **Start Small**: Begin with low VU counts and gradually increase
2. **Monitor Resources**: Watch CPU, memory, and disk I/O during tests
3. **Test Incrementally**: Test individual endpoints before full suite
4. **Use Realistic Data**: Ensure test data matches production patterns
5. **Test Regularly**: Run load tests on every major release
6. **Document Baselines**: Record baseline performance for comparison
7. **Test Edge Cases**: Include error scenarios and edge cases

## Performance Optimization Tips

### Database
- Add indexes on frequently queried columns
- Optimize slow queries
- Increase connection pool size
- Use read replicas for read-heavy workloads

### Caching
- Increase cache TTL for stable data
- Use cache warming strategies
- Monitor cache hit rates
- Implement cache invalidation properly

### Application
- Enable compression for large responses
- Use connection pooling
- Implement rate limiting
- Optimize serialization/deserialization

### Infrastructure
- Use CDN for static assets
- Enable HTTP/2
- Use load balancers
- Scale horizontally with multiple instances

## Support

For issues or questions:
1. Check server logs: `docker logs stellar-insights-backend`
2. Review k6 documentation: https://k6.io/docs/
3. Check application metrics and monitoring
