# Security Testing Guide

## Local Security Testing

This guide covers running security tests locally before committing code.

### Prerequisites

```bash
# Install security scanning tools
cargo install cargo-audit

# For OWASP ZAP scanning (Docker required)
docker pull owasp/zap2docker-stable

# For container scanning
docker pull aquasec/trivy

# For Kubernetes config scanning
docker pull aquasec/kubesec
```

### 1. Dependency Scanning (Cargo Audit)

Scans Rust dependencies for known vulnerabilities:

```bash
cd backend
cargo audit

# Deny any warnings (fail the build)
cargo audit --deny warnings

# Check for unmaintained dependencies
cargo audit --deny unmaintained
```

**Output:** Lists CVE-affected crates with remediation guidance

### 2. Static Analysis (Clippy)

Catches common mistakes and security issues:

```bash
cd backend

# Full lint pass
cargo clippy -- -W clippy::all

# Security-focused lints
cargo clippy -- -W clippy::unwrap_used -W clippy::panic -W clippy::unimplemented

# Fix automatically where possible
cargo clippy --fix
```

### 3. Build & Test

Ensure tests pass before submitting PR:

```bash
cd backend

# Run unit tests
cargo test

# Run with all features enabled
cargo test --all-features

# Run security-specific tests
cargo test vault::
cargo test crypto::
```

### 4. OWASP ZAP Baseline Scan (Staging Only)

Automated API scanning against a running staging instance:

```bash
# Requires staging environment running

# Run baseline (quick, ~5 minutes)
docker run -t --rm owasp/zap2docker-stable \
  zap-baseline.py -t https://staging-api.payraider.internal \
  -J /tmp/zap-report.json

# Check findings
cat /tmp/zap-report.json | jq '.site[0].alerts[] | select(.riskcode >= 2)'
```

### 5. Container Image Scanning

Scan Docker images for vulnerabilities:

```bash
# Build the backend image
cd backend
docker build -t payraider-backend:dev .

# Scan with Trivy
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
  aquasec/trivy image payraider-backend:dev

# Show only HIGH and CRITICAL
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
  aquasec/trivy image --severity HIGH,CRITICAL payraider-backend:dev
```

### 6. Kubernetes Config Validation

Verify Kubernetes manifests for misconfigurations:

```bash
# Run kubesec scan
docker run -v $(pwd)/k8s:/k8s aquasec/kubesec scan /k8s/**/*.yaml

# Check score (>5 is acceptable)
docker run -v $(pwd)/k8s:/k8s aquasec/kubesec scan -f json /k8s/**/*.yaml \
  | jq '.[] | select(.score < 5)'
```

### 7. Smart Contract Testing

For Soroban contract code:

```bash
cd contracts

# Run contract tests
cargo test

# Build contract
stellar contract build --docker

# Run static analysis (if available)
soroban lint .
```

## CI/CD Security Scanning

Security scans run automatically on:

1. **Pull Requests:** Basic checks (Clippy, cargo-audit, code review)
2. **On Merge:** Full test suite passes
3. **Nightly:** OWASP ZAP baseline against staging
4. **Weekly:** Full ZAP scan, container scan, K8s config audit
5. **Monthly:** Manual security review + contract audit

### GitHub Actions Status

View CI/CD security scan results:

```bash
# List all security workflows
gh workflow list --all | grep -i security

# View latest run
gh run view --workflow=security-scan.yml

# Download scan artifacts
gh run download <run-id> --name scan-results
```

## False Positive Management

Some security scanners produce false positives. To suppress a known safe finding:

1. **Verify it's safe:** Confirm the finding doesn't represent a real vulnerability
2. **Document the reason:** Add comment explaining why it's safe
3. **Add to suppressions:**

```yaml
# backend/.zap-suppressions.yaml
- alert_id: 40016
  rule: "Cookie No HttpOnly Flag"
  url: "/api/v1/health"
  reason: "JWT stored in memory, not cookies (CORS API)"
  verified_by: "@security-lead"
  date: "2026-08-26"
```

## Responding to Findings

### High/Critical Findings

1. **Create Issue:** GitHub issue with CVSS score and scanner output
2. **Assign:** Route to appropriate team based on component
3. **Target:** Fix within SLA (24h for Critical, 3d for High)
4. **Verify:** Rescan to confirm fix
5. **Deploy:** Include fix in next release

Example response:

```
Issue: [SECURITY] XSS in error message display
Severity: MEDIUM (CVSS 5.2)
Scanner: OWASP ZAP

Finding: Error messages from API reflect user input without sanitization
Status: CONFIRMED (false positive in test mode, affects real deploy)

Fix:
- Sanitize error messages before returning to client
- Use structured error responses (no plaintext)
- Add test case for XSS vector

PR: #XXXX
```

### Low/Informational Findings

- Review during next sprint planning
- May be deferred if low exploitability
- Document tradeoffs in decision log

## Security Checklist Before Submitting PR

- [ ] `cargo audit` passes with no warnings
- [ ] `cargo clippy -- -W clippy::all` passes
- [ ] All tests pass: `cargo test`
- [ ] No secrets/API keys committed
- [ ] API endpoints require authentication (if applicable)
- [ ] Error messages don't leak PII
- [ ] Input validation in place for user inputs
- [ ] Database queries use parameterized queries (ORM)
- [ ] Dependencies updated to latest safe versions

## Escalation Procedures

**Critical Finding Discovered:**

1. Do NOT commit/push if security impact is immediate
2. Contact security lead immediately (email + Slack)
3. Discuss remediation strategy
4. File confidential issue (mark as security)
5. Remediate in hotfix branch
6. Fast-track review and deployment

**Contact:** security@payraider.internal

## Tools & References

- [Cargo Audit Documentation](https://docs.rs/cargo-audit/latest/cargo_audit/)
- [Clippy Lints](https://doc.rust-lang.org/clippy/)
- [OWASP ZAP Documentation](https://www.zaproxy.org/docs/)
- [Trivy Scanner](https://aquasecurity.github.io/trivy/)
- [Kubesec](https://kubesec.io/)
- [Soroban Security Docs](https://developers.stellar.org/docs/smart-contracts/overview/security)
