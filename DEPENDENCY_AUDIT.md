# Dependency Security Audit Results

**Date:** 2026-02-22
**Tools:** cargo-audit v0.22.1, pnpm audit

---

## Backend (Rust) - cargo audit

### Vulnerabilities

| Severity | Crate | Version | Advisory | Description | Fix Available |
|----------|-------|---------|----------|-------------|---------------|
| **Medium (5.9)** | `rsa` | 0.9.10 | RUSTSEC-2023-0071 | Marvin Attack: potential key recovery through timing sidechannels | No fix available |

**Note:** `rsa` is a transitive dependency via `sqlx-mysql`. Since the project uses SQLite/PostgreSQL (not MySQL), the affected code path is not exercised. However, it ships in the binary.

**Mitigation:** Disable the `mysql` feature in `sqlx` if not needed:
```toml
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "postgres"], default-features = false }
```

### Warnings (Unmaintained Crates)

| Crate | Version | Advisory | Status | Recommendation |
|-------|---------|----------|--------|----------------|
| `dotenv` | 0.15.0 | RUSTSEC-2021-0141 | Unmaintained | Migrate to `dotenvy` (maintained fork) |
| `proc-macro-error` | 1.0.4 | RUSTSEC-2024-0370 | Unmaintained | Transitive via `utoipa-gen`; update `utoipa` to v5.x |

### Outdated Dependencies (Security-Relevant)

| Crate | Current | Latest | Notes |
|-------|---------|--------|-------|
| `jsonwebtoken` | 9.3.1 | 10.3.0 | JWT handling - check changelog for security fixes |
| `stellar-xdr` | 21.2.0 | 25.0.0 | Stellar protocol - major version behind |
| `redis` | 0.25.4 | 1.0.4 | Major version behind |
| `tower` | 0.4.13 | 0.5.3 | Middleware framework |
| `utoipa` | 4.2.3 | 5.4.0 | OpenAPI docs - would fix `proc-macro-error` warning |

---

## Frontend (JavaScript/TypeScript) - pnpm audit

### High Severity (2 unique, 3 total)

| Severity | Package | Vulnerable Versions | Patched | Path | Advisory |
|----------|---------|-------------------|---------|------|----------|
| **High** | `next` | >=16.1.0-canary.0 <16.1.5 | >=16.1.5 | Direct | GHSA-h25m-26qc-wcjf - HTTP request deserialization DoS with insecure React Server Components |
| **High** | `minimatch` | <10.2.1 | >=10.2.1 | eslint>minimatch, eslint-config-next>...>minimatch | GHSA-3ppc-4f35-3m26 - ReDoS via repeated wildcards |

### Moderate Severity (8 total)

| Package | Count | Patched | Path | Advisories |
|---------|-------|---------|------|------------|
| `next` | 2 | >=16.1.5 | Direct | GHSA-9g9p-9gw9-jx7f (Image Optimizer DoS), GHSA-5f7q-jpqc-wp7h (PPR endpoint memory) |
| `hono` | 4 | >=4.11.7 | prisma>@prisma/dev>hono | GHSA-9r54-q6cx-xmh5 (XSS), GHSA-6wqw-2p9w-4vw4 (Cache Deception), GHSA-r354-f388-2fhh (IP spoofing), GHSA-w332-q679-j88p (Arbitrary key read) |
| `lodash` | 1 | >=4.17.23 | prisma>@prisma/dev>...>lodash | GHSA-xxjr-mmjv-4gpg (Prototype Pollution) |
| `ajv` | 1 | >=6.14.0 | eslint>ajv | GHSA-2g4f-4pwh-qvx6 (ReDoS) |

### Low Severity (1)

| Package | Patched | Path | Advisory |
|---------|---------|------|----------|
| `hono` | >=4.11.10 | prisma>@prisma/dev>hono | GHSA-gq3j-xvxp-8hrf (timing comparison hardening) |

---

## Remediation Plan

### Immediate Actions (Before Audit)

1. **Update `next` to >=16.1.5** - Fixes 3 vulnerabilities (1 high, 2 moderate) in direct dependency
   ```bash
   cd frontend && pnpm update next
   ```

2. **Update `eslint` and `minimatch`** - Fixes ReDoS vulnerabilities
   ```bash
   cd frontend && pnpm update eslint minimatch
   ```

### Short-Term Actions

3. **Migrate `dotenv` to `dotenvy`** in backend Cargo.toml
   ```toml
   # Replace: dotenv = "0.15"
   # With:
   dotenvy = "0.15"
   ```

4. **Disable unused `sqlx` features** to remove `rsa` dependency
   ```toml
   sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "postgres", "migrate"], default-features = false }
   ```

5. **Update `prisma`** to get patched `hono` and `lodash` versions

### Long-Term Actions

6. **Update `utoipa` to v5.x** to resolve `proc-macro-error` unmaintained warning
7. **Update `jsonwebtoken` to v10.x** for latest security improvements
8. **Update `stellar-xdr` to v25.x** for latest protocol support
9. **Update `redis` to v1.x** for latest fixes and improvements

---

## Summary

| Component | Critical | High | Medium | Low | Total |
|-----------|---------|------|--------|-----|-------|
| Backend (Rust) | 0 | 0 | 1 | 0 | 1 (+2 warnings) |
| Frontend (JS) | 0 | 3 | 8 | 1 | 12 |
| **Total** | **0** | **3** | **9** | **1** | **13** |

**Overall Risk Assessment:** No critical vulnerabilities. High-severity issues in `next` and `minimatch` should be patched before audit. The `rsa` vulnerability is not in an active code path but should be eliminated by disabling unused `sqlx` features.
