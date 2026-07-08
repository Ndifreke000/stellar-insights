# Stellar Insights — Detailed Project Description

---

## What Is Stellar Insights?

Stellar Insights is a full-stack, production-grade analytics and monitoring platform built specifically for the **Stellar blockchain network**. Its primary purpose is to give developers, anchor operators, financial institutions, and end users a real-time, data-rich view into the health, performance, and reliability of cross-border payment corridors on the Stellar network.

The core problem it solves is visibility. The Stellar network processes thousands of cross-border payments between assets like USDC, EURC, XLM, BRL, NGN, and many more every day. While these payments are transparent on-chain, there has been no dedicated tool that aggregates that data, measures corridor-level success rates, tracks liquidity depth, monitors anchor health, and surfaces actionable insights to the people building on top of Stellar. Stellar Insights fills that gap.

The platform is not just a dashboard. It is a complete observability stack — from a high-performance Rust backend that ingests live RPC data from the Stellar Horizon API, to a Next.js frontend with real-time WebSocket updates, to Soroban smart contracts that anchor analytics snapshots on-chain for tamper-proof verification, to a React Native mobile app for monitoring on the go.

---

## Architecture Overview

The project is organized as a monorepo with six major components:

```
stellar-insights/
├── backend/        Rust analytics engine + REST/GraphQL/WebSocket API
├── frontend/       Next.js 16 dashboard with real-time updates
├── mobile/         React Native mobile app (Expo)
├── contracts/      Soroban smart contracts (Rust, no_std)
├── sdk/            TypeScript + Python SDKs for API consumers
└── docs/           Comprehensive technical documentation
```

Each component is independently deployable but designed to work together as a unified platform.

---

## Backend — Rust Analytics Engine

The backend is built in Rust using the **Axum** web framework and is the heart of the platform. It is responsible for everything from ingesting raw Stellar network data to serving it through a structured, versioned API.

### Technology Stack

- **Language:** Rust (edition 2021)
- **Web framework:** Axum 0.7 with Tower middleware
- **Database:** PostgreSQL (primary) with SQLite fallback for local development, accessed via SQLx with compile-time query verification
- **Cache:** Redis with in-memory fallback
- **Async runtime:** Tokio
- **Observability:** OpenTelemetry + Jaeger for distributed tracing, Prometheus for metrics, structured JSON logging via tracing-subscriber
- **Authentication:** JWT tokens + Argon2 password hashing + SEP-10 Stellar authentication
- **Secret management:** HashiCorp Vault integration

### What the Backend Does

**1. RPC Data Ingestion**

The backend connects to the Stellar Horizon API and continuously ingests payment data. It uses a `StellarRpcClient` with circuit breaker patterns and configurable retry logic to handle network instability gracefully. Payments are parsed to extract asset pairs, volumes, success/failure status, and latency metrics. The ingestion pipeline supports both paginated historical fetch and streaming real-time data.

**2. Corridor Analytics**

The most important analytical concept in the system is the "corridor" — a directional asset pair such as `USDC:issuerA → XLM:native` or `BRL:issuerB → EURC:issuerC`. For each corridor the backend computes:

- **Success rate** — percentage of payments that completed successfully
- **Transaction volume** — total USD-equivalent value transacted
- **Latency distribution** — histogram buckets at 100ms, 250ms, 500ms, 1s, and 2s+
- **Liquidity depth** — current orderbook depth in USD
- **Health score** — composite score (0–100) weighted by success rate (60%), volume (20%), and transaction count (20%)
- **Liquidity trend** — increasing, stable, or decreasing based on volume thresholds

These metrics are computed both for real-time snapshots and for historical time windows (24h, 7d, 30d, 90d) using an aggregation layer backed by the `corridor_aggregates` database table.

**3. Anchor Monitoring**

The backend monitors Stellar anchor operators — the financial institutions that issue and redeem assets on the network. It tracks each anchor's SEP-6, SEP-24, and SEP-31 compliance endpoints, measures response times, and computes health scores. The `AssetVerifier` service validates assets against the anchor registry and verifies `stellar.toml` configurations.

**4. Machine Learning Layer**

A lightweight ML module provides anomaly detection for payment corridors. It uses statistical models to flag unusual patterns — sudden drops in success rate, abnormal latency spikes, or volume anomalies — and triggers alerts through the alert service.

**5. Verification and Rewards**

The `VerificationRewards` service incentivizes early snapshot verification. When a contributor verifies an analytics snapshot within one hour of its creation, they receive bonus rewards. This is tracked on-chain via the Soroban contract and reconciled in the backend.

**6. API Surface**

The backend exposes a fully versioned REST API under `/api/v1/`, a GraphQL endpoint at `/graphql`, and WebSocket connections at `/ws`. Key endpoint groups include:

- `GET /api/corridors` — list corridors with filtering and pagination
- `GET /api/corridors/:id` — corridor detail with historical data
- `GET /api/anchors` — anchor list with health scores
- `GET /api/network` — network-wide statistics
- `GET /api/analytics` — aggregated analytics data
- `GET /api/export/csv` and `/excel` — data export with rate limiting
- `POST /api/auth/*` — JWT + SEP-10 authentication
- `GET /api/governance` — on-chain governance data
- `POST /graphql` — full GraphQL query support

**7. Infrastructure and Operations**

The backend includes a complete operational toolkit:

- **Database migrations** — 30 versioned SQL migrations covering every schema change
- **Backfill jobs** — historical data backfill with progress tracking
- **Webhook dispatcher** — event-driven notifications with exponential backoff and configurable max restarts
- **Backup manager** — scheduled database backups
- **Graceful shutdown** — coordinated shutdown of WebSocket connections, cache flush, and database pool drain
- **Rate limiting** — per-endpoint and per-client limits (anonymous, authenticated, premium tiers)
- **Concurrency limiting** — caps in-flight requests at a configurable threshold to prevent 500 errors under spike load
- **Request timeout** — configurable timeout (5–300s) with proper 408 responses
- **CORS** — configurable allowed origins from environment
- **Compression** — gzip + brotli for responses above a configurable size threshold
- **ETag caching** — HTTP cache headers for conditional requests
- **IP whitelist** — optional IP-based access control for admin endpoints
- **Audit logging** — admin action audit trail stored in database

---

## Frontend — Next.js Dashboard

The frontend is a Next.js 16 application using React 19, Tailwind CSS 4, and a real-time WebSocket layer that keeps the dashboard live without manual refreshes.

### Technology Stack

- **Framework:** Next.js 16.2.7 with App Router
- **UI library:** React 19.2.5
- **Styling:** Tailwind CSS 4 with custom design system
- **Charts:** Recharts 3.7 for time-series and distribution charts
- **State management:** Zustand 5 for global state, TanStack Query 5 for server state
- **Forms:** React Hook Form 7 + Zod 4 for validation
- **Animations:** Framer Motion 12
- **Internationalisation:** next-intl with English, Spanish, and Chinese locale support
- **Error monitoring:** Sentry
- **Testing:** Vitest 4 + Testing Library + axe-core for accessibility

### Key Pages and Features

**Dashboard (`/dashboard`)**

The main dashboard shows four KPI cards (payment success rate, active corridors, liquidity depth, average settlement speed), a liquidity depth chart over time, a top assets table with 24h volume and price change, a corridor health panel, and a settlement speed chart. All data auto-refreshes every 30 seconds and is also updated in real-time via WebSocket when new payment data arrives. The page shows skeleton loading states while data is fetching and gracefully handles API errors with clear error messages.

**Corridors (`/corridors`)**

A searchable, filterable list of all active payment corridors with health indicators, success rates, and volume. Each corridor links to a detail page showing historical success rate charts, latency distribution histograms, liquidity trends, and related corridors.

**Corridor Comparison (`/corridors/compare`)**

Side-by-side comparison of up to four corridors across all key metrics — useful for evaluating which corridor to route a payment through.

**Anchors (`/anchors`)**

Directory of Stellar anchor operators with health scores, supported assets, SEP protocol compliance status, and response time metrics.

**Network (`/network`)**

Network-wide view with an interactive force-directed graph showing asset and anchor relationships, plus global statistics.

**Analytics (`/analytics`)**

Deep analytics with time range selectors, corridor heatmaps, reliability trend charts, and liquidity heatmaps.

**Governance (`/governance`)**

On-chain governance proposals and voting status, pulled from the Soroban governance contract.

**SEP-6 and SEP-24 Flows (`/sep6`, `/sep24`)**

Interactive deposit and withdrawal flow testers for SEP-6 (non-interactive) and SEP-24 (interactive) anchor protocols. Developers can test anchor integrations directly in the dashboard without writing code.

**Transaction Builder (`/transactions/builder`)**

A visual Stellar transaction builder that lets users construct, sign, and submit transactions from the browser.

**Quests (`/quests`)**

A gamified onboarding system that tracks progress through a series of explorer quests — visiting pages, performing actions, and interacting with features — to help new users discover the platform's capabilities.

**Liquidity Pools (`/liquidity-pools`)**

Stellar AMM liquidity pool analytics including pool composition, fee APR, and volume trends.

**Settings and GDPR (`/settings/gdpr`)**

User settings with full GDPR compliance — consent management, data export, and account deletion workflows.

### Real-Time Architecture

The frontend maintains WebSocket connections to the backend for live updates. Separate hooks (`useRealtimeCorridors`, `useRealtimeAnchors`) manage these connections with automatic reconnection, exponential backoff, and connection status indicators. The `WebSocketStatus` component shows users whether the live feed is connected, connecting, or disconnected, and provides a manual reconnect button.

### Notification System

A rich notification system with multiple layers:

- **Toast notifications** — transient alerts with auto-hide and progress bar
- **Notification center** — persistent notification history with filtering, search, and bulk actions
- **Bell indicator** — unread count badge on the header
- **Notification preferences** — per-category and per-channel (browser, email, Telegram) settings

### Progressive Web App

The frontend is configured as a PWA with a web manifest, service worker for offline support, and app icons. It supports installation to the home screen on mobile and desktop.

---

## Soroban Smart Contracts

The contracts layer is built with Soroban (Stellar's smart contract platform) in Rust with `no_std`. They provide tamper-proof on-chain anchoring of analytics data and governance.

### Contracts Overview

**1. `stellar_insights` — Core Analytics Contract**

The primary contract. It stores cryptographic SHA-256 hashes of analytics snapshots on-chain, creating an immutable audit trail that anyone can verify. Key functions:

- `initialize(admin)` — sets up the contract with an admin address, prevents re-initialization
- `submit_snapshot(epoch, hash, caller)` — stores a snapshot hash for a given epoch; enforces monotonic epoch ordering to prevent rollback attacks; emits events for off-chain indexing
- `get_snapshot(epoch)` — retrieves the hash for a specific epoch
- `latest_snapshot()` — returns hash, epoch, and timestamp for the most recent snapshot
- `pause(caller)` / `unpause(caller)` — emergency controls that halt snapshot submission while keeping reads available
- `upgrade(new_wasm_hash)` — admin-only contract upgrade
- `get_contract_info()` — returns full metadata, pause state, admin address, and snapshot count

The contract uses Soroban's persistent storage with automatic TTL extension (~30 days at 5s/ledger) to keep data alive. Events are emitted on every state change for off-chain indexers.

**2. `governance` / `governance-voting` — Governance Contracts**

On-chain governance for platform parameters. Stakeholders can create proposals, vote with weighted tokens, and execute approved changes. The voting contract tracks quorum requirements and voting deadlines.

**3. `escrow` — Escrow Contract**

Time-locked escrow for cross-border payment guarantees. Funds are held in escrow until conditions are met or a timeout expires.

**4. `token-swap` — AMM Token Swap Contract**

Automated market maker for token swaps with configurable fee tiers.

**5. `multi-sig-wallet` — Multi-Signature Wallet**

M-of-N multisig wallet for treasury management and high-value operations.

**6. `access-control` — Role-Based Access Control**

Shared access control contract used by other contracts for role management.

**7. `time-locked-transactions` — Time-Locked Transaction Contract**

Schedules transactions to execute at a future ledger timestamp.

---

## Mobile App — React Native

The mobile app provides on-the-go monitoring for corridor health and payment activity.

### Technology Stack

- **Framework:** React Native with Expo
- **Navigation:** React Navigation with tab and stack navigators
- **State:** Redux Toolkit
- **Language:** TypeScript

### Key Features

- **Real-time corridor monitoring** — same data as the web dashboard, optimized for mobile
- **Push notifications** — alerts for corridor degradation, anchor outages, and governance events
- **SEP-10 authentication** — mobile-native Stellar wallet authentication
- **QR code scanning** — scan payment addresses and Stellar federation addresses
- **NFC support** — tap-to-pay integration for NFC-capable devices
- **Biometric authentication** — Face ID / Touch ID for app access
- **Camera integration** — document capture for KYC flows
- **Bluetooth support** — peripheral device integration
- **App Clips / Live Activities** — iOS-specific lightweight experiences
- **Wear OS / Watch App** — wrist-level payment status monitoring
- **Offline support** — cached data available without network

---

## SDKs

Two client SDKs allow developers to integrate Stellar Insights data into their own applications.

**TypeScript SDK (`sdk/typescript/`)**

```typescript
import { StellarInsightsClient } from '@stellar-insights/sdk';

const client = new StellarInsightsClient({ apiKey: 'your-key' });

const corridors = await client.corridors.list({ limit: 20 });
const detail = await client.corridors.get('USDC:issuer->XLM:native');
const analytics = await client.analytics.network({ period: '7d' });
```

**Python SDK (`sdk/python/`)**

```python
from stellar_insights import StellarInsightsClient

client = StellarInsightsClient(api_key="your-key")

corridors = client.corridors.list(limit=20)
detail = client.corridors.get("USDC:issuer->XLM:native")
```

---

## Infrastructure and Deployment

The project ships with a complete infrastructure-as-code and container orchestration setup.

### Docker

- `Dockerfile` for the Rust backend with multi-stage builds
- `docker-compose.elk.yml` for the ELK (Elasticsearch, Logstash, Kibana) logging stack
- `docker-compose.jaeger.yml` for distributed tracing

### Kubernetes

A full Kubernetes deployment under `k8s/` with:

- Backend deployment with HPA (Horizontal Pod Autoscaler) and PDB (Pod Disruption Budget)
- Frontend deployment with HPA and PDB
- PostgreSQL StatefulSet
- Redis deployment
- Ingress with TLS termination
- Network policies for pod-to-pod communication
- ConfigMaps and Secrets templates
- Kustomize overlays for dev, staging, and production environments
- Prometheus ServiceMonitor and AlertManager rules
- ELK stack integration

### Terraform

AWS infrastructure under `terraform/` with modules for:

- **Networking** — VPC, subnets, security groups
- **Compute** — ECS Fargate tasks for backend and frontend
- **Database** — RDS PostgreSQL with read replicas
- **Caching** — ElastiCache Redis
- **Load balancing** — Application Load Balancer with WAF
- **Monitoring** — CloudWatch dashboards and alarms
- **Vault** — HashiCorp Vault on EC2 for secrets management
- **CodeDeploy** — Blue/green deployment pipeline
- **CDN** — CloudFront for frontend assets

Separate environment configurations for dev, staging, and production ensure full environment parity.

### CI/CD Pipelines (GitHub Actions)

30+ GitHub Actions workflows covering:

- **Code quality** — Clippy (Rust), ESLint (TypeScript), rustfmt
- **Security** — CodeQL analysis, dependency audit, ZAP OWASP scan, Snyk
- **Testing** — Unit tests, integration tests, accessibility tests, contract fuzzing
- **Performance** — Load testing with k6, gas regression testing for contracts, performance budget enforcement
- **Deployment** — Automated deploy to staging on merge to main, manual approval for production
- **Rollback** — One-click rollback to previous version
- **Changelog** — Automated CHANGELOG generation from conventional commits
- **Vault** — Automated secret rotation via AppRole

---

## Observability Stack

Production-grade observability is built in from the start, not added as an afterthought.

**Metrics (Prometheus + Grafana)**

Custom Prometheus metrics for:
- HTTP request rate, latency, and error rate per endpoint
- Database pool utilization (size, idle, active connections)
- WebSocket connection count
- Cache hit/miss ratio
- RPC request success/failure rate
- Corridor count and health score distribution

A pre-built Grafana dashboard (`docs/grafana/observability-dashboard.json`) visualizes all metrics.

**Tracing (OpenTelemetry + Jaeger)**

Every HTTP request, database query, and RPC call generates a distributed trace. Trace context is propagated through HTTP headers for end-to-end visibility across the frontend, backend, and external Horizon API.

**Logging (ELK Stack)**

Structured JSON logs are shipped via Filebeat to Logstash and indexed in Elasticsearch. Pre-built Kibana dashboards provide log search, error rate visualization, and alert configuration.

**Health Checks**

A dedicated enhanced health check endpoint (`/health`) reports on database connectivity, cache connectivity, RPC client status, and background job health. Kubernetes liveness and readiness probes use this endpoint.

---

## Security Architecture

Security is treated as a first-class concern throughout the stack.

**Authentication and Authorization**

- JWT-based session tokens with configurable expiry
- Argon2id password hashing with per-user salts
- SEP-10 Stellar challenge/response authentication for wallet-based login
- Role-based access control with admin, user, and read-only roles
- API key authentication for programmatic access with per-key rate limits

**Secret Management**

HashiCorp Vault manages all secrets — database credentials, API keys, JWT secrets, and encryption keys. Secrets are never stored in environment files in production. AppRole authentication allows the backend to retrieve secrets at startup without storing a master token.

**Transport Security**

- TLS everywhere via Kubernetes ingress
- CORS configured with explicit allowed origins
- CSRF protection via double-submit cookie pattern (`CsrfTokenProvider`)
- Content Security Policy headers
- HSTS enforcement

**Input Validation**

- All API inputs validated with the `validator` crate before processing
- SQL injection prevention via SQLx parameterized queries (compile-time verified)
- GraphQL query depth limiting to prevent DoS
- Request payload size limits

**Dependency Security**

- Automated Dependabot PRs for dependency updates
- `npm audit` and `cargo audit` in CI
- Snyk scanning for known vulnerabilities

**Data Privacy**

Full GDPR compliance:
- Explicit consent management with granular categories
- Right to data export (JSON download)
- Right to erasure (account deletion with cascading data removal)
- Audit trail of all data processing
- PII redaction in logs via `redact_user_id()` and `redact_hash()` utilities

---

## Database Schema

30 SQL migrations define a comprehensive schema:

- `anchors` — anchor operator registry
- `corridor_metrics` — daily corridor performance snapshots
- `corridor_aggregates` — pre-aggregated metrics for fast queries
- `payment_records` — individual payment ingestion records
- `liquidity_pools` — AMM pool data
- `trustlines` — account trustline tracking
- `transactions` — transaction history
- `account_merges` — account merge events
- `snapshot_verifications` — on-chain snapshot verification records
- `api_keys` — API key management
- `users` — user accounts
- `gdpr_*` — consent and data deletion records
- `telegram_subscriptions` — Telegram alert subscriptions
- `vault_audit_log` — Vault secret access audit trail
- `admin_audit_log` — admin action audit trail
- `oauth_tokens` and `webhooks` — OAuth and webhook management
- `api_usage_stats` — per-key API usage tracking
- `governance` — on-chain governance proposal cache
- `replay_events` — event replay system for data recovery
- `contract_events` — Soroban event index
- `verified_assets` — cross-referenced asset verification
- `alert_rules` and `alert_history` — configurable alerting

23 additional indexes optimize the most common query patterns, with a dedicated migration for performance-critical indexes.

---

## Contribution Activity

The repository has accumulated **1,779 commits** from **196+ contributors** across a broad open-source contributor base. The project follows conventional commits (enforced by commitlint in CI), semantic versioning, and a detailed contribution guide. Issues are tracked via GitHub Issues with structured templates for bugs, features, and performance reports.

---

## Summary

Stellar Insights is a comprehensive, production-ready analytics platform for the Stellar blockchain. It combines a high-performance Rust backend with real-time data ingestion, a polished Next.js dashboard with live WebSocket updates, Soroban smart contracts for on-chain data verification, a React Native mobile app, and a complete cloud infrastructure stack. The project is built to handle the demands of a real-time financial data platform — low latency, high availability, strong security, and full observability — while remaining accessible to open-source contributors through clear documentation, structured issue tracking, and automated quality gates.
