# Graceful Shutdown

The Stellar Insights backend implements graceful shutdown to ensure clean termination of the server without data loss or connection issues.

## Overview

When the server receives a shutdown signal (SIGTERM or SIGINT), it:

1. Stops accepting new connections
2. Waits for in-flight requests to complete (with timeout)
3. Flushes caches
4. Closes database connections cleanly
5. Logs shutdown summary

## Supported Signals

- **SIGTERM**: Standard termination signal (used by Docker, Kubernetes, systemd)
- **SIGINT**: Interrupt signal (Ctrl+C in terminal)

On Windows, only Ctrl+C (SIGINT) is supported.

## Configuration

Configure shutdown timeouts via environment variables:

```bash
# Maximum time to wait for in-flight requests (default: 30 seconds)
SHUTDOWN_GRACEFUL_TIMEOUT=30

# Maximum time to wait for background tasks (default: 10 seconds)
SHUTDOWN_BACKGROUND_TIMEOUT=10

# Maximum time to wait for database connections to close (default: 5 seconds)
SHUTDOWN_DB_TIMEOUT=5
```

Add these to your `.env` file or set them in your deployment environment.

## Shutdown Process

### 1. Signal Detection
The server listens for SIGTERM and SIGINT signals in a dedicated task.

### 2. Stop Accepting Connections
When a signal is received, the server immediately stops accepting new connections.

### 3. Wait for In-Flight Requests
The server waits up to `SHUTDOWN_GRACEFUL_TIMEOUT` seconds for active requests to complete.

### 4. Cache Flush
Any pending cache operations are flushed to ensure data consistency.

### 5. Database Cleanup
Database connections are closed cleanly within `SHUTDOWN_DB_TIMEOUT` seconds.

### 6. Shutdown Complete
The server logs a summary and exits with code 0.

## Testing

Use the provided test script to verify graceful shutdown behavior:

```bash
cd backend
chmod +x test_graceful_shutdown.sh
./test_graceful_shutdown.sh
```

The script tests:
- Basic SIGTERM shutdown
- SIGINT (Ctrl+C) shutdown
- Shutdown with in-flight requests
- Timeout behavior

### Manual Testing

1. Start the server:
   ```bash
   cargo run
   ```

2. In another terminal, send a request:
   ```bash
   curl http://localhost:8080/health
   ```

3. Send shutdown signal:
   ```bash
   # Find the process ID
   ps aux | grep stellar-insights-backend
   
   # Send SIGTERM
   kill -TERM <PID>
   
   # Or just press Ctrl+C in the server terminal
   ```

4. Observe the logs:
   ```
   INFO Shutdown signal received, starting graceful shutdown
   INFO Graceful shutdown initiated, waiting up to 30s for in-flight requests
   INFO Server stopped accepting new connections, performing cleanup
   INFO Flushing caches
   INFO Cache flush completed
   INFO Closing database connections
   INFO Database connections closed successfully
   INFO Graceful shutdown completed in 2.34s
   ```

## Deployment Considerations

### Docker

Docker sends SIGTERM by default when stopping containers. Ensure your Dockerfile uses `SIGTERM`:

```dockerfile
# The default signal is SIGTERM, which is handled
STOPSIGNAL SIGTERM
```

### Kubernetes

Kubernetes sends SIGTERM and waits for `terminationGracePeriodSeconds` (default: 30s) before sending SIGKILL.

Configure your pod to allow enough time:

```yaml
spec:
  terminationGracePeriodSeconds: 45  # Should be > SHUTDOWN_GRACEFUL_TIMEOUT
  containers:
  - name: backend
    env:
    - name: SHUTDOWN_GRACEFUL_TIMEOUT
      value: "30"
```

### systemd

systemd sends SIGTERM by default. Configure timeout in your service file:

```ini
[Service]
TimeoutStopSec=45
Environment="SHUTDOWN_GRACEFUL_TIMEOUT=30"
```

## Troubleshooting

### Server doesn't shut down gracefully

Check logs for timeout warnings:
```
WARN Background tasks did not complete within 10s, proceeding with shutdown
```

Increase the relevant timeout:
```bash
export SHUTDOWN_BACKGROUND_TIMEOUT=20
```

### Requests are dropped during shutdown

Ensure your load balancer or reverse proxy:
1. Stops sending new requests when the server returns connection errors
2. Has health checks configured to detect shutdown
3. Allows enough time for graceful shutdown

### Database connections not closing

Check for:
- Long-running queries
- Connection pool issues
- Insufficient `SHUTDOWN_DB_TIMEOUT`

Increase the database timeout:
```bash
export SHUTDOWN_DB_TIMEOUT=10
```

## Implementation Details

The graceful shutdown implementation is in `backend/src/shutdown.rs` and integrated in `backend/src/main.rs`.

Key components:
- `ShutdownCoordinator`: Manages shutdown state and notifications
- `wait_for_signal()`: Listens for OS signals
- `shutdown_database()`: Closes database connections
- `flush_caches()`: Flushes pending cache operations

## Best Practices

1. **Set appropriate timeouts**: Balance between clean shutdown and deployment speed
2. **Monitor shutdown logs**: Watch for timeout warnings in production
3. **Test in staging**: Verify shutdown behavior under load
4. **Configure load balancers**: Ensure they respect connection draining
5. **Use health checks**: Help orchestrators detect shutdown state

## Related Documentation

- [Database Configuration](./DATABASE.md)
- [Deployment Guide](./DEPLOYMENT.md)
- [Monitoring](./MONITORING.md)
