# Graceful Shutdown Testing Guide

## Quick Start

### Linux/macOS

```bash
# Make the test script executable
chmod +x test_graceful_shutdown.sh

# Run the test
./test_graceful_shutdown.sh
```

### Windows (PowerShell)

```powershell
# Start the server
$env:RUST_LOG="info"
cargo run --release

# In another terminal, send Ctrl+C or:
Stop-Process -Name "stellar-insights-backend" -Force
```

## Manual Testing

### Test 1: Basic Shutdown

1. Start the server:
   ```bash
   cd backend
   RUST_LOG=info cargo run
   ```

2. In another terminal, send SIGTERM:
   ```bash
   # Find the process ID
   ps aux | grep stellar-insights-backend
   
   # Send SIGTERM
   kill -TERM <PID>
   ```

3. Observe the shutdown sequence in logs:
   - Signal received
   - Server stops accepting connections
   - Background tasks shutdown
   - WebSocket connections closed
   - Cache flushed
   - Database closed
   - Shutdown complete

### Test 2: Shutdown Under Load

1. Start the server:
   ```bash
   cargo run --release
   ```

2. Start a load test:
   ```bash
   cd load-tests
   k6 run --duration 60s anchors-load-test.js
   ```

3. During the load test (after ~10s), send shutdown signal:
   ```bash
   kill -TERM <PID>
   ```

4. Verify:
   - In-flight requests complete successfully
   - No connection errors during graceful period
   - Load test shows minimal errors
   - Shutdown completes within timeout

### Test 3: WebSocket Shutdown

1. Start the server:
   ```bash
   cargo run
   ```

2. Connect a WebSocket client:
   ```javascript
   // In browser console or Node.js
   const ws = new WebSocket('ws://localhost:8080/ws');
   
   ws.onmessage = (event) => {
     console.log('Received:', event.data);
   };
   
   ws.onclose = (event) => {
     console.log('Connection closed:', event.code, event.reason);
   };
   ```

3. Send shutdown signal

4. Verify:
   - Client receives `ServerShutdown` message
   - Connection closes gracefully
   - No abrupt disconnection

### Test 4: Timeout Behavior

1. Set very short timeouts in `.env`:
   ```bash
   SHUTDOWN_GRACEFUL_TIMEOUT=2
   SHUTDOWN_BACKGROUND_TIMEOUT=1
   SHUTDOWN_DB_TIMEOUT=1
   ```

2. Start server and send shutdown signal

3. Verify:
   - Timeouts are respected
   - Warning logs for timeout conditions
   - Shutdown proceeds despite timeouts

## Docker Testing

### Build and Test

```bash
# Build the image
docker build -t stellar-insights-backend .

# Run the container
docker run -d --name backend \
  -p 8080:8080 \
  -e SHUTDOWN_GRACEFUL_TIMEOUT=30 \
  stellar-insights-backend

# Check it's running
curl http://localhost:8080/health

# Send SIGTERM (Docker stop)
docker stop backend

# Check logs
docker logs backend

# Verify graceful shutdown in logs
docker logs backend | grep -i shutdown
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'
services:
  backend:
    build: .
    ports:
      - "8080:8080"
    environment:
      - SHUTDOWN_GRACEFUL_TIMEOUT=30
      - SHUTDOWN_BACKGROUND_TIMEOUT=10
      - SHUTDOWN_DB_TIMEOUT=5
    stop_grace_period: 45s  # Must be > SHUTDOWN_GRACEFUL_TIMEOUT
```

Test:
```bash
docker-compose up -d
docker-compose stop
docker-compose logs | grep -i shutdown
```

## Kubernetes Testing

### Deploy Test Pod

```yaml
# test-pod.yaml
apiVersion: v1
kind: Pod
metadata:
  name: stellar-backend-test
spec:
  terminationGracePeriodSeconds: 60
  containers:
  - name: backend
    image: stellar-insights-backend:latest
    ports:
    - containerPort: 8080
    env:
    - name: SHUTDOWN_GRACEFUL_TIMEOUT
      value: "30"
    - name: SHUTDOWN_BACKGROUND_TIMEOUT
      value: "10"
    - name: SHUTDOWN_DB_TIMEOUT
      value: "5"
    livenessProbe:
      httpGet:
        path: /health
        port: 8080
      initialDelaySeconds: 10
      periodSeconds: 5
    readinessProbe:
      httpGet:
        path: /health
        port: 8080
      initialDelaySeconds: 5
      periodSeconds: 3
```

Test:
```bash
# Deploy
kubectl apply -f test-pod.yaml

# Wait for ready
kubectl wait --for=condition=ready pod/stellar-backend-test

# Check it's running
kubectl port-forward stellar-backend-test 8080:8080 &
curl http://localhost:8080/health

# Delete pod (triggers SIGTERM)
kubectl delete pod stellar-backend-test

# Check logs
kubectl logs stellar-backend-test | grep -i shutdown
```

## Load Testing During Shutdown

### Using k6

```javascript
// shutdown-load-test.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '10s', target: 50 },  // Ramp up
    { duration: '30s', target: 50 },  // Steady state (send SIGTERM here)
    { duration: '10s', target: 0 },   // Ramp down
  ],
};

export default function () {
  const res = http.get('http://localhost:8080/api/anchors');
  
  check(res, {
    'status is 200 or 503': (r) => r.status === 200 || r.status === 503,
    'response time < 500ms': (r) => r.timings.duration < 500,
  });
  
  sleep(0.1);
}
```

Run test:
```bash
# Terminal 1: Start server
cargo run --release

# Terminal 2: Start load test
k6 run shutdown-load-test.js

# Terminal 3: After 15s, send shutdown
kill -TERM <PID>
```

Expected results:
- Most requests complete successfully (200)
- Some requests may get 503 during shutdown
- No connection errors or timeouts
- Graceful degradation

## Monitoring Shutdown Metrics

### Log Analysis

```bash
# Extract shutdown duration
grep "Graceful shutdown completed" backend.log | \
  grep -oP 'in \K[0-9.]+' | \
  awk '{sum+=$1; count++} END {print "Average:", sum/count, "seconds"}'

# Count successful shutdowns
grep -c "Graceful shutdown complete" backend.log

# Find timeout warnings
grep "did not complete within" backend.log

# Check background task shutdown
grep "task shutting down" backend.log | wc -l
```

### Prometheus Metrics (Future)

```promql
# Shutdown duration histogram
histogram_quantile(0.95, rate(shutdown_duration_seconds_bucket[5m]))

# Shutdown success rate
rate(shutdown_success_total[5m]) / rate(shutdown_attempts_total[5m])

# Background task completion rate
rate(background_task_shutdown_success[5m])
```

## Troubleshooting

### Issue: Shutdown hangs

**Symptoms:**
- Process doesn't exit
- Logs show no progress after certain step

**Debug:**
```bash
# Check what the process is doing
strace -p <PID>

# Check open connections
lsof -p <PID>

# Check threads
ps -T -p <PID>
```

**Solutions:**
- Increase timeout values
- Check for blocking operations in background tasks
- Review database query performance

### Issue: Requests fail during shutdown

**Symptoms:**
- Connection refused errors
- 503 responses

**Expected Behavior:**
- This is normal during shutdown
- Load balancers should stop routing traffic
- Clients should retry

**Solutions:**
- Ensure load balancer health checks work
- Implement client retry logic
- Increase graceful timeout if needed

### Issue: Database connection errors

**Symptoms:**
- "connection closed" errors in logs
- Database queries fail during shutdown

**Debug:**
```bash
# Check database connections
sqlite3 stellar_insights.db ".databases"

# Check for locks
fuser stellar_insights.db
```

**Solutions:**
- Increase `SHUTDOWN_DB_TIMEOUT`
- Ensure no new queries after shutdown signal
- Check connection pool settings

## Best Practices

1. **Always test shutdown under load**
   - Simulates production conditions
   - Reveals race conditions
   - Validates timeout settings

2. **Monitor shutdown metrics**
   - Track shutdown duration
   - Alert on failures
   - Analyze patterns

3. **Set appropriate timeouts**
   - Graceful timeout > longest request
   - Background timeout > task iteration
   - DB timeout > connection cleanup

4. **Test in production-like environment**
   - Use Docker/Kubernetes
   - Test with real load
   - Verify orchestrator integration

5. **Document expected behavior**
   - What clients should expect
   - How to handle 503 responses
   - Retry strategies

## Automated Testing

### CI/CD Integration

```yaml
# .github/workflows/test-shutdown.yml
name: Test Graceful Shutdown

on: [push, pull_request]

jobs:
  test-shutdown:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Build
        run: cargo build --release
      
      - name: Run shutdown test
        run: |
          cd backend
          ./test_graceful_shutdown.sh
      
      - name: Check logs
        if: failure()
        run: cat /tmp/stellar_backend.log
```

### Unit Tests

```rust
#[tokio::test]
async fn test_graceful_shutdown() {
    // Start server
    let server = tokio::spawn(async {
        // Server code
    });
    
    // Wait for ready
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Send shutdown signal
    shutdown_coordinator.trigger_shutdown();
    
    // Wait for completion
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        server
    ).await;
    
    assert!(result.is_ok(), "Server should shutdown within timeout");
}
```

## Performance Benchmarks

Expected shutdown times:
- Idle server: < 2 seconds
- Under light load (10 req/s): < 5 seconds
- Under heavy load (100 req/s): < 15 seconds
- With active WebSockets: < 10 seconds

If shutdown takes longer:
1. Check background task logic
2. Review database query performance
3. Analyze Redis connection latency
4. Consider increasing timeouts
