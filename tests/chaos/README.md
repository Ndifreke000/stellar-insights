# Chaos Engineering Tests

This directory contains chaos engineering tests that validate Stellar Insights backend resilience under failure scenarios. These tests help identify weaknesses in error handling, circuit breaker implementation, and graceful degradation.

## Overview

### Test Scenarios

1. **RPC Network Partition** (`rpc_partition_test.sh`)
   - Simulates Stellar RPC endpoint becoming unreachable
   - Validates backend returns 503 when RPC is unavailable
   - Verifies frontend displays degraded mode banner
   - Tests WebSocket reconnection resilience

2. **Database Network Partition** (`db_partition_test.sh`)
   - Simulates database becoming unreachable
   - Validates all data-dependent endpoints return 503
   - Tests read-only/cached endpoints if caching is implemented
   - Monitors backend error logging during partition

## Prerequisites

### System Requirements
- Root or sudo access (required for `iptables` or `tc` network manipulation)
- Linux system with network manipulation tools installed
- Docker (recommended for backend and database management)
- `curl` for HTTP testing
- `jq` for JSON parsing (optional)

### Tool Installation

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y iptables iproute2 curl docker.io

# macOS (using Homebrew)
brew install iptables2 iproute2mac curl

# Optional: WebSocket testing
cargo install websocat

# Optional: Network utilities
sudo apt-get install -y netcat-openbsd dnsutils
```

### Backend Requirements
- Backend running and accessible at `BASE_URL` (default: `http://localhost:8080`)
- Database accessible at `DB_HOST:DB_PORT` (default: `localhost:5432`)
- RPC endpoint accessible at `RPC_HOST` (default: `horizon.stellar.org`)

## Running the Tests

### Prerequisites Check
Verify your environment before running chaos tests:

```bash
# Check for required tools
command -v iptables && echo "✓ iptables"
command -v curl && echo "✓ curl"
command -v docker && echo "✓ docker"

# Verify backend is running
curl http://localhost:8080/health

# Verify database is accessible
nc -zv localhost 5432
```

### RPC Partition Test

```bash
# Run with default configuration (localhost backend)
sudo ./tests/chaos/rpc_partition_test.sh

# Run against staging environment
sudo BASE_URL=https://api-staging.example.com \
       RPC_HOST=horizon-testnet.stellar.org \
       ./tests/chaos/rpc_partition_test.sh

# Run with custom frontend URL
sudo BASE_URL=http://localhost:8080 \
       FRONTEND_URL=http://localhost:3000 \
       ./tests/chaos/rpc_partition_test.sh
```

### Database Partition Test

```bash
# Run with default configuration (localhost database)
sudo ./tests/chaos/db_partition_test.sh

# Run against staging database
sudo BASE_URL=https://api-staging.example.com \
       DB_HOST=db-staging.internal \
       DB_PORT=5432 \
       ./tests/chaos/db_partition_test.sh

# Run with specific database credentials
sudo BASE_URL=http://localhost:8080 \
       DB_HOST=postgres.internal \
       DB_PORT=3306 \
       ./tests/chaos/db_partition_test.sh
```

### Running All Chaos Tests

```bash
# Create a test runner script
#!/bin/bash
echo "Running chaos engineering test suite..."

sudo ./tests/chaos/rpc_partition_test.sh
sleep 60  # Wait for backend to stabilize

sudo ./tests/chaos/db_partition_test.sh
sleep 60

echo "All chaos tests completed"
```

## Expected Outcomes

### RPC Network Partition Test

✅ **Expected Behavior**
1. Network partition successfully blocks RPC connectivity
2. Backend responds with HTTP 503 to `/api/rpc/health` endpoint
3. Backend continues serving cached responses if implemented
4. Frontend displays degraded mode indicator (if implemented)
5. WebSocket connections attempt reconnection (logs show retry attempts)
6. Network partition is restored after test completes
7. Backend gradually recovers as connections are re-established
8. No persistent data corruption occurs

❌ **Failure Indicators**
- Backend continues returning 200 when RPC is unreachable (missing circuit breaker)
- Frontend shows no indication of service degradation
- WebSocket connections drop without reconnection attempts
- RPC connectivity not restored after test cleanup
- Backend error logs show unhandled exceptions instead of graceful 503

### Database Network Partition Test

✅ **Expected Behavior**
1. Network partition successfully blocks database connectivity
2. All data-dependent endpoints return HTTP 503
3. Backend logs show "connection pool timeout" or similar messages
4. Read-only endpoints return 200 if caching is implemented
5. Database connection pool eventually exhausted (logs show warning)
6. Network partition is restored after test completes
7. Backend re-establishes database connections within 30 seconds
8. All endpoints return to normal operation

❌ **Failure Indicators**
- Backend continues serving stale data without indicating degradation
- Connection pool exhaustion causes unhandled panics (logs show crashes)
- Database connections not restored after partition is lifted
- No differentiation between data and read-only endpoints
- Connection leaks prevent backend from recovering

## Interpreting Results

### Connection Pool Exhaustion

Watch backend logs during database partition test:

```bash
# Terminal 1: Run the chaos test
sudo ./tests/chaos/db_partition_test.sh

# Terminal 2: Monitor logs in real-time
docker logs -f stellar-insights-backend | grep -iE "pool|connection|timeout"
```

Expected log progression:
```
[00:00] Database partition starts
[00:05] WARN: Connection pool acquiring connection timeout
[00:10] ERROR: Unable to acquire connection from pool
[00:15] WARN: Connection pool (50/50 used) - exhausted
[00:20] Circuit breaker activated - returning 503
[02:30] Database partition restored
[02:35] INFO: Successfully re-established database connection
[02:40] INFO: Connection pool recovered (2/50 used)
```

### Metrics to Monitor

During chaos tests, monitor these key metrics:

1. **Error Rate** - Should spike to 100% when partition is active, then drop to near-zero after recovery
2. **Latency** - Should remain low (<100ms) for endpoints that don't need the partitioned service
3. **Connection Pool Usage** - Monitor growth during partition; should drop after recovery
4. **HTTP Status Codes** - Count 503 responses during partition, 200+ responses after recovery

## Troubleshooting

### "Permission Denied" Error

```bash
# Solution: Run with sudo
sudo ./tests/chaos/rpc_partition_test.sh

# Or grant sudo permissions for this specific script
sudo visudo
# Add line: yourusername ALL=(ALL) NOPASSWD: /path/to/tests/chaos/*.sh
```

### "iptables: No such file or directory"

```bash
# Install iptables
sudo apt-get install iptables

# Or use traffic control instead
command -v tc  # Should be available from iproute2 package
```

### Backend Not Recovering After Test

The backend may take time to close stale connections:

```bash
# Force restart if stuck
docker-compose restart backend

# Or manually clean up connections
docker exec stellar-insights-backend bash -c "pkill -f 'connection_pool'"
```

### Partition Not Taking Effect

Verify the partition is actually blocking traffic:

```bash
# For iptables partition
sudo iptables -L | grep DROP  # Should show your rule

# For tc partition
sudo tc qdisc show dev lo  # Should show netem rules

# Test connectivity directly
timeout 5 curl -v http://horizon.stellar.org
# Should timeout or refuse connection
```

## Security Warnings

⚠️ **NEVER run these tests against production environments** unless explicitly authorized by your infrastructure team.

- These tests disrupt network connectivity and may cascade to other services
- Database partition tests may cause data consistency issues if held too long
- Always run in isolated environments (staging, development, or dedicated test clusters)
- Inform other team members before running to avoid false alerts

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Chaos Tests
on: [push, pull_request]

jobs:
  chaos:
    runs-on: ubuntu-latest
    services:
      db:
        image: postgres:14
        env:
          POSTGRES_DB: stellar_insights
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      backend:
        image: stellar-insights:latest
    steps:
      - uses: actions/checkout@v2
      - name: Run RPC partition test
        run: sudo tests/chaos/rpc_partition_test.sh
      - name: Run DB partition test
        run: sudo tests/chaos/db_partition_test.sh
```

## Further Reading

- [Principles of Chaos Engineering](https://principlesofchaosengineering.org/)
- [Linux traffic control (tc) man page](https://man7.org/linux/man-pages/man8/tc.8.html)
- [iptables man page](https://man7.org/linux/man-pages/man8/iptables.8.html)
- [Circuit Breaker Pattern](https://martinfowler.com/bliki/CircuitBreaker.html)
- [Graceful Degradation Best Practices](https://www.smashingmagazine.com/2017/07/designing-resilient-systems-part-1/)
