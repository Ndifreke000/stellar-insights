# Load Testing Quick Start Guide

## Prerequisites

1. Install k6:
```bash
# macOS
brew install k6

# Windows
choco install k6

# Linux
sudo apt-get install k6
```

2. Start the backend:
```bash
cd backend
cargo run --release
```

## Run Tests

### Individual Tests

```bash
# Most critical endpoints
k6 run load-tests/k6-corridors.js

# Anchor endpoints
k6 run load-tests/k6-anchors.js

# RPC endpoints
k6 run load-tests/k6-rpc.js

# Liquidity pools
k6 run load-tests/k6-liquidity-pools.js

# Fee bumps
k6 run load-tests/k6-fee-bumps.js

# System monitoring
k6 run load-tests/k6-cache-metrics.js

# Full comprehensive test
k6 run load-tests/k6-full-suite.js
```

### Run All Tests

**Unix/Linux/macOS:**
```bash
chmod +x load-tests/run-all-tests.sh
./load-tests/run-all-tests.sh
```

**Windows:**
```powershell
.\load-tests\run-all-tests.ps1
```

## Quick Tests

### Smoke Test (30 seconds)
```bash
k6 run --vus 10 --duration 30s load-tests/k6-corridors.js
```

### Stress Test (10 minutes)
```bash
k6 run --vus 100 --duration 10m load-tests/k6-full-suite.js
```

### Custom URL
```bash
k6 run -e BASE_URL=http://staging:8080 load-tests/k6-corridors.js
```

## Understanding Results

✅ **Good Performance:**
- p(95) < target
- Error rate < 5%
- All checks passing

⚠️ **Warning:**
- p(95) slightly above target
- Error rate 5-10%
- Some checks failing

❌ **Poor Performance:**
- p(95) >> target
- Error rate > 10%
- Many checks failing

## Common Issues

**Server not accessible:**
```bash
curl http://localhost:8080/health
```

**High error rates:**
```bash
tail -f backend.log | grep ERROR
```

**Slow responses:**
```bash
# Check database
sqlite3 backend/stellar_insights.db ".tables"

# Check Redis
redis-cli ping
```

## Next Steps

1. Run baseline tests
2. Document current performance
3. Identify bottlenecks
4. Optimize critical paths
5. Re-test to verify improvements

See full documentation:
- load-tests/README.md
- docs/PERFORMANCE_TESTING.md
