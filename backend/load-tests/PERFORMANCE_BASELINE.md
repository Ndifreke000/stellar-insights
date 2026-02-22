# Performance Baseline Documentation

This document establishes performance baselines and targets for the Stellar Insights Backend.

## System Specifications

Document your test environment:

```
CPU: [e.g., Intel i7-9700K @ 3.6GHz, 8 cores]
RAM: [e.g., 16GB DDR4]
Storage: [e.g., NVMe SSD]
OS: [e.g., Windows 11 / Ubuntu 22.04]
Database: SQLite / PostgreSQL
Redis: Version X.X
```

## Performance Targets

### Response Time Targets

| Metric | Target | Acceptable | Critical |
|--------|--------|------------|----------|
| Average Response Time | < 200ms | < 300ms | > 500ms |
| p95 Response Time | < 400ms | < 500ms | > 800ms |
| p99 Response Time | < 800ms | < 1000ms | > 2000ms |
| Max Response Time | < 2000ms | < 3000ms | > 5000ms |

### Throughput Targets

| Metric | Target | Acceptable | Critical |
|--------|--------|------------|----------|
| Requests/sec | > 200 | > 100 | < 50 |
| Concurrent Users | 150+ | 100+ | < 50 |

### Error Rate Targets

| Metric | Target | Acceptable | Critical |
|--------|--------|------------|----------|
| Error Rate | < 0.1% | < 1% | > 5% |
| 5xx Errors | 0% | < 0.5% | > 2% |

## Endpoint-Specific Baselines

### /api/corridors

**Expected Performance:**
- Average: 250ms
- p95: 450ms
- p99: 800ms
- Cache Hit Rate: > 80%

**Load Capacity:**
- 100 concurrent users
- 150 req/sec sustained

**Bottlenecks:**
- RPC payment fetching (200 payments)
- Price feed lookups
- Asset pair extraction

### /api/anchors

**Expected Performance:**
- Average: 200ms
- p95: 400ms
- p99: 700ms
- Cache Hit Rate: > 85%

**Load Capacity:**
- 100 concurrent users
- 180 req/sec sustained

**Bottlenecks:**
- Database anchor queries
- RPC account payment fetching
- Asset aggregation

### /api/rpc/payments

**Expected Performance:**
- Average: 400ms
- p95: 800ms
- p99: 1500ms

**Load Capacity:**
- 50 concurrent users
- 80 req/sec sustained

**Bottlenecks:**
- External Horizon API calls
- Network latency
- Response parsing

### /api/rpc/trades

**Expected Performance:**
- Average: 450ms
- p95: 900ms
- p99: 1600ms

**Load Capacity:**
- 50 concurrent users
- 75 req/sec sustained

**Bottlenecks:**
- External Horizon API calls
- Trade data volume
- Response parsing

## Baseline Test Results

### Test Date: [YYYY-MM-DD]

#### Corridors Load Test
```
Duration: 9 minutes
Virtual Users: 100
Total Requests: [FILL IN]
Requests/sec: [FILL IN]
Average Response Time: [FILL IN]ms
p95 Response Time: [FILL IN]ms
p99 Response Time: [FILL IN]ms
Error Rate: [FILL IN]%
```

#### Anchors Load Test
```
Duration: 9 minutes
Virtual Users: 100
Total Requests: [FILL IN]
Requests/sec: [FILL IN]
Average Response Time: [FILL IN]ms
p95 Response Time: [FILL IN]ms
p99 Response Time: [FILL IN]ms
Error Rate: [FILL IN]%
```

#### RPC Endpoints Load Test
```
Duration: 9 minutes
Virtual Users: 50
Total Requests: [FILL IN]
Requests/sec: [FILL IN]
Average Response Time: [FILL IN]ms
p95 Response Time: [FILL IN]ms
p99 Response Time: [FILL IN]ms
Error Rate: [FILL IN]%
```

#### Full Suite Load Test
```
Duration: 16 minutes
Max Virtual Users: 150
Total Requests: [FILL IN]
Requests/sec: [FILL IN]
Average Response Time: [FILL IN]ms
p95 Response Time: [FILL IN]ms
p99 Response Time: [FILL IN]ms
Error Rate: [FILL IN]%
```

## Performance Degradation Indicators

Watch for these warning signs:

### Response Time Degradation
- p95 > 500ms consistently
- p99 > 1000ms consistently
- Increasing trend over time

### Throughput Degradation
- Requests/sec dropping below 100
- Increasing queue times
- Connection pool exhaustion

### Error Rate Increase
- Error rate > 1%
- 5xx errors appearing
- Timeout errors increasing

### Resource Exhaustion
- CPU > 80% sustained
- Memory > 90%
- Database connections maxed
- Redis connection issues

## Optimization Recommendations

### Database Optimization
1. Add indexes on frequently queried columns
2. Optimize slow queries (use EXPLAIN)
3. Increase connection pool size
4. Consider read replicas

### Cache Optimization
1. Increase cache TTL for stable data
2. Implement cache warming
3. Monitor cache hit rates
4. Optimize cache key structure

### Application Optimization
1. Enable response compression
2. Implement connection pooling
3. Optimize serialization
4. Reduce N+1 queries

### Infrastructure Optimization
1. Use CDN for static assets
2. Enable HTTP/2
3. Implement load balancing
4. Scale horizontally

## Monitoring and Alerting

### Key Metrics to Monitor

**Application Metrics:**
- Request rate
- Response times (avg, p95, p99)
- Error rates
- Cache hit rates

**System Metrics:**
- CPU usage
- Memory usage
- Disk I/O
- Network I/O

**Database Metrics:**
- Connection pool usage
- Query execution time
- Slow query count
- Lock wait time

**External Dependencies:**
- Horizon API response time
- Redis latency
- Price feed API response time

### Alert Thresholds

**Critical Alerts:**
- Error rate > 5%
- p99 response time > 2000ms
- CPU > 90% for 5 minutes
- Memory > 95%
- Database connections > 90%

**Warning Alerts:**
- Error rate > 1%
- p95 response time > 500ms
- CPU > 80% for 10 minutes
- Memory > 85%
- Cache hit rate < 70%

## Regression Testing

Run load tests:
- Before each major release
- After infrastructure changes
- After database schema changes
- Monthly for baseline tracking

Compare results against this baseline to detect performance regressions.

## Notes

- All tests should be run with realistic data volumes
- Test environment should match production as closely as possible
- Run tests during off-peak hours to avoid interference
- Document any deviations from baseline configuration
- Update baselines after significant optimizations

## Changelog

| Date | Version | Changes | Baseline Updated |
|------|---------|---------|------------------|
| YYYY-MM-DD | 1.0 | Initial baseline | Yes |
| | | | |
