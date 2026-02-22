# Quick Start Guide - Load Testing

Get started with load testing in 5 minutes!

## Prerequisites

1. **Install k6**

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

2. **Start the Backend Server**
   ```bash
   cd backend
   cargo run --release
   ```

   Wait for the server to start on `http://localhost:8080`

## Running Your First Load Test

### Option 1: Run Individual Test (Recommended for First Time)

```bash
cd backend/load-tests
k6 run corridors-load-test.js
```

This will:
- Ramp up to 100 virtual users over 2 minutes
- Maintain 100 users for 5 minutes
- Ramp down over 2 minutes
- Show real-time metrics in your terminal

### Option 2: Run All Tests

**Windows:**
```powershell
cd backend/load-tests
.\run-all-tests.ps1
```

**Linux/macOS:**
```bash
cd backend/load-tests
./run-all-tests.sh
```

This will run all tests sequentially and generate a summary report.

## Understanding the Output

### During the Test

You'll see real-time metrics like:
```
running (5m30s), 100/100 VUs, 15000 complete and 0 interrupted iterations

     ✓ status is 200
     ✓ response time < 500ms

     checks.........................: 100.00% ✓ 30000      ✗ 0
     http_req_duration..............: avg=250ms    p(95)=450ms
     http_reqs......................: 15000   45.45/s
```

### Key Metrics Explained

- **checks**: Percentage of successful validations
- **http_req_duration**: Response time statistics
  - `avg`: Average response time
  - `p(95)`: 95% of requests were faster than this
  - `p(99)`: 99% of requests were faster than this
- **http_reqs**: Total requests and requests per second
- **http_req_failed**: Percentage of failed requests

### What's Good?

✅ **Good Performance:**
- Checks: 100%
- p95 < 500ms
- http_req_failed < 1%
- No 5xx errors

⚠️ **Needs Attention:**
- Checks: 95-99%
- p95: 500-800ms
- http_req_failed: 1-5%

❌ **Critical Issues:**
- Checks: < 95%
- p95 > 800ms
- http_req_failed > 5%
- 5xx errors present

## Common Issues and Solutions

### Issue: "k6: command not found"
**Solution:** Install k6 using the instructions above

### Issue: "Server health check failed"
**Solution:** Make sure the backend server is running:
```bash
cd backend
cargo run --release
```

### Issue: High error rates
**Solution:** 
1. Check server logs for errors
2. Verify database is accessible
3. Check Redis connection
4. Reduce load (lower VU count)

### Issue: Slow response times
**Solution:**
1. Check if server is in debug mode (use `--release`)
2. Verify database indexes exist
3. Check cache hit rates
4. Monitor system resources

## Next Steps

1. **Review Results**: Check the generated JSON files in `results/` directory
2. **Establish Baseline**: Document your results in `PERFORMANCE_BASELINE.md`
3. **Run Regularly**: Set up automated tests in CI/CD
4. **Optimize**: Use results to identify and fix bottlenecks

## Test Scenarios

### Quick Test (2 minutes)
```bash
k6 run --vus 10 --duration 2m corridors-load-test.js
```

### Standard Load Test (9 minutes)
```bash
k6 run corridors-load-test.js
```

### Stress Test (26 minutes)
```bash
k6 run stress-test.js
```

### Spike Test (7 minutes)
```bash
k6 run spike-test.js
```

## Customization

### Change Base URL
```bash
export BASE_URL=http://your-server:8080
k6 run corridors-load-test.js
```

### Change Response Time Threshold
```bash
export MAX_RESPONSE_TIME=1000
k6 run corridors-load-test.js
```

### Custom VUs and Duration
```bash
k6 run --vus 50 --duration 5m corridors-load-test.js
```

## Getting Help

- **k6 Documentation**: https://k6.io/docs/
- **k6 Community**: https://community.k6.io/
- **Project README**: See `README.md` in this directory

## Tips for Success

1. **Start Small**: Begin with low VU counts (10-20) and short durations
2. **Monitor Resources**: Watch CPU, memory, and disk I/O during tests
3. **Test Incrementally**: Test one endpoint at a time before running full suite
4. **Use Realistic Data**: Ensure test patterns match production usage
5. **Document Everything**: Record baselines and any changes
6. **Test Regularly**: Run tests before each release

## Example Workflow

```bash
# 1. Start the server
cd backend
cargo run --release

# 2. In another terminal, run a quick test
cd backend/load-tests
k6 run --vus 10 --duration 1m corridors-load-test.js

# 3. If successful, run the full test
k6 run corridors-load-test.js

# 4. Review results
cat results/corridors-load-test_*.json

# 5. Document baseline
# Edit PERFORMANCE_BASELINE.md with your results
```

That's it! You're now ready to load test your Stellar Insights Backend. 🚀
