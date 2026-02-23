# Data Flow & Trust Boundary Analysis

**Project:** Stellar Insights
**Date:** 2026-02-22
**Classification:** Confidential

---

## 1. Data Classification

### Classification Levels

| Level | Description | Examples |
|-------|-------------|---------|
| **Secret** | Compromise causes severe damage | Encryption keys, JWT secrets, DB credentials, Vault tokens |
| **Confidential** | Restricted to authorized parties | User credentials, OAuth tokens, webhook secrets, session data |
| **Internal** | Not public, limited business impact | Analytics algorithms, infrastructure config, audit logs |
| **Public** | Publicly available data | Blockchain data, published metrics, API documentation |

### Data Inventory

| Data Element | Classification | At Rest | In Transit | Retention |
|-------------|---------------|---------|------------|-----------|
| JWT signing key | Secret | Env var / Vault | N/A | Until rotated |
| AES-256 encryption key | Secret | Env var / Vault | N/A | Until rotated |
| Database credentials | Secret | Env var / Vault | TLS (internal) | Until rotated |
| SEP-10 server keypair | Secret | Env var / Vault | N/A | Until rotated |
| User passwords | Confidential | **Hardcoded** | TLS | N/A |
| JWT access tokens | Confidential | Client-side | TLS | 1 hour (stateless) |
| JWT refresh tokens | Confidential | Redis | TLS | 7 days |
| OAuth client secrets | Confidential | DB (AES-256-GCM) | TLS | Until revoked |
| OAuth access tokens | Confidential | DB (AES-256-GCM) | TLS | 7 days |
| SEP-10 session tokens | Confidential | Redis | TLS | 7 days |
| Webhook HMAC secrets | Confidential | DB | TLS | Until webhook deleted |
| Webhook payload data | Internal | N/A | TLS/HTTP | Transient |
| Anchor metrics | Internal | SQLite/PostgreSQL | TLS | Indefinite |
| Corridor analytics | Internal | SQLite/PostgreSQL | TLS | Indefinite |
| Payment records | Internal | SQLite/PostgreSQL | TLS | Indefinite |
| Price feed data | Public | Redis (cached) | HTTPS | 15 min cache |
| Blockchain data | Public | SQLite/PostgreSQL | HTTPS | Indefinite |
| Admin audit logs | Internal | SQLite/PostgreSQL | N/A | Indefinite |

---

## 2. Data Flow Diagrams

### 2.1 Authentication Flow (JWT)

```
┌──────────┐         ┌─────────┐        ┌──────────┐
│  Client  │         │ Backend │        │  Redis   │
└────┬─────┘         └────┬────┘        └────┬─────┘
     │                     │                  │
     │ POST /api/auth/login│                  │
     │ {username, password}│                  │
     │────────────────────>│                  │
     │                     │                  │
     │              [Validate credentials]    │
     │              [RISK: Hardcoded creds]   │
     │                     │                  │
     │              [Generate JWT tokens]     │
     │              [RISK: Default secret]    │
     │                     │                  │
     │                     │ SET refresh_token│
     │                     │────────────────>│
     │                     │                  │
     │ {access_token,      │                  │
     │  refresh_token}     │                  │
     │<────────────────────│                  │
     │                     │                  │
     │ GET /api/protected  │                  │
     │ Authorization:Bearer│                  │
     │────────────────────>│                  │
     │                     │                  │
     │              [Validate JWT]            │
     │              [Check expiry]            │
     │              [Check token_type]        │
     │                     │                  │
     │ {response data}     │                  │
     │<────────────────────│                  │
```

**Security Controls:**
- TLS encryption in transit (via ALB)
- JWT signature validation (HS256)
- Token type verification (access vs refresh)
- Token expiry enforcement

**Gaps:**
- No TLS between ALB and backend (internal network)
- Hardcoded fallback credentials
- No brute force protection
- No account lockout

### 2.2 SEP-10 Authentication Flow

```
┌──────────┐         ┌─────────┐        ┌──────────┐
│  Client  │         │ Backend │        │  Redis   │
│ (Stellar │         │         │        │          │
│  Wallet) │         │         │        │          │
└────┬─────┘         └────┬────┘        └────┬─────┘
     │                     │                  │
     │ POST /api/sep10/auth│                  │
     │ {account, domain}   │                  │
     │────────────────────>│                  │
     │                     │                  │
     │              [Validate account format] │
     │              [Generate nonce (32 bytes)]│
     │              [Create challenge JSON]   │
     │                     │                  │
     │                     │ SET challenge     │
     │                     │ (5 min TTL)      │
     │                     │────────────────>│
     │                     │                  │
     │ {transaction: b64,  │                  │
     │  network_passphrase}│                  │
     │<────────────────────│                  │
     │                     │                  │
     │ [Client signs       │                  │
     │  challenge with     │                  │
     │  Stellar key]       │                  │
     │                     │                  │
     │ POST /sep10/verify  │                  │
     │ {transaction: b64}  │                  │
     │────────────────────>│                  │
     │                     │                  │
     │              [Decode challenge]        │
     │              [Check expiry]            │
     │              [RISK: No sig verify]     │
     │                     │                  │
     │                     │ GET+DEL nonce    │
     │                     │────────────────>│
     │                     │                  │
     │                     │ [RISK: If Redis  │
     │                     │  down, validation│
     │                     │  skipped]        │
     │                     │                  │
     │                     │ SET session      │
     │                     │ (7 day TTL)      │
     │                     │────────────────>│
     │                     │                  │
     │ {token, expires_in} │                  │
     │<────────────────────│                  │
```

**Security Controls:**
- Challenge-response protocol
- Nonce-based replay protection
- Challenge expiry (5 minutes)
- Session expiry (7 days)

**Gaps:**
- Simplified implementation - no actual Stellar signature verification
- Redis unavailability silently skips nonce validation
- Session token reveals account address

### 2.3 Webhook Data Flow

```
┌──────────┐     ┌─────────┐     ┌────────┐     ┌──────────────┐
│  Client  │     │ Backend │     │   DB   │     │ Webhook      │
│          │     │         │     │        │     │ Target (User)│
└────┬─────┘     └────┬────┘     └───┬────┘     └──────┬───────┘
     │                 │              │                  │
     │ POST /webhooks  │              │                  │
     │ {url, events}   │              │                  │
     │────────────────>│              │                  │
     │                 │              │                  │
     │          [Validate URL]        │                  │
     │          [RISK: No SSRF       │                  │
     │           protection]          │                  │
     │                 │              │                  │
     │                 │ INSERT       │                  │
     │                 │ webhook      │                  │
     │                 │─────────────>│                  │
     │                 │              │                  │
     │ {webhook_id}    │              │                  │
     │<────────────────│              │                  │
     │                 │              │                  │
     │        [Event occurs]          │                  │
     │                 │              │                  │
     │                 │ GET webhooks │                  │
     │                 │ for event    │                  │
     │                 │<────────────>│                  │
     │                 │              │                  │
     │          [Generate HMAC-SHA256]│                  │
     │          [Sign payload]        │                  │
     │                 │              │                  │
     │                 │ POST payload │                  │
     │                 │ X-Signature  │                  │
     │                 │ [RISK: HTTP  │                  │
     │                 │  allowed]    │                  │
     │                 │─────────────────────────────────>│
     │                 │              │                  │
     │                 │ UPDATE status│                  │
     │                 │─────────────>│                  │
```

**Security Controls:**
- JWT authentication required for registration
- Ownership verification on operations
- HMAC-SHA256 signed payloads
- Retry logic (3 attempts)

**Gaps:**
- No SSRF protection (private IPs, metadata endpoints allowed)
- HTTP delivery allowed (HMAC protects integrity but not confidentiality)
- No webhook URL validation beyond protocol prefix

### 2.4 Price Feed Data Flow

```
┌───────────┐         ┌─────────┐       ┌─────────┐       ┌──────────┐
│ CoinGecko │         │ Backend │       │  Redis  │       │  Client  │
│   API     │         │         │       │  Cache  │       │          │
└─────┬─────┘         └────┬────┘       └────┬────┘       └────┬─────┘
      │                     │                 │                  │
      │                     │                 │   GET /api/prices│
      │                     │                 │<─────────────────│
      │                     │                 │                  │
      │                     │ GET cached price│                  │
      │                     │────────────────>│                  │
      │                     │                 │                  │
      │              [Cache HIT → return]     │                  │
      │              [Cache MISS ↓]           │                  │
      │                     │                 │                  │
      │ GET /simple/price   │                 │                  │
      │<────────────────────│                 │                  │
      │                     │                 │                  │
      │ {prices}            │                 │                  │
      │────────────────────>│                 │                  │
      │                     │                 │                  │
      │              [Validate response]      │                  │
      │              [Map asset codes]        │                  │
      │                     │                 │                  │
      │                     │ SET with 15min  │                  │
      │                     │ TTL             │                  │
      │                     │────────────────>│                  │
      │                     │                 │                  │
      │                     │ {price response}│                  │
      │                     │────────────────────────────────────>│
```

**Security Controls:**
- HTTPS to CoinGecko
- 15-minute cache with stale fallback
- 10-second request timeout
- Rate limiting (<100 req/min)

**Gaps:**
- No response integrity verification
- Stale data served on API failure (acceptable tradeoff)

### 2.5 WebSocket Data Flow

```
┌──────────┐                    ┌─────────┐
│  Client  │                    │ Backend │
│ (Browser)│                    │         │
└────┬─────┘                    └────┬────┘
     │                               │
     │ WS Upgrade /ws?token=xxx      │
     │ [RISK: Token in URL]          │
     │──────────────────────────────>│
     │                               │
     │                        [Optional auth]
     │                        [RISK: Auth not
     │                         enforced]
     │                               │
     │ {type: connected,             │
     │  connection_id: uuid}         │
     │<──────────────────────────────│
     │                               │
     │ {type: subscribe,             │
     │  channels: ["corridors"]}     │
     │──────────────────────────────>│
     │                               │
     │                        [Register subscription]
     │                        [No channel validation]
     │                        [No subscription limit]
     │                               │
     │ {type: subscription_confirm}  │
     │<──────────────────────────────│
     │                               │
     │         [Broadcast events]    │
     │                               │
     │ {type: corridor_update, ...}  │
     │<──────────────────────────────│
     │                               │
     │ {type: ping, timestamp}       │
     │<──────────────────────────────│
     │                               │ (every 30s)
```

**Security Controls:**
- Optional token authentication
- Connection cleanup on disconnect
- Ping/pong keepalive

**Gaps:**
- No mandatory authentication
- No connection limit per IP
- No subscription count limit
- No message size validation
- Token in URL query parameter

---

## 3. Sensitive Data Exposure Points

### 3.1 Logging

| Location | Data at Risk | Current Protection |
|----------|-------------|-------------------|
| `env_config.rs` | Database/Redis URLs | Credential redaction |
| `main.rs:92` | Raw DATABASE_URL | **None (logged before sanitization)** |
| `oauth.rs:354` | Token prefix (20 chars) | **Partial - still leaks prefix** |
| WebSocket handler | Connection events | Connection ID only |
| Webhook dispatcher | Delivery status | URL logged (acceptable) |
| Request signing | Signature validation | No sensitive data logged |

### 3.2 Error Responses

| Endpoint | Potential Leakage | Assessment |
|----------|------------------|------------|
| Auth endpoints | "Invalid credentials" (generic) | Good |
| SEP-10 endpoints | Detailed error messages | Review needed |
| Database errors | SQLx errors may leak schema info | Review needed |
| Webhook errors | Server errors passed through | Review needed |

### 3.3 Client-Side Storage

| Data | Storage Mechanism | Risk |
|------|------------------|------|
| JWT access token | Browser (localStorage/cookie) | XSS theft |
| JWT refresh token | Browser (localStorage/cookie) | XSS theft |
| SEP-10 session token | Browser | XSS theft |
| User preferences | Context/localStorage | Low risk |

---

## 4. Network Security Boundaries

### 4.1 Production Network Topology

```
Internet
    │
    ▼
┌──────────────────────────────────────────────┐
│                  AWS VPC                      │
│                                               │
│  ┌────────────────────────────────────────┐  │
│  │          Public Subnets                 │  │
│  │  ┌──────────────────────────────────┐  │  │
│  │  │       Application Load Balancer  │  │  │
│  │  │       (HTTPS termination)        │  │  │
│  │  └──────────────┬───────────────────┘  │  │
│  └─────────────────┼──────────────────────┘  │
│                    │                          │
│  ┌─────────────────┼──────────────────────┐  │
│  │          Private Subnets                │  │
│  │                 │                       │  │
│  │  ┌──────────────▼───────────────────┐  │  │
│  │  │    ECS Fargate (Backend)         │  │  │
│  │  │    Port 8080 (HTTP internal)     │  │  │
│  │  └──────┬──────────────┬────────────┘  │  │
│  │         │              │                │  │
│  │  ┌──────▼──────┐ ┌────▼─────────────┐ │  │
│  │  │ RDS (PG)    │ │ ElastiCache      │ │  │
│  │  │ Port 5432   │ │ (Redis) Port 6379│ │  │
│  │  │ Encrypted   │ │                  │ │  │
│  │  └─────────────┘ └──────────────────┘ │  │
│  │                                        │  │
│  │  ┌────────────────────────────────┐   │  │
│  │  │ ELK Stack                      │   │  │
│  │  │ ES:9200, Logstash:5044,        │   │  │
│  │  │ Kibana:5601                    │   │  │
│  │  └────────────────────────────────┘   │  │
│  └────────────────────────────────────────┘  │
└──────────────────────────────────────────────┘
```

### 4.2 Security Group Rules (Verify)

| Resource | Inbound | Outbound | Concern |
|----------|---------|----------|---------|
| ALB | 443 (HTTPS) from 0.0.0.0/0 | All to VPC | Standard |
| ECS Task | 8080 from ALB SG only | All (egress) | Review egress rules |
| RDS | 5432 from ECS SG only | None | Good |
| Redis | 6379 from ECS SG only | None | Good |
| ELK | 5601 from ? | ? | **Verify access control** |

---

## 5. Compliance Considerations

### Data Handling Requirements

| Regulation | Relevance | Current Compliance |
|-----------|-----------|-------------------|
| GDPR | Low (no EU PII by default) | N/A unless user data collected |
| SOC 2 | Medium (if enterprise customers) | Audit logging present |
| PCI DSS | N/A (no payment card data) | N/A |
| Stellar SEPs | High (SEP-10 implementation) | Partial (simplified impl) |

### Audit Trail Coverage

| Event | Logged | Searchable | Retained |
|-------|--------|-----------|----------|
| Login attempts | No | N/A | N/A |
| Failed logins | No | N/A | N/A |
| API access | Via HTTP logs | ELK | Configurable |
| Data modifications | Admin audit log | Database | Indefinite |
| Webhook deliveries | Event queue | Database | Indefinite |
| Token issuance | Debug log only | Logs | Configurable |
| Token revocation | Log only | Logs | Configurable |
| Configuration changes | No | N/A | N/A |
