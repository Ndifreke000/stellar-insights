# Graceful Shutdown - Quick Reference

## TL;DR

The backend now handles shutdown signals gracefully, ensuring no data loss and clean resource cleanup.

## Quick Commands

### Start Server
```bash
cd backend
cargo run --release
```

### Trigger Shutdown
```bash
# Send SIGTERM (recommended)
kill -TERM <PID>

# Or press Ctrl+C in terminal
```

### Test Shutdown
```bash
./test_graceful_shutdown.sh
```

## Configuration

Add to `.env`:
```bash
SHUTDOWN_GRACEFUL_TIMEOUT=30      # Wait for in-flight requests (seconds)
SHUTDOWN_BACKGROUND_TIMEOUT=10    # Wait for background tasks (seconds)
SHUTDOWN_DB_TIMEOUT=5             # Wait for DB/cache close (seconds)
```

## What Happens During Shutdown?

1. **Signal Received** → Server stops accepting new connections
2. **In-Flight Requests** → Complete within timeout (default: 30s)
3. **Background Tasks** → Stop gracefully (default: 10s)
4. **WebSockets** → Clients notified and disconnected (5s)
5. **Cache** → Redis flushed and closed (default: 5s)
6. **Database** → Connections closed (default: 5s)
7. **Complete** → Process exits cleanly

## Expected Logs

```
[INFO] Shutdown signal received, initiating graceful shutdown
[INFO] Server stopped accepting new connections, starting cleanup
[INFO] Step 1/4: Shutting down background tasks
[INFO] Metrics synchronization task shutting down
[INFO] Ledger ingestion task shutting down
[INFO] All background tasks completed within timeout
[INFO] Step 2/4: Closing WebSocket connections
[INFO] All WebSocket connections closed
[INFO] Step 3/4: Flushing cache and closing Redis connections
[INFO] Cache statistics - Hits: X, Misses: Y, Hit Rate: Z%
[INFO] Redis connection closed
[INFO] Step 4/4: Closing database connections
[INFO] Database connections closed successfully
[INFO] Graceful shutdown completed in X.XXs
[INFO] Graceful shutdown complete
```

## Docker

```bash
# Build
docker build -t stellar-insights-backend .

# Run
docker run -d --name backend \
  -e SHUTDOWN_GRACEFUL_TIMEOUT=30 \
  stellar-insights-backend

# Stop (triggers graceful shutdown)
docker stop backend

# Check logs
docker logs backend | grep -i shutdown
```

## Kubernetes

```yaml
spec:
  terminationGracePeriodSeconds: 60  # Must be > SHUTDOWN_GRACEFUL_TIMEOUT
  containers:
  - name: backend
    env:
    - name: SHUTDOWN_GRACEFUL_TIMEOUT
      value: "30"
```

## Troubleshooting

### Shutdown takes too long
- Increase timeout values in `.env`
- Check for long-running queries
- Review background task logic

### Requests fail during shutdown
- Normal behavior - server is shutting down
- Ensure load balancer stops routing traffic
- Implement client retry logic

### Database errors
- Increase `SHUTDOWN_DB_TIMEOUT`
- Check connection pool settings
- Ensure no new queries after shutdown signal

## Key Files

- `src/shutdown.rs` - Shutdown logic
- `src/main.rs` - Server lifecycle
- `GRACEFUL_SHUTDOWN.md` - Full documentation
- `SHUTDOWN_TESTING.md` - Testing guide

## Production Checklist

- [ ] Set appropriate timeout values
- [ ] Configure container stop grace period
- [ ] Set up health checks
- [ ] Implement client retry logic
- [ ] Monitor shutdown metrics
- [ ] Test under load
- [ ] Document for team

## Support

For detailed information, see:
- `GRACEFUL_SHUTDOWN.md` - Complete guide
- `SHUTDOWN_TESTING.md` - Testing procedures
- `IMPLEMENTATION_SUMMARY.md` - Technical details
