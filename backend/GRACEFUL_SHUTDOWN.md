# Graceful Shutdown Implementation

## Overview

The backend now implements comprehensive graceful shutdown handling to ensure clean termination when receiving shutdown signals (SIGTERM, SIGINT/Ctrl+C). This prevents data loss, ensures in-flight requests complete, and properly releases resources.

## Features

### Signal Handling
- **SIGTERM**: Graceful termination signal (used by orchestrators like Kubernetes, Docker)
- **SIGINT**: Interrupt signal (Ctrl+C in terminal)
- **Cross-platform**: Works on Unix-like systems and Windows

### Shutdown Sequence

When a shutdown signal is received, the following sequence executes:

1. **Stop Accepting New Connections** (30s timeout by default)
   - Server stops accepting new requests
   - Existing connections are allowed to complete
   - In-flight requests finish processing

2. **Shutdown Background Tasks** (10s timeout by default)
   - Metrics synchronization task receives shutdown signal
   - Task completes current operation and exits cleanly
   - Other background tasks can be added to the shutdown list

3. **Flush Caches** (immediate)
   - Any pending cache writes are flushed
   - Placeholder for future caching implementations

4. **Close Database Connections** (5s timeout by default)
   - SQLite connection pool is closed gracefully
   - All active connections are terminated properly
   - Prevents database corruption

## Configuration

Shutdown behavior can be configured via environment variables:

```bash
# Maximum time to wait for in-flight requests (seconds)
SHUTDOWN_GRACEFUL_TIMEOUT=30

# Maximum time to wait for background tasks (seconds)
SHUTDOWN_BACKGROUND_TIMEOUT=10

# Maximum time to wait for database closure (seconds)
SHUTDOWN_DB_TIMEOUT=5
```

### Default Values

If not specified, the following defaults are used:
- Graceful timeout: 30 seconds
- Background task timeout: 10 seconds
- Database close timeout: 5 seconds

## Usage

### Development

```bash
# Start the server
cargo run

# Trigger graceful shutdown with Ctrl+C
# Watch the logs for shutdown sequence
```

### Production

```bash
# Start the server
./backend

# Send SIGTERM for graceful shutdown
kill -TERM <pid>

# Or use systemd
systemctl stop stellar-insights-backend
```

### Docker

```bash
# Docker automatically sends SIGTERM on stop
docker stop stellar-insights-backend

# With custom timeout (default is 10s)
docker stop -t 45 stellar-insights-backend
```

### Kubernetes

```yaml
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: backend
    image: stellar-insights-backend:latest
    # Kubernetes sends SIGTERM, then SIGKILL after terminationGracePeriodSeconds
  terminationGracePeriodSeconds: 45  # Should be > SHUTDOWN_GRACEFUL_TIMEOUT
```

## Logging

The shutdown process is fully logged for observability:

```
INFO  Shutdown signal received, starting graceful shutdown
INFO  Initiating graceful shutdown sequence...
INFO  Step 1/4: Waiting for server to finish in-flight requests...
INFO  Server received shutdown signal, stopping accepting new connections
INFO  Server shutdown completed successfully
INFO  Step 2/4: Shutting down background tasks...
INFO  Background sync task received shutdown signal
INFO  Background sync task stopped
INFO  Shutting down 1 background tasks
INFO  Background task 0 completed successfully
INFO  All background tasks completed within timeout
INFO  Step 3/4: Flushing caches...
INFO  Flushing caches
INFO  Cache flush completed
INFO  Step 4/4: Closing database connections...
INFO  Closing database connections
INFO  Database connections closed successfully
INFO  Database closed within timeout
INFO  Graceful shutdown completed in 2.34s
INFO  Graceful shutdown complete. Goodbye!
```

## Testing

### Manual Testing

1. **Test with Ctrl+C**:
   ```bash
   cargo run
   # Press Ctrl+C
   # Verify logs show graceful shutdown sequence
   ```

2. **Test with SIGTERM** (Unix):
   ```bash
   cargo run &
   PID=$!
   sleep 5
   kill -TERM $PID
   # Verify logs show graceful shutdown
   ```

3. **Test with in-flight requests**:
   ```bash
   # Terminal 1: Start server
   cargo run
   
   # Terminal 2: Send long-running request
   curl http://localhost:8080/api/anchors &
   
   # Terminal 1: Press Ctrl+C immediately
   # Verify request completes before shutdown
   ```

### Automated Testing

The shutdown module includes unit tests:

```bash
cd backend
cargo test shutdown
```

Tests cover:
- Shutdown coordinator creation and configuration
- Broadcast signal distribution
- Background task shutdown with timeout
- Configuration from environment variables

## Architecture

### Components

1. **ShutdownCoordinator**
   - Manages shutdown state
   - Broadcasts shutdown signals to all components
   - Provides timeout configuration

2. **Signal Handler**
   - Listens for OS signals (SIGTERM, SIGINT)
   - Triggers shutdown coordinator
   - Cross-platform implementation

3. **Graceful Server Shutdown**
   - Integrated with Axum's `with_graceful_shutdown`
   - Stops accepting new connections
   - Waits for in-flight requests

4. **Background Task Management**
   - Tasks listen for shutdown signals via broadcast channel
   - Complete current work and exit cleanly
   - Timeout protection prevents hanging

5. **Resource Cleanup**
   - Database connection pool closure
   - Cache flushing
   - Extensible for additional resources

### Flow Diagram

```
Signal (SIGTERM/SIGINT)
    ↓
wait_for_signal()
    ↓
ShutdownCoordinator.trigger_shutdown()
    ↓
Broadcast to all subscribers
    ↓
┌─────────────────────────────────────┐
│ 1. Server stops accepting requests  │
│    - In-flight requests complete    │
│    - Timeout: 30s                   │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│ 2. Background tasks shutdown        │
│    - Sync task exits cleanly        │
│    - Timeout: 10s                   │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│ 3. Flush caches                     │
│    - Write pending data             │
└─────────────────────────────────────┘
    ↓
┌─────────────────────────────────────┐
│ 4. Close database connections       │
│    - Pool.close()                   │
│    - Timeout: 5s                    │
└─────────────────────────────────────┘
    ↓
Log summary and exit
```

## Best Practices

### For Operators

1. **Set appropriate timeouts**: Ensure `terminationGracePeriodSeconds` (K8s) or stop timeout (Docker) is greater than `SHUTDOWN_GRACEFUL_TIMEOUT`

2. **Monitor shutdown logs**: Watch for timeout warnings which may indicate:
   - Slow database queries
   - Stuck background tasks
   - Long-running requests

3. **Load balancer configuration**: Configure health checks to fail immediately on shutdown signal

### For Developers

1. **Add new background tasks**: Register them with the shutdown coordinator
   ```rust
   let mut shutdown_rx = shutdown_coordinator.subscribe();
   let task = tokio::spawn(async move {
       loop {
           tokio::select! {
               _ = do_work() => {},
               _ = shutdown_rx.recv() => {
                   tracing::info!("Task shutting down");
                   break;
               }
           }
       }
   });
   ```

2. **Add new resources**: Include cleanup in shutdown sequence
   ```rust
   // After step 4
   tracing::info!("Step 5/5: Closing Redis connections...");
   redis_client.close().await;
   ```

3. **Test shutdown behavior**: Always test new features with graceful shutdown

## Troubleshooting

### Server doesn't shutdown

**Symptom**: Server hangs after receiving signal

**Possible causes**:
- Background task not listening for shutdown signal
- Database query hanging
- Deadlock in application code

**Solution**:
- Check logs for which step is hanging
- Reduce timeouts temporarily to identify issue
- Add more detailed logging

### Requests are dropped

**Symptom**: 502/503 errors during shutdown

**Possible causes**:
- Load balancer not respecting shutdown
- Timeout too short for request completion

**Solution**:
- Increase `SHUTDOWN_GRACEFUL_TIMEOUT`
- Configure load balancer health checks properly
- Implement connection draining

### Database corruption

**Symptom**: Database errors after restart

**Possible causes**:
- Forced termination (SIGKILL)
- Database timeout too short

**Solution**:
- Never use SIGKILL unless absolutely necessary
- Increase `SHUTDOWN_DB_TIMEOUT`
- Ensure SQLite WAL mode is enabled

## Future Enhancements

- [ ] Metrics for shutdown duration
- [ ] Graceful shutdown for WebSocket connections
- [ ] Connection draining with exponential backoff
- [ ] Shutdown hooks for plugins/extensions
- [ ] Distributed shutdown coordination (for multi-instance deployments)
- [ ] Shutdown health endpoint for orchestrators

## References

- [Tokio Signal Handling](https://docs.rs/tokio/latest/tokio/signal/)
- [Axum Graceful Shutdown](https://docs.rs/axum/latest/axum/serve/struct.Serve.html#method.with_graceful_shutdown)
- [Kubernetes Pod Lifecycle](https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/)
- [Docker Stop Behavior](https://docs.docker.com/engine/reference/commandline/stop/)