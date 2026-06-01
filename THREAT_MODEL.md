# Threat Model - Stellar Insights Platform

**Methodology:** STRIDE + DREAD Risk Rating
**Date:** 2026-02-22
**Classification:** Confidential

---

## Table of Contents

1. [Threat Actors](#1-threat-actors)
2. [Assets Under Protection](#2-assets-under-protection)
3. [Trust Boundaries](#3-trust-boundaries)
4. [STRIDE Analysis by Component](#4-stride-analysis-by-component)
5. [Attack Trees](#5-attack-trees)
6. [Risk Matrix](#6-risk-matrix)
7. [Mitigations Summary](#7-mitigations-summary)

---

## 1. Threat Actors

| Actor | Motivation | Capability | Access Level |
|-------|-----------|------------|-------------|
| **Opportunistic Attacker** | Data theft, defacement | Low-Medium | External, unauthenticated |
| **Sophisticated Attacker** | Financial gain, manipulation of analytics | High | External, may obtain credentials |
| **Malicious Insider** | Data exfiltration, sabotage | High | Internal, authenticated |
| **Competitor** | Business intelligence, disruption | Medium | External |
| **Automated Bot** | Credential stuffing, scraping, DDoS | Medium | External, unauthenticated |
| **Supply Chain Attacker** | Backdoor insertion via dependencies | High | Indirect (dependency) |
| **Compromised Webhook Consumer** | Data interception, SSRF pivot | Medium | Registered user |

---

## 2. Assets Under Protection

### Critical Assets
| Asset | Classification | Storage | Impact if Compromised |
|-------|---------------|---------|----------------------|
| JWT signing secret | Secret | Environment variable | Full auth bypass |
| Encryption key | Secret | Environment variable | All encrypted data exposed |
| Database credentials | Secret | Environment variable | Full data breach |
| OAuth client secrets | Confidential | DB (encrypted) | Third-party impersonation |
| Refresh tokens | Confidential | Redis | Session hijacking |
| User credentials | Confidential | Hardcoded (CRITICAL) | Account takeover |
| SEP-10 server keypair | Secret | Environment variable | Auth challenge forgery |

### Important Assets
| Asset | Classification | Storage | Impact if Compromised |
|-------|---------------|---------|----------------------|
| Anchor reliability scores | Business-sensitive | Database | Market manipulation |
| Corridor health metrics | Business-sensitive | Database | Misleading analytics |
| Payment transaction data | Business-sensitive | Database | Privacy violation |
| Price feed data | Public (cached) | Redis/memory | Incorrect calculations |
| Webhook configurations | User-private | Database | Data redirection |
| Liquidity pool analytics | Business-sensitive | Database | Trading advantage |

### Infrastructure Assets
| Asset | Classification | Impact if Compromised |
|-------|---------------|----------------------|
| Terraform state | Secret | Infrastructure takeover |
| Docker images | Internal | Supply chain attack |
| CI/CD pipeline | Internal | Code injection |
| AWS credentials (OIDC) | Secret | Cloud account compromise |
| Vault tokens | Secret | All secrets exposed |

---

## 3. Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    UNTRUSTED ZONE                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────────┐  │
│  │ Browser  │  │  Zapier  │  │ Webhook Consumer Servers │  │
│  │  Client  │  │  Client  │  │                          │  │
│  └────┬─────┘  └────┬─────┘  └──────────────────────────┘  │
│       │              │                                       │
├───────┼──────────────┼───────────────────────────────────────┤
│       │   BOUNDARY 1: TLS Termination (ALB)                 │
│       │              │                                       │
│  ┌────▼──────────────▼────────────────────────────────────┐ │
│  │              SEMI-TRUSTED ZONE                          │ │
│  │                                                         │ │
│  │  ┌──────────────────────────────────────────────────┐  │ │
│  │  │            Application Server (Axum)             │  │ │
│  │  │                                                  │  │ │
│  │  │  ┌─────────────┐  ┌──────────────────────────┐  │  │ │
│  │  │  │ Unauthed    │  │ Authenticated Routes     │  │  │ │
│  │  │  │ Public API  │  │ (JWT/SEP-10/OAuth)       │  │  │ │
│  │  │  └─────┬───────┘  └──────────┬───────────────┘  │  │ │
│  │  │        │                     │                   │  │ │
│  │  │  ┌─────▼─────────────────────▼───────────────┐  │  │ │
│  │  │  │         Business Logic Layer              │  │  │ │
│  │  │  └────────┬──────────┬──────────┬────────────┘  │  │ │
│  │  └───────────┼──────────┼──────────┼───────────────┘  │ │
│  │              │          │          │                    │ │
│  ├──────────────┼──────────┼──────────┼────────────────────┤ │
│  │   BOUNDARY 2: Data Store Access                        │ │
│  │              │          │          │                    │ │
│  │  ┌───────────▼──┐  ┌───▼───┐  ┌──▼─────────────────┐ │ │
│  │  │   Database   │  │ Redis │  │ External Services  │ │ │
│  │  │  (SQLite/PG) │  │       │  │ Stellar/CoinGecko  │ │ │
│  │  └──────────────┘  └───────┘  └────────────────────┘ │ │
│  │              TRUSTED ZONE                              │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Trust Boundary Crossings

| ID | Boundary | Data Crossing | Direction | Controls |
|----|----------|--------------|-----------|----------|
| TB-1 | Client → ALB | HTTP requests | Inbound | TLS, CORS |
| TB-2 | ALB → Backend | HTTP requests | Inbound | Rate limiting |
| TB-3 | Backend → Database | SQL queries | Outbound | Prepared statements |
| TB-4 | Backend → Redis | Key-value ops | Bidirectional | Network isolation |
| TB-5 | Backend → Stellar APIs | HTTPS requests | Outbound | Circuit breaker |
| TB-6 | Backend → CoinGecko | HTTPS requests | Outbound | Rate limiting |
| TB-7 | Backend → Webhook targets | HTTPS/HTTP | Outbound | HMAC signing |
| TB-8 | Client → WebSocket | WS messages | Bidirectional | Optional auth |

---

## 4. STRIDE Analysis by Component

### 4.1 Authentication System

#### Spoofing
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| S-AUTH-1 | Attacker guesses hardcoded credentials (admin/password123) | **Very High** | Critical | **Critical** |
| S-AUTH-2 | Token forgery using known default JWT secret | **High** | Critical | **Critical** |
| S-AUTH-3 | Credential stuffing on login endpoint (no rate limit) | High | High | **High** |
| S-AUTH-4 | Session hijacking via token theft from logs | Medium | High | **High** |
| S-AUTH-5 | SEP-10 challenge bypass when Redis is down | Medium | High | **High** |

#### Tampering
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| T-AUTH-1 | JWT claims modification (if secret compromised) | Medium | Critical | **High** |
| T-AUTH-2 | Refresh token replay when Redis unavailable | Medium | High | **High** |
| T-AUTH-3 | OAuth token cannot be revoked (no-op revocation) | High | High | **High** |

#### Repudiation
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| R-AUTH-1 | No login attempt logging for failed attempts | Medium | Medium | **Medium** |
| R-AUTH-2 | Stub user ID in request signing breaks audit trail | High | Medium | **Medium** |

#### Information Disclosure
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| I-AUTH-1 | Token prefix logged in OAuth revocation | Low | Low | **Low** |
| I-AUTH-2 | Account ID leaked in SEP-10 session token format | Low | Low | **Low** |
| I-AUTH-3 | JWT secret visible in source code (fallback string) | Medium | Critical | **High** |

#### Denial of Service
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| D-AUTH-1 | Login endpoint brute force (no rate limiting) | High | Medium | **High** |
| D-AUTH-2 | Token refresh flood | Medium | Medium | **Medium** |

#### Elevation of Privilege
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| E-AUTH-1 | Horizontal access: webhook ownership not checked on all ops | Low | Medium | **Medium** |
| E-AUTH-2 | No role-based access control (single role: authenticated) | Medium | High | **High** |

---

### 4.2 API Layer

#### Spoofing
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| S-API-1 | Unauthenticated access to sensitive analytics | Medium | Medium | **Medium** |
| S-API-2 | Cross-origin request via CORS misconfiguration | Medium | High | **High** |

#### Tampering
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| T-API-1 | SQL injection (mitigated by SQLx prepared statements) | Low | Critical | **Medium** |
| T-API-2 | Malicious anchor/corridor creation via authenticated API | Medium | High | **High** |
| T-API-3 | Metrics manipulation via PUT endpoints | Medium | High | **High** |

#### Information Disclosure
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| I-API-1 | Swagger UI exposes full API documentation | Medium | Medium | **Medium** |
| I-API-2 | Verbose error messages reveal internal state | Medium | Medium | **Medium** |
| I-API-3 | Database URL logged before sanitization | Low | Medium | **Low** |
| I-API-4 | Cache stats endpoint reveals infrastructure details | Low | Low | **Low** |

#### Denial of Service
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| D-API-1 | Memory exhaustion via unbounded body read | High | High | **High** |
| D-API-2 | Rate limit bypass via IP spoofing behind proxy | Medium | Medium | **Medium** |
| D-API-3 | Rate limiter memory store grows unbounded (no cleanup) | Medium | Medium | **Medium** |

---

### 4.3 WebSocket Service

#### Spoofing
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| S-WS-1 | Unauthenticated WebSocket connections | **High** | High | **High** |
| S-WS-2 | Token in URL query params logged by intermediaries | Medium | Medium | **Medium** |

#### Tampering
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| T-WS-1 | Malicious subscribe to arbitrary channels | Medium | Low | **Low** |
| T-WS-2 | Message injection via WebSocket | Low | Medium | **Low** |

#### Denial of Service
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| D-WS-1 | Connection exhaustion (no connection limit) | High | High | **High** |
| D-WS-2 | Subscription flooding (unlimited subscriptions) | Medium | Medium | **Medium** |
| D-WS-3 | Broadcast channel saturation (channel capacity: 100) | Medium | Medium | **Medium** |

---

### 4.4 Webhook System

#### Spoofing
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| S-WH-1 | Attacker registers webhook for internal URLs (SSRF) | High | High | **High** |

#### Tampering
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| T-WH-1 | HMAC-signed payload modified in transit (HTTP allowed) | Medium | Medium | **Medium** |

#### Information Disclosure
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| I-WH-1 | Webhook payloads sent over HTTP (cleartext) | Medium | Medium | **Medium** |
| I-WH-2 | Internal network topology discovered via SSRF | Medium | High | **High** |

---

### 4.5 External Service Integrations

#### Spoofing
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| S-EXT-1 | DNS hijacking of Stellar API endpoints | Low | High | **Medium** |
| S-EXT-2 | Man-in-the-middle on CoinGecko price feed | Low | High | **Medium** |

#### Tampering
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| T-EXT-1 | Manipulated price data from compromised feed | Low | High | **Medium** |
| T-EXT-2 | Manipulated blockchain data from compromised RPC | Low | High | **Medium** |

#### Denial of Service
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| D-EXT-1 | Stellar API outage causes cascade failure | Medium | High | **High** |
| D-EXT-2 | CoinGecko rate limiting causes price feed gaps | Medium | Medium | **Medium** |

---

### 4.6 Infrastructure & Deployment

#### Spoofing
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| S-INF-1 | Compromised CI/CD pipeline deploys malicious code | Low | Critical | **High** |
| S-INF-2 | Container image tampering in ECR | Low | Critical | **Medium** |

#### Tampering
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| T-INF-1 | Terraform state manipulation | Low | Critical | **Medium** |
| T-INF-2 | Supply chain attack via compromised dependency | Medium | Critical | **High** |

#### Information Disclosure
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| I-INF-1 | Secrets leaked in CI/CD logs | Low | Critical | **Medium** |
| I-INF-2 | ELK stack accessible without authentication | Medium | High | **High** |

#### Denial of Service
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| D-INF-1 | DDoS on public ALB endpoint | Medium | High | **High** |
| D-INF-2 | Resource exhaustion in ECS task | Medium | Medium | **Medium** |

---

### 4.7 Smart Contracts (Soroban)

#### Tampering
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| T-SC-1 | Unauthorized snapshot anchoring on-chain | Low | Medium | **Low** |
| T-SC-2 | Data integrity compromise in analytics contract | Low | Medium | **Low** |

#### Denial of Service
| ID | Threat | Likelihood | Impact | Risk |
|----|--------|-----------|--------|------|
| D-SC-1 | Contract resource exhaustion (Soroban limits mitigate) | Low | Low | **Low** |

---

## 5. Attack Trees

### Attack Tree 1: Authentication Bypass

```
Goal: Gain Authenticated Access
├── 1. Exploit Hardcoded Credentials [CRITICAL]
│   └── Login as admin/password123
│
├── 2. Forge JWT Token [CRITICAL if default secret used]
│   ├── 2a. Use default JWT secret from source code
│   └── 2b. Brute-force weak JWT secret
│
├── 3. Bypass Token Validation
│   ├── 3a. Wait for Redis outage → refresh token validation skipped
│   └── 3b. Replay expired refresh token during Redis downtime
│
├── 4. Steal Valid Token
│   ├── 4a. Extract from WebSocket URL query parameter logs
│   ├── 4b. Extract from OAuth token logged during revocation
│   └── 4c. XSS in frontend → steal from browser storage
│
├── 5. OAuth Token Abuse
│   ├── 5a. Use OAuth token after "revocation" (no-op)
│   └── 5b. Decrypt OAuth secrets with default encryption key
│
└── 6. SEP-10 Bypass
    ├── 6a. Replay challenge during Redis downtime
    └── 6b. Forge challenge (simplified impl doesn't verify signatures)
```

### Attack Tree 2: Data Manipulation

```
Goal: Manipulate Analytics Data
├── 1. Authenticated API Abuse
│   ├── 1a. Create fake anchors with inflated metrics
│   ├── 1b. Modify corridor health scores
│   └── 1c. Insert false payment records
│
├── 2. External Data Poisoning
│   ├── 2a. Compromise CoinGecko response (MITM unlikely - HTTPS)
│   └── 2b. Feed manipulated data via Stellar RPC (MITM unlikely)
│
└── 3. Direct Database Access
    ├── 3a. SQL injection (mitigated by prepared statements)
    └── 3b. Database credential theft → direct DB manipulation
```

### Attack Tree 3: Denial of Service

```
Goal: Disrupt Service Availability
├── 1. Application Layer
│   ├── 1a. Send large request bodies (unbounded read)
│   ├── 1b. Exhaust WebSocket connections (no limit)
│   ├── 1c. Brute force login endpoint (no rate limit)
│   └── 1d. Fill memory rate limiter store (no cleanup)
│
├── 2. Infrastructure Layer
│   ├── 2a. DDoS on ALB
│   ├── 2b. Exhaust ECS task resources
│   └── 2c. Redis memory exhaustion (rate limit + session keys)
│
└── 3. Dependency Disruption
    ├── 3a. Stellar API outage (circuit breaker mitigates)
    └── 3b. CoinGecko API outage (stale cache mitigates)
```

### Attack Tree 4: SSRF via Webhooks

```
Goal: Access Internal Resources via Webhook SSRF
├── 1. Register webhook with internal URL
│   ├── 1a. Target: http://169.254.169.254 (AWS metadata)
│   │   └── Extract IAM role credentials
│   ├── 1b. Target: http://localhost:9200 (Elasticsearch)
│   │   └── Read/modify log data
│   ├── 1c. Target: http://localhost:6379 (Redis)
│   │   └── Interact with Redis (if HTTP-accessible)
│   └── 1d. Target: http://internal-service:port
│       └── Scan internal network
│
└── 2. Trigger webhook delivery
    ├── 2a. Via test endpoint POST /api/webhooks/:id/test
    └── 2b. Via natural event triggering
```

---

## 6. Risk Matrix

### Risk Heat Map

|                | Low Impact | Medium Impact | High Impact | Critical Impact |
|----------------|-----------|---------------|-------------|-----------------|
| **Very Likely** |           |               |             | SEC-001, SEC-002 |
| **Likely**      |           | D-AUTH-1      | SEC-005, SEC-006, SEC-009 | SEC-003, SEC-004 |
| **Possible**    |           | SEC-012, SEC-014 | SEC-007, SEC-008, SEC-011 | |
| **Unlikely**    | SEC-017, SEC-018 | SEC-015 | T-INF-2 | |
| **Rare**        |           |               | S-INF-1 | |

### Top 10 Risks by Priority

| Rank | ID | Risk Description | Severity | Status |
|------|-----|-----------------|----------|--------|
| 1 | SEC-001 | Hardcoded admin credentials | Critical | **Open** |
| 2 | SEC-002 | Default JWT signing secret | Critical | **Open** |
| 3 | SEC-003 | Default encryption key in OAuth | Critical | **Open** |
| 4 | SEC-004 | OAuth token revocation is no-op | Critical | **Open** |
| 5 | SEC-005 | Unbounded request body read (DoS) | High | **Open** |
| 6 | SEC-009 | No login brute force protection | High | **Open** |
| 7 | SEC-008 | SSRF via webhook registration | High | **Open** |
| 8 | SEC-006 | WebSocket auth not enforced | High | **Open** |
| 9 | SEC-007 | Redis failure bypasses auth controls | High | **Open** |
| 10 | SEC-010 | Missing security response headers | High | **Open** |

---

## 7. Mitigations Summary

### Immediate Actions Required (Before Audit)

| ID | Action | Component | Effort |
|----|--------|-----------|--------|
| M-1 | Remove hardcoded credentials, implement proper user store | `auth.rs` | 2-4 hours |
| M-2 | Make JWT_SECRET required, fail on startup if missing | `auth.rs`, `oauth.rs` | 1 hour |
| M-3 | Remove encryption key fallback in OAuth | `oauth.rs` | 30 min |
| M-4 | Implement token revocation (Redis blacklist) | `oauth.rs` | 2-3 hours |
| M-5 | Add max body size limit to request signing middleware | `request_signing_middleware.rs` | 30 min |
| M-6 | Add login rate limiting | `auth.rs`, `main.rs` | 1-2 hours |
| M-7 | Block private IPs in webhook URL validation | `webhooks.rs` | 1-2 hours |
| M-8 | Make WebSocket auth mandatory (configurable) | `websocket.rs` | 1 hour |

### Short-Term Improvements (During/After Audit)

| ID | Action | Component | Effort |
|----|--------|-----------|--------|
| M-9 | Add security headers middleware (CSP, HSTS, etc.) | `main.rs` | 2-3 hours |
| M-10 | Fail closed when Redis unavailable for auth operations | `auth.rs`, `sep10_simple.rs` | 2 hours |
| M-11 | Disable Swagger UI in production | `main.rs` | 30 min |
| M-12 | Add WebSocket connection limits | `websocket.rs` | 1-2 hours |
| M-13 | CORS: fail startup instead of allow-all fallback | `main.rs` | 30 min |
| M-14 | Add RBAC (role-based access control) | Multiple | 1-2 days |
| M-15 | Require HTTPS for webhook URLs | `webhooks.rs` | 30 min |

### Long-Term Architectural Improvements

| ID | Action | Component | Effort |
|----|--------|-----------|--------|
| M-16 | Full SEP-10 implementation with XDR signature verification | `sep10_simple.rs` | 1-2 weeks |
| M-17 | PKCE support for OAuth | `oauth.rs` | 2-3 days |
| M-18 | Implement DAST in CI/CD | `.github/workflows/` | 1-2 days |
| M-19 | Container image scanning in CI/CD | `.github/workflows/` | 1 day |
| M-20 | Pin GitHub Actions to SHA hashes | `.github/workflows/` | 2-3 hours |
| M-21 | Add manual approval gate for production deployments | `.github/workflows/deploy.yml` | 1-2 hours |
