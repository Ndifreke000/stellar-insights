# Graceful Shutdown Implementation

## Overview

The Stellar Insights Backend now implements comprehensive graceful shutdown handling to ensure data integrity and clean resource cleanup when the server receives termination signals (SIGTERM, SIGINT/Ctrl+C).

## Features

### Signal Handling
- **SIGTERM**: Gracefully handles termination signals from orchestrators (Docker, Kubernetes, systemd)
- **SIGINT (Ctrl+C)**: Handles manual interruption during development
- **Cross-platform**: Works on Unix-like systems (Linux, macOS) and Windows

### Shutdown Sequence

When a shutdown signal is received, the server executes the following sequence:

1. **Stop Accepting New Connections** (configurable timeout: 30s default)
   - Server stops accepting new HTTP requests
   - In-flight requests are allowed to complete within the timeout
   - Axum's built-in graceful shutdown ensures proper connection draining

2. **Shutdown Background Tasks** (configurable timeout: 10s default)
   - Metrics synchronization task
   - Ledger ingestion task
   - Liquidity pool sync task
   - Trustline stats sync task
   - RealtimeBroadcaster task
   - Webhook dispatcher task
   - All tasks receive shutdown signal via broadcast channel
   - Tasks complete current operations and exit cleanly

3. **Close WebSocket Connections** (timeout: 5s)
   - Broadcasts shutdown notification to all connected clients
   - Allows clients 500ms to receive the message
   - Closes all active WebSocket connections
   - Cleans up connection state

4. **Flush Cache and Close Redis** (configurable timeout: 5s default)
   - Logs cache statistics (hits, misses, invalidations, hit rate)
   - Ensures pending Redis operations complete
   - Closes Redis connections gracefully

5. **Close Database Connections** (configurable timeout: 5s default)
   - Closes SQLite connection pool
   - Ensures all pending queries complete
   - Prevents connection leaks

6. **Log Shutdown Summary**
   - Reports total shutdown time
   - Provides visibility into shutdown performance

## Configuration

Configure shutdown timeouts via environment variables in `.env`:

```bash
# Maximum time to wait for in-flight requests (default: 30 seconds)
SHUTDOWN_GRACEFUL_TIMEOUT=30

# Maximum time to wait for background tasks (default: 10 seconds)
SHUTDOWN_BACKGROUND_TIMEOUT=10

# Maximum time to wait for database/cache close (default: 5 seconds)
SHUTDOWN_DB_TIMEOUT=5
```

## Architecture

### Core Components

#### `shutdown.rs` Module
- `ShutdownConfig`: Configuration for shutdown timeouts
- `ShutdownCoordinator`: Manages shutdown signal broadcasting
- `wait_for_signal()`: Listens for OS signals
- `shutdown_background_tasks()`: Gracefully stops background tasks
- `shutdown_websockets()`: Closes WebSocket connections
- `flush_cache()`: Flushes Redis cache and closes connections
- `shutdown_database()`: Closes database pool
- `log_shutdown_summary()`: Logs shutdown metrics

#### Background Task Management
All background tasks use `tokio::select!` to listen for shutdown signals:

```rust
let mut shutdown_rx = shutdown_coordinator.subscribe();
loop {
    tokio::select! {
        // Normal task operation
        _ = interval.tick() => {
            // Do work
        }
        // Shutdown signal received
        _ = shutdown_rx.recv() => {
            tracing::info!("Task shutting down");
            break;
        }
    }
}
```

#### WebSocket Shutdown
- New `ServerShutdown` message variant notifies clients
- `close_all_connections()` method cleans up all connections
- Graceful notification period before forced closure

#### Cache Shutdown
- New `close()` method on `CacheManager`
- Verifies Redis connection with PING before close
- Logs cache statistics for monitoring

## Testing

### Manual Testing

1. **Start the server:**
   ```bash
   cd backend
   cargo run
   ```

2. **Send shutdown signal:**
   - Press `Ctrl+C` in the terminal
   - Or send SIGTERM: `kill -TERM <pid>`

3. **Observe logs:**
   ```
   [INFO] Shutdown signal received, initiating graceful shutdown
   [INFO] Server received shutdown signal, stopping to accept new connections
   [INFO] Server stopped accepting new connections, starting cleanup
   [INFO] Step 1/4: Shutting down background tasks
   [INFO] Metrics synchronization task shutting down
   [INFO] Ledger ingestion task shutting down
   [INFO] Liquidity pool sync task shutting down
   [INFO] Trustline stats sync task shutting down
   [INFO] RealtimeBroadcaster task shutting down
   [INFO] Webhook dispatcher task shutting down
   [INFO] All background tasks completed within timeout
   [INFO] Step 2/4: Closing WebSocket connections
   [INFO] Closing 0 WebSocket connections
   [INFO] All WebSocket connections closed
   [INFO] Step 3/4: Flushing cache and closing Redis connections
   [INFO] Cache statistics - Hits: 42, Misses: 8, Invalidations: 3, Hit Rate: 84.00%
   [INFO] Redis connection closed
   [INFO] Step 4/4: Closing database connections
   [INFO] Database connections closed successfully
   [INFO] Graceful shutdown completed in 2.34s
   [INFO] Graceful shutdown complete
   ```

### Load Testing During Shutdown

1. **Start load test:**
   ```bash
   cd backend/load-tests
   k6 run anchors-load-test.js
   ```

2. **During load test, send shutdown signal**

3. **Verify:**
   - In-flight requests complete successfully
   - No connection errors during graceful period
   - Clean shutdown after timeout

### Docker Testing

```bash
# Build image
docker build -t stellar-insights-backend .

# Run container
docker run -d --name backend stellar-insights-backend

# Send SIGTERM (Docker stop)
docker stop backend

# Check logs
docker logs backend
```

### Kubernetes Testing

```yaml
apiVersion: v1
kind: Pod
spec:
  terminationGracePeriodSeconds: 60  # Should be > SHUTDOWN_GRACEFUL_TIMEOUT
  containers:
  - name: backend
    image: stellar-insights-backend
    env:
    - name: SHUTDOWN_GRACEFUL_TIMEOUT
      value: "30"
```

## Production Deployment

### Recommended Settings

```bash
# Production .env settings
SHUTDOWN_GRACEFUL_TIMEOUT=30
SHUTDOWN_BACKGROUND_TIMEOUT=15
SHUTDOWN_DB_TIMEOUT=10
```

### Container Orchestration

#### Docker Compose
```yaml
services:
  backend:
    image: stellar-insights-backend
    stop_grace_period: 45s  # Should be > SHUTDOWN_GRACEFUL_TIMEOUT
```

#### Kubernetes
```yaml
spec:
  terminationGracePeriodSeconds: 60
  containers:
  - name: backend
    lifecycle:
      preStop:
        exec:
          command: ["/bin/sh", "-c", "sleep 5"]  # Optional delay
```

### Monitoring

Monitor shutdown metrics:
- Shutdown duration (should be < graceful timeout)
- Background task completion rate
- Connection drain success rate
- Cache flush success rate

### Health Checks

Ensure health check endpoints respect shutdown state:
- Return 503 Service Unavailable during shutdown
- Load balancers should stop routing traffic

## Troubleshooting

### Shutdown Takes Too Long

**Symptom:** Shutdown exceeds configured timeout

**Solutions:**
1. Increase `SHUTDOWN_GRACEFUL_TIMEOUT`
2. Check for long-running database queries
3. Review background task logic for blocking operations
4. Monitor Redis connection latency

### Background Tasks Not Stopping

**Symptom:** Tasks continue running after shutdown signal

**Solutions:**
1. Verify tasks use `tokio::select!` with shutdown receiver
2. Check for infinite loops without shutdown checks
3. Ensure tasks don't ignore shutdown signals
4. Review task logs for errors

### Database Connection Errors

**Symptom:** Connection errors during shutdown

**Solutions:**
1. Increase `SHUTDOWN_DB_TIMEOUT`
2. Ensure no new queries start after shutdown signal
3. Check for connection pool exhaustion
4. Review database connection settings

### WebSocket Clients Not Notified

**Symptom:** Clients don't receive shutdown notification

**Solutions:**
1. Verify `ServerShutdown` message is sent
2. Check WebSocket connection state
3. Increase notification delay (currently 500ms)
4. Review client-side WebSocket handling

## Best Practices

1. **Set Appropriate Timeouts**
   - Graceful timeout should accommodate longest expected request
   - Background timeout should allow tasks to complete current iteration
   - DB timeout should be sufficient for connection cleanup

2. **Monitor Shutdown Performance**
   - Track shutdown duration in production
   - Alert on timeouts or failures
   - Review logs for patterns

3. **Test Regularly**
   - Include shutdown testing in CI/CD
   - Test under load conditions
   - Verify behavior in orchestrated environments

4. **Handle Partial Failures**
   - System continues shutdown even if individual steps timeout
   - Logs warnings for timeout conditions
   - No data loss on forced shutdown

5. **Client Resilience**
   - WebSocket clients should handle `ServerShutdown` messages
   - HTTP clients should implement retry logic
   - Load balancers should respect health check status

## Future Enhancements

Potential improvements for future versions:

1. **Metrics Export**
   - Export shutdown metrics to Prometheus
   - Track shutdown success rate
   - Monitor resource cleanup

2. **Configurable Shutdown Hooks**
   - Allow custom cleanup logic
   - Plugin-based shutdown handlers
   - Pre/post shutdown callbacks

3. **Graceful Degradation**
   - Read-only mode during shutdown
   - Reject writes but allow reads
   - Gradual service degradation

4. **Health Check Integration**
   - Automatic health check status update
   - Kubernetes readiness probe integration
   - Load balancer coordination

## References

- [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown)
- [Axum Graceful Shutdown](https://docs.rs/axum/latest/axum/serve/struct.Serve.html#method.with_graceful_shutdown)
- [Kubernetes Pod Lifecycle](https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/)
- [Docker Stop Behavior](https://docs.docker.com/engine/reference/commandline/stop/)
