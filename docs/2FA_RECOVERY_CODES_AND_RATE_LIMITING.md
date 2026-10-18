# 2FA: Recovery Codes and Rate-Limit Verification

**GitHub Issue**: [#2582](https://github.com/Ndifreke000/stellar-insights/issues/2582)  
**Estimated size**: S — roughly half a day to a day for someone familiar with the stack.

---

## Overview

This document describes the implementation requirements for two hardening measures on the existing 2FA system:

1. **Single-use hashed recovery codes** — allow account recovery when an authenticator device is lost.
2. **TOTP brute-force rate limiting** — throttle failed verification attempts to prevent exhaustive guessing of 6-digit codes.

Both gaps address concrete, exploitable weaknesses in the current `twofa.rs` implementation.

---

## Threat Model

| Threat | Without this fix | After this fix |
|---|---|---|
| User loses authenticator device | Permanently locked out | Can log in with a recovery code |
| Attacker guesses 6-digit TOTP codes | ~1 million attempts possible | Locked out after 5 attempts / 15 min |
| Stolen database reveals recovery codes | — | Codes are hashed; not directly usable |

---

## Affected Files

| File | Lines | Notes |
|---|---|---|
| `backend/src/twofa.rs` | 310 | Core 2FA service logic |
| `backend/src/api/twofa.rs` | 289 (L242–253 relevant) | Route handlers |

---

## Implementation Requirements

### 1. Recovery Codes

#### Generation
- Generate **10 single-use recovery codes** per user at enrollment time and on explicit regeneration.
- Each code should be a cryptographically random value (e.g., 16 hex characters or a formatted alphanumeric string).
- **Hash** each code (bcrypt or Argon2id recommended; SHA-256 acceptable as a minimum) before persisting. The raw codes are returned to the user exactly once and never stored in plaintext.
- Invalidate the previous set of codes when regeneration is triggered (the existing `generate_backup_codes` already deletes old codes before inserting new ones — preserve this behaviour).

#### Storage schema (existing `user_2fa_backup_codes` table)
```sql
CREATE TABLE user_2fa_backup_codes (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL,
    hashed_code TEXT NOT NULL UNIQUE,  -- hashed recovery code
    used_at     TIMESTAMP,             -- NULL = unused; set on first use
    created_at  TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES user_2fa_secrets(user_id)
);
```

#### Verification logic
1. Accept a recovery code from the user.
2. Retrieve all unused codes for that user.
3. Compare the submitted code against each hash using constant-time comparison.
4. On match: set `used_at = NOW()` and complete authentication.
5. On no match: increment the attempt counter (see rate limiting below).

#### API endpoints

```
POST /api/auth/2fa/backup-code
Authorization: Bearer <limited-scope token>
{
  "code": "abc123def456ghi7"
}

Response 200:
{
  "message": "Recovery code verified.",
  "access_token": "<full-scope token>"
}

Response 401:
{
  "error": "invalid_recovery_code",
  "message": "The code is invalid or has already been used."
}

Response 429:
{
  "error": "too_many_attempts",
  "message": "Too many failed attempts. Try again after 15 minutes.",
  "retry_after": 900
}
```

```
POST /api/auth/2fa/regenerate-backup
Authorization: Bearer <full-scope token>

Response 200:
{
  "backup_codes": ["abc123...", "def456...", ...],  // 10 codes, shown once
  "message": "Recovery codes regenerated. Store these in a safe place."
}
```

---

### 2. Rate Limiting

#### Scope
Rate limiting applies to **both** TOTP verification (`POST /api/auth/2fa/verify`) and recovery code verification (`POST /api/auth/2fa/backup-code`).

#### Parameters
| Parameter | Value |
|---|---|
| Max failed attempts | 5 |
| Window | 15 minutes |
| Lockout duration | 15 minutes (sliding from last failed attempt) |
| Key | `user_id` + session identifier |

#### Response headers
On every response from the 2FA verification endpoints:
```
X-RateLimit-Limit: 5
X-RateLimit-Remaining: <remaining attempts>
X-RateLimit-Reset: <Unix timestamp when window resets>
```

On lockout (`429 Too Many Requests`):
```
Retry-After: 900
```

#### Storage
Use Redis (preferred) or the existing SQLite/Postgres DB with a `twofa_attempt_log` table:
```sql
CREATE TABLE twofa_attempt_log (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL,
    attempted_at TIMESTAMP NOT NULL,
    succeeded   BOOLEAN NOT NULL
);
```
Query: count `succeeded = false` in the last 15 minutes for the user. If ≥ 5, reject.

---

## Acceptance Criteria

- [ ] A user with a lost authenticator can log in using a recovery code.
- [ ] Each recovery code can only be used once; reuse is rejected with `401`.
- [ ] Recovery codes are stored as hashes — raw codes are never persisted.
- [ ] After 5 failed TOTP or recovery-code attempts within 15 minutes, further attempts return `429` with `Retry-After: 900`.
- [ ] A successful verification resets the failed-attempt counter.
- [ ] Existing tests pass unchanged.
- [ ] CI is green (lint, type-check, tests).
- [ ] No new TODOs without a linked issue.

---

## Audit Events

Extend the existing audit log with:

| Event | Trigger |
|---|---|
| `2fa_recovery_code_used` | Successful recovery code verification |
| `2fa_recovery_code_failed` | Failed recovery code attempt |
| `2fa_rate_limit_triggered` | Lockout imposed on a user |
| `2fa_backup_regenerated` | User regenerates recovery codes |

See [AUDIT_LOG.md](./AUDIT_LOG.md) for the audit log schema.

---

## Security Notes

- Use constant-time comparison when checking hashed codes to avoid timing attacks.
- Never log raw recovery codes or TOTP secrets. See project logging guidelines.
- Rate limit keys must be scoped to the user, not just the IP, to prevent evasion via IP rotation.
- If rate limiting is backed by Redis, handle Redis unavailability with a fail-secure fallback (deny the request or fall back to DB-backed counting).

---

## References

- [IETF RateLimit header fields draft](https://datatracker.ietf.org/doc/draft-ietf-httpapi-ratelimit-headers/)
- [Soroban state archival / TTL](https://developers.stellar.org/docs/learn/encyclopedia/storage/state-archival)
- [TWO_FACTOR_AUTHENTICATION.md](./TWO_FACTOR_AUTHENTICATION.md) — existing 2FA documentation
- [AUDIT_LOG.md](./AUDIT_LOG.md)
