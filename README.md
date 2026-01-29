# Stellar Insights

**Real-time payment analytics and reliability metrics for the Stellar network.**

[![React](https://img.shields.io/badge/React-19-blue)](https://react.dev) [![Rust](https://img.shields.io/badge/Rust-1.70+-orange)](https://rust-lang.org) [![Stellar](https://img.shields.io/badge/Stellar-Network-brightgreen)](https://stellar.org)

![Backend CI](https://github.com/Ndifreke000/stellar-insights/workflows/Backend%20CI/badge.svg)
![Frontend CI](https://github.com/Ndifreke000/stellar-insights/workflows/Frontend%20CI/badge.svg)
![Contracts CI](https://github.com/Ndifreke000/stellar-insights/workflows/Smart%20Contracts%20CI/badge.svg)
![Full Stack CI](https://github.com/Ndifreke000/stellar-insights/workflows/Full%20Stack%20CI/badge.svg)

---

## 🎯 What It Does

Stellar Insights quantifies payment reliability and liquidity health across the Stellar network, helping wallets, apps, and anchors make payments with confidence.

**Key Features:**
- 📊 Payment success rate tracking by corridor
- 💧 Real-time liquidity depth analysis
- ⚓ Anchor reliability scoring
- 🛣️ Corridor health metrics
- ⚡ Settlement time monitoring
- 🔗 On-chain verification via Soroban smart contracts

---

## 🚀 Quick Start

### Prerequisites
- **Frontend:** Node.js 18+
- **Backend:** Rust 1.70+, PostgreSQL 14+
- **Contracts:** Soroban CLI

### 1. Start Database
```bash
docker run --name stellar-postgres \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=stellar_insights \
  -p 5432:5432 -d postgres:14
```

### 2. Run Backend
```bash
cd backend
cp .env.example .env
cargo run
```
Server starts at `http://localhost:8080`

### 3. Run Frontend
```bash
cd frontend
npm install
npm run dev
```
App available at `http://localhost:3000`

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

## 🔌 API Endpoints

**RPC Endpoints:**
- `GET /api/rpc/health` - Network health check
- `GET /api/rpc/payments` - Recent payments
- `GET /api/rpc/trades` - Recent trades
- `GET /api/rpc/orderbook` - Order book data

**Analytics Endpoints:**
- `GET /api/anchors` - List all anchors
- `GET /api/corridors` - List payment corridors
- `GET /api/corridors/:key` - Corridor details

See [RPC.md](./docs/RPC.md) for complete API documentation.

---

## 🏗️ Architecture

```
Frontend (Next.js) → Backend (Rust) → Stellar RPC
                          ↓
                    Smart Contract (Soroban)
                          ↓
                  On-Chain Verification
```

**Tech Stack:**
- **Frontend:** Next.js 16, React 19, TypeScript, Tailwind CSS
- **Backend:** Rust, Axum, SQLx, PostgreSQL
- **Contracts:** Soroban (Rust), WASM
- **Blockchain:** Stellar Network

---

## 📊 What You Get

| Metric | Description |
|--------|-------------|
| **Payment Success Rate** | % of successful payments per corridor |
| **Corridor Health Score** | Composite reliability metric (0-100) |
| **Liquidity Depth** | Available capital in order books |
| **Settlement Time** | Median payment confirmation time |
| **Anchor Reliability** | Issuer performance scoring |

---

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

**Quick Links:**
- [GitHub Issues](https://github.com/Ndifreke000/stellar-insights/issues) - 70 issues ready to work on
- [API Documentation](./docs/RPC.md) - Complete endpoint reference
- [Smart Contracts](./contracts/README.md) - Soroban contract docs

---

## 📖 Documentation

- [RPC.md](./docs/RPC.md) - API endpoints and usage
- [CONTRIBUTING.md](./CONTRIBUTING.md) - Development guidelines
- [GITHUB_ISSUES.md](./GITHUB_ISSUES.md) - 70 development tasks
- [backend/README.md](./backend/README.md) - Backend setup
- [contracts/README.md](./contracts/README.md) - Smart contract guide

---

## 🎓 Use Cases

**For Wallets & Apps:**
- Predict payment success before sending
- Suggest optimal routing paths
- Display real-time corridor health

**For Anchors & Issuers:**
- Monitor asset performance
- Identify liquidity gaps
- Track reliability metrics

**For Developers:**
- Access payment analytics via API
- Verify data on-chain
- Build on top of metrics

---

## 🔒 Security

Analytics snapshots are anchored on-chain via Soroban smart contracts, providing:
- ✅ Tamper-proof verification
- ✅ Immutable audit trails
- ✅ Trustless data integrity

---

## 📄 License

MIT License - see [LICENSE](./LICENSE) file for details.

---

## 🌟 Support

- **Issues:** [GitHub Issues](https://github.com/Ndifreke000/stellar-insights/issues)
- **Discussions:** [GitHub Discussions](https://github.com/Ndifreke000/stellar-insights/discussions)
- **Stellar:** [Stellar Developers](https://developers.stellar.org)

---

**Built for the Stellar ecosystem** 🚀
