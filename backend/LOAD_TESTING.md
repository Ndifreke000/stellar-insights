# Load Testing - Stellar Insights Backend

## 🎯 Overview

Comprehensive load testing suite to verify the backend can handle expected traffic levels and identify performance bottlenecks.

## 📍 Location

All load testing files are located in: **`backend/load-tests/`**

## 🚀 Quick Start

### 1. Install k6

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

### 2. Start the Backend

```bash
cd backend
cargo run --release
```

### 3. Run Your First Test

```bash
cd backend/load-tests
k6 run corridors-load-test.js
```

## 📚 Documentation

| File | Description |
|------|-------------|
| [INDEX.md](load-tests/INDEX.md) | Navigation hub and quick reference |
| [QUICK_START.md](load-tests/QUICK_START.md) | 5-minute getting started guide |
| [README.md](load-tests/README.md) | Comprehensive documentation |
| [TROUBLESHOOTING.md](load-tests/TROUBLESHOOTING.md) | Common issues and solutions |
| [PERFORMANCE_BASELINE.md](load-tests/PERFORMANCE_BASELINE.md) | Performance targets and baselines |
| [IMPLEMENTATION_SUMMARY.md](load-tests/IMPLEMENTATION_SUMMARY.md) | Complete implementation details |

## 🧪 Available Tests

### Individual Endpoint Tests

1. **Corridors Load Test** (`corridors-load-test.js`)
   - Tests `/api/corridors` endpoint
   - 100 VUs, 9 minutes
   - Multiple query patterns

2. **Anchors Load Test** (`anchors-load-test.js`)
   - Tests `/api/anchors` endpoint
   - 100 VUs, 9 minutes
   - Pagination scenarios

3. **RPC Endpoints Test** (`rpc-endpoints-load-test.js`)
   - Tests RPC endpoints
   - 50 VUs, 9 minutes
   - Payments, trades, ledger

### Comprehensive Tests

4. **Full Suite Test** (`full-suite-load-test.js`)
   - Tests all endpoints together
   - 150 VUs peak, 16 minutes
   - Realistic user behavior

5. **Stress Test** (`stress-test.js`)
   - Finds system limits
   - Up to 400 VUs, 26 minutes
   - Identifies bottlenecks

6. **Spike Test** (`spike-test.js`)
   - Tests sudden traffic bursts
   - Instant spike to 500 VUs
   - 7 minutes

## 🎯 Performance Targets

| Metric | Target | Acceptable | Critical |
|--------|--------|------------|----------|
| p95 Response Time | < 400ms | < 500ms | > 800ms |
| p99 Response Time | < 800ms | < 1000ms | > 2000ms |
| Error Rate | < 0.1% | < 1% | > 5% |
| Throughput | > 200 req/s | > 100 req/s | < 50 req/s |

## 🔧 Common Commands

### Run Individual Test
```bash
cd backend/load-tests
k6 run corridors-load-test.js
```

### Run All Tests (Windows)
```powershell
cd backend/load-tests
.\run-all-tests.ps1
```

### Run All Tests (Linux/macOS)
```bash
cd backend/load-tests
./run-all-tests.sh
```

### Custom Parameters
```bash
# Custom VUs and duration
k6 run --vus 50 --duration 5m corridors-load-test.js

# Custom base URL
export BASE_URL=http://your-server:8080
k6 run corridors-load-test.js

# Generate JSON output
k6 run --out json=results.json corridors-load-test.js
```

## 📊 Understanding Results

### Good Performance ✅
- Checks: 100%
- p95 < 500ms
- Error rate < 1%
- No 5xx errors

### Needs Attention ⚠️
- Checks: 95-99%
- p95: 500-800ms
- Error rate: 1-5%

### Critical Issues ❌
- Checks: < 95%
- p95 > 800ms
- Error rate > 5%
- 5xx errors present

## 🔄 CI/CD Integration

Automated load tests run via GitHub Actions:
- **Manual trigger**: Run any test type on demand
- **Weekly schedule**: Every Sunday at 2 AM UTC
- **On release**: Automatically on new releases

See: `.github/workflows/load-tests.yml`

## 🛠️ Troubleshooting

### Server Not Running
```bash
# Start the server
cd backend
cargo run --release

# Verify it's running
curl http://localhost:8080/health
```

### High Error Rates
1. Check server logs
2. Verify database connection
3. Check Redis connection
4. Reduce load (lower VUs)

### Slow Response Times
1. Use release build (`--release`)
2. Check cache hit rates
3. Monitor system resources
4. Review database indexes

For more help, see [TROUBLESHOOTING.md](load-tests/TROUBLESHOOTING.md)

## 📈 Best Practices

1. **Start Small**: Begin with 10 VUs and 1 minute
2. **Monitor Resources**: Watch CPU, memory, disk I/O
3. **Test Incrementally**: One endpoint at a time
4. **Document Baselines**: Record performance metrics
5. **Test Regularly**: Before each release
6. **Use Realistic Data**: Match production patterns

## 🎓 Learning Path

### Beginner
1. Read [QUICK_START.md](load-tests/QUICK_START.md)
2. Run `corridors-load-test.js`
3. Understand the output

### Intermediate
1. Run all individual tests
2. Analyze results
3. Set up baseline

### Advanced
1. Run stress and spike tests
2. Identify bottlenecks
3. Optimize application
4. Set up CI/CD

## 📞 Support

- **Documentation**: See `load-tests/` directory
- **k6 Docs**: https://k6.io/docs/
- **k6 Community**: https://community.k6.io/

## ✅ What's Included

- ✅ 6 comprehensive test scripts
- ✅ 6 detailed documentation files
- ✅ 2 automation scripts (Bash + PowerShell)
- ✅ 1 CI/CD workflow
- ✅ Performance baselines
- ✅ Troubleshooting guide
- ✅ Quick start guide

## 🎉 Ready to Use

The load testing suite is complete and ready for immediate use. Start with the [QUICK_START.md](load-tests/QUICK_START.md) guide!

---

**For detailed information, see the [load-tests/](load-tests/) directory.**
