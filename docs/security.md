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

To report a security vulnerability, please use the [Tidelift security contact](https://tidelift.com/security). Tidelift will coordinate the fix and disclosure.
