# Security Audit Preparation Document

**Project:** Stellar Insights
**Version:** Current (dev branch)
**Date:** 2026-02-22
**Classification:** Confidential
**Prepared for:** External Security Audit Team

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Architecture Overview](#2-architecture-overview)
3. [Technology Stack](#3-technology-stack)
4. [Attack Surface Inventory](#4-attack-surface-inventory)
5. [Authentication & Authorization](#5-authentication--authorization)
6. [Cryptographic Controls](#6-cryptographic-controls)
7. [Existing Security Controls](#7-existing-security-controls)
8. [Known Vulnerabilities & Risks](#8-known-vulnerabilities--risks)
9. [Security-Critical Files](#9-security-critical-files)
10. [Environment & Secrets Management](#10-environment--secrets-management)
11. [CI/CD Security](#11-cicd-security)
12. [Infrastructure Security](#12-infrastructure-security)
13. [Pre-Audit Checklist](#13-pre-audit-checklist)
14. [Scope Recommendations](#14-scope-recommendations)

---

## 1. Executive Summary

Stellar Insights is a full-stack blockchain analytics platform for the Stellar network providing real-time payment analytics, anchor reliability scoring, corridor health metrics, and on-chain verification via Soroban smart contracts.

The platform handles:
- **Public blockchain data** from Stellar Horizon/RPC APIs
- **User credentials** via JWT-based authentication and SEP-10 (Stellar auth protocol)
- **OAuth tokens** for third-party integrations (Zapier)
- **Webhook configurations** with HMAC-signed deliveries
- **Financial analytics data** including price feeds and payment corridors
- **Smart contract interactions** for on-chain snapshot verification

### Key Risk Areas
- Authentication system has multiple auth paths (JWT, SEP-10, OAuth) with inconsistent security posture
- Hardcoded demo credentials and fallback secrets present in codebase
- WebSocket connections lack mandatory authentication
- Redis unavailability silently degrades security controls (token validation skipped)
- Request signing middleware reads unbounded request bodies

---

## 2. Architecture Overview

```
                    ┌──────────────────────────┐
                    │     Next.js Frontend      │
                    │  (React 19 / TypeScript)   │
                    └────────────┬───────────────┘
                                 │ HTTPS
                    ┌────────────▼───────────────┐
                    │    AWS ALB (Load Balancer)  │
                    └────────────┬───────────────┘
                                 │
              ┌──────────────────▼──────────────────┐
              │         Axum Backend (Rust)          │
              │  ┌─────────┐  ┌─────────┐           │
              │  │   Auth   │  │  Rate   │           │
              │  │Middleware│  │ Limiter │           │
              │  └────┬────┘  └────┬────┘           │
              │       │            │                 │
              │  ┌────▼────────────▼────┐           │
              │  │    API Handlers       │           │
              │  └──────────┬───────────┘           │
              │             │                        │
              │  ┌──────────▼───────────┐           │
              │  │   Business Services   │           │
              │  └──┬───┬───┬───┬──────┘           │
              └─────┼───┼───┼───┼──────────────────┘
                    │   │   │   │
         ┌──────────┘   │   │   └──────────┐
         │              │   │              │
    ┌────▼────┐   ┌────▼──▼────┐    ┌────▼────┐
    │ SQLite/ │   │  Stellar   │    │  Redis  │
    │PostgreSQL│   │Horizon/RPC │    │  Cache  │
    └─────────┘   └────────────┘    └─────────┘
                        │
                  ┌─────▼─────┐
                  │  Soroban   │
                  │ Contracts  │
                  └───────────┘
```

### Component Responsibilities

| Component | Purpose | Trust Level |
|-----------|---------|-------------|
| Frontend (Next.js) | Dashboard UI, user interaction | Untrusted |
| ALB | TLS termination, load balancing | Trusted (AWS-managed) |
| Backend (Axum) | API, business logic, auth | Trusted |
| SQLite/PostgreSQL | Persistent data storage | Trusted |
| Redis | Session cache, rate limiting | Semi-trusted |
| Stellar Horizon/RPC | Blockchain data source | External/Semi-trusted |
| CoinGecko API | Price feed data | External/Untrusted |
| Soroban Contracts | On-chain verification | Trusted (immutable) |
| Webhook Targets | User-configured endpoints | Untrusted |

---

## 3. Technology Stack

### Backend
| Technology | Version | Purpose |
|-----------|---------|---------|
| Rust | 1.70+ | Backend language |
| Axum | 0.7 | Web framework |
| SQLx | 0.8 | Database ORM (prepared statements) |
| jsonwebtoken | 9.0 | JWT auth |
| aes-gcm | (latest) | AES-256-GCM encryption |
| hmac + sha2 | (latest) | HMAC-SHA256 request signing |
| redis | 0.25 | Session/cache store |
| reqwest | 0.13 | HTTP client |
| tower-http | 0.6 | Middleware (CORS, compression) |

### Frontend
| Technology | Version | Purpose |
|-----------|---------|---------|
| Next.js | 16.1 | React framework |
| React | 19 | UI library |
| @stellar/stellar-sdk | 14.5 | Stellar integration |
| Prisma | 7.3 | Database ORM |

### Smart Contracts
| Technology | Version | Purpose |
|-----------|---------|---------|
| Soroban SDK | 21.0 | Smart contract framework |
| Target | WASM | Compilation target |

### Infrastructure
| Technology | Purpose |
|-----------|---------|
| Docker (multi-stage) | Containerization |
| AWS ECS (Fargate) | Container orchestration |
| AWS ALB | Load balancing |
| AWS ECR | Container registry |
| AWS CodeDeploy | Blue-green deployment |
| Terraform | Infrastructure-as-Code |
| GitHub Actions | CI/CD pipelines |
| ELK Stack | Logging/monitoring |
| HashiCorp Vault | Secrets management |

---

## 4. Attack Surface Inventory

### 4.1 HTTP API Endpoints

#### Public Endpoints (No Authentication)
| Endpoint | Method | Risk Level | Notes |
|----------|--------|------------|-------|
| `GET /health` | GET | Low | Health check, rate limited (1000/min) |
| `GET /api/anchors` | GET | Medium | Public data, 100 req/min |
| `GET /api/anchors/:id` | GET | Medium | Public data |
| `GET /api/corridors` | GET | Medium | Public data |
| `GET /api/corridors/:key` | GET | Medium | Public data |
| `GET /api/rpc/health` | GET | Low | RPC health |
| `GET /api/rpc/payments` | GET | Medium | Proxied blockchain data |
| `GET /api/rpc/trades` | GET | Medium | Proxied blockchain data |
| `GET /api/rpc/orderbook` | GET | Medium | Proxied blockchain data |
| `GET /api/prices/*` | GET | Medium | Price feed data |
| `POST /api/cost-calculator/estimate` | POST | Medium | Accepts user input |
| `GET /api/liquidity-pools` | GET | Medium | Pool data |
| `GET /api/trustlines` | GET | Medium | Trustline data |
| `GET /api/account-merges/*` | GET | Medium | Merge analytics |
| `GET /api/analytics/muxed` | GET | Medium | Muxed account data |
| `GET /api/network/*` | GET | Low | Network config |
| `GET /api/cache-stats` | GET | Low | Cache metrics |
| `GET /swagger-ui/*` | GET | Medium | OpenAPI docs exposed |

#### Authenticated Endpoints (JWT Required)
| Endpoint | Method | Risk Level | Notes |
|----------|--------|------------|-------|
| `POST /api/anchors` | POST | High | Creates anchor records |
| `PUT /api/anchors/:id/metrics` | PUT | High | Modifies anchor metrics |
| `POST /api/anchors/:id/assets` | POST | High | Creates asset records |
| `POST /api/corridors` | POST | High | Creates corridor records |
| `PUT /api/corridors/:id/metrics-from-transactions` | PUT | High | Modifies corridor metrics |
| `POST /api/webhooks` | POST | High | Registers webhook URLs |
| `GET /api/webhooks` | GET | Medium | Lists user webhooks |
| `DELETE /api/webhooks/:id` | DELETE | High | Removes webhooks |
| `POST /api/webhooks/:id/test` | POST | Medium | Triggers test delivery |

#### Authentication Endpoints
| Endpoint | Method | Risk Level | Notes |
|----------|--------|------------|-------|
| `POST /api/auth/login` | POST | Critical | Login with credentials |
| `POST /api/auth/refresh` | POST | High | Token refresh |
| `POST /api/auth/logout` | POST | Medium | Session invalidation |
| `GET /api/sep10/info` | GET | Low | SEP-10 server info |
| `POST /api/sep10/auth` | POST | High | Challenge generation |
| `POST /api/sep10/verify` | POST | Critical | Challenge verification |
| `POST /api/sep10/logout` | POST | Medium | Session invalidation |
| `POST /api/oauth/authorize` | POST | Critical | OAuth authorization |
| `POST /api/oauth/token` | POST | Critical | OAuth token exchange |

### 4.2 WebSocket Endpoint
| Endpoint | Risk Level | Notes |
|----------|------------|-------|
| `GET /ws` | High | Optional auth via query param token |

### 4.3 External Service Connections (Outbound)
| Service | Protocol | Auth | Risk |
|---------|----------|------|------|
| Stellar Horizon (mainnet) | HTTPS | None | Medium |
| Stellar RPC (mainnet) | HTTPS | None | Medium |
| CoinGecko API | HTTPS | Optional API key | Low |
| Webhook targets | HTTP/HTTPS | HMAC signature | High |
| Redis | TCP | Optional password | Medium |
| PostgreSQL | TCP | Password | Medium |
| Elasticsearch | HTTP | None (internal) | Medium |

### 4.4 Infrastructure Attack Surface
- Docker container (debian:bookworm-slim)
- AWS ECS Fargate tasks
- AWS ALB (public-facing)
- AWS ECR (container images)
- AWS RDS (PostgreSQL)
- AWS ElastiCache (Redis)
- GitHub Actions runners
- Terraform state (S3 + DynamoDB lock)

---

## 5. Authentication & Authorization

### 5.1 JWT Authentication
**Files:** `backend/src/auth.rs`, `backend/src/auth_middleware.rs`

| Property | Value | Assessment |
|----------|-------|------------|
| Algorithm | HS256 (HMAC-SHA256) | Adequate |
| Access token lifetime | 1 hour | Adequate |
| Refresh token lifetime | 7 days | Adequate |
| Token storage | Redis (refresh), stateless (access) | Acceptable |
| Secret source | `JWT_SECRET` env var | **CRITICAL: Insecure fallback** |
| User store | Hardcoded demo credentials | **CRITICAL: Must remove** |

**Findings:**
- **[CRITICAL]** `auth.rs:21-22` - Hardcoded credentials: `admin`/`password123`
- **[CRITICAL]** `auth.rs:83` - JWT secret fallback: `"your-secret-key-change-in-production"`
- **[HIGH]** `auth.rs:206` - Redis unavailability silently skips refresh token validation
- **[MEDIUM]** No token revocation list for access tokens
- **[LOW]** No JWT algorithm restriction in validation (allows algorithm confusion if misconfigured)

### 5.2 SEP-10 Authentication (Stellar)
**File:** `backend/src/auth/sep10_simple.rs`

| Property | Value | Assessment |
|----------|-------|------------|
| Challenge expiry | 5 minutes | Adequate |
| Session expiry | 7 days | Adequate |
| Nonce generation | 32 random bytes | Adequate |
| Replay protection | Single-use nonce in Redis | Adequate |
| Account validation | Format check (G* + 56 chars) | Basic |

**Findings:**
- **[MEDIUM]** Simplified implementation - does not perform actual Stellar transaction signature verification
- **[MEDIUM]** When Redis is unavailable, challenge validation is silently skipped
- **[LOW]** Session token is base64(account:random) - account is revealed in token structure

### 5.3 OAuth 2.0 (Zapier Integration)
**File:** `backend/src/auth/oauth.rs`

| Property | Value | Assessment |
|----------|-------|------------|
| Flow | Authorization Code | Standard |
| Token lifetime | 7 days (configurable) | Adequate |
| Refresh token lifetime | 30 days | Adequate |
| Client secret storage | AES-256-GCM encrypted | Good |
| Token revocation | **Stub implementation** | **CRITICAL** |

**Findings:**
- **[CRITICAL]** `oauth.rs:350-357` - Token revocation is a no-op (just logs, doesn't actually revoke)
- **[CRITICAL]** `oauth.rs:74` - Same JWT secret fallback as auth.rs
- **[CRITICAL]** `oauth.rs:89` - Encryption key falls back to all-zeros
- **[HIGH]** No PKCE (Proof Key for Code Exchange) support
- **[MEDIUM]** `Validation::default()` does not enforce audience claim in JWT validation

### 5.4 Request Signing
**File:** `backend/src/request_signing_middleware.rs`

| Property | Value | Assessment |
|----------|-------|------------|
| Algorithm | HMAC-SHA256 | Adequate |
| Replay protection | 5-minute timestamp window | Adequate |
| Body inclusion | Full body in signature | Good |

**Findings:**
- **[HIGH]** `request_signing_middleware.rs:53` - `to_bytes(body, usize::MAX)` reads entire body into memory without size limit - DoS vector
- **[HIGH]** `request_signing_middleware.rs:69-72` - Stub user identity ("stub-user-id") always inserted

### 5.5 WebSocket Authentication
**File:** `backend/src/websocket.rs`

**Findings:**
- **[HIGH]** `websocket.rs:222-232` - Authentication is optional; unauthenticated connections allowed when no token provided
- **[HIGH]** `websocket.rs:242-249` - Without `WS_AUTH_TOKEN` env var, all connections accepted
- **[MEDIUM]** Token passed via query parameter (visible in server logs, browser history)
- **[MEDIUM]** No per-connection rate limiting on subscriptions

---

## 6. Cryptographic Controls

### 6.1 Encryption at Rest
| Data | Method | Key Source | Assessment |
|------|--------|-----------|------------|
| OAuth client secrets | AES-256-GCM | `ENCRYPTION_KEY` env | Good |
| OAuth tokens (DB) | AES-256-GCM | `ENCRYPTION_KEY` env | Good |
| JWT signing | HMAC-SHA256 | `JWT_SECRET` env | **Insecure fallback** |

### 6.2 Encryption in Transit
| Channel | Method | Assessment |
|---------|--------|------------|
| Frontend-Backend | HTTPS (via ALB) | Good |
| Backend-Database | Configurable | Check production config |
| Backend-Redis | Optional TLS | Check production config |
| Backend-Stellar APIs | HTTPS | Good |
| Backend-CoinGecko | HTTPS | Good |
| Webhook deliveries | HTTP or HTTPS | **HTTP allowed** |

### 6.3 Key Management
- `ENCRYPTION_KEY`: 32-byte hex string, validated on startup
- `JWT_SECRET`: **No validation, insecure fallback exists**
- `SEP10_SERVER_PUBLIC_KEY`: Stellar keypair, validated format
- Webhook HMAC secrets: Per-webhook, generated server-side

---

## 7. Existing Security Controls

### 7.1 Controls Present
| Control | Implementation | Effectiveness |
|---------|---------------|---------------|
| Rate limiting | Dual-layer (Redis + memory fallback) | Good |
| CORS | Configurable origin allowlist | Good (with caveat) |
| SQL injection prevention | SQLx prepared statements | Strong |
| Request compression | Gzip + Brotli (>1KB) | N/A (performance) |
| Non-root Docker user | `appuser` in Dockerfile | Good |
| Credential redaction in logs | `env_config.rs` sanitization | Good |
| HMAC request signing | `request_signing_middleware.rs` | Good (with caveats) |
| Circuit breaker | RPC client failure isolation | Good |
| Graceful shutdown | Coordinated shutdown sequence | Good |
| Admin audit logging | Database-backed audit trail | Good |
| Input validation | Env config validation on startup | Basic |
| Dependency scanning | Trivy + tfsec + cargo-audit | Good |
| CodeQL analysis | GitHub Actions workflow | Good |

### 7.2 Controls Missing or Incomplete
| Control | Status | Priority |
|---------|--------|----------|
| Content Security Policy (CSP) headers | Missing | High |
| HTTP Strict Transport Security (HSTS) | Missing | High |
| X-Content-Type-Options | Missing | Medium |
| X-Frame-Options | Missing | Medium |
| Request body size limits | Missing globally | High |
| API input validation/sanitization | Minimal | High |
| Brute force protection (login) | Missing | Critical |
| Account lockout | Missing | High |
| Password complexity requirements | N/A (hardcoded) | Critical |
| Access token revocation | Missing | High |
| Session fixation protection | Missing | Medium |
| SSRF protection (webhook URLs) | Missing | High |
| IP-based access control (admin) | Missing | Medium |
| Security event logging | Partial | Medium |
| Error message information leakage | Not reviewed | Medium |

---

## 8. Known Vulnerabilities & Risks

### CRITICAL Severity

#### SEC-001: Hardcoded Demo Credentials
- **File:** `backend/src/auth.rs:19-22`
- **Description:** Admin credentials (`admin`/`password123`) are hardcoded in source code
- **Impact:** Any attacker with access to the source or who guesses common credentials gains admin access
- **Remediation:** Remove hardcoded credentials, implement database-backed user store with bcrypt/argon2 password hashing

#### SEC-002: Insecure JWT Secret Fallback
- **File:** `backend/src/auth.rs:83`, `backend/src/auth/oauth.rs:74`
- **Description:** JWT signing secret falls back to `"your-secret-key-change-in-production"` when `JWT_SECRET` env var is not set
- **Impact:** Token forgery, authentication bypass
- **Remediation:** Make `JWT_SECRET` a required env variable, fail startup if not set with sufficient entropy

#### SEC-003: Insecure Encryption Key Fallback
- **File:** `backend/src/auth/oauth.rs:89`
- **Description:** `ENCRYPTION_KEY` falls back to all-zeros (64 hex zeros) when env var missing
- **Impact:** All encrypted OAuth secrets can be trivially decrypted
- **Remediation:** Already validated as required in `env_config.rs` for main flow, but OAuth creates its own fallback. Remove the fallback.

#### SEC-004: OAuth Token Revocation is a No-Op
- **File:** `backend/src/auth/oauth.rs:350-357`
- **Description:** `revoke_token()` only logs, doesn't actually revoke the token
- **Impact:** Compromised OAuth tokens cannot be invalidated
- **Remediation:** Implement token revocation via database flag or Redis blacklist

### HIGH Severity

#### SEC-005: Unbounded Request Body Read
- **File:** `backend/src/request_signing_middleware.rs:53`
- **Description:** `axum::body::to_bytes(body, usize::MAX)` allows reading arbitrarily large request bodies into memory
- **Impact:** Denial of Service via memory exhaustion
- **Remediation:** Set reasonable max body size (e.g., 10MB)

#### SEC-006: WebSocket Authentication Optional
- **File:** `backend/src/websocket.rs:222-249`
- **Description:** WebSocket connections are accepted without authentication when no token is provided or `WS_AUTH_TOKEN` is not set
- **Impact:** Unauthorized access to real-time data feeds
- **Remediation:** Make WebSocket authentication mandatory in production

#### SEC-007: Redis Unavailability Bypasses Security Controls
- **Files:** `backend/src/auth.rs:206`, `backend/src/auth/sep10_simple.rs:262-281`
- **Description:** When Redis is down, refresh token validation and SEP-10 challenge validation silently pass
- **Impact:** Token replay attacks, authentication bypass
- **Remediation:** Fail closed when Redis is unavailable for security-critical operations

#### SEC-008: No SSRF Protection on Webhook URLs
- **File:** `backend/src/api/webhooks.rs:22-26`
- **Description:** Webhook URL validation only checks for `http://` or `https://` prefix, no SSRF protection
- **Impact:** Attacker can register internal/private IP webhooks to probe internal network
- **Remediation:** Block private IP ranges (10.x, 172.16-31.x, 192.168.x, 169.254.x, 127.x), localhost, and metadata endpoints

#### SEC-009: No Login Brute Force Protection
- **File:** `backend/src/auth.rs:92-101`
- **Description:** No rate limiting, lockout, or delay on failed login attempts
- **Impact:** Credential stuffing and brute force attacks
- **Remediation:** Implement per-IP and per-account rate limiting on login endpoint

#### SEC-010: Missing Security Headers
- **File:** `backend/src/main.rs`
- **Description:** No security headers middleware (CSP, HSTS, X-Content-Type-Options, X-Frame-Options)
- **Impact:** XSS, clickjacking, MIME sniffing attacks
- **Remediation:** Add security headers middleware

#### SEC-011: CORS Falls Back to Allow-All
- **File:** `backend/src/main.rs:612-621`
- **Description:** If no valid CORS origins are parsed, falls back to `allow_origin(Any)`
- **Impact:** Any website can make authenticated requests to the API
- **Remediation:** Fail startup instead of falling back to permissive CORS

### MEDIUM Severity

#### SEC-012: Swagger UI Exposed in Production
- **File:** `backend/src/main.rs:841-842`
- **Description:** `/swagger-ui` endpoint is always available, not gated by environment
- **Impact:** Exposes complete API documentation to attackers
- **Remediation:** Disable Swagger UI in production builds

#### SEC-013: Stub User in Request Signing Middleware
- **File:** `backend/src/request_signing_middleware.rs:69-72`
- **Description:** Always inserts `"stub-user-id"` / `"stub-username"` regardless of actual identity
- **Impact:** All signed requests appear from the same user, breaking audit trails
- **Remediation:** Integrate with actual auth system or remove stub

#### SEC-014: WebSocket Token in Query Parameter
- **File:** `backend/src/websocket.rs:210-213`
- **Description:** Auth token passed as URL query parameter
- **Impact:** Token visible in server access logs, proxy logs, browser history
- **Remediation:** Use WebSocket subprotocol or first-message auth pattern

#### SEC-015: Webhook HTTP Delivery Allowed
- **File:** `backend/src/api/webhooks.rs:22`
- **Description:** Both HTTP and HTTPS webhook URLs are accepted
- **Impact:** Webhook payloads (including HMAC secrets context) sent in cleartext
- **Remediation:** Require HTTPS for webhook URLs in production

#### SEC-016: Database URL Logged at Startup
- **File:** `backend/src/main.rs:92`
- **Description:** Raw `DATABASE_URL` logged before sanitization function is called
- **Impact:** Database credentials may appear in logs
- **Remediation:** Use sanitized URL for logging

### LOW Severity

#### SEC-017: OAuth Token Logging Leaks Token Prefix
- **File:** `backend/src/auth/oauth.rs:354`
- **Description:** First 20 chars of token logged during revocation
- **Impact:** Partial token exposure in logs
- **Remediation:** Log token hash instead of prefix

#### SEC-018: SEP-10 Session Token Reveals Account
- **File:** `backend/src/auth/sep10_simple.rs:246-247`
- **Description:** Session token format is `base64(account:random)`, leaking account in token
- **Impact:** Information disclosure
- **Remediation:** Use opaque random token

---

## 9. Security-Critical Files

### Must Review (Priority 1)
| File | Description |
|------|-------------|
| `backend/src/auth.rs` | JWT auth service, credential validation |
| `backend/src/auth_middleware.rs` | JWT validation middleware |
| `backend/src/auth/oauth.rs` | OAuth 2.0 service |
| `backend/src/auth/sep10_simple.rs` | SEP-10 auth implementation |
| `backend/src/auth/sep10_middleware.rs` | SEP-10 auth middleware |
| `backend/src/request_signing_middleware.rs` | HMAC request verification |
| `backend/src/crypto.rs` | AES-256-GCM encryption |
| `backend/src/rate_limit.rs` | Rate limiting |
| `backend/src/websocket.rs` | WebSocket handler |
| `backend/src/main.rs` | Application bootstrap, middleware chain |

### Should Review (Priority 2)
| File | Description |
|------|-------------|
| `backend/src/api/webhooks.rs` | Webhook registration |
| `backend/src/services/webhook_dispatcher.rs` | Outbound webhook delivery |
| `backend/src/database.rs` | Database queries |
| `backend/src/handlers.rs` | Core API handlers |
| `backend/src/env_config.rs` | Environment validation |
| `backend/src/cache.rs` | Cache layer |
| `backend/src/api/auth.rs` | Auth API routes |
| `backend/src/api/sep10.rs` | SEP-10 API routes |
| `backend/src/vault/` | Vault integration |

### Infrastructure Review (Priority 3)
| File | Description |
|------|-------------|
| `backend/Dockerfile` | Container build |
| `.github/workflows/deploy.yml` | CI/CD pipeline |
| `.github/workflows/security-scan.yml` | Security scanning |
| `terraform/modules/networking/security_groups.tf` | Network security |
| `terraform/modules/database/main.tf` | Database config |
| `docker-compose.elk.prod.yml` | Production logging |

### Smart Contract Review (Priority 4)
| File | Description |
|------|-------------|
| `contracts/stellar_insights/src/lib.rs` | Main contract |
| `contracts/snapshot-contract/` | Snapshot verification |

---

## 10. Environment & Secrets Management

### Required Secrets (Production)
| Variable | Purpose | Rotation Policy |
|----------|---------|----------------|
| `DATABASE_URL` | PostgreSQL connection string | On compromise |
| `ENCRYPTION_KEY` | AES-256-GCM key (32-byte hex) | Annually |
| `JWT_SECRET` | JWT signing key | Quarterly |
| `REDIS_URL` | Redis connection (may include password) | On compromise |
| `SEP10_SERVER_PUBLIC_KEY` | Stellar keypair | On compromise |
| `PRICE_FEED_API_KEY` | CoinGecko API key | Annually |
| `WS_AUTH_TOKEN` | WebSocket auth token | Quarterly |

### Secrets Distribution
- **Development:** `.env` file (gitignored)
- **CI/CD:** GitHub Actions secrets
- **Production:** HashiCorp Vault (via `vault/` module) + AWS Secrets Manager
- **Infrastructure:** Terraform variables + AWS SSM Parameter Store

### Concerns
- No `.env` file committed (verified - `.env.example` only)
- Vault integration exists but usage may not be enforced
- Multiple fallback values create risk of running with insecure defaults

---

## 11. CI/CD Security

### Existing Workflows
| Workflow | Purpose | Frequency |
|----------|---------|-----------|
| `security-scan.yml` | Trivy + tfsec | PR + nightly |
| `security-audit.yml` | cargo-audit + npm audit | PR |
| `codeql-analysis.yml` | CodeQL static analysis | PR |
| `clippy.yml` | Rust linting | PR |
| `deploy.yml` | Blue-green deployment | Push to main |

### CI/CD Security Concerns
- GitHub Actions uses OIDC federation for AWS (good)
- Actions versions should be pinned to SHA (some use @v4 tags)
- Trivy action uses v0.9.1 (check if latest)
- No SAST tool for Rust-specific vulnerabilities beyond Clippy
- No DAST (Dynamic Application Security Testing) pipeline
- No container image scanning of built images
- Deployment to production triggers on push to main (no manual gate)

---

## 12. Infrastructure Security

### AWS Security Configuration
| Resource | Security Control | Status |
|----------|-----------------|--------|
| ECS Tasks | Fargate (no host access) | Good |
| ALB | HTTPS listener | Verify TLS 1.2+ |
| RDS | Private subnet, encryption | Verify |
| ElastiCache | Private subnet, auth | Verify |
| ECR | Image scanning | Enable if not |
| VPC | Security groups, NACLs | Review rules |
| S3 (Terraform state) | Encryption, versioning | Verify |
| IAM | Least privilege | Review policies |

### Docker Security
- Multi-stage build (no build tools in runtime)
- Non-root user (`appuser`)
- Health check configured
- `curl` installed in runtime image (potential concern but needed for healthcheck)
- No read-only filesystem enforced
- No security scanning of base image (debian:bookworm-slim)

---

## 13. Pre-Audit Checklist

### Before Audit Begins
- [ ] Provide auditors with read access to GitHub repository
- [ ] Provide access to staging environment for dynamic testing
- [ ] Share this document and THREAT_MODEL.md
- [ ] Share DATA_FLOW_SECURITY.md
- [ ] Provide architecture diagrams
- [ ] Set up dedicated test accounts (non-production)
- [ ] Document all API endpoints with Swagger/OpenAPI export
- [ ] Provide Terraform state review access (read-only)
- [ ] Share dependency audit results (cargo audit + npm audit output)
- [ ] Confirm smart contract deployment addresses (testnet)

### During Audit
- [ ] Assign internal security champion as point of contact
- [ ] Provide access to logging infrastructure (ELK/Kibana)
- [ ] Be available for Q&A sessions
- [ ] Track findings in shared issue tracker

### After Audit
- [ ] Review and triage all findings
- [ ] Create remediation plan with timelines
- [ ] Address critical/high findings before next deployment
- [ ] Schedule retest for remediated items
- [ ] Update threat model based on findings

---

## 14. Scope Recommendations

### Recommended Audit Scope

#### Tier 1 - Critical (Must Test)
1. Authentication flows (JWT, SEP-10, OAuth)
2. Authorization enforcement on all endpoints
3. Input validation and injection testing
4. Cryptographic implementation review
5. Session management
6. Secrets handling and configuration

#### Tier 2 - High Priority
1. WebSocket security
2. Rate limiting effectiveness
3. CORS configuration
4. Webhook SSRF testing
5. API business logic flaws
6. Error handling and information disclosure

#### Tier 3 - Standard
1. Docker container security
2. Infrastructure (Terraform) review
3. CI/CD pipeline security
4. Dependency vulnerability assessment
5. Smart contract review (Soroban)
6. Frontend XSS and client-side security

### Out of Scope (Suggested)
- Physical security
- Social engineering
- Stellar network/protocol security (third-party)
- AWS cloud infrastructure beyond configured resources
- Third-party service security (CoinGecko, Horizon)
