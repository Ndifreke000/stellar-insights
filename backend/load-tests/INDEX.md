# Load Testing Documentation Index

Complete guide to load testing the Stellar Insights Backend.

## 📚 Documentation Files

### Getting Started
- **[QUICK_START.md](QUICK_START.md)** - Get up and running in 5 minutes
- **[README.md](README.md)** - Comprehensive overview and usage guide

### Test Scripts
- **[corridors-load-test.js](corridors-load-test.js)** - Test `/api/corridors` endpoint
- **[anchors-load-test.js](anchors-load-test.js)** - Test `/api/anchors` endpoint
- **[rpc-endpoints-load-test.js](rpc-endpoints-load-test.js)** - Test RPC endpoints
- **[full-suite-load-test.js](full-suite-load-test.js)** - Comprehensive test suite
- **[stress-test.js](stress-test.js)** - Stress test to find limits
- **[spike-test.js](spike-test.js)** - Test sudden traffic spikes

### Helper Scripts
- **[run-all-tests.sh](run-all-tests.sh)** - Bash script to run all tests (Linux/macOS)
- **[run-all-tests.ps1](run-all-tests.ps1)** - PowerShell script to run all tests (Windows)

### Reference Documentation
- **[PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md)** - Performance targets and baselines
- **[TROUBLESHOOTING.md](TROUBLESHOOTING.md)** - Common issues and solutions

## 🚀 Quick Navigation

### I want to...

#### Run my first load test
→ Start with [QUICK_START.md](QUICK_START.md)

#### Understand what each test does
→ Read [README.md](README.md) - Test Scenarios section

#### Run all tests at once
→ Use [run-all-tests.sh](run-all-tests.sh) or [run-all-tests.ps1](run-all-tests.ps1)

#### Fix an error I'm seeing
→ Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

#### Set performance targets
→ Review [PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md)

#### Customize a test
→ Edit the `.js` test files directly

#### Integrate with CI/CD
→ See [README.md](README.md) - Integration with CI/CD section

## 📊 Test Types Overview

| Test | Duration | VUs | Purpose |
|------|----------|-----|---------|
| Corridors | 9 min | 100 | Test corridor endpoint performance |
| Anchors | 9 min | 100 | Test anchor endpoint performance |
| RPC Endpoints | 9 min | 50 | Test RPC endpoint performance |
| Full Suite | 16 min | 150 | Test all endpoints together |
| Stress Test | 26 min | 400 | Find system limits |
| Spike Test | 7 min | 500 | Test sudden traffic bursts |

## 🎯 Performance Targets

| Metric | Target | Acceptable | Critical |
|--------|--------|------------|----------|
| p95 Response Time | < 400ms | < 500ms | > 800ms |
| p99 Response Time | < 800ms | < 1000ms | > 2000ms |
| Error Rate | < 0.1% | < 1% | > 5% |
| Throughput | > 200 req/s | > 100 req/s | < 50 req/s |

## 🔧 Common Commands

### Run individual test
```bash
k6 run corridors-load-test.js
```

### Run with custom parameters
```bash
k6 run --vus 50 --duration 5m corridors-load-test.js
```

### Run with custom base URL
```bash
export BASE_URL=http://your-server:8080
k6 run corridors-load-test.js
```

### Run all tests
```bash
# Linux/macOS
./run-all-tests.sh

# Windows
.\run-all-tests.ps1
```

### Generate JSON output
```bash
k6 run --out json=results.json corridors-load-test.js
```

## 📁 Directory Structure

```
load-tests/
├── INDEX.md                      # This file
├── README.md                     # Main documentation
├── QUICK_START.md               # Quick start guide
├── PERFORMANCE_BASELINE.md      # Performance baselines
├── TROUBLESHOOTING.md           # Troubleshooting guide
├── .gitignore                   # Git ignore rules
│
├── corridors-load-test.js       # Corridors endpoint test
├── anchors-load-test.js         # Anchors endpoint test
├── rpc-endpoints-load-test.js   # RPC endpoints test
├── full-suite-load-test.js      # Full test suite
├── stress-test.js               # Stress test
├── spike-test.js                # Spike test
│
├── run-all-tests.sh             # Bash runner script
├── run-all-tests.ps1            # PowerShell runner script
│
└── results/                     # Test results (generated)
    ├── *.json                   # JSON results
    ├── *.html                   # HTML reports
    └── summary_*.txt            # Summary reports
```

## 🔗 External Resources

- **k6 Documentation**: https://k6.io/docs/
- **k6 Community**: https://community.k6.io/
- **k6 Examples**: https://k6.io/docs/examples/
- **Performance Testing Guide**: https://k6.io/docs/testing-guides/

## 📝 Workflow

### First Time Setup
1. Read [QUICK_START.md](QUICK_START.md)
2. Install k6
3. Start backend server
4. Run first test: `k6 run corridors-load-test.js`

### Regular Testing
1. Run tests before releases
2. Compare against baseline
3. Document any regressions
4. Optimize if needed

### After Changes
1. Run relevant tests
2. Compare with previous results
3. Update baseline if improved
4. Document changes

## 🎓 Learning Path

### Beginner
1. Read [QUICK_START.md](QUICK_START.md)
2. Run `corridors-load-test.js`
3. Understand the output
4. Try customizing VUs and duration

### Intermediate
1. Read [README.md](README.md) fully
2. Run all individual tests
3. Analyze results
4. Set up baseline in [PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md)

### Advanced
1. Run stress and spike tests
2. Identify bottlenecks
3. Optimize application
4. Set up CI/CD integration
5. Create custom test scenarios

## 💡 Tips

- **Start small**: Begin with 10 VUs and 1 minute duration
- **Monitor resources**: Watch CPU, memory, and disk I/O
- **Test incrementally**: One endpoint at a time
- **Document everything**: Record baselines and changes
- **Test regularly**: Before each release
- **Use realistic data**: Match production patterns

## 🆘 Need Help?

1. Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
2. Review [README.md](README.md)
3. Search k6 documentation
4. Ask in k6 community forum

## 📅 Maintenance

- Update baselines after optimizations
- Review and update thresholds quarterly
- Keep test scripts in sync with API changes
- Archive old test results
- Update documentation as needed

---

**Last Updated**: 2026-02-21  
**Version**: 1.0  
**Maintainer**: Stellar Insights Team
