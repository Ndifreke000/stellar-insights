# Graceful Shutdown Verification Checklist

Use this checklist to verify the graceful shutdown implementation is working correctly.

## Pre-Deployment Checks

### Code Review
- [x] `shutdown.rs` module exists with all utilities
- [x] `shutdown` module exported in `lib.rs`
- [x] `main.rs` integrates shutdown coordinator
- [x] Signal handler spawned in background task
- [x] Axum server configured with `with_graceful_shutdown()`
- [x] Cleanup sequence implemented (cache flush, DB close)
- [x] Shutdown summary logging added
- [x] No compilation errors

### Configuration
- [x] `.env.example` updated with shutdown timeouts
- [ ] Production `.env` file includes shutdown configuration
- [ ] Timeout values appropriate for your workload

### Documentation
- [x] `docs/GRACEFUL_SHUTDOWN.md` created
- [x] Implementation summary documented
- [ ] Team briefed on shutdown behavior

## Testing Checklist

### Local Testing

#### Test 1: Basic SIGTERM
```bash
cd backend
cargo run &
PID=$!
sleep 5
kill -TERM $PID
```
- [ ] Server logs "Shutdown signal received"
- [ ] Server logs "Graceful shutdown initiated"
- [ ] Server logs "performing cleanup"
- [ ] Server logs "Graceful shutdown completed"
- [ ] Exit code is 0

#### Test 2: SIGINT (Ctrl+C)
```bash
cd backend
cargo run
# Press Ctrl+C
```
- [ ] Same log sequence as Test 1
- [ ] Clean exit

#### Test 3: With In-Flight Request
```bash
cd backend
cargo run &
PID=$!
sleep 5
curl http://localhost:8080/health &
sleep 1
kill -TERM $PID
```
- [ ] Request completes successfully
- [ ] Server waits for request before shutdown
- [ ] Clean exit

#### Test 4: Automated Test Suite
```bash
cd backend
chmod +x test_graceful_shutdown.sh
./test_graceful_shutdown.sh
```
- [ ] All tests pass
- [ ] No errors in output

### Load Testing
- [ ] Test shutdown under moderate load (10-50 req/s)
- [ ] Test shutdown under high load (100+ req/s)
- [ ] Verify no requests dropped during graceful period
- [ ] Verify timeout behavior under load

## Deployment Checks

### Docker
- [ ] Dockerfile uses appropriate base image
- [ ] `STOPSIGNAL SIGTERM` set (or default)
- [ ] Container stops cleanly with `docker stop`
- [ ] Logs show graceful shutdown sequence

### Kubernetes
- [ ] `terminationGracePeriodSeconds` configured
- [ ] Value > `SHUTDOWN_GRACEFUL_TIMEOUT` + buffer (e.g., 45s)
- [ ] Pod shutdown tested with `kubectl delete pod`
- [ ] No errors in pod logs during termination
- [ ] Readiness probe configured
- [ ] Liveness probe configured

### systemd
- [ ] Service file includes `TimeoutStopSec`
- [ ] Value > `SHUTDOWN_GRACEFUL_TIMEOUT` + buffer
- [ ] Service stops cleanly with `systemctl stop`
- [ ] Logs show graceful shutdown

### Load Balancer
- [ ] Health check endpoint configured
- [ ] Connection draining enabled
- [ ] Drain timeout > graceful timeout
- [ ] No 502/503 errors during deployment

## Monitoring Setup

### Metrics
- [ ] Shutdown duration metric tracked
- [ ] In-flight request count at shutdown tracked
- [ ] Timeout occurrences tracked
- [ ] Failed shutdowns alerted

### Logs
- [ ] Shutdown logs aggregated
- [ ] Timeout warnings monitored
- [ ] Failed cleanup operations alerted

### Alerts
- [ ] Alert on shutdown timeouts
- [ ] Alert on failed database cleanup
- [ ] Alert on abnormal shutdown duration

## Production Validation

### Rolling Deployment
- [ ] Perform rolling deployment
- [ ] Monitor error rates during deployment
- [ ] Verify no request drops
- [ ] Check shutdown logs for all pods

### Blue-Green Deployment
- [ ] Switch traffic to new version
- [ ] Gracefully shutdown old version
- [ ] Verify clean shutdown
- [ ] No errors in logs

### Canary Deployment
- [ ] Deploy canary with 5% traffic
- [ ] Monitor shutdown behavior
- [ ] Compare with stable version
- [ ] Gradually increase traffic

## Troubleshooting Scenarios

### Scenario 1: Timeouts Occurring
- [ ] Check `SHUTDOWN_GRACEFUL_TIMEOUT` value
- [ ] Review in-flight request durations
- [ ] Increase timeout if needed
- [ ] Investigate slow endpoints

### Scenario 2: Database Not Closing
- [ ] Check for long-running queries
- [ ] Review connection pool settings
- [ ] Increase `SHUTDOWN_DB_TIMEOUT`
- [ ] Check database logs

### Scenario 3: Requests Dropped
- [ ] Verify load balancer configuration
- [ ] Check health check settings
- [ ] Ensure connection draining enabled
- [ ] Review graceful timeout value

### Scenario 4: Background Tasks Not Completing
- [ ] Check background task implementation
- [ ] Increase `SHUTDOWN_BACKGROUND_TIMEOUT`
- [ ] Add shutdown signal handling to tasks
- [ ] Review task logs

## Sign-Off

### Development Team
- [ ] Code reviewed and approved
- [ ] Tests passing
- [ ] Documentation complete

### DevOps Team
- [ ] Deployment configs updated
- [ ] Monitoring configured
- [ ] Alerts set up

### QA Team
- [ ] Manual testing complete
- [ ] Load testing complete
- [ ] Edge cases tested

### Product Owner
- [ ] Feature accepted
- [ ] Ready for production

## Notes

Date: _______________
Tested by: _______________
Environment: _______________
Issues found: _______________
Resolution: _______________

---

## Quick Reference

### Environment Variables
```bash
SHUTDOWN_GRACEFUL_TIMEOUT=30      # In-flight requests
SHUTDOWN_BACKGROUND_TIMEOUT=10    # Background tasks
SHUTDOWN_DB_TIMEOUT=5             # Database cleanup
```

### Test Commands
```bash
# Start server
cargo run

# Send SIGTERM
kill -TERM <PID>

# Send SIGINT
kill -INT <PID>

# Run test suite
./test_graceful_shutdown.sh
```

### Expected Log Output
```
INFO Shutdown signal received, starting graceful shutdown
INFO Graceful shutdown initiated, waiting up to 30s for in-flight requests
INFO Server stopped accepting new connections, performing cleanup
INFO Flushing caches
INFO Cache flush completed
INFO Closing database connections
INFO Database connections closed successfully
INFO Graceful shutdown completed in X.XXs
```
