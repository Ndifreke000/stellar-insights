# JWT Signing Key Rotation with `kid` Support

**GitHub Issue**: [#2583](https://github.com/Ndifreke000/stellar-insights/issues/2583)  
**Estimated size**: M — roughly 2–4 days; a short design note in the PR is recommended before implementation.

---

## Overview

This document describes how to add graceful JWT signing key rotation to the Stellar Insights backend so that keys can be cycled without logging out active users or causing downtime.

The implementation introduces:
- A `kid` (Key ID) header in every issued JWT.
- An in-process key ring that holds the **active** key and up to N **previous** keys.
- A public `/.well-known/jwks.json` endpoint for external verifiers.

---

## Threat Model

| Threat | Without key rotation | After key rotation |
|---|---|---|
| A signing key is compromised | All existing tokens remain valid indefinitely | Compromised key can be retired; its tokens expire naturally |
| Long-lived keys increase exposure window | Key in use for months or years | Keys rotated on a schedule; exposure window bounded |
| Forced logout during key rotation | — | Previous key stays in the key ring until tokens signed by it expire |

**Who is the attacker?** An insider or external actor who obtains the JWT signing secret (e.g. via a leaked `.env`, a compromised CI pipeline, or a misconfigured secrets manager). Without rotation, they can forge valid tokens indefinitely. With rotation, the window of validity is bounded by the token TTL of any tokens already in circulation at the time of rotation.

---

## Affected Files

| File | Lines | Notes |
|---|---|---|
| `backend/src/auth.rs` | 862 | Token issuance and verification logic |
| `backend/src/middleware/jwt_token_refresh.rs` | 223 | Token refresh middleware |

---

## Design

### Key Ring

The key ring is a small, in-process structure:

```
KeyRing {
    active:   SigningKey,       // used to sign all new tokens
    previous: Vec<SigningKey>,  // accepted for verification only
}
```

Each `SigningKey` contains:
- `kid`: a stable, unique identifier (e.g. a UUID or a timestamp-based ID like `2026-10-18T00:00:00Z`)
- `secret`: the HMAC secret or RSA/ECDSA private key material
- `algorithm`: `HS256`, `RS256`, `ES256`, etc.
- `created_at`: timestamp (for TTL reasoning)

### Token Issuance

When issuing a JWT:
1. Select the **active** key from the ring.
2. Set the `kid` claim in the JOSE header: `{ "alg": "HS256", "kid": "<active-key-id>" }`.
3. Sign with the active key's secret.

### Token Verification

When verifying a JWT:
1. Decode the header without verification to extract `kid`.
2. Look up `kid` in the key ring (active + all previous keys).
3. If found, verify the signature with that key's secret.
4. If not found, reject with `401 Unauthorized`.

This means tokens signed by the previous key remain valid until they naturally expire — no forced logout.

### Key Rotation Process

1. Generate a new signing key and assign it a unique `kid`.
2. Demote the current active key to `previous`.
3. Promote the new key to active.
4. Remove previous keys whose `created_at` is older than the maximum token TTL (e.g. if access tokens live 1 hour, previous keys can be pruned after 1 hour).

---

## JWKS Endpoint

Expose public key material at:

```
GET /.well-known/jwks.json
```

Response (example for RS256):
```json
{
  "keys": [
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "2026-10-18T00:00:00Z",
      "alg": "RS256",
      "n": "<base64url modulus>",
      "e": "AQAB"
    },
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "2026-07-01T00:00:00Z",
      "alg": "RS256",
      "n": "<base64url modulus>",
      "e": "AQAB"
    }
  ]
}
```

For **HMAC** (`HS256`), the JWKS endpoint should **only** be accessible to internal services or omitted entirely, since HMAC key material is symmetric and secret. In that case, expose only the `kid` values (not the key bytes) so clients can detect an unknown key.

The endpoint:
- Must not require authentication (it is a public discovery document for RS256/ES256).
- Should include all keys currently in the ring (active + previous).
- Should be cached with a short TTL (e.g. `Cache-Control: max-age=300`).

---

## Configuration

Add to `.env` / secrets manager:

```
# Comma-separated JSON objects or a path to a key ring file
JWT_KEY_RING='[{"kid":"2026-10-18","secret":"<hex>","alg":"HS256","created_at":"2026-10-18T00:00:00Z"}]'

# Maximum number of previous keys to retain
JWT_KEY_RING_MAX_PREVIOUS=2

# Token TTL (used to determine when previous keys can be pruned)
JWT_ACCESS_TOKEN_TTL_SECONDS=3600
```

---

## API / Behaviour Changes

| Behaviour | Before | After |
|---|---|---|
| JWT header | `{ "alg": "HS256" }` | `{ "alg": "HS256", "kid": "<key-id>" }` |
| Verification | Single key from env | Key ring lookup by `kid` |
| Unknown `kid` | — | `401 Unauthorized` |
| JWKS discovery | Not available | `GET /.well-known/jwks.json` |

**Migration path for existing clients**: Tokens issued before this change carry no `kid` header. For a safe rollover period, fall back to the legacy single-key verification when `kid` is absent. Remove the fallback after all pre-rotation tokens have expired (one token TTL after the deploy).

---

## Acceptance Criteria

- [ ] All newly issued tokens include a `kid` header.
- [ ] Tokens signed by the previous key remain valid until their `exp` claim is reached.
- [ ] Tokens with an unrecognised `kid` are rejected with `401`.
- [ ] `GET /.well-known/jwks.json` returns all currently active key IDs.
- [ ] Rotating the active key does not force users to re-authenticate.
- [ ] CI is green (lint, type-check, tests).
- [ ] No new TODOs without a linked issue.

---

## Test Plan

### Rotation test (integration)
1. Issue a token with key A (active).
2. Rotate: promote key B to active, demote A to previous.
3. Verify the token from step 1 still validates successfully (A is in previous).
4. Issue a new token — verify it is signed with key B.
5. Prune key A from the ring.
6. Verify the old token from step 1 now fails validation.
7. Present a token with an invented `kid` — verify `401` is returned.

### JWKS test
1. Fetch `/.well-known/jwks.json`.
2. Assert both active and previous `kid` values appear.
3. After pruning a previous key, assert it no longer appears.

---

## Security Notes

- Never log signing secrets, token payloads, or full JWT strings.
- Store key material in the secrets manager (Vault / AWS Secrets Manager / `.env` encrypted at rest), not in the database.
- If using HMAC, treat the JWKS endpoint as internal-only.
- Prefer asymmetric keys (RS256 / ES256) for public JWKS — this allows external services to verify tokens without access to the private key.
- The key ring should be initialised at startup; prefer panicking on misconfiguration over silently using an insecure default.

---

## References

- [RFC 7515 — JSON Web Signature (JWS)](https://datatracker.ietf.org/doc/html/rfc7515)
- [RFC 7517 — JSON Web Key (JWK)](https://datatracker.ietf.org/doc/html/rfc7517)
- [RFC 7519 — JSON Web Token (JWT)](https://datatracker.ietf.org/doc/html/rfc7519)
- [SECRETS_MANAGEMENT.md](./SECRETS_MANAGEMENT.md)
- [SESSION_MANAGEMENT.md](./SESSION_MANAGEMENT.md)
- [AUDIT_LOG.md](./AUDIT_LOG.md)
