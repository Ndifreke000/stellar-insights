# Stellar Insights — Official Code Documentation

> **Real-time payment analytics for Stellar.**
> Production-grade stack for measuring and improving cross-border payment reliability on the Stellar network.

---

## Table of Contents

1. [Overview](#overview)
2. [Repository Structure](#repository-structure)
3. [Backend — Rust Analytics Engine](#backend--rust-analytics-engine)
4. [Frontend — Next.js Dashboard](#frontend--nextjs-dashboard)
5. [Soroban Smart Contracts](#soroban-smart-contracts)
6. [Mobile App — React Native](#mobile-app--react-native)
7. [SDKs](#sdks)
8. [API Reference](#api-reference)
9. [RPC API Reference](#rpc-api-reference)
10. [WebSocket API](#websocket-api)
11. [Database Schema](#database-schema)
12. [Infrastructure & Deployment](#infrastructure--deployment)
13. [Observability Stack](#observability-stack)
14. [Security Architecture](#security-architecture)
15. [Performance Standards](#performance-standards)
16. [Dependencies & Versioning](#dependencies--versioning)
17. [Secrets Management](#secrets-management)
18. [Gas Costs & Benchmarks](#gas-costs--benchmarks)
19. [CI/CD Pipelines](#cicd-pipelines)
20. [Contributing](#contributing)
21. [Quick Start](#quick-start)

---

## 1. Overview

**Stellar Insights** is a full-stack, production-grade analytics and monitoring platform built specifically for the Stellar blockchain network. It gives developers, anchor operators, financial institutions, and end users a real-time, data-rich view into the health, performance, and reliability of cross-border payment corridors on Stellar.

### Core Problem Solved

The core problem it solves is **visibility**. The Stellar network processes thousands of cross-border payments between assets like USDC, EURC, XLM, BRL, NGN, and many more every day. While these payments are transparent on-chain, there has been no dedicated tool that aggregates that data, measures corridor-level success rates, tracks liquidity depth, monitors anchor health, and surfaces actionable insights.

### Key Features

- Real-time payment corridor analytics
- Anchor operator SEP-6, SEP-24, SEP-31 compliance monitoring
- Soroban on-chain analytics anchoring and governance
- React Native mobile monitoring
- Comprehensive SDKs (TypeScript & Python)
- Full Kubernetes + Terraform infrastructure
- Prometheus, OpenTelemetry/Jaeger, and ELK observability

---

## 2. Repository Structure

```
stellar-insights/
├── backend/        # Rust analytics engine + REST/GraphQL/WebSocket API
├── frontend/       # Next.js 16 dashboard with real-time updates
├── mobile/         # React Native mobile app (Expo)
├── contracts/      # Soroban smart contracts (Rust, no_std)
├── sdk/            # TypeScript + Python SDKs for API consumers
├── docs/           # Comprehensive technical documentation
├── scripts/        # Build and deployment scripts
├── k8s/            # Kubernetes configs
├── elk/            # ELK stack configs
└── terraform/      # AWS infrastructure as code
```

---

## 3. Backend — Rust Analytics Engine

### Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (edition 2021) |
| Web Framework | Axum 0.7 with Tower middleware |
| Database | PostgreSQL (primary), SQLite (dev), accessed via SQLx with compile-time query verification |
| Cache | Redis with in-memory fallback |
| Async Runtime | Tokio |
| Observability | OpenTelemetry + Jaeger for distributed tracing, Prometheus for metrics, structured JSON logging via tracing-subscriber |
| Authentication | JWT tokens + Argon2 password hashing + SEP-10 Stellar authentication |
| Secret Management | HashiCorp Vault integration |

### Backend Modules (`backend/src/`)

| Module | Purpose |
|--------|---------|
| `main.rs` | Application entry point |
| `lib.rs` | Library root |
| `database.rs` | Database connection and queries |
| `analytics.rs` | Corridor analytics engine |
| `websocket.rs` | WebSocket real-time feed |
| `auth.rs` | Authentication (JWT, SEP-10) |
| `cache.rs` | Caching layer |
| `rate_limit.rs` | Rate limiting |
| `logging.rs` | Structured logging |
| `health_check_enhanced.rs` | Health checks |
| `backup.rs` | Database backup |
| `monitor.rs` | System monitoring |
| `multi_network.rs` | Multi-network support |
| `ml.rs` | Machine learning anomaly detection |
| `shutdown.rs` | Graceful shutdown |

### Key Capabilities

**1. RPC Data Ingestion**
Connects to the Stellar Horizon API and continuously ingests payment data. Uses `StellarRpcClient` with circuit breaker patterns and configurable retry logic. Supports both paginated historical fetch and streaming real-time data.

**2. Corridor Analytics**
For each corridor, the backend computes:
- **Success rate** — percentage of payments that completed successfully
- **Transaction volume** — total USD-equivalent value transacted
- **Latency distribution** — histogram buckets at 100ms, 250ms, 500ms, 1s, and 2s+
- **Liquidity depth** — current orderbook depth in USD
- **Health score** — composite score (0–100) weighted by success rate (60%), volume (20%), and transaction count (20%)
- **Liquidity trend** — increasing, stable, or decreasing based on volume thresholds

**3. Anchor Monitoring**
Tracks each anchor's SEP-6, SEP-24, and SEP-31 compliance endpoints, measures response times, and computes health scores.

**4. Machine Learning Layer**
Lightweight ML module provides anomaly detection for payment corridors. Flags unusual patterns — sudden drops in success rate, abnormal latency spikes, or volume anomalies — and triggers alerts.

**5. Verification and Rewards**
`VerificationRewards` service incentivizes early snapshot verification. Contributors who verify analytics snapshots within one hour receive bonus rewards tracked on-chain via Soroban.

---

## 4. Frontend — Next.js Dashboard

### Technology Stack

| Component | Technology |
|-----------|------------|
| Framework | Next.js 16.2.7 with App Router |
| UI Library | React 19.2.5 |
| Styling | Tailwind CSS 4 with custom design system |
| Charts | Recharts 3.7 for time-series and distribution charts |
| State Management | Zustand 5 for global state, TanStack Query 5 for server state |
| Forms | React Hook Form 7 + Zod 4 for validation |
| Animations | Framer Motion 12 |
| Internationalisation | next-intl with English, Spanish, and Chinese locale support |
| Error Monitoring | Sentry |
| Testing | Vitest 4 + Testing Library + axe-core for accessibility |

### Key Pages

| Route | Description |
|-------|-------------|
| `/dashboard` | Main dashboard with KPI cards, liquidity depth chart, top assets table, corridor health panel, settlement speed chart. Auto-refreshes every 30 seconds with WebSocket live updates. |
| `/corridors` | Searchable, filterable list of all active payment corridors with health indicators, success rates, and volume. Each links to a detail page with historical charts, latency distribution, liquidity trends. |
| `/corridors/compare` | Side-by-side comparison of up to four corridors across all key metrics. |
| `/anchors` | Directory of Stellar anchor operators with health scores, supported assets, SEP protocol compliance status, and response time metrics. |
| `/network` | Network-wide view with interactive force-directed graph showing asset and anchor relationships. |
| `/analytics` | Deep analytics with time range selectors, corridor heatmaps, reliability trend charts, liquidity heatmaps. |
| `/governance` | On-chain governance proposals and voting status (Soroban governance contract). |
| `/sep6`, `/sep24` | Interactive deposit and withdrawal flow testers for SEP-6 and SEP-24 protocols. |
| `/transactions/builder` | Visual Stellar transaction builder for constructing, signing, and submitting transactions from the browser. |
| `/quests` | Gamified onboarding tracking progress through explorer quests. |
| `/liquidity-pools` | Stellar AMM liquidity pool analytics including pool composition, fee APR, and volume trends. |
| `/settings/gdpr` | User settings with full GDPR compliance — consent management, data export, and account deletion workflows. |

### Real-Time Architecture

The frontend maintains WebSocket connections to the backend for live updates. Separate hooks (`useRealtimeCorridors`, `useRealtimeAnchors`) manage these connections with automatic reconnection, exponential backoff, and connection status indicators.

---

## 5. Soroban Smart Contracts

### Contracts Overview

**1. `stellar_insights` — Core Analytics Contract**
Stores cryptographic SHA-256 hashes of analytics snapshots on-chain, creating an immutable audit trail.

Key functions:
- `initialize(admin)` — sets up contract with admin address, prevents re-initialization
- `submit_snapshot(epoch, hash, caller)` — stores snapshot hash for a given epoch; enforces monotonic epoch ordering to prevent rollback attacks; emits events for off-chain indexing
- `get_snapshot(epoch)` — retrieves the hash for a specific epoch
- `latest_snapshot()` — returns hash, epoch, and timestamp for the most recent snapshot
- `pause(caller)` / `unpause(caller)` — emergency controls that halt snapshot submission while keeping reads available
- `upgrade(new_wasm_hash)` — admin-only contract upgrade
- `get_contract_info()` — returns full metadata, pause state, admin address, and snapshot count

**2. `governance` / `governance-voting` — Governance Contracts**
On-chain governance for platform parameters. Stakeholders can create proposals, vote with weighted tokens, and execute approved changes.

**3. `escrow` — Escrow Contract**
Time-locked escrow for cross-border payment guarantees.

**4. `token-swap` — AMM Token Swap Contract**
Automated market maker for token swaps with configurable fee tiers.

**5. `multi-sig-wallet` — Multi-Signature Wallet**
M-of-N multisig wallet for treasury management and high-value operations.

**6. `access-control` — Role-Based Access Control**
Shared access control contract used by other contracts for role management.

**7. `time-locked-transactions` — Time-Locked Transaction Contract**
Schedules transactions to execute at a future ledger timestamp.

---

## 6. Mobile App — React Native

### Technology Stack

- Framework: React Native with Expo
- Navigation: React Navigation with tab and stack navigators
- State: Redux Toolkit
- Language: TypeScript

### Key Features

- Real-time corridor monitoring
- Push notifications for corridor degradation, anchor outages, and governance events
- SEP-10 mobile-native Stellar wallet authentication
- QR code scanning for payment addresses and Stellar federation addresses
- NFC support for tap-to-pay integration
- Biometric authentication (Face ID / Touch ID)
- Camera integration for KYC flows
- Bluetooth peripheral device integration
- App Clips / Live Activities (iOS)
- Wear OS / Watch App
- Offline support with cached data

---

## 7. SDKs

### TypeScript SDK (`sdk/typescript/`)

```typescript
import { StellarInsightsClient } from '@stellar-insights/sdk';

const client = new StellarInsightsClient({ apiKey: 'your-key' });

const corridors = await client.corridors.list({ limit: 20 });
const detail = await client.corridors.get('USDC:issuer->XLM:native');
const analytics = await client.analytics.network({ period: '7d' });
```

### Python SDK (`sdk/python/`)

```python
from stellar_insights import StellarInsightsClient

client = StellarInsightsClient(api_key="your-key")

corridors = client.corridors.list(limit=20)
detail = client.corridors.get("USDC:issuer->XLM:native")
```

---

## 8. API Reference

### Base URLs

- **Local Development:** `http://localhost:8080`
- **Production:** `https://api.stellarinsights.io`

### Authentication

**API Key Authentication**
Include your API key in the `Authorization` header:

```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
  https://api.stellarinsights.io/api/anchors
```

**OAuth 2.0**

```bash
# Get authorization code
GET /api/oauth/authorize?client_id=YOUR_CLIENT_ID&redirect_uri=YOUR_REDIRECT_URI

# Exchange for access token
POST /api/oauth/token
Content-Type: application/json

{
  "grant_type": "authorization_code",
  "code": "AUTH_CODE",
  "client_id": "YOUR_CLIENT_ID",
  "client_secret": "YOUR_CLIENT_SECRET"
}
```

### Core Endpoints

#### Anchors

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/anchors` | List all anchors |
| GET | `/api/anchors/{id}` | Get anchor details |
| GET | `/api/anchors/account/{account}` | Get anchor by account |
| GET | `/api/anchors/{id}/muxed` | Get muxed account analytics |
| POST | `/api/anchors` | Create new anchor |
| PUT | `/api/anchors/{id}/metrics` | Update anchor metrics |

#### Payment Corridors

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/corridors` | List payment corridors |
| GET | `/api/corridors/{source}/{destination}` | Get corridor details |
| GET | `/api/corridors/{source}/{destination}/metrics` | Get corridor metrics |
| POST | `/api/corridors` | Create new corridor |

#### Price & Analytics

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/prices` | Get current asset prices |
| GET | `/api/prices/{asset}` | Get specific asset price |
| POST | `/api/prices/convert` | Convert between assets |
| GET | `/api/network` | Network-wide statistics |
| GET | `/api/analytics` | Aggregated analytics data |

#### Alerts

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/alerts/rules` | List alert rules |
| POST | `/api/alerts/rules` | Create alert rule |
| PUT | `/api/alerts/rules/{id}` | Update alert rule |
| DELETE | `/api/alerts/rules/{id}` | Delete alert rule |
| GET | `/api/alerts/history` | Get alert history |

#### Webhooks

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/webhooks` | Register webhook |
| GET | `/api/webhooks` | List webhooks |
| DELETE | `/api/webhooks/{id}` | Delete webhook |
| POST | `/api/webhooks/{id}/test` | Test webhook |

#### Data Export

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/export/csv` | Export data as CSV (rate limited) |
| GET | `/api/export/excel` | Export data as Excel (rate limited) |

#### Governance

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/governance` | On-chain governance data |

#### Health

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Enhanced health check (DB, cache, RPC, jobs) |

### Rate Limiting

| Tier | Requests | Description |
|------|----------|-------------|
| Anonymous | 100 req/min | Unauthenticated access |
| Authenticated | 1,000 req/min | With valid API key |
| Premium | 10,000 req/min | Premium tier clients |

Headers:
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1234567890
```

### Error Handling

All errors follow a consistent format:

```json
{
  "error": "INVALID_REQUEST",
  "message": "Missing required parameter: source_asset",
  "status": 400,
  "request_id": "req-12345"
}
```

Common Error Codes:

| Code | Status | Description |
|------|--------|-------------|
| `INVALID_REQUEST` | 400 | Invalid request parameters |
| `UNAUTHORIZED` | 401 | Missing or invalid authentication |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `RATE_LIMITED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |

---

## 9. RPC API Reference

**Version:** v0.1.0  
**Last Updated:** February 26, 2026

### Configuration Environment Variables

```env
DATABASE_URL=sqlite:stellar_insights.db
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
STELLAR_RPC_URL=https://stellar.api.onfinality.io/public
STELLAR_HORIZON_URL=https://horizon.stellar.org
RPC_MOCK_MODE=false
RUST_LOG=info
```

### RPC Endpoints

#### `GET /api/rpc/health`

Check Stellar RPC connection health and network status.

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": "healthy",
    "latestLedger": 51583040,
    "oldestLedger": 51565760,
    "ledgerRetentionWindow": 17281
  }
}
```

#### `GET /api/rpc/ledger/latest`

Get the most recent ledger information.

Response:
```json
{
  "sequence": 51583040,
  "hash": "abc123...",
  "closed_at": "2026-01-26T10:30:00Z",
  "transaction_count": 142,
  "operation_count": 389
}
```

#### `GET /api/rpc/payments`

Fetch recent payment operations from the Stellar network.

Query Parameters: `limit` (max 200, default 20), `cursor` (string, pagination cursor)

Response:
```json
{
  "_embedded": {
    "records": [
      {
        "id": "123456789",
        "type": "payment",
        "from": "GABC...",
        "to": "GDEF...",
        "asset_type": "credit_alphanum4",
        "asset_code": "USDC",
        "asset_issuer": "GBBD...",
        "amount": "100.0000000",
        "created_at": "2026-01-26T10:30:00Z"
      }
    ]
  },
  "_links": {
    "next": {
      "href": "/api/rpc/payments?cursor=123456789&limit=20"
    }
  }
}
```

#### `GET /api/rpc/payments/account/:account_id`

Get payment history for a specific Stellar account.

Path Parameters: `account_id` — Stellar account address (G...)

#### `GET /api/rpc/trades`

Fetch recent trade operations from the Stellar DEX.

Query Parameters: `limit` (integer, default 20), `cursor` (string, pagination cursor)

#### `GET /api/rpc/orderbook`

Get order book for a specific trading pair.

Query Parameters:
- `selling_asset_type` (string, required)
- `selling_asset_code` (string, conditional — required if not native)
- `selling_asset_issuer` (string, conditional — required if not native)
- `buying_asset_type` (string, required)
- `buying_asset_code` (string, conditional — required if not native)
- `buying_asset_issuer` (string, conditional — required if not native)
- `limit` (integer, default 20)

### Response Codes

| Code | Description |
|------|-------------|
| 200 | Success |
| 201 | Created |
| 400 | Bad Request - Invalid parameters |
| 401 | Unauthorized - Authentication required |
| 403 | Forbidden - Insufficient permissions |
| 404 | Not Found - Resource doesn't exist |
| 429 | Too Many Requests - Rate limit exceeded |
| 500 | Internal Server Error |
| 503 | Service Unavailable - RPC connection failed |

---

## 10. WebSocket API

Real-time updates via WebSocket at `wss://api.stellarinsights.io/ws`.

```javascript
const ws = new WebSocket('wss://api.stellarinsights.io/ws');

ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'corridor:USD:EUR'
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Update:', data);
};
```

---

## 11. Database Schema

30 SQL migrations define the schema with 23 additional indexes:

| Table | Purpose |
|-------|---------|
| `anchors` | Anchor operator registry |
| `corridor_metrics` | Daily corridor performance snapshots |
| `corridor_aggregates` | Pre-aggregated metrics for fast queries |
| `payment_records` | Individual payment ingestion records |
| `liquidity_pools` | AMM pool data |
| `trustlines` | Account trustline tracking |
| `transactions` | Transaction history |
| `account_merges` | Account merge events |
| `snapshot_verifications` | On-chain snapshot verification records |
| `api_keys` | API key management |
| `users` | User accounts |
| `gdpr_*` | Consent and data deletion records |
| `telegram_subscriptions` | Telegram alert subscriptions |
| `vault_audit_log` | Vault secret access audit trail |
| `admin_audit_log` | Admin action audit trail |
| `oauth_tokens` and `webhooks` | OAuth and webhook management |
| `api_usage_stats` | Per-key API usage tracking |
| `governance` | On-chain governance proposal cache |
| `replay_events` | Event replay system for data recovery |
| `contract_events` | Soroban event index |
| `verified_assets` | Cross-referenced asset verification |
| `alert_rules` and `alert_history` | Configurable alerting |

---

## 12. Infrastructure & Deployment

### Docker

- `Dockerfile` for the Rust backend with multi-stage builds
- `docker-compose.elk.yml` for the ELK (Elasticsearch, Logstash, Kibana) logging stack
- `docker-compose.jaeger.yml` for distributed tracing

### Kubernetes

- Backend deployment with HPA (Horizontal Pod Autoscaler) and PDB (Pod Disruption Budget)
- Frontend deployment with HPA and PDB
- PostgreSQL StatefulSet
- Redis deployment
- Ingress with TLS termination
- Network policies for pod-to-pod communication
- ConfigMaps and Secrets templates
- Kustomize overlays for dev, staging, and production
- Prometheus ServiceMonitor and AlertManager rules
- ELK stack integration

### Terraform (AWS)

- **Networking** — VPC, subnets, security groups
- **Compute** — ECS Fargate tasks for backend and frontend
- **Database** — RDS PostgreSQL with read replicas
- **Caching** — ElastiCache Redis
- **Load Balancing** — Application Load Balancer with WAF
- **Monitoring** — CloudWatch dashboards and alarms
- **Vault** — HashiCorp Vault on EC2 for secrets management
- **CodeDeploy** — Blue/green deployment pipeline
- **CDN** — CloudFront for frontend assets

---

## 13. Observability Stack

### Metrics (Prometheus + Grafana)

Custom Prometheus metrics for:
- HTTP request rate, latency, and error rate per endpoint
- Database pool utilization (size, idle, active connections)
- WebSocket connection count
- Cache hit/miss ratio
- RPC request success/failure rate
- Corridor count and health score distribution

### Tracing (OpenTelemetry + Jaeger)

Every HTTP request, database query, and RPC call generates a distributed trace. Trace context is propagated through HTTP headers for end-to-end visibility.

### Logging (ELK Stack)

Structured JSON logs shipped via Filebeat to Logstash and indexed in Elasticsearch. Pre-built Kibana dashboards provide log search, error rate visualization, and alert configuration.

### Health Checks

Dedicated enhanced health check endpoint (`/health`) reports on database connectivity, cache connectivity, RPC client status, and background job health.

---

## 14. Security Architecture

### Authentication & Authorization

- JWT-based session tokens with configurable expiry
- Argon2id password hashing with per-user salts
- SEP-10 Stellar challenge/response authentication
- Role-based access control with admin, user, and read-only roles
- API key authentication for programmatic access with per-key rate limits

### Secret Management

- HashiCorp Vault manages all secrets (database credentials, API keys, JWT secrets, encryption keys)
- Secrets are never stored in environment files in production
- AppRole authentication for backend secret retrieval
- 90-day automatic rotation

### Transport Security

- TLS everywhere via Kubernetes ingress
- CORS configured with explicit allowed origins
- CSRF protection via double-submit cookie pattern
- Content Security Policy headers
- HSTS enforcement

### Input Validation

- All API inputs validated before processing
- SQL injection prevention via SQLx parameterized queries (compile-time verified)
- GraphQL query depth limiting to prevent DoS
- Request payload size limits

### Data Privacy (GDPR Compliance)

- Explicit consent management with granular categories
- Right to data export (JSON download)
- Right to erasure (account deletion with cascading data removal)
- Audit trail of all data processing
- PII redaction in logs via `redact_user_id()` and `redact_hash()` utilities

---

## 15. Performance Standards

| Metric | Budget | Enforcement |
|--------|--------|-------------|
| Main bundle (gzipped) | ≤ 200 KB | CI |
| Per-asset raw size | ≤ 500 KB | webpack `performance.hints` |
| Largest Contentful Paint (LCP) | ≤ 2.5 s | Lighthouse |
| First Input Delay (FID) | ≤ 100 ms | Lighthouse |
| Cumulative Layout Shift (CLS) | ≤ 0.1 | Lighthouse |
| Time to Interactive (TTI) | ≤ 3 s | Lighthouse |
| Lighthouse Performance score | ≥ 90 | CI (optional) |

---

## 16. Dependencies & Versioning

### Pinning Policy

- **Frontend** (`frontend/package.json`): exact versions, managed via `pnpm-lock.yaml`
- **Backend** (`backend/Cargo.toml`): major/minor ranges (e.g. `"1.0"`, `"0.8"`), pinned via `Cargo.lock`

Both lockfiles are committed to the repository and are the source of truth for reproducible builds.

### Updating Dependencies

1. Update the version in `package.json` or `Cargo.toml`
2. Run `pnpm install` (frontend) or `cargo update` (backend) to regenerate the lockfile
3. Run the full test suite before merging
4. Document the reason for the update in the PR description

### Security Audits

- Frontend: `pnpm audit` — run in CI on every PR
- Backend: `cargo audit` — run in CI on every PR

---

## 17. Secrets Management

Stellar Insights implements a comprehensive secrets management solution using **HashiCorp Vault** for secure storage, rotation, and audit logging of sensitive credentials.

### Architecture

- **HashiCorp Vault** — Central secrets store
- **Vault Agent** — Automatic secret injection
- **Audit Logging** — Track all secret access
- **Secret Rotation** — Automatic 90-day rotation

### Setup

1. Install Vault
2. Start Vault Server (`vault server -dev` for development)
3. Initialize Vault (`vault operator init -key-shares=5 -key-threshold=3`)
4. Unseal Vault (requires 3 of 5 keys)
5. Enable KV Secrets Engine (`vault secrets enable -path=secret kv-v2`)

### Secret Storage Example

```bash
# Database credentials
vault kv put secret/database/postgres \
  username=postgres \
  password=secure-password \
  host=db.example.com \
  port=5432

# API keys
vault kv put secret/api/stellar \
  rpc_url=https://rpc.stellar.org \
  api_key=your-api-key
```

### Kubernetes Integration

Vault Agent Injector automatically injects secrets into application pods as sidecar containers, removing the need for Kubernetes secrets in production.

---

## 18. Gas Costs & Benchmarks

Gas cost benchmarks for Soroban contracts, measured with Criterion.

### Running Benchmarks

```bash
# All contracts
cargo bench --package contract-benches

# Single benchmark
cargo bench --package contract-benches -- analytics::submit_snapshot
```

### Analytics Contract Benchmarks

| Benchmark | What it measures |
|-----------|-------------------|
| `analytics::submit_snapshot` | Single snapshot write (epoch + hash + submitter) |
| `analytics::get_snapshot` | Point read from a 100-snapshot store |
| `analytics::get_latest_snapshot` | Read the latest-epoch pointer |
| `analytics::batch_submit/N` | Sequential write of N snapshots (N = 5, 10, 25, 50) |

### Governance Contract Benchmarks

| Benchmark | What it measures |
|-----------|-------------------|
| `governance::create_proposal` | Proposal creation (title + wasm hash + tally init) |
| `governance::vote` | Single vote cast on a fresh proposal |
| `governance::multi_vote/N` | N votes on one proposal (N = 5, 10, 25, 50) |

### Regression Policy

CI fails a pull request if any benchmark regresses by more than **20%** relative to the baseline.

### Optimization Guide

- Prefer `instance` storage for small, frequently-read values
- Use `persistent` storage keyed by a specific ID rather than loading an entire map
- Avoid unbounded `Map` or `Vec` reads — cost scales linearly with size
- Batch multiple writes in a single invocation where possible
- Use `symbol_short!` for event topics to avoid heap allocation

---

## 19. CI/CD Pipelines

30+ GitHub Actions workflows covering:

- **Code Quality** — Clippy (Rust), ESLint (TypeScript), rustfmt
- **Security** — CodeQL analysis, dependency audit, ZAP OWASP scan, Snyk
- **Testing** — Unit tests, integration tests, accessibility tests, contract fuzzing
- **Performance** — Load testing with k6, gas regression testing, performance budget enforcement
- **Deployment** — Automated deploy to staging on merge to main, manual approval for production
- **Rollback** — One-click rollback to previous version
- **Changelog** — Automated CHANGELOG generation from conventional commits
- **Vault** — Automated secret rotation via AppRole

---

## 20. Contributing

The project follows conventional commits (enforced by commitlint in CI), semantic versioning, and a detailed contribution guide. Issues are tracked via GitHub Issues with structured templates for bugs, features, and performance reports.

### Development Standards

- Markdown format for documentation
- Clear headings and structure
- Code examples where applicable
- Links to related documentation
- Regular updates with project changes

---

## 21. Quick Start

### Prerequisites

- Node.js (use `.nvmrc` for correct version)
- Rust (edition 2021)
- Docker
- Git LFS (for large binaries)

### Backend Setup

```bash
cd backend
cp .env.example .env
# set DATABASE_URL, STELLAR_RPC_URL, etc.
cargo run
```

### Frontend Setup

```bash
cd frontend
npm install
npm run dev
```

### Mobile Setup

See `mobile/README.md` for React Native / Expo setup instructions.

### Running Tests

```bash
# Backend
cd backend
cargo test

# Frontend
cd frontend
npm run test -- path/to/TestFile.spec.tsx

# Mutation tests (Rust)
cargo bench --package contract-benches
```

---

*Generated from consolidated project documentation. For detailed component docs, see the `docs/` directory.*
