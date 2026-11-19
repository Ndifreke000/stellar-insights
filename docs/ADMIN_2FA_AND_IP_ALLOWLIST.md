# Admin Endpoints: Mandatory 2FA + IP Allowlist

**GitHub Issue**: [#2584](https://github.com/Ndifreke000/stellar-insights/issues/2584)  
**Estimated size**: S — roughly half a day to a day for someone familiar with the stack.

---

## Overview

Admin routes currently enforce an IP allowlist via `ip_whitelist_middleware.rs`. This document describes the additional requirement: admin calls must also carry a JWT with a valid `amr` (Authentication Methods Reference) claim that includes `otp`, confirming the token was issued after 2FA verification.

The two controls together — network-level IP allowlisting and token-level 2FA assertion — ensure that compromising either the network perimeter or the credential alone is insufficient to reach admin endpoints.

---

## Threat Model

| Threat | IP allowlist only | IP allowlist + 2FA claim |
|---|---|---|
| Admin credential stolen (password/token) | Attacker blocked by IP allowlist | Attacker still blocked |
| Admin's network is compromised (VPN breach, rogue device on the allowed subnet) | Admin endpoint reachable with any valid token | Token must also carry `amr=otp`; stolen token without 2FA is rejected |
| Admin token reused after session expiry (replay) | Blocked only if IP changed | `exp` + `amr` both validated |

**Who is the attacker?** An actor who has compromised a network path into the allowed IP range (e.g. via a VPN breach, a rogue device added to the corporate subnet, or a misconfigured proxy) and has obtained a valid admin JWT that was issued without 2FA. Without the `amr` check, they can reach admin routes. With it, the token is rejected unless it was issued after a successful TOTP or recovery-code verification.

---

## Affected Files

| File | Lines | Notes |
|---|---|---|
| `backend/src/ip_whitelist_middleware.rs` | 280 | Existing IP allowlist middleware |
| `backend/src/auth.rs` | 862 | JWT claims definitions and token issuance |
| `backend/src/middleware/jwt_token_refresh.rs` | 223 | Token refresh; must preserve `amr` claim |

---

## Implementation

### 1. `amr` claim in JWT

When a user completes 2FA verification (TOTP or recovery code), the issued full-scope access token must include:

```json
{
  "sub": "<user_id>",
  "exp": 1761436800,
  "iat": 1761433200,
  "amr": ["pwd", "otp"]
}
```

- `pwd` — password authentication was used.
- `otp` — a one-time password (TOTP or recovery code) was verified.

Tokens issued after password-only authentication (i.e. for accounts without 2FA enrolled, or during the pending-2FA window) must **not** include `otp` in `amr`.

### 2. Middleware: `amr` claim check

Create (or extend) a middleware that runs **after** the IP allowlist middleware on admin routes:

```
Request
  └─► IP allowlist check (existing)          → 403 if IP not allowed
  └─► JWT verification (existing)            → 401 if token invalid
  └─► amr_otp_required middleware (new)      → 403 if amr missing otp
  └─► Admin route handler
```

The `amr_otp_required` middleware:
1. Extracts the validated JWT claims from request context (already populated by the JWT middleware).
2. Checks whether the `amr` array contains `"otp"`.
3. If not present: returns `403 Forbidden` with a structured error response.
4. If present: passes the request to the next handler.

### 3. Route registration

Apply both middlewares to all admin route groups. Example (pseudo-code):

```rust
let admin_routes = Router::new()
    .nest("/admin", admin_handlers())
    .layer(amr_otp_required_middleware())   // new
    .layer(ip_whitelist_middleware())       // existing
    .layer(jwt_auth_middleware());          // existing
```

Middleware is applied inside-out in most frameworks — confirm correct ordering so IP check runs before JWT parsing, and `amr` check runs after JWT validation.

---

## API Behaviour Changes

### Request — admin endpoint without `otp` in `amr`

```
GET /admin/users
Authorization: Bearer <token without amr=otp>

Response: 403 Forbidden
{
  "error": "insufficient_authentication",
  "message": "Admin endpoints require two-factor authentication. Please re-authenticate with 2FA."
}
```

### Request — admin endpoint with `otp` in `amr`

```
GET /admin/users
Authorization: Bearer <token with amr=["pwd","otp"]>

Response: 200 OK
{ ... }
```

### Token refresh

When the token refresh endpoint issues a new access token, it must preserve the `amr` claim from the original token — **not** re-elevate a non-2FA token to include `otp`. Stripping `amr` on refresh would silently downgrade security.

---

## Configuration

No new environment variables are required. The middleware is always-on for admin routes. If a gradual rollout is needed, a feature flag `REQUIRE_ADMIN_2FA=true` can be used during the transition, defaulting to `true`.

---

## Acceptance Criteria

- [ ] An admin request with a valid IP and a JWT that lacks `otp` in `amr` returns `403 Forbidden`.
- [ ] An admin request with a valid IP and a JWT containing `amr=["pwd","otp"]` succeeds.
- [ ] A non-admin endpoint is not affected by this change.
- [ ] Token refresh preserves the `amr` claim; it does not elevate or strip it.
- [ ] CI is green (lint, type-check, tests).
- [ ] No new TODOs without a linked issue.

---

## Test Plan

### Unit tests (middleware logic)
- Token with `amr: ["pwd", "otp"]` → middleware passes the request through.
- Token with `amr: ["pwd"]` → middleware returns `403`.
- Token with no `amr` field → middleware returns `403`.
- Token with `amr: []` → middleware returns `403`.

### Integration tests
- Full request to `GET /admin/users` with a 2FA-verified token and an allowed IP → `200 OK`.
- Same request with a password-only token → `403 Forbidden`.
- Same request with a 2FA-verified token but a blocked IP → `403 Forbidden` (IP middleware fires first).

---

## Security Notes

- Deny-by-default: any token that does not explicitly carry `otp` in `amr` is rejected. Do not try to infer 2FA status from other claims.
- Never log the full JWT string; log only `kid` and `sub` for diagnostics.
- If the `amr` claim is absent entirely, treat it as equivalent to `[]` (rejected).
- This change does not affect non-admin routes; keep the middleware scoped to the admin route group.

---

## Audit Events

| Event | Trigger |
|---|---|
| `admin_access_denied_no_2fa` | Admin endpoint rejected because `amr` lacks `otp` |
| `admin_access_denied_ip` | Admin endpoint rejected by IP allowlist (existing) |
| `admin_access_granted` | Admin endpoint successfully accessed |

See [AUDIT_LOG.md](./AUDIT_LOG.md) for the audit log schema and [IP_WHITELIST.md](./IP_WHITELIST.md) for the existing IP allowlist documentation.

---

## References

- [RFC 8176 — Authentication Method Reference Values](https://datatracker.ietf.org/doc/html/rfc8176)
- [IP_WHITELIST.md](./IP_WHITELIST.md)
- [TWO_FACTOR_AUTHENTICATION.md](./TWO_FACTOR_AUTHENTICATION.md)
- [AUDIT_LOG.md](./AUDIT_LOG.md)
- [SECRETS_MANAGEMENT.md](./SECRETS_MANAGEMENT.md)
