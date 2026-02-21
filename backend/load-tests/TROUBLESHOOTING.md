# Load Testing Troubleshooting Guide

Common issues and their solutions when running load tests.

## Installation Issues

### k6 Not Found

**Symptoms:**
```
k6: command not found
```

**Solutions:**

**Windows:**
```powershell
winget install k6 --source winget
```

**macOS:**
```bash
brew install k6
```

**Linux:**
```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Verify Installation:**
```bash
k6 version
```

## Server Issues

### Server Not Running

**Symptoms:**
```
Error: Server health check failed
Failed to connect to localhost:8080
```

**Solutions:**

1. **Start the server:**
   ```bash
   cd backend
   cargo run --release
   ```

2. **Verify server is running:**
   ```bash
   curl http://localhost:8080/health
   ```

3. **Check if port is already in use:**
   ```bash
   # Windows
   netstat -ano | findstr :8080
   
   # Linux/macOS
   lsof -i :8080
   ```

4. **Use a different port:**
   ```bash
   export SERVER_PORT=8081
   cargo run --release
   
   # Then run tests with:
   export BASE_URL=http://localhost:8081
   k6 run corridors-load-test.js
   ```

### Server Crashes During Test

**Symptoms:**
- Connection refused errors mid-test
- Server process terminates
- Out of memory errors

**Solutions:**

1. **Check server logs:**
   ```bash
   # Look for panic messages or errors
   tail -f backend/logs/server.log
   ```

2. **Increase system resources:**
   - Close other applications
   - Increase available memory
   - Check disk space

3. **Reduce test load:**
   ```bash
   # Start with fewer VUs
   k6 run --vus 10 --duration 1m corridors-load-test.js
   ```

4. **Check database connection pool:**
   ```bash
   # In .env file, increase pool size
   DB_POOL_MAX_CONNECTIONS=20
   ```

## Performance Issues

### High Error Rates

**Symptoms:**
```
http_req_failed: 15.5%
errors: 15.5%
```

**Solutions:**

1. **Check error types:**
   - Look at k6 output for specific error codes
   - 429: Rate limiting (reduce load)
   - 500: Server errors (check logs)
   - 503: Service unavailable (server overloaded)

2. **Reduce concurrent load:**
   ```bash
   k6 run --vus 25 --duration 5m corridors-load-test.js
   ```

3. **Check database:**
   ```bash
   # Verify database is accessible
   sqlite3 backend/stellar_insights.db "SELECT COUNT(*) FROM anchors;"
   ```

4. **Check Redis:**
   ```bash
   redis-cli ping
   # Should return: PONG
   ```

### Slow Response Times

**Symptoms:**
```
http_req_duration: avg=2500ms p(95)=5000ms
```

**Solutions:**

1. **Use release build:**
   ```bash
   # Debug builds are much slower
   cargo build --release
   ./target/release/stellar-insights-backend
   ```

2. **Check cache hit rates:**
   ```bash
   curl http://localhost:8080/api/cache/stats
   ```

3. **Monitor system resources:**
   ```bash
   # Linux/macOS
   top
   
   # Windows
   Task Manager or:
   Get-Process | Sort-Object CPU -Descending | Select-Object -First 10
   ```

4. **Check database indexes:**
   ```sql
   -- Verify indexes exist
   .schema anchors
   .schema corridors
   ```

5. **Reduce RPC call limits:**
   - Edit test scripts to fetch fewer records
   - Change `limit=200` to `limit=50`

### Connection Errors

**Symptoms:**
```
dial tcp: lookup localhost: no such host
connection refused
timeout waiting for response
```

**Solutions:**

1. **Check network configuration:**
   ```bash
   ping localhost
   curl http://localhost:8080/health
   ```

2. **Verify firewall settings:**
   - Allow connections on port 8080
   - Check antivirus software

3. **Check connection limits:**
   ```bash
   # Linux: Check file descriptor limits
   ulimit -n
   
   # Increase if needed
   ulimit -n 4096
   ```

4. **Increase timeouts:**
   - Edit test scripts
   - Increase `MAX_RESPONSE_TIME` environment variable

## Test Script Issues

### Module Import Errors

**Symptoms:**
```
ERRO[0000] Module resolution error
```

**Solutions:**

1. **Verify k6 version:**
   ```bash
   k6 version
   # Should be v0.40.0 or higher
   ```

2. **Check script syntax:**
   ```bash
   k6 run --no-usage-report corridors-load-test.js
   ```

### Threshold Failures

**Symptoms:**
```
✗ http_req_duration: p(95)<500
```

**Solutions:**

1. **Adjust thresholds:**
   - Edit test script
   - Increase threshold values for your environment

2. **Optimize application:**
   - Add database indexes
   - Increase cache TTL
   - Optimize slow queries

3. **Accept current performance:**
   - Document baseline in `PERFORMANCE_BASELINE.md`
   - Set realistic thresholds

## Database Issues

### Database Locked

**Symptoms:**
```
database is locked
SQLITE_BUSY
```

**Solutions:**

1. **Use PostgreSQL for load testing:**
   ```bash
   # In .env
   DATABASE_URL=postgresql://user:pass@localhost/stellar_insights
   ```

2. **Increase SQLite timeout:**
   ```rust
   // In database.rs
   .connect_timeout(Duration::from_secs(30))
   ```

3. **Reduce concurrent writes:**
   - SQLite has limited write concurrency
   - Consider read-only tests

### Connection Pool Exhausted

**Symptoms:**
```
connection pool timeout
too many connections
```

**Solutions:**

1. **Increase pool size:**
   ```bash
   # In .env
   DB_POOL_MAX_CONNECTIONS=50
   ```

2. **Reduce test load:**
   ```bash
   k6 run --vus 25 corridors-load-test.js
   ```

3. **Check for connection leaks:**
   - Review application code
   - Ensure connections are properly closed

## Redis Issues

### Redis Connection Failed

**Symptoms:**
```
Failed to connect to Redis
Connection refused on port 6379
```

**Solutions:**

1. **Start Redis:**
   ```bash
   # Linux/macOS
   redis-server
   
   # Windows (with WSL)
   sudo service redis-server start
   
   # Docker
   docker run -d -p 6379:6379 redis:7-alpine
   ```

2. **Verify Redis is running:**
   ```bash
   redis-cli ping
   ```

3. **Check Redis URL:**
   ```bash
   # In .env
   REDIS_URL=redis://127.0.0.1:6379
   ```

### Redis Memory Issues

**Symptoms:**
```
OOM command not allowed when used memory > 'maxmemory'
```

**Solutions:**

1. **Increase Redis memory:**
   ```bash
   # In redis.conf
   maxmemory 256mb
   ```

2. **Clear Redis cache:**
   ```bash
   redis-cli FLUSHALL
   ```

3. **Configure eviction policy:**
   ```bash
   # In redis.conf
   maxmemory-policy allkeys-lru
   ```

## System Resource Issues

### Out of Memory

**Symptoms:**
- Server crashes
- System becomes unresponsive
- OOM killer messages

**Solutions:**

1. **Monitor memory usage:**
   ```bash
   # Linux
   free -h
   
   # macOS
   vm_stat
   
   # Windows
   Get-Counter '\Memory\Available MBytes'
   ```

2. **Reduce test load:**
   ```bash
   k6 run --vus 25 --duration 2m corridors-load-test.js
   ```

3. **Increase system memory:**
   - Close other applications
   - Add more RAM
   - Use swap space

### High CPU Usage

**Symptoms:**
- CPU at 100%
- System lag
- Slow response times

**Solutions:**

1. **Monitor CPU:**
   ```bash
   # Linux/macOS
   top
   
   # Windows
   Get-Process | Sort-Object CPU -Descending
   ```

2. **Optimize application:**
   - Profile code for hot spots
   - Reduce computational complexity
   - Use caching

3. **Scale horizontally:**
   - Run multiple instances
   - Use load balancer

## Getting More Help

### Enable Debug Logging

```bash
# In .env
RUST_LOG=debug

# Or for specific modules
RUST_LOG=stellar_insights_backend=debug,tower_http=debug
```

### Collect Diagnostic Information

```bash
# System info
uname -a
cargo --version
k6 version

# Server logs
tail -n 100 backend/logs/server.log

# Database info
sqlite3 backend/stellar_insights.db ".tables"

# Redis info
redis-cli INFO
```

### Report Issues

When reporting issues, include:
1. k6 version
2. Rust version
3. Operating system
4. Test script used
5. Error messages
6. Server logs
7. System resource usage

### Useful Resources

- k6 Documentation: https://k6.io/docs/
- k6 Community: https://community.k6.io/
- Rust Performance Book: https://nnethercote.github.io/perf-book/
- SQLite Performance: https://www.sqlite.org/performance.html
- Redis Performance: https://redis.io/docs/management/optimization/
