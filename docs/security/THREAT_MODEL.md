# Stellar Insights STRIDE Threat Model

## Methodology

This threat model uses the STRIDE framework (Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege) to systematically identify and assess security risks across the Stellar Insights system.

---

## 1. SPOOFING (Identity Impersonation)

### Threat: Attacker impersonates a legitimate user to access their data

| Threat | Severity | Likelihood | Existing Control | Gap | Status |
|--------|----------|-----------|-------------------|-----|--------|
| Steal JWT token via XSS | High | Medium | HTTPOnly cookies, CSP headers | Need CSP audit | Mitigated |
| Crack password offline | High | Low | Argon2 with tuned params | Weak password policy | Acceptable |
| Compromise API key | High | Low | HMAC signing + rate limiting | Key rotation not enforced | Mitigated |
| Hijack session via network | Medium | Low | HTTPS only, secure flag | HTTP enforced in middleware | Mitigated |
| Steal refresh token | Medium | Low | Rotation on use, short TTL | No revocation on logout | Gap |
| Forge JWT (HMAC-SHA256) | Low | Very Low | JWT signature verification | RSA recommended for multi-service | Acceptable |

### Mitigations
- ✅ Implemented: JWT validation, Argon2 password hashing, HTTPOnly cookies
- ✅ Implemented: HMAC-SHA256 request signing with timestamp/nonce
- ⚠️ Recommended: Implement token revocation list (blacklist expired JWTs)
- ⚠️ Recommended: Enforce strong password policy (min 12 chars, complexity)
- ⚠️ Recommended: Add Content Security Policy headers to prevent XSS

---

## 2. TAMPERING (Data Modification)

### Threat: Attacker modifies API requests, database records, or audit logs

| Threat | Severity | Likelihood | Existing Control | Gap | Status |
|--------|----------|-----------|-------------------|-----|--------|
| Tamper with HTTP request body | High | Medium | HMAC-SHA256 signatures | Only on signed endpoints | Mitigated |
| Tamper with query parameters | High | Medium | HMAC-SHA256 (canonical format) | Included in signature | Mitigated |
| Modify database records directly | High | Low | RDS encryption, backups | IAM access controls weak | Gap |
| Modify audit logs | Low | Very Low | Chained hashes (append-only) | Hash chain validation missing | Gap |
| Replay signed requests | Medium | Low | Timestamp + nonce tracking | 5-min window could be shorter | Mitigated |
| Fake admin audit entries | Low | Very Low | Audit log append-only | No signed/attested entries | Gap |

### Mitigations
- ✅ Implemented: HMAC-SHA256 canonical request signing
- ✅ Implemented: Timestamp freshness validation (±5 min)
- ✅ Implemented: Nonce-based replay detection (Redis-backed)
- ✅ Implemented: Encrypted database (KMS) and automated backups
- ⚠️ Gap: Audit log chained hash verification not implemented
- ⚠️ Gap: IAM policies require review for database access restrictions

---

## 3. REPUDIATION (Denial of Action)

### Threat: User or attacker denies performing an action, or log is incomplete

| Threat | Severity | Likelihood | Existing Control | Gap | Status |
|--------|----------|-----------|-------------------|-----|--------|
| Deny data export request was made | Low | Low | Audit logs (immutable) | Verified by user | Acceptable |
| Deny deletion request was confirmed | Low | Low | Email/2FA confirmation + audit | User can confirm | Acceptable |
| Deny API call was made | Medium | Low | Audit logging, chained hashes | Incomplete hash verification | Gap |
| Deny consent was changed | Low | Very Low | Timestamp, audit trail | Verified by user | Acceptable |
| Admin denies action taken | Low | Very Low | Audit log chained hashes | Integrity not cryptographically proven | Gap |

### Mitigations
- ✅ Implemented: Immutable audit logs with append-only database
- ✅ Implemented: Chained hashes in audit log (previous_hash + data)
- ✅ Implemented: Deletion confirmation tokens (email/2FA)
- ⚠️ Gap: Audit log hash chain validation algorithm not implemented
- ⚠️ Recommended: Cryptographic signatures on audit entries (operator key)

---

## 4. INFORMATION DISCLOSURE (Unauthorized Data Access)

### Threat: Confidential data exposed to unauthorized parties

| Threat | Severity | Likelihood | Existing Control | Gap | Status |
|--------|----------|-----------|-------------------|-----|--------|
| Expose personal data via data breach | Critical | Medium | Encryption at rest (KMS) | Backup encryption unclear | Gap |
| Expose password hash via database leak | High | Low | Argon2 (slow hash) | Rainbow tables possible | Mitigated |
| Expose API key secret | High | Low | Hash storage, single exposure | Plaintext in logs risk | Gap |
| Expose JWT secret / signing key | Critical | Very Low | Environment variable | Vault integration weak | Gap |
| Expose request signing secret | High | Low | Environment variable | Plaintext in Vault? | Gap |
| Expose user consent choices | Medium | Low | Encrypted at rest | User can audit | Acceptable |
| Expose error messages (info leakage) | Medium | High | Generic error responses | "Invalid signature" is non-revealing | Mitigated |
| Logs expose sensitive data | Medium | Medium | Log redaction incomplete | PII in CloudWatch? | Gap |
| S3 backup exposure | High | Low | Bucket policies, versioning | Lifecycle policy in progress | Mitigated |
| Redis in-memory data exposure | Medium | Low | At-rest encryption, auth token | Snapshots might not be encrypted | Gap |

### Mitigations
- ✅ Implemented: Encrypted at rest (KMS) for RDS
- ✅ Implemented: Non-revealing error messages on signature failure
- ✅ Implemented: API keys stored as hashes, not plaintext
- ✅ Implemented: S3 lifecycle policies (transition to Glacier, delete)
- ⚠️ Gap: Backup encryption (verify RDS snapshot encryption)
- ⚠️ Gap: Log redaction (audit logs, CloudWatch may contain PII)
- ⚠️ Gap: Secret management (JW Secret, API key secret in Vault vs plaintext)
- ⚠️ Gap: Redis snapshot encryption not verified

**Recommendations**:
1. Implement log redaction for sensitive fields (user_id → hash)
2. Verify RDS automated backup encryption is enabled
3. Migrate secrets to AWS Secrets Manager or HashiCorp Vault
4. Audit CloudWatch logs for PII leakage
5. Enable Redis snapshot encryption

---

## 5. DENIAL OF SERVICE (Availability)

### Threat: Attacker disrupts service availability

| Threat | Severity | Likelihood | Existing Control | Gap | Status |
|--------|----------|-----------|-------------------|-----|--------|
| HTTP request flood (DDoS) | High | High | ALB rate limiting, WAF | WAF rules unclear | Mitigated |
| Expensive query attack | High | Medium | Query timeouts, indexes | Slow-query detection missing | Gap |
| Billion laughs / XML bomb | Medium | Low | Payload size limit (10MB default) | JSON bombs possible | Gap |
| Slowloris (slow-reading) | Medium | Low | HTTP timeout (30s default) | May be insufficient | Acceptable |
| Database connection pool exhaustion | High | Medium | Connection pool (100 max) | Monitoring in place | Mitigated |
| Redis memory exhaustion | Medium | Medium | Maxmemory policy (allkeys-lru) | Eviction could cause data loss | Acceptable |
| Infinite loop in signing verification | Low | Very Low | Timeout on middleware | Signature check is fast | Acceptable |
| Cache stampede (thundering herd) | Medium | Low | Cache TTL, circuit breaker | Incomplete implementation | Gap |

### Mitigations
- ✅ Implemented: Rate limiting (token bucket per IP/user)
- ✅ Implemented: Payload size limit (10MB, configurable)
- ✅ Implemented: Connection pool limits (100 max connections)
- ✅ Implemented: Database query timeouts (30s)
- ✅ Implemented: Redis eviction policy (allkeys-lru)
- ⚠️ Gap: Query cost analysis (prevent expensive queries)
- ⚠️ Gap: Cache stampede protection (lock-free cache patterns)
- ⚠️ Recommended: Enable AWS WAF (IP reputation, rate-based rules)

---

## 6. ELEVATION OF PRIVILEGE (Unauthorized Access Levels)

### Threat: Attacker gains higher privileges than authorized

| Threat | Severity | Likelihood | Existing Control | Gap | Status |
|--------|----------|-----------|-------------------|-----|--------|
| Bypass authorization checks | High | Low | Middleware chain, AuthUser extractor | Some handlers lack auth | Gap |
| Modify own user role | Medium | Low | Role stored server-side only | Role definition unclear | Gap |
| Access other user's data | High | Low | User_id parameter validation | SQL injection mitigated (sqlx) | Mitigated |
| Access admin endpoints as user | High | Low | Admin middleware (not visible) | Missing explicit role checks | Gap |
| Impersonate admin via JWT tampering | High | Very Low | JWT signature verification | Forging high entropy HMAC hard | Acceptable |
| Exploit GDPR to access others' data | Low | Very Low | User_id ownership check in handlers | Should audit all GDPR endpoints | Gap |
| Escalate via API key sharing | Medium | Medium | No explicit key ownership check | API key tied to user at creation | Mitigated |

### Mitigations
- ✅ Implemented: AuthUser extractor (enforced on protected endpoints)
- ✅ Implemented: Parameterized queries (sqlx) prevent SQL injection
- ✅ Implemented: User_id ownership checks in GDPR flows
- ⚠️ Gap: Missing role/permission model documentation
- ⚠️ Gap: Not all handlers explicitly check authorization
- ⚠️ Recommended: Add authorization middleware for role-based access
- ⚠️ Recommended: Audit all user_id parameter bindings for ownership checks

---

## 7. Summary: Risk Heat Map

### By Severity
- **Critical**: Data breach (backups), Crypto key compromise
- **High**: Tampering (database), Request spoofing, Access control bypass
- **Medium**: Repudiation (audit logs), Information disclosure (logs), DoS, Privilege escalation
- **Low**: Most others

### By Likelihood
- **High**: DDoS, Log PII leakage
- **Medium**: Session hijacking, Query attacks, Cache issues
- **Low**: Crypto compromise, Database tampering, Privilege escalation
- **Very Low**: Hash chain tampering, JWT forgery

### Top Risks (High Severity × Medium/High Likelihood)
1. **DDoS via HTTP flood**: Mitigated by ALB + rate limiting; WAF recommended
2. **Data breach via database access**: Mitigated by KMS encryption; IAM audit needed
3. **PII leakage via logs**: GAP; log redaction needed
4. **Unauthorized access via missing auth checks**: GAP; audit all endpoints
5. **Request tampering (non-signed endpoints)**: Mitigated for signed endpoints; expand coverage if needed

---

## 8. Risk Acceptance and Recommendations

### Accepted Risks (Severity < High OR Likelihood < Medium)
- ✅ Weak password policy (accept, with enforcement recommendations)
- ✅ Slowloris attacks (accept, timeouts in place)
- ✅ Redis eviction causing data loss (accept, cache is ephemeral)

### Recommended Fixes (Before Audit)
1. **P0 (Critical)**: Verify backup encryption; audit secrets storage (Vault vs plaintext)
2. **P1 (High)**: Implement log redaction; audit authorization on all endpoints
3. **P2 (Medium)**: Implement audit log hash verification; add role-based access control
4. **P3 (Low)**: Enhance WAF rules; implement cache stampede protection

### For External Auditor
- Request auditor focus on: authorization model, secret storage, log redaction
- Penetration test: OWASP Top 10 + API tampering (try to bypass HMAC), GDPR access control
- Code review: All handlers checking AuthUser; all queries parameterized; all secrets managed securely

---

## 9. Compliance Alignment

### GDPR
- ✅ Data export (Right to Access) — confirmed in architecture
- ✅ Data deletion (Right to Erasure) — 30-day grace, anonymization for audit logs
- ✅ Consent management — implemented, auditable
- ⚠️ Data minimization — inventory complete, could reduce API key metadata retention
- ⚠️ Breach notification — no incident response plan documented

### OWASP Top 10
| Risk | Status | Control |
|------|--------|---------|
| A01 Broken Access Control | 🟡 Gap | Auth checks on most endpoints; admin endpoints not documented |
| A02 Cryptographic Failure | 🟢 Mitigated | Encryption at rest (KMS), in transit (HTTPS) |
| A03 Injection | 🟢 Mitigated | Prepared statements (sqlx) |
| A04 Insecure Design | 🟡 Gap | Security review incomplete |
| A05 Security Misconfiguration | 🟡 Gap | IAM policies, WAF rules need audit |
| A06 Vulnerable Components | 🟢 Mitigated | Dependency scanning in progress |
| A07 Authentication Failure | 🟢 Mitigated | Argon2, JWT, rate limiting |
| A08 Software and Data Integrity | 🟡 Gap | Audit log integrity not cryptographically enforced |
| A09 Logging & Monitoring | 🟡 Gap | CloudWatch logs, but PII leakage risk |
| A10 SSRF | 🟢 Low Risk | Limited external requests (Stellar Horizon only) |

---

## 10. Next Steps for Audit Preparation

- [ ] Verify backup encryption is enabled on RDS
- [ ] Audit all secret management (JWT secret, API signing secret location)
- [ ] Implement log redaction for all PII fields
- [ ] Document and review all authorization checks (per-endpoint)
- [ ] Verify all queries use parameterized statements
- [ ] Implement audit log hash chain verification algorithm
- [ ] Enable and configure AWS WAF rules
- [ ] Add incident response plan for data breaches
- [ ] Penetration testing on request signing endpoints
- [ ] Compliance audit (GDPR vs. implementation)
