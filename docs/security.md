# Security Policy

## Supported Versions

Version 1.0.0 will be supported with security updates.

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Secret Management Policy

### Never Commit Secrets

Secrets must **never** be committed to git, even in private repositories:

- Database credentials
- API keys and tokens (GitHub, AWS, Stellar, etc.)
- Private keys and certificates
- Webhook signing keys
- OAuth client secrets

### Using Environment Variables

All credentials must be stored in environment variables or secure secret management systems:

1. **Local development**: Use `.env` files (never commit `.env`)
2. **CI/CD**: Use GitHub Secrets or your CI provider's secret management
3. **Production**: Use a dedicated secret manager (e.g., HashiCorp Vault, AWS Secrets Manager)

Example:
```bash
# ❌ WRONG
const apiKey = "sk_test_123abc...";

# ✓ CORRECT
const apiKey = process.env.STELLAR_API_KEY;
if (!apiKey) throw new Error("STELLAR_API_KEY not set");
```

### Handling a Leaked Secret

If a secret is accidentally committed:

1. **Revoke immediately** — Invalidate the compromised credential in its issuing service
2. **Rotate** — Generate and deploy a new credential
3. **Audit** — Check logs for unauthorized use during the exposure window
4. **Communicate** — Notify security stakeholders and affected systems
5. **Remove from history** — Use `git filter-repo` or `BFG` to purge from git history (if acceptable for your workflow)

### Adding False Positives to Baseline

If `detect-secrets` flags a legitimate value (e.g., a test token), add it to the baseline:

```bash
detect-secrets scan > .secrets.baseline
```

Then manually review and commit the updated baseline:
```bash
git add .secrets.baseline
git commit -m "chore: update detect-secrets baseline"
```

### Running TruffleHog Locally

Scan your working directory for secrets before pushing:

```bash
# Install
pip install trufflesecurity

# Scan current branch against main
trufflehog git file://. --only-verified --base main --head HEAD

# Scan a specific directory
trufflehog filesystem . --only-verified
```

---

## Reporting a Vulnerability

To report a security vulnerability, please follow these steps:

1. **Do NOT** open a public GitHub issue for the vulnerability
2. Contact the security team via email: security@stellar-insights.example.com (replace with actual contact)
3. Include:
   - Description of the vulnerability
   - Steps to reproduce (if applicable)
   - Affected component(s)
   - Proposed fix (if any)
4. Allow 90 days for:
   - Initial acknowledgment (24-48 hours)
   - Triage and severity assessment (1 week)
   - Fix development (4-6 weeks)
   - Patch release and notification (remaining time)
5. Embargo: Do not disclose the vulnerability publicly until patch is released

### CVE and Public Disclosure

- Vulnerabilities with CVSS ≥ 7.0 (High/Critical) trigger CVE assignment
- Public disclosure date: After patch is released to all supported versions
- Security advisory published on GitHub Releases

### Responsible Disclosure (Bug Bounty)

Currently, we do not operate a formal bug bounty program, but security researchers who responsibly disclose vulnerabilities may be eligible for:
- Public credit/acknowledgment in security advisory
- Consideration for future bounty program

---

## Security Audit Preparation

### Architecture and Threat Modeling

See [ARCHITECTURE.md](security/ARCHITECTURE.md) for:
- System data flows and trust boundaries
- Sensitive data inventory
- External dependencies and risks
- Compliance posture (GDPR)
- Recommendations for external audit

### STRIDE Threat Model

See [THREAT_MODEL.md](security/THREAT_MODEL.md) for:
- Systematic threat enumeration (Spoofing, Tampering, Repudiation, Information Disclosure, DoS, Elevation of Privilege)
- Risk assessment and likelihood evaluation
- Existing controls and gaps
- Heat map of top risks
- OWASP Top 10 alignment

### Known Gaps and Future Work

See [KNOWN_GAPS.md](security/KNOWN_GAPS.md) for:
- Outstanding security issues identified during audit prep
- Risk acceptance matrix
- Timeline and effort estimates for remediation
- By-component and by-effort tracking

---

## Dependency and Supply Chain Security

### Cargo (Rust Backend)

**Status**: ⚠️ In Progress

- Dependency scanning: `cargo audit` runs in CI/CD
- Configuration: See `.github/workflows/security.yml` (if present)
- Known advisories: Check `cargo audit` output for current vulnerabilities
- Update policy: Security patches applied within 2 weeks; major updates quarterly

**Running locally**:
```bash
cd backend
cargo audit
```

### NPM (Frontend and Scripts)

**Status**: ⚠️ In Progress

- Dependency scanning: `npm audit` runs in CI/CD
- Configuration: See `.github/workflows/security.yml` (if present)
- Known advisories: Check `npm audit` output for current vulnerabilities
- Update policy: Security patches applied within 2 weeks; major updates quarterly

**Running locally**:
```bash
cd frontend
npm audit
```

### SBOM (Software Bill of Materials)

**Status**: 🟡 Partial (dependency-audit-2026-07-26.md exists)

- Maintained in: `docs/security/dependency-audit-*.md`
- Format: JSON/text list of all direct and transitive dependencies
- Updated: Quarterly or on major version upgrades
- Purpose: Enable rapid vulnerability assessment when CVEs are published

---

## Scope of Security Reviews

### Included in Security Scope

- Authentication and authorization
- Data protection and encryption
- API security (request signing, tampering prevention)
- GDPR compliance (data export, deletion, consent)
- Audit logging and forensics
- Secret management
- Input validation and injection prevention
- Error handling (information leakage)

### Out of Scope (External Services)

- Stellar Horizon API security (maintained by Stellar Foundation)
- AWS infrastructure security (AWS responsibility)
- Browser security and XSS prevention (frontend framework responsibility)
- TLS/PKI certificate management (AWS/DNS provider responsibility)

---

## Security Configuration

### Environment Variables (Secrets)

**Warning**: Never commit secrets to version control.

Required for production:
- `JWT_SECRET` — Signing key for JWT tokens (min 32 bytes, cryptographically random)
- `API_SIGNING_SECRET` — Key for HMAC-SHA256 request signing (min 32 bytes)
- `DATABASE_URL` — Database connection string (postgres:// or sqlite://)
- `REDIS_URL` — Redis connection string (redis://)
- `AWS_REGION` — AWS region for KMS, S3, etc.
- `AWS_KMS_KEY_ID` — Customer-managed KMS key for encryption

### TLS/HTTPS

- **Enforcement**: HSTS header (Strict-Transport-Security) with preload
- **Minimum version**: TLS 1.2
- **Certificates**: AWS Certificate Manager (auto-renew) or self-signed (dev only)
- **Test**: `curl -I https://api.example.com | grep Strict-Transport-Security`

### Rate Limiting

- **Login endpoint**: < 5 attempts per 15 minutes per IP
- **GDPR endpoints**: 1 request per minute per authenticated user
- **General API**: 100 requests per minute per authenticated user
- **Unauthenticated**: 10 requests per minute per IP

### IAM and Access Control

- **Database access**: Restricted to application IAM role (no direct admin access)
- **S3 buckets**: Restricted to application IAM role (list/get only for exports)
- **KMS keys**: Restricted to RDS service and application role
- **CloudWatch logs**: Restricted to application execution role

---

## Contact and Escalation

**Security Issues**: security@stellar-insights.example.com (replace with actual contact)  
**GDPR/Privacy**: privacy@stellar-insights.example.com (replace with actual contact)  
**Incident Response**: On-call security lead (contact via operations team)
