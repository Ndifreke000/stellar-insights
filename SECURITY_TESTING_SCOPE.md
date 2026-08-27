# Security Testing Scope & Framework

## Overview

This document defines the attack surface for stellar-insights security testing and the continuous automated penetration testing framework.

## Attack Surface Scope

### 1. Backend API (REST/RPC)

**Endpoints Covered:**
- Authentication & Authorization
  - `/api/v1/auth/*` - Session management, JWT validation
  - `/api/v1/login` - Credential validation
  - `/api/v1/logout` - Session revocation
  
- API Key Management
  - `/api/v1/api-keys/*` - CRUD operations
  - Scopes and permissions validation

- Data Access Endpoints
  - `/api/v1/corridors/*` - Market corridor data
  - `/api/v1/assets/*` - Asset information
  - `/api/v1/metrics/*` - Performance metrics
  - `/api/v1/transactions/*` - Transaction history
  
- Admin Endpoints
  - `/api/v1/admin/*` - Admin-only operations
  - Role-based access control (RBAC) enforcement

**Threat Model:**
- SQLi: Parameterized queries + ORM reduce risk, but scan anyway
- XSS: JSON responses (not HTML), but verify in error messages
- Broken Authentication: JWT expiry, token refresh, revocation
- Broken Authorization: IDOR (accessing resources by ID), privilege escalation
- Rate Limiting: DDoS mitigation effectiveness
- Input Validation: File uploads, query parameters, body payloads
- CORS: Cross-origin request handling

### 2. Authentication & Authorization Flows

**Areas of Focus:**
- JWT Token Validation
  - Expired tokens rejection
  - Invalid signatures caught
  - Key rotation handled transparently
  
- Session Management
  - Session hijacking prevention
  - Concurrent session limits
  - IP whitelist enforcement (#2135)
  - 2FA/MFA flows (#2203)
  
- OAuth Integration (if applicable)
  - Token exchange vulnerability (CSRF, state parameter)
  - Refresh token handling
  - Scope limitations

### 3. Soroban Smart Contracts

**Scope:**
- Contract authorization logic
- Reentrancy-equivalent vulnerabilities (cross-contract calls)
- Integer overflow/underflow in calculations
- State manipulation via malicious inputs
- Access control in contract ACL (#2140)
- Emergency stop mechanism (#2141)

**Tools:**
- Soroban static analysis tools
- Fuzzing on contract state transitions
- Symbolic execution for numeric overflow

### 4. Infrastructure (Kubernetes Manifests #2143)

**Configuration Checks:**
- Pod security policies (PSP)
- Network policies (ingress/egress)
- RBAC configuration
- Secret management (no hardcoded secrets)
- Resource limits (CPU, memory)
- Container image scanning
- TLS certificate validity

**Tools:**
- `kube-bench` - CIS Kubernetes Benchmark
- `kubesec` - K8s manifest security scoring
- `trivy` - Container image vulnerability scanning

### 5. Dependency Vulnerabilities (SCA)

**Coverage:**
- Rust crates (Cargo.lock)
- JavaScript/Node packages (if any)
- Container base images
- OS-level packages

**Tools:**
- `cargo audit` - Rust dependency scanning
- `npm audit` / `yarn audit` - JS dependency scanning (if applicable)
- `snyk` - Multi-language SCA
- Trivy - OCI container scanning

## Severity Classification

| Severity | CVSS Score | Response SLA | Definition |
|----------|-----------|-------------|-----------|
| **Critical** | 9.0-10.0 | 24 hours | Immediate RCE, auth bypass, full data compromise |
| **High** | 7.0-8.9 | 3 days | Significant access escalation, data leakage |
| **Medium** | 4.0-6.9 | 2 weeks | Partial compromise, information disclosure |
| **Low** | 0.1-3.9 | 30 days | Minor issues, low exploitability |
| **Informational** | N/A | N/A | Best practices, configuration recommendations |

## Automated Testing Framework

### 1. Scanning Tools & Configuration

#### OWASP ZAP (API/Web Scanning)

**Configuration:** `backend/security/zap-config.yaml`
```yaml
# Baseline scan: quick checks on every PR
profiles:
  baseline:
    timeout: 30  # minutes
    rules:
      - XSS
      - SQLi
      - Path Traversal
      - Weak Authentication
    
  full:
    timeout: 120  # minutes
    rules: all  # All 120+ OWASP ZAP checks
    scan_depth: 3
    request_timeout: 45

target:
  url: https://staging-api.stellar-insights.internal
  auth:
    method: api_key
    header: Authorization
    value: ${ZAPSCAN_TOKEN}
```

#### Cargo Audit (Rust)

```bash
cd backend
cargo audit --deny warnings
cargo clippy -- -W clippy::all
```

#### Kube-Bench (K8s)

```bash
kube-bench run --profile cis-1.23
```

### 2. CI/CD Integration

#### GitHub Actions Workflow (`.github/workflows/security-scan.yml`)

```yaml
name: Security Scanning

on:
  schedule:
    # Full scan weekly (Sundays at 2 AM UTC)
    - cron: '0 2 * * 0'
    # Baseline scan nightly
    - cron: '0 3 * * *'
  pull_request:
    paths:
      - 'backend/src/**'
      - 'contracts/**'
      - 'k8s/**'

jobs:
  dependency-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo audit --deny warnings
      - run: npm audit (if applicable)

  sast-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo clippy -- -W clippy::all
      - run: soroban build && soroban test

  dast-scan:
    runs-on: ubuntu-latest
    if: github.event_name == 'schedule'  # Full scan only on schedule
    steps:
      - uses: actions/checkout@v3
      - name: Run ZAP Baseline
        uses: zaproxy/action-baseline@v0.7.0
        with:
          target: https://staging-api.stellar-insights.internal
          rules_file_name: '.zap-rules.tsv'
          
  container-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Scan with Trivy
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: 'stellar-insights-backend:latest'
          
  k8s-config-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: |
          docker run -v $(pwd)/k8s:/k8s aquasec/trivy config /k8s
          docker run -v $(pwd)/k8s:/k8s kubesec scan /k8s/**/*.yaml
```

### 3. Baseline Scan & Triage

#### Running Locally

```bash
# Install tools
cargo install cargo-audit
docker pull owasp/zap2docker-stable
docker pull aquasec/trivy
docker pull aquasec/kube-bench

# Run baseline against staging
./security/run-baseline-scan.sh staging

# Output: scan-report-$(date +%Y%m%d).json
# File contains all findings with CVSS scores
```

#### Triage Process

1. **Collect Findings:** Parse scanner output into structured format
2. **Categorize:** Classify by severity and type
3. **Verify:** Manual verification of scanner findings (high false positive rate)
4. **Track:** File issues in GitHub with:
   - Scanner name and version
   - CVSS score
   - Affected component
   - Reproduction steps (if applicable)
   - Proposed remediation

#### Example Triage Issue

```
Title: [SECURITY] SQL Injection in /api/v1/transactions endpoint
Severity: HIGH (CVSS 7.5)
Scanner: OWASP ZAP
Status: False Positive (parameterized queries in use)

Description: ZAP detected potential SQL injection in query parameter.
Verification: False positive — using SQLx with parameterized queries.
Mitigation: N/A (properly defended)
Assigned: @backend-team
```

### 4. False Positive Management

**Strategy:** Suppress known safe findings with documented justification

```yaml
# backend/security/zap-suppressions.yaml
suppressions:
  - rule_id: "40016"  # "Cookie No HttpOnly Flag"
    justification: "JWT stored in memory, not cookies (CORS API)"
    verified_by: "@security-lead"
    ticket: "#XXXX"
    
  - rule_id: "10010"  # "Buffer Overflow"
    justification: "Rust memory safety guarantees; false positive"
    verified_by: "@backend-lead"
```

### 5. Scheduled Scans

| Schedule | Scope | Tools | Retention |
|----------|-------|-------|-----------|
| **On Every PR** | Code only | Clippy, cargo-audit | Pass/Fail (no artifact) |
| **Nightly** | API baseline | ZAP (30 min) | Artifact (90 days) |
| **Weekly** | Full API + K8s | ZAP full, kube-bench, trivy | Artifact (1 year) |
| **Monthly** | Contract audit | Soroban tools + fuzzing | Report (on-demand review) |

## Scanner Credentials & Secrets

**CRITICAL:** Scanner tools must never expose credentials

```bash
# Store in GitHub Secrets
ZAPSCAN_TOKEN=<read-only API key for staging>  # from vault
TRIVY_GITHUB_TOKEN=<personal access token>      # for GH API rate limits
SNYK_TOKEN=<snyk integration token>             # from vault

# Export only to scanner steps
export ZAPSCAN_TOKEN="${{ secrets.ZAPSCAN_TOKEN }}"
```

## Reporting & Dashboards

### Scan Report Format

```json
{
  "scan_date": "2026-08-26",
  "scanner": "owasp-zap",
  "target": "https://staging-api.stellar-insights.internal",
  "summary": {
    "total_alerts": 12,
    "critical": 0,
    "high": 2,
    "medium": 5,
    "low": 5
  },
  "findings": [
    {
      "id": "40016",
      "name": "Cookie No HttpOnly Flag",
      "severity": "low",
      "cvss": 2.5,
      "url": "/api/v1/health",
      "parameter": "Set-Cookie",
      "justification": "false_positive",
      "status": "acknowledged"
    }
  ]
}
```

### Metrics to Track

- Trend of findings over time (should be decreasing)
- Mean time to remediation (MTTR) by severity
- Scanner accuracy (false positive rate)
- Coverage (% of attack surface scanned)

## Documentation

### For Contributors

- `backend/SECURITY_TESTING.md` - Local testing guide
- `backend/.zap-rules.tsv` - Suppressed rules and justifications
- `.github/workflows/security-scan.yml` - CI pipeline

### For Operations

- Runbook for responding to high-severity findings
- Escalation procedures
- Vault access for scanner credentials

## References

- [OWASP Testing Guide](https://owasp.org/www-project-web-security-testing-guide/)
- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [CWE Top 25](https://cwe.mitre.org/top25/)
- [Soroban Security Best Practices](https://developers.stellar.org/docs/smart-contracts/overview/security)
