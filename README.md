# Stellar Insights

**Real-time payment analytics and reliability metrics for the Stellar network.**

[![React](https://img.shields.io/badge/React-19-blue)](https://react.dev) [![Rust](https://img.shields.io/badge/Rust-1.70+-orange)](https://rust-lang.org) [![Stellar](https://img.shields.io/badge/Stellar-Network-brightgreen)](https://stellar.org)

![Backend CI](https://github.com/Ndifreke000/stellar-insights/workflows/Backend%20CI/badge.svg)
![Frontend CI](https://github.com/Ndifreke000/stellar-insights/workflows/Frontend%20CI/badge.svg)
![Contracts CI](https://github.com/Ndifreke000/stellar-insights/workflows/Smart%20Contracts%20CI/badge.svg)
![Full Stack CI](https://github.com/Ndifreke000/stellar-insights/workflows/Full%20Stack%20CI/badge.svg)

---

## 📋 Overview

### The Problem

Cross-border payments on Stellar are fast and cheap, but **reliability varies dramatically** depending on:
- Which assets you're exchanging
- Which anchors are involved
- What time of day you're sending
- Market liquidity conditions

Today, payment providers have **no reliable way to predict success rates** for specific corridors before sending payments. This leads to failed transactions, poor user experience, and wasted resources.

### The Solution

**Stellar Insights** solves this by:
1. **Analyzing real payment flows** across the Stellar network in real-time
2. **Scoring reliability** for every payment corridor (e.g., USD→EUR via specific anchors)
3. **Tracking liquidity** to identify bottlenecks and opportunities
4. **Verifying data on-chain** using Soroban smart contracts for trustless integrity
5. **Providing APIs** that wallets, apps, and anchors can use to make smarter routing decisions

### What You Can Do With It

- **If you're a wallet/app:** Predict payment success before sending, suggest optimal routes, display corridor health to users
- **If you're an anchor:** Monitor your own asset performance, identify liquidity gaps, track reliability vs competitors
- **If you're a developer:** Access rich payment analytics via REST API, build prediction models, create dashboards

---

## 🎯 Key Features

- 📊 **Payment Success Rates** - Real-time success/failure tracking for every payment corridor
- 💧 **Liquidity Analysis** - Depth of available capital in order books, updated continuously
- ⚓ **Anchor Scoring** - Reliability ratings for asset issuers based on settlement times and success rates
- 🛣️ **Corridor Health** - Composite metrics that combine success rate, liquidity, and settlement time
- ⚡ **Settlement Tracking** - Median and P95 payment confirmation times
- 💰 **Price Integration** - Real-time USD pricing via CoinGecko for all major Stellar assets
- 🔗 **On-Chain Verification** - Snapshots anchored to Stellar blockchain via Soroban contracts
- 📱 **Live Dashboard** - Interactive UI showing real-time metrics and trends
- 🚀 **REST API** - Comprehensive endpoints for programmatic access to all analytics data

---

## 🚀 Getting Started

### Prerequisites

Before you begin, ensure you have installed:

- **Node.js 18+** - For the frontend dashboard
- **Rust 1.70+** - For the analytics backend ([Install Rust](https://rustup.rs))
- **PostgreSQL 14+** - For storing analytics data and metrics
- **Soroban CLI** (optional) - Only needed if deploying smart contracts ([Guide](https://developers.stellar.org/docs/learn/fundamentals/soroban))
- **Docker & Docker Compose** (optional) - For containerized database

### Step 1: Set Up the Database

The easiest way to get a PostgreSQL database running is with Docker:

```bash
docker run --name stellar-postgres \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=stellar_insights \
  -p 5432:5432 \
  -d postgres:14
```

Or if you prefer to use an existing PostgreSQL instance, just create a database:

```bash
psql -U postgres -c "CREATE DATABASE stellar_insights;"
```

### Step 2: Run the Backend

The backend ingests data from the Stellar network and computes analytics metrics.

```bash
cd backend

# Copy the example environment file
cp .env.example .env

# Edit .env to configure:
# - DATABASE_URL: PostgreSQL connection string
# - STELLAR_RPC_URL: Stellar RPC endpoint (e.g., Horizon)
# - COINGECKO_API_KEY: For real-time pricing (optional, free tier works)
nano .env  # or use your preferred editor
```

Key environment variables to set:

```bash
# Database connection
DATABASE_URL=postgres://postgres:password@localhost:5432/stellar_insights

# Stellar Network
STELLAR_RPC_URL=https://horizon.stellar.org  # Testnet: https://horizon-testnet.stellar.org

# Pricing (optional, CoinGecko free tier is sufficient)
PRICE_FEED_PROVIDER=coingecko
PRICE_FEED_CACHE_TTL_SECONDS=900  # Cache prices for 15 minutes

# Server configuration
RUST_LOG=info
SERVER_PORT=8080
```

Then start the backend:

```bash
cargo run
```

The API will be available at `http://localhost:8080`

**Expected output:**
```
[INFO] Starting Stellar Insights backend...
[INFO] Connected to database
[INFO] Listening on 0.0.0.0:8080
```

### Step 3: Run the Frontend Dashboard

The frontend is a Next.js React application that visualizes all the analytics data.

```bash
cd frontend

# Install dependencies
npm install

# Start the development server
npm run dev
```

The dashboard will be available at `http://localhost:3000`

Open it in your browser and you should see:
- Real-time corridors with success rates
- Anchor reliability scores
- Liquidity depth charts
- Payment settlement time distributions

### Step 4 (Optional): Deploy Smart Contracts

If you want to anchor analytics snapshots on-chain for verification:

```bash
cd contracts

# Build the contracts
cargo build --target wasm32-unknown-unknown --release

# Deploy to Testnet (requires funded account)
# See contracts/README.md for detailed instructions
```

See [contracts/README.md] for complete Soroban deployment guide.

---

## 📁 Project Structure

```
stellar-insights/
├── frontend/          # Next.js dashboard
├── backend/           # Rust analytics engine
├── contracts/         # Soroban smart contracts
└── docs/             # Documentation
```

---

## 🔌 API Usage & Examples

### Key Endpoints

**Get All Payment Corridors:**
```bash
curl http://localhost:8080/api/corridors
```

Response shows all detected corridors with real-time metrics:
```json
{
  "data": [
    {
      "key": "USD.GBUQWP3... → EUR.GAEEB2...",
      "source_asset": "USD",
      "destination_asset": "EUR",
      "success_rate": 0.98,
      "liquidity_depth_usd": 5000000,
      "settlement_time_median_ms": 8500,
      "health_score": 94,
      "updated_at": "2026-02-25T10:30:00Z"
    }
  ]
}
```

**Get Specific Corridor Details:**
```bash
curl "http://localhost:8080/api/corridors/USD.GBUQWP3...%20→%20EUR.GAEEB2..."
```

Shows historical trends and detailed statistics for one corridor.

**Get Anchor Scores:**
```bash
curl http://localhost:8080/api/anchors
```

Lists all anchors with their reliability scores and metrics.

**Get Price Data:**
```bash
# Single asset price
curl "http://localhost:8080/api/prices?asset=XLM:native"

# Convert amount to USD
curl "http://localhost:8080/api/prices/convert?asset=USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN&amount=1000"

# Batch price request
curl "http://localhost:8080/api/prices/batch?assets=XLM:native,USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
```

**Get Cost Calculator Estimate:**
```bash
curl -X POST http://localhost:8080/api/cost-calculator/estimate \
  -H "Content-Type: application/json" \
  -d '{
    "source_asset": "USD.GBUQWP3BOUZX34ULNQG23RQ6F4YUSXHTGYZOMBGSMBUT743KYTLCL7V4",
    "destination_asset": "EUR.GAEEB2B3YGNEQ5OLFUJI73AIQGZLYFVZB5XXNKS2EAAWZABPQ3BFVGYT",
    "amount": 1000
  }'
```

Returns estimated costs and multiple payment routes ranked by cost.

See [docs/RPC.md] for complete API documentation.

---

## 💰 Price Feed & Currency Conversion

The system integrates with **CoinGecko** to provide real-time USD pricing for all major Stellar assets. This enables accurate volume calculations and liquidity analysis.

### Configuration

In your `.env` file:

```bash
PRICE_FEED_PROVIDER=coingecko
PRICE_FEED_API_KEY=              # Leave blank for free tier (10-50 calls/min)
PRICE_FEED_CACHE_TTL_SECONDS=900 # Cache for 15 minutes
PRICE_FEED_REQUEST_TIMEOUT_SECONDS=10
```

### Supported Assets

- **Native:** XLM (Stellar native)
- **Stablecoins:** USDC, USDT, EURC, EOSC
- **Wrapped Assets:** BTC, ETH, AQUA
- **Ecosystem Tokens:** yXLM, SRT, and others

### How Price Caching Works

- Fresh prices are cached for 15 minutes
- If CoinGecko is unreachable, stale cache is used (fallback)
- Minimizes API calls while keeping data fresh
- Handles rate limits gracefully

---

## 📈 Understanding the Metrics

### Success Rate

**What it measures:** Percentage of payments that complete successfully in a specific corridor

**How it's calculated:** `(successful_payments / total_payment_attempts) * 100` over last 24 hours

**Why it matters:** High success rate (>95%) means the route is reliable for production use. Low rate (<80%) suggests you might want to use an alternate corridor.

**Example:**
```
Corridor: USD → EURC via Stellar Anchor
Success Rate: 97.5%
Failed: 3 out of 120 payments in last 24h
Recommendation: Safe to use, but have fallback routes ready
```

### Liquidity Depth

**What it measures:** Total value of assets available in order books, measured in USD

**Why it matters:** 
- High liquidity = immediate settlement, predictable prices
- Low liquidity = longer wait times, potential price slippage
- Zero liquidity = route may not exist

**Example:**
```
USD → EUR Liquidity Depth:
└─ $5M available at worst spread of 0.5%
└─ $12M available at worst spread of 1.0%
└─ $25M available at worst spread of 2.0%
```

If you want to send $100K, the depth shows what price impact you'll face.

### Settlement Time

**What it measures:** How long from payment submission to ledger confirmation

**Metrics provided:**
- **Median:** 50th percentile (typical case)
- **P95:** 95th percentile (slow case)

**Example:**
```
Corridor: XLM → USDC
├─ Median: 8.5 seconds
├─ P95: 45 seconds
└─ Implication: 95% of payments settle in <45s, typical is ~8s
```

### Corridor Health Score

**What it measures:** Composite score combining all metrics

**Calculation:** Weighted average of:
- SUCCESS_RATE (40%)
- LIQUIDITY_ADEQUACY (35%)
- SETTLEMENT_TIME (25%)

**Scoring:**
- 90-100: Excellent (production-ready)
- 75-89: Good (suitable for most use cases)
- 50-74: Fair (use with caution, have fallbacks)
- <50: Poor (consider alternative corridors)

---

## 🏗️ Architecture

Stellar Insights has three core components that work together:

```
┌─────────────────────────────────────────────────────────┐
│  Frontend Dashboard (Next.js + React)                   │
│  - Real-time corridor health visualizations             │
│  - Anchor reliability scorecards                        │
│  - Liquidity and settlement time charts                 │
└────────────────┬────────────────────────────────────────┘
                 │
                 │ HTTP API calls
                 ▼
┌─────────────────────────────────────────────────────────┐
│  Backend Analytics Engine (Rust + Axum)                │
│  - Ingest payment data from Stellar RPC                 │
│  - Compute corridor success rates in real-time          │
│  - Calculate liquidity depth                             │
│  - Score anchor reliability                              │
│  - Cache prices from CoinGecko                          │
└────────────────┬────────────────────────────────────────┘
                 │
        ┌────────┴──────────┬──────────────┐
        ▼                   ▼              ▼
   PostgreSQL          Stellar RPC      CoinGecko
   (Metrics DB)      (Payment Data)    (Pricing)
        │                              
        └─────────┬──────────────────┘
                  │
        ┌─────────▼──────────┐
        │  Smart Contracts   │
        │  (Soroban WASM)    │
        │ Verify snapshots   │
        │ on-chain           │
        └────────────────────┘
```

### How It Works

1. **Data Ingestion:** Backend continuously polls Stellar RPC to get latest payments, trades, and order books
2. **Analytics Computation:** For each payment corridor (e.g., USD→EUR), the backend calculates:
   - Success rate (% of successful payments in the last 24h)
   - Liquidity depth (total XLM/assets available in order books)
   - Settlement time (median time for payment confirmation)
3. **Price Tracking:** Prices from CoinGecko are cached and used to convert all metrics to USD
4. **Storage:** All computed metrics are stored in PostgreSQL for historical analysis and trending
5. **API:** Frontend and external clients query the backend REST API for real-time data
6. **Verification:** Periodic snapshots are anchored to Stellar blockchain via Soroban contracts

### Tech Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Frontend** | Next.js 16, React 19, TypeScript, Tailwind CSS | Interactive dashboard |
| **Backend** | Rust, Axum, SQLx, Tokio | High-performance data processing |
| **Database** | PostgreSQL 14+ | Stores metrics, snapshots, history |
| **Blockchain** | Stellar, Soroban | Immutable verification |
| **APIs** | RESTful, Stellar Horizon RPC | Data exchange |

---

## 📊 Understanding Key Concepts

### Payment Corridor

A **corridor** is a specific payment route: `{source_asset} → {destination_asset} via {anchor(s)}`.

Examples:
- `USD.GBUQWP3BOUZX34ULNQG23RQ6F4YUSXHTGYZOMBGSMBUT743KYTLCL7V4 → EUR.GAEEB2B3YGNEQ5OLFUJI73AIQGZLYFVZB5XXNKS2EAAWZABPQ3BFVGYT`
- `XLM:native → USDC via Lobstr`

### Liquidity Depth

**Liquidity depth** measures how much of an asset is available in order books at different price points. Higher liquidity = more predictable settlement times and better prices.

### Settlement Time

How long it takes from when you submit a payment to when it's confirmed on the Stellar ledger. Measured as median and P95 (95th percentile).

### Anchor Reliability Score

A composite score (0-100) for each asset issuer based on:
- SUCCESS_RATE: % of payments that settle successfully
- SETTLEMENT_TIME: Median confirmation time (lower is better)
- UPTIME: Historical availability (do they maintain their account/service)
- LIQUIDITY: Available capital in their order books

Example:
```
Lobstr.com score: 94/100
├─ Success Rate: 98%
├─ Settlement Time: 15 seconds (median)
├─ Uptime: 99.9%
└─ Liquidity: $15M USD equivalent
```

---

## � Troubleshooting

### Backend won't start - "Connection refused" error

**Problem:** Backend can't connect to PostgreSQL

**Solution:**
```bash
# Check if PostgreSQL is running
docker ps | grep postgres

# If not running, start it:
docker run --name stellar-postgres \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=stellar_insights \
  -p 5432:5432 -d postgres:14

# Verify DATABASE_URL in .env
cat backend/.env | grep DATABASE_URL
```

### Frontend shows "Cannot reach server"

**Problem:** Frontend can't connect to backend at `localhost:8080`

**Solution:**
```bash
# Make sure backend is running
curl http://localhost:8080/api/anchors

# If that works, check frontend .env configuration
cat frontend/.env.local | grep NEXT_PUBLIC_API

# Restart frontend in development:
cd frontend && npm run dev
```

### Backend is running but no data appears

**Problem:** API returns empty responses for "/api/corridors"

**Possible causes:**
1. Stellar RPC data hasn't been synced yet (takes 1-2 minutes)
2. STELLAR_RPC_URL is incorrect
3. Network connectivity issue

**Solution:**
```bash
# Check backend logs
tail -f backend/logs/app.log

# Verify Stellar RPC is reachable
curl https://horizon.stellar.org/

# Check if any data was ingested
psql $DATABASE_URL -c "SELECT COUNT(*) FROM payments;"
```

### Prices showing as null or zero

**Problem:** Price API returns null for assets

**Solution:**
```bash
# Check CoinGecko is reachable
curl https://api.coingecko.com/api/v3/simple/price?ids=stellar

# Verify PRICE_FEED_PROVIDER in backend/.env
cat backend/.env | grep PRICE_FEED

# If using API key, ensure it's valid:
curl "https://api.coingecko.com/api/v3/simple/price?ids=stellar&x_cg_pro_api_key=YOUR_KEY"

# Restart backend:
cd backend && cargo run
```

### High CPU usage

**Problem:** Backend using excessive CPU

**Solution:**
```bash
# Check which process is consuming CPU
top -p $(pgrep -f "cargo run")

# May indicate inefficient queries
# Check PostgreSQL connection pool settings in backend/.env:
DATABASE_POOL_MIN=5
DATABASE_POOL_MAX=20

# If pool is too large, reduce it
```

### Database growing too large

**Problem:** PostgreSQL database size exceeds available disk

**Solution:**
```bash
# Check database size
psql $DATABASE_URL -c "\l+ stellar_insights"

# Run maintenance:
psql $DATABASE_URL -c "VACUUM ANALYZE;"

# Or set up data retention policy (see PERFORMANCE_INDEXES_GUIDE.md)
# Delete old snapshots:
psql $DATABASE_URL -c "DELETE FROM analytics_snapshots WHERE created_at < NOW() - INTERVAL '30 days';"
```

---

## 📦 Deployment

### Docker Deployment

We provide Docker Compose configurations for containerized deployment:

```bash
# Development environment
docker-compose -f docker-compose.yml up

# Production with ELK stack (monitoring/logging)
docker-compose -f docker-compose.elk.prod.yml up -d
```

### Environment Setup for Production

For production deployment, see [backend/ENVIRONMENT_SETUP.md] for detailed configuration of:
- Database hardening
- Security credentials
- Rate limits
- Logging levels
- Performance tuning

### Database Pool Configuration

For high-traffic scenarios, consult [backend/DATABASE_POOL_CONFIG.md] for:
- Connection pool sizing
- Timeout configuration
- Connection reuse strategy
- Performance monitoring

---

## 🤝 Contributing

We'd love your contributions! Whether it's reporting bugs, suggesting features, or submitting code improvements, every contribution helps.

### Quick Start for Contributors

1. **Fork the repo** on GitHub
2. **Create a feature branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** and test thoroughly
4. **Submit a pull request** with a clear description

### Development Environment

```bash
# Clone and setup
git clone https://github.com/YOUR_USERNAME/stellar-insights.git
cd stellar-insights

# Install pre-commit hooks
pre-commit install

# Run all tests
./test_fixes.sh
```

### Contribution Areas

We need help with:
- 🐛 **Bug fixes** - See open [GitHub Issues](https://github.com/Ndifreke000/stellar-insights/issues)
- 📚 **Documentation** - Improve guides, add examples, fix typos
- ✨ **Feature development** - Check [Remaining Tasks](./issues/REMAINING-ISSUES-022-090.md)
- 🧪 **Testing** - Write tests, improve coverage, add edge cases
- 🎨 **Frontend improvements** - Better UX, new visualizations, accessibility
- ⚡ **Performance** - Query optimization, caching improvements

See [CONTRIBUTING.md](./CONTRIBUTING.md) for complete guidelines.

---

## 📖 Documentation

### Getting Started
- **[Environment Setup]** - Configure your dev environment (START HERE)
- **[Quick Start Guide]** - Get running in 5 minutes
- **[Environment Variables Explained]** - All `.env` options documented

### Architecture & Design
- **[Architecture Overview]** - System design and component interactions
- **[Database Pool Configuration]** - Connection pooling and performance
- **[RPC Integration]** - How we connect to Stellar RPC

### API & Integration
- **[Complete API Reference]** - All endpoints with examples
- **[RPC Data Sources]** - What data comes from where
- **[Webhook Documentation]** - Real-time data pushes
- **[Cost Calculator]** - API for payment cost estimation

### Features
- **[SEP-24 Hosted Deposits/Withdrawals]** - Anchor integration
- **[SEP-31 Cross-Border Payments]** - Payment protocol
- **[Account Merges]** - Tracking account consolidation events
- **[Price Feed Integration]** - CoinGecko and currency conversion
- **[Observability & Monitoring]** - Metrics, traces, and dashboards
- **[Alert System]** - Custom alert configuration

### Operations
- **[Rate Limiting]** - Protection against abuse
- **[SEP-10 Authentication]** - Stellar PKI integration
- **[IP Whitelisting]** - Access control
- **[Graceful Shutdown]** - Proper deployment procedures
- **[Load Testing]** - Performance benchmarks

### Troubleshooting & Support
- **[Security Fixes]** - Vulnerability patches and updates
- **[Build Verification]** - Checking compilation and tests
- **[CI/CD Checks]** - GitHub Actions configuration

See [backend/] and [docs/] directories for complete documentation.

---

## 🎓 Use Cases & Examples

### Scenario 1: Wallet App Integration

**You:** Building a mobile wallet that supports multiple payment corridors

**How you'd use Stellar Insights:**
```
1. User enters: "Send 100 USD to recipient in EUR"
2. Your app calls: GET /api/corridors → finds USD→EUR routes
3. Your app sorts by: health_score, liquidity_depth, settlement_time
4. Your app shows user: "This route is 94% reliable, settles in ~10s"
5. User approves and payment succeeds 98% of the time
```

**API calls:**
```bash
# 1. Get available corridors
curl http://stellar-insights:8080/api/corridors | \
  jq '.data[] | select(.destination_asset=="EUR")'

# 2. Get cost estimate for specific route
curl -X POST http://stellar-insights:8080/api/cost-calculator/estimate \
  -d '{"source_asset":"USD:...", "destination_asset":"EUR:...", "amount":100}'
```

### Scenario 2: Anchor Performance Monitoring

**You:** Running a Stellar anchor (liquidity provider)

**How you'd use Stellar Insights:**
```
1. Get your own scores: GET /api/anchors
2. See how users rate your reliability vs competitors
3. Monitor liquidity depth in your order books
4. Get alerted if settlement times spike
5. Track success rates and identify problem corridors
```

### Scenario 3: Building a Payment Analytics Dashboard

**You:** Want to provide customers with real-time payment corridor insights

**How you'd use Stellar Insights:**
```
1. Frontend queries: GET /api/corridors (every 10 seconds)
2. Display: Success rates, liquidity, settlement times
3. Show trends: Historical performance over 24h/7d
4. Recommend: Best corridors based on current conditions
```

### Scenario 4: Risk Management & Fraud Detection

**You:** Processing high-volume payments, want to detect unusual patterns

**Solution:**
```bash
# Get settlement time p95
curl http://stellar-insights:8080/api/corridors/{key}

# If settlement time spikes unexpectedly → possible issue
# Calculate: movement from normal p95 → % deviation
# Alert if deviation > 50% for that corridor
```

---

## 🔒 Security

Stellar Insights implements multiple security layers:

### Data Integrity
- ✅ **On-chain verification** via Soroban smart contracts
- ✅ **Immutable audit trails** for all analytics snapshots
- ✅ **Cryptographic signing** of sensitive data
- ✅ **Tamper detection** on verified snapshots

### Access Control
- ✅ **SEP-10 authentication** - Stellar PKI-based login
- ✅ **Rate limiting** - Protection against brute force and DoS
- ✅ **IP whitelisting** (optional) - Restrict to known IPs
- ✅ **API key management** - Secure credential storage

### Code & Dependencies
- ✅ **Automated security scanning** via GitHub Actions
- ✅ **Regular dependency updates** via Dependabot
- ✅ **Zero known vulnerabilities** in main dependencies
- ✅ **SBOM generation** for supply chain transparency

### Operational Security
- ✅ **Environment variable protection** - No secrets in code
- ✅ **Graceful deployment** - No data loss on updates
- ✅ **Encrypted database connections** - TLS for all DB traffic
- ✅ **Audit logging** - All admin actions logged

### Security Updates

Check [SECURITY_FIX_README.md] for:
- Vulnerability patches
- Security recommendations
- Update procedures
- Incident response

**Automatic scanning:**
```bash
# Run security audit for dependencies
npm audit  # Frontend
cargo audit  # Backend

# Auto-fix vulnerabilities
npm audit fix
cargo update
```

---

## 📊 Real-World Example

Let's trace a complete user journey:

```
┌─ User opens wallet app ─────────────────────────────────┐
│                                                          │
│  1. Enters: "Send $500 USD to friend in Germany (EUR)" │
│                                                          │
│  2. Frontend queries Stellar Insights:                  │
│     GET /api/corridors?destination=EUR                 │
│                                                          │
│  3. Stellar Insights returns best corridors:            │
│     Route 1: USD.Anchor1 → EUR.Anchor2                 │
│     ├─ Health Score: 96/100                             │
│     ├─ Success Rate: 98.5%                              │
│     ├─ Liquidity: $50M                                  │
│     ├─ Settlement Time: 8 seconds (median)              │
│     └─ Est. Cost: $2.50 (0.5%)                          │
│                                                          │
│  4. Frontend shows: "95% chance this works in ~10s"     │
│                                                          │
│  5. User confirms payment                               │
│                                                          │
│  6. Wallet signs transaction with Stellar               │
│                                                          │
│  7. Payment submitted to network                        │
│                                                          │
│  8. ~8 seconds later: Payment confirmed ✓               │
│                                                          │
│  9. Stellar Insights records metrics:                   │
│     ├─ Success: YES                                     │
│     ├─ Settlement time: 8.3 seconds                     │
│     ├─ Actual cost: $2.50                               │
│     └─ Updates corridor success rate                    │
│                                                          │
└────────────────────────────────────────────────────────┘
```

---

## 🚀 Next Steps

1. **[Get Started](./backend/ENVIRONMENT_SETUP.md)** - Follow the setup guide
2. **Explore the Dashboard** - `http://localhost:3000`
3. **Read the API Docs** - [docs/RPC.md]
4. **Join the Community** - Star, fork, discuss on GitHub

---

## 📄 License

MIT License - See [LICENSE](./LICENSE) file for complete terms.

You're free to use, modify, and distribute Stellar Insights in your own projects.

---

## 🌟 Support & Resources

### Getting Help

- **Technical Questions:** [GitHub Discussions](https://github.com/Ndifreke000/stellar-insights/discussions)
- **Bug Reports:** [GitHub Issues](https://github.com/Ndifreke000/stellar-insights/issues)
- **Feature Requests:** [GitHub Issues - Feature](https://github.com/Ndifreke000/stellar-insights/issues?q=label%3Afeature)

### Official Resources

- **[Stellar Documentation](https://developers.stellar.org)** - Stellar protocol basics
- **[Soroban Guide](https://developers.stellar.org/docs/learn/fundamentals/soroban)** - Smart contracts
- **[Horizon RPC](https://developers.stellar.org/api/introduction/grpc/)** - Stellar data source

### Community

- **[Stellar Community](https://stellar.org/community)** - Official forum and chat
- **[Stellar Developers](https://developers.stellar.org)** - Resources and tools

---

## 🎉 Acknowledgments

Stellar Insights is built with:

- **Stellar Network** - For trustless payment infrastructure
- **Soroban** - For on-chain verification
- **Rust Ecosystem** - For performance and reliability
- **React & Next.js** - For the beautiful dashboard
- **Open Source Community** - For amazing libraries and tools

---

## 📈 Roadmap

See [Remaining Tasks](./issues/REMAINING-ISSUES-022-090.md) for:
- Planned improvements
- Known issues
- Performance enhancements
- Feature development

---

**Built with ❤️ for the Stellar ecosystem** 🚀

Questions? Start with [Environment Setup](./backend/ENVIRONMENT_SETUP.md) or open an issue on GitHub!
