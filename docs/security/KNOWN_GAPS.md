# Known Security Gaps and Future Work

This document consolidates security findings from the threat model and architecture review that are out of scope for the current audit-prep batch but should be addressed before production deployment.

---

## Critical Gaps (Must Fix Before Production)

### 1. Secret Storage and Rotation

**Issue**: JWT secret, API signing secret, and other credentials stored in environment variables or basic Vault  
**Risk**: Plaintext secret exposure in build logs, container images, or Vault admin access  
**Recommendation**:
- Migrate all secrets to AWS Secrets Manager or HashiCorp Vault with proper RBAC
- Implement secret rotation policy (quarterly minimum)
- Audit who has access to secrets
- Enable automatic key rotation for KMS

**Owner**: DevOps/Security  
**Effort**: 2-3 days  
**Status**: 🔴 Not started

---

### 2. Backup Encryption Verification

**Issue**: RDS automated backups may not be encrypted with customer-managed KMS keys  
**Risk**: AWS service account compromise could expose backups  
**Recommendation**:
- Verify RDS snapshots use customer-managed KMS encryption (not AWS-managed)
- Enable automated snapshot copy to secondary region with separate KMS key
- Test restore from snapshot in non-production

**Owner**: DevOps  
**Effort**: 1 day  
**Status**: 🟡 Partially verified

---

### 3. Log Redaction (PII Leakage)

**Issue**: CloudWatch logs, audit logs, and application logs may contain PII (usernames, emails)  
**Risk**: Breach of log aggregation system exposes all user personal data  
**Recommendation**:
- Implement log redaction middleware for all user IDs, usernames, emails
- Scrub from: CloudWatch logs, audit logs, error responses, debug output
- Example: `user_id=xxxxx...abc` (hash + last 3 chars)
- Audit existing logs for PII

**Owner**: Backend/Platform  
**Effort**: 3-4 days  
**Status**: 🔴 Not started

---

### 4. Authorization Model Documentation and Audit

**Issue**: Role-based access control (RBAC) is not explicitly documented; some endpoints may lack auth checks  
**Risk**: Privilege escalation via missing authorization on admin endpoints  
**Recommendation**:
- Document authorization model (who can access what)
- Audit every handler for explicit AuthUser extraction and role/permission check
- Add authorization middleware for admin endpoints
- Test unauthorized access attempts (negative test cases)

**Owner**: Backend  
**Effort**: 3-5 days  
**Status**: 🔴 Not started

---

## High-Priority Gaps (Should Fix Before Production)

### 5. Audit Log Integrity Verification

**Issue**: Audit log chained hashes are computed but never verified; hash chain could be tampered with  
**Risk**: Attacker replays audit log entries or tampers with the chain without detection  
**Recommendation**:
- Implement hash chain verification algorithm (verify hash(previous_entry + current_entry) == stored_hash)
- Run weekly integrity check job
- Alert on hash chain breaks
- Store one committed hash (e.g., at night) separate from the chain as a checkpoint

**Owner**: Backend  
**Effort**: 2 days  
**Status**: 🔴 Not started

---

### 6. AWS WAF Configuration

**Issue**: No explicit WAF rules; ALB accepts any traffic  
**Risk**: DDoS, SQL injection attempts, malformed requests not filtered  
**Recommendation**:
- Enable AWS WAF on ALB
- Rate-based rules (e.g., >2000 req/5min from single IP)
- IP reputation blocking
- Managed rule groups (AWS Core, SQL Injection, XSS)
- Geographic blocking (if applicable)

**Owner**: DevOps/Security  
**Effort**: 1-2 days  
**Status**: 🔴 Not started

---

### 7. API Key Ownership Enforcement

**Issue**: API key ownership not consistently enforced across all endpoints  
**Risk**: User can manipulate API key IDs to access other users' keys  
**Recommendation**:
- Audit all API key endpoints for ownership checks
- Verify query filters include `AND user_id = ?`
- Add test cases for cross-user API key access

**Owner**: Backend  
**Effort**: 1 day  
**Status**: 🟡 Mostly mitigated (needs audit)

---

### 8. GDPR Access Control Audit

**Issue**: GDPR export/deletion endpoints check user_id but full coverage unknown  
**Risk**: User can export/delete another user's data if authorization is incomplete  
**Recommendation**:
- Audit all GDPR handlers (/export, /export/{id}, /deletion, /deletion/{id}, /consents)
- Verify user_id ownership check on every database query
- Add negative test cases (unauthorized user tries to access)

**Owner**: Backend  
**Effort**: 1 day  
**Status**: 🟡 Likely mitigated (needs audit)

---

## Medium-Priority Gaps (Fix Before Audit or Production)

### 9. Dependency Scanning and SBOM

**Issue**: No automated dependency scanning in CI/CD; unknown vulnerability status of Cargo/npm packages  
**Risk**: Known vulnerabilities in dependencies used in production  
**Recommendation**:
- Enable `cargo audit` and `npm audit` in CI/CD (fail on high/critical)
- Generate SBOM (Software Bill of Materials) for each release
- Subscribe to security mailing lists for dependencies
- Review high-impact dependencies quarterly

**Owner**: DevOps/Platform  
**Effort**: 1 day  
**Status**: 🟡 In progress (dependency-audit-2026-07-26.md exists)

---

### 10. Rate Limiting Coverage

**Issue**: Rate limiting applies per-IP/per-user; some high-value endpoints may need tighter limits  
**Risk**: Brute force attacks, export data enumeration  
**Recommendation**:
- Verify login endpoint has aggressive rate limiting (<5 attempts / 15 min per IP)
- Add lower limits to GDPR endpoints (1 request/min per user)
- Monitor for retry patterns in audit logs

**Owner**: Backend  
**Effort**: 1 day  
**Status**: 🟡 Partial (needs endpoint-specific tuning)

---

### 11. HTTPS Enforcement

**Issue**: Middleware may not strictly enforce HTTPS; redirects possible  
**Risk**: Downgrade attacks, man-in-the-middle on first request  
**Recommendation**:
- Add HSTS header (Strict-Transport-Security) with includeSubdomains, preload
- Disable HTTP entirely (return 426 or 400, no redirects)
- Test with curl -I http://api.example.com (should fail)

**Owner**: Backend/DevOps  
**Effort**: 1 day  
**Status**: 🟡 Partial (HSTS headers may exist)

---

### 12. CORS Configuration Audit

**Issue**: CORS rules defined but not audited for overly permissive configuration  
**Risk**: Unintended cross-origin requests allowed; CSRF-like attacks  
**Recommendation**:
- Review CorsLayer configuration (allowed origins, methods, headers)
- Ensure only specific frontend origin(s) allowed (not `*`)
- Test with curl -H Origin:attacker.com (should fail)

**Owner**: Backend  
**Effort**: 1 day  
**Status**: 🟡 Likely correct (needs audit)

---

## Low-Priority Gaps (Nice to Have)

### 13. Cache Stampede Protection

**Issue**: Popular cache keys expiring simultaneously could overwhelm database  
**Risk**: Degraded performance during cache miss thundering herd  
**Recommendation**:
- Implement cache lock pattern (first requester waits, others wait for result)
- Or: Implement cache warming job (refresh keys before expiry)
- Or: Use probabilistic early expiration (refresh at 80% TTL, not 100%)

**Owner**: Backend  
**Effort**: 2 days  
**Status**: 🟡 Incomplete (architecture exists)

---

### 14. Cryptographic Algorithm Documentation

**Issue**: HMAC-SHA256 chosen for request signing; no documentation of why (vs. RSA, ECDSA)  
**Risk**: Auditor questions algorithm choice; potential future replacement needed  
**Recommendation**:
- Document design decision: HMAC-SHA256 chosen for:
  - Symmetric key (simpler than PKI)
  - Speed (HMAC faster than RSA)
  - Stellar ecosystem conventions
- Migrate to RSA-PSS or ECDSA if multi-service signing needed (each service has own signing secret)

**Owner**: Security/Architecture  
**Effort**: 0.5 day (documentation only)  
**Status**: 🟢 Documented in API_REQUEST_SIGNING.md

---

### 15. Incident Response Plan

**Issue**: No documented incident response procedures for data breach, service outage, or compromise  
**Risk**: Slow detection/response; unclear who does what  
**Recommendation**:
- Document roles: incident commander, security lead, DevOps lead
- Define escalation path (severity thresholds)
- Define notification procedures (legal, customers, regulators)
- Define forensics procedures (log preservation, evidence collection)
- Conduct tabletop exercise (simulate breach scenario)

**Owner**: Security/Management  
**Effort**: 2-3 days  
**Status**: 🔴 Not started

---

## Summary by Timeline

### Before Production (Must Have)
1. Secret storage and rotation (Vault/AWS Secrets Manager)
2. Backup encryption verification
3. Log redaction (PII scrubbing)
4. Authorization model audit
5. Incident response plan

### Before External Audit (Should Have)
6. Audit log integrity verification
7. AWS WAF configuration
8. Dependency scanning (SBOM)
9. GDPR access control audit
10. HTTPS and CORS enforcement

### Post-Audit (Nice to Have)
11. Cache stampede protection
12. Rate limiting tuning
13. Cryptographic algorithm documentation (already done)

---

## Tracking and Accountability

### By Component
- **Authentication/Authorization**: Gaps 4, 7, 8, 10
- **Data Protection**: Gaps 2, 3, 5, 11
- **API Security**: Gaps 6, 12, 13
- **Operations**: Gaps 1, 9, 15
- **Architecture/Design**: Gap 14

### By Effort Estimate
- **Quick wins** (1 day or less): Gaps 1, 7, 11, 12, 14
- **Medium** (2-3 days): Gaps 3, 4, 5, 6, 9, 10, 13
- **Larger** (3+ days): Gaps 2, 8, 15

---

## Risk Acceptance Matrix

| Gap | Severity | Likelihood | Risk Level | Acceptance | Notes |
|-----|----------|-----------|-----------|-----------|--------|
| Secret storage | Critical | Medium | 🔴 | Unacceptable | Fix before production |
| Backup encryption | Critical | Low | 🟡 | Needs verification | Likely already correct |
| Log PII leakage | High | Medium | 🔴 | Unacceptable | Fix before production |
| Authorization audit | High | Medium | 🔴 | Unacceptable | Fix before production |
| Audit log verification | High | Low | 🟡 | Needs mitigation | Implement before audit |
| WAF config | High | Medium | 🟡 | Acceptable (mitigating) | Implement before audit |
| Dependency scanning | High | Low | 🟡 | Acceptable (in progress) | Already underway |
| GDPR auth audit | High | Low | 🟡 | Likely acceptable | Needs audit |
| Rate limiting | Medium | Low | 🟢 | Acceptable | Refine before production |
| Cache stampede | Medium | Low | 🟢 | Acceptable | Nice to have |
| Incident response | Medium | Medium | 🟡 | Needs plan | Required for compliance |

