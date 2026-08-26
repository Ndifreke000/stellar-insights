# Stellar Insights Security Architecture

## System Overview

Stellar Insights is a Rust/Axum backend API with a React frontend, providing data analytics for Stellar blockchain networks. The system ingests blockchain data, processes it, and exposes insights via REST APIs and GraphQL.

### Technology Stack
- **Backend**: Rust/Axum (async web framework)
- **Database**: SQLite (primary), PostgreSQL (optional)
- **Cache**: Redis (ElastiCache)
- **Frontend**: React (separate deployment)
- **Authentication**: JWT, SEP-10 (Stellar) 
- **Infrastructure**: AWS (RDS, ElastiCache, ECS, ALB, CloudWatch)

---

## Data Flow Architecture

### 1. User Authentication Flow

```
┌─ Client ─────────────────────────────────────────────┐
│                                                       │
│  1. POST /auth/login {username, password}           │
│        ↓                                              │
│  2. Auth Service validates credentials              │
│        ↓                                              │
│  3. Returns JWT (access + refresh tokens)           │
│        ↓                                              │
│  4. Client stores JWT in secure HttpOnly cookie    │
│                                                       │
└───────────────────────────────────────────────────────┘
        ↓
    [Secured Endpoints]
        ↓
┌─ Middleware Stack ──────────────────────────────────┐
│                                                      │
│  1. JWT validation (verify signature, exp)         │
│  2. AuthUser extraction (user_id, username)        │
│  3. Authorization checks (role-based if needed)    │
│  4. Request signing verification (HMAC-SHA256)     │
│  5. Rate limiting                                   │
│  6. IP whitelist checking (if configured)          │
│  7. Audit logging                                   │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### 2. API Request Signing Flow (Request Signing Middleware)

```
┌─ Authenticated Client ────────────────────────────────┐
│                                                        │
│  1. Build canonical request:                         │
│     - METHOD, PATH                                   │
│     - Sorted query parameters                        │
│     - SHA256(body)                                   │
│     - Unix timestamp                                 │
│     - Random nonce                                   │
│                                                        │
│  2. HMAC-SHA256(canonical, client_secret)           │
│                                                        │
│  3. Send with headers:                              │
│     - X-Signature: <hmac>                           │
│     - X-Timestamp: <unix_ts>                        │
│     - X-Nonce: <random_uuid>                        │
│                                                        │
└────────────────────────────────────────────────────────┘
        ↓
┌─ Server Middleware ──────────────────────────────────┐
│                                                       │
│  1. Check timestamp freshness (±5 min window)       │
│  2. Check nonce not previously used (Redis cache)   │
│  3. Reconstruct canonical request                   │
│  4. Verify HMAC matches                             │
│  5. Record nonce in Redis (TTL = clock skew)        │
│                                                       │
│  → Success: Route to handler                        │
│  → Failure: Return 401 Unauthorized (non-revealing) │
│                                                       │
└───────────────────────────────────────────────────────┘
```

### 3. Data Export Flow (GDPR Right to Access)

```
┌─ Authenticated User ──────────────────────────────────┐
│                                                        │
│  POST /api/v1/gdpr/export {data_types}             │
│        ↓                                              │
└────────────────────────────────────────────────────────┘
        ↓
┌─ Server Processing ──────────────────────────────────┐
│                                                       │
│  1. Create export request (status=pending)          │
│  2. Background job processes:                       │
│     a. Query user_consents                          │
│     b. Query api_keys (metadata only)               │
│     c. Query admin_audit_log (user's actions)       │
│     d. Query data_processing_log                    │
│     e. Compile into JSON                            │
│     f. Encrypt and store in S3                      │
│     g. Update request (status=completed)            │
│  3. Generate signed download URL (7-day TTL)        │
│  4. Return download link to user                    │
│                                                       │
└───────────────────────────────────────────────────────┘
        ↓
┌─ User Download ────────────────────────────────────┐
│                                                     │
│  GET /api/v1/gdpr/export/{id}/download            │
│  + signed S3 token (expires in 7 days)            │
│        ↓                                            │
│  S3 returns encrypted JSON file                   │
│        ↓                                            │
│  File deleted from S3 after 7 days (lifecycle)    │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### 4. Data Deletion Flow (GDPR Right to be Forgotten)

```
┌─ Authenticated User ──────────────────────────────────┐
│                                                        │
│  POST /api/v1/gdpr/deletion {reason}               │
│        ↓                                              │
│  Server creates request (status=pending)            │
│  Confirmation token sent via email/2FA             │
│                                                        │
└────────────────────────────────────────────────────────┘
        ↓ (User confirms via email/2FA)
┌─ Confirmation ─────────────────────────────────────┐
│                                                     │
│  POST /api/v1/gdpr/deletion/confirm {token}       │
│        ↓                                            │
│  Status changes: pending → confirmed              │
│  Deletion scheduled for 30 days from now          │
│                                                     │
└─────────────────────────────────────────────────────┘
        ↓ (After 30-day grace period)
┌─ Automatic Execution ──────────────────────────────┐
│                                                    │
│  Background job executes atomically:              │
│  1. BEGIN TRANSACTION                             │
│  2. DELETE user_consents                          │
│  3. DELETE api_keys                               │
│  4. DELETE data_export_requests                   │
│  5. UPDATE users SET username=anonymized          │
│  6. UPDATE admin_audit_log SET user_id=anonymized │
│  7. UPDATE vault_audit_log SET user_id=anonymized │
│  8. COMMIT TRANSACTION                            │
│  9. Update deletion request (status=completed)    │
│                                                    │
└────────────────────────────────────────────────────┘
```

### 5. Blockchain Data Ingestion

```
┌─ Stellar Horizon Server ──────────────────────────────┐
│ Real-time or historical ledger data                  │
└──────────────────────┬─────────────────────────────────┘
                       ↓
┌─ Event Indexer Service ───────────────────────────────┐
│ 1. Fetch ledgers from Horizon                        │
│ 2. Parse transactions, payments, operations          │
│ 3. Validate signatures                               │
│ 4. Store in database                                │
│ 5. Update replay state (for idempotency)            │
└──────────────────────┬─────────────────────────────────┘
                       ↓
┌─ Database (SQLite/PostgreSQL) ────────────────────────┐
│ - transactions, payments, trustlines                  │
│ - liquidity_pools, contract_events                   │
│ - metrics, snapshots (historical analytics)          │
└──────────────────────┬─────────────────────────────────┘
                       ↓
┌─ API Handlers ────────────────────────────────────────┐
│ - Cached queries (Redis)                              │
│ - Aggregations and analytics                          │
│ - GraphQL subscriptions (WebSockets)                 │
│ - REST endpoints (/api/v1/*)                         │
└──────────────────────┬─────────────────────────────────┘
                       ↓
┌─ Frontend (React) ────────────────────────────────────┐
│ - Dashboard, charts, tables                           │
│ - User settings (consents, API keys)                 │
│ - Export/deletion requests                           │
└───────────────────────────────────────────────────────┘
```

---

## Trust Boundaries and Sensitive Data Flows

### User Credentials and Authentication
- **Stored**: Password hash (Argon2) in `users` table
- **In transit**: JWT token in secure HTTPOnly cookie (HTTPS only)
- **Risk**: Password hash compromise allows offline crack attempts
- **Mitigation**: Argon2 with tuned parameters, rate limiting on login

### API Keys
- **Stored**: Key hash (not plaintext) in `api_keys` table
- **Generated**: Cryptographically random (crypto::SecureRandom)
- **Exposed once**: Returned to user only at creation time
- **Risk**: Key compromise allows API access impersonation
- **Mitigation**: Rotation support, signing requests with key

### Session Data
- **JWT tokens**: Stored in HTTPOnly cookies (not accessible to JavaScript)
- **Redis cache**: Session state, nonces, rate limit counters
- **Risk**: Token theft via XSS (mitigated by HTTPOnly)
- **Mitigation**: Short expiry (1 hour access, 7 day refresh), CSRF tokens

### Audit Logs
- **Content**: Admin actions, user actions, vault operations
- **Stored**: Immutable append-only in `admin_audit_log`, `vault_audit_log`
- **Integrity**: Chained hash (each entry includes hash of previous)
- **Deletion**: Not deleted on user erasure (anonymized instead)
- **Risk**: Tampering (chained hash detects; requires replaying entire chain)

### Personal Data (GDPR)
- **User profile**: username, email, created_at (in users table)
- **Consents**: Privacy choices, timestamps (in user_consents table)
- **API keys**: Metadata (name, creation date, not secrets)
- **Audit logs**: User's own actions (anonymized if deleted)
- **Risk**: Breach exposes all user personal data
- **Mitigation**: Encrypted at rest (KMS), in transit (HTTPS), export/deletion workflows

---

## Security Controls by Threat

### Authentication & Authorization
| Threat | Control | Implementation |
|--------|---------|-----------------|
| Credential theft | HTTPS only, HTTPOnly cookies | TLS 1.2+, Cookie flags |
| Brute force | Rate limiting | Per-IP, per-user limits |
| Session hijacking | JWT expiry | 1 hour access, 7 day refresh |
| Token replay | Nonce tracking | Redis-backed nonce cache |
| Privilege escalation | Role-based access | AuthUser + handler checks |

### Data Integrity
| Threat | Control | Implementation |
|--------|---------|-----------------|
| Request tampering | HMAC-SHA256 signatures | Canonical request format |
| Data corruption | Encrypted at rest | AWS KMS encryption |
| Audit trail tampering | Chained hashes | Immutable audit logs |
| Replay attacks | Timestamp + nonce | 5-min window, Redis tracking |

### API Security
| Threat | Control | Implementation |
|--------|---------|-----------------|
| DDoS | Rate limiting, ALB | Token bucket, WAF |
| Injection | Input validation | sqlx prepared statements |
| CORS misconfiguration | Explicit CORS rules | Tower CorsLayer |
| API key leakage | Hash storage | Never log or expose plaintext |

### Data Privacy (GDPR)
| Requirement | Control | Implementation |
|-------------|---------|-----------------|
| Right to access | Data export | /gdpr/export endpoint |
| Right to deletion | Data erasure | 30-day grace, anonymization |
| Consent management | Tracking, audit | user_consents, audit logs |
| Data minimization | Inventory | Only collect necessary data |

---

## Sensitive Data Inventory

| Data | Classification | Storage | Lifecycle | Access |
|------|-----------------|---------|-----------|--------|
| Passwords | Secret | users table (hash) | Delete on account deletion | Auth only |
| API Keys | Secret | api_keys table (hash) | Delete on user deletion | Key owner, admin |
| JWT Tokens | Confidential | HTTPOnly cookie | Expire (1h access, 7d refresh) | Browser/client |
| User emails | Personal | users table | Export/anonymize on deletion | User, admin |
| Audit logs | Confidential | audit tables (immutable) | Keep indefinitely (anonymize) | Admin only |
| Blockchain data | Public | Public blockchain | N/A (public data) | Everyone |
| Consents | Personal | user_consents table | Delete/export on request | User, admin |

---

## External Dependencies and Risks

| Dependency | Risk | Mitigation |
|------------|------|-----------|
| AWS (RDS, ElastiCache, S3) | Service availability | Multi-AZ, automated backups, failover |
| Stellar Horizon API | Data availability | Fallback nodes, retry logic |
| Redis | In-memory data loss | Snapshots, replay logs |
| TLS/Crypto libraries | Cryptographic vulnerabilities | Regular updates, dependency scanning |
| npm/Cargo packages | Supply chain compromise | Dependency audits, SBOM |

---

## Compliance Posture

### GDPR Compliance
- ✅ Data inventory complete
- ✅ Right to access (export) implemented
- ✅ Right to deletion (erasure) implemented
- ✅ Consent management implemented
- ✅ Anonymization of audit logs on deletion
- ✅ Data retention policies documented

### Secure Development
- ✅ Input validation (sqlx prepared statements)
- ✅ Secure authentication (JWT, Argon2)
- ✅ Audit logging (immutable chained hashes)
- ✅ Error handling (no information leakage)
- ⚠️ Automated dependency scanning (in progress)

---

## Recommendations for External Audit

1. **Code review**: Authentication, GDPR workflows, error handling
2. **Cryptography review**: HMAC-SHA256 scheme, JWT implementation
3. **Infrastructure review**: AWS IAM policies, network isolation, encryption
4. **Penetration testing**: OWASP Top 10 + custom endpoints (request signing, GDPR)
5. **Compliance audit**: GDPR documentation vs. implementation
