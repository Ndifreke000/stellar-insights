# 2FA: WebAuthn/Passkeys as a Second Factor, and Recovery Codes

**GitHub Issue**: [#2852](https://github.com/Ndifreke000/stellar-insights/issues/2852)
**Related**: [TWO_FACTOR_AUTHENTICATION.md](./TWO_FACTOR_AUTHENTICATION.md), [2FA_RECOVERY_CODES_AND_RATE_LIMITING.md](./2FA_RECOVERY_CODES_AND_RATE_LIMITING.md) (#2582), [ADMIN_2FA_AND_IP_ALLOWLIST.md](./ADMIN_2FA_AND_IP_ALLOWLIST.md) (#2584)

---

## Overview

TOTP is the only second factor today. TOTP codes can be phished: a fake login page relays the code to the real site within its 30-second window. WebAuthn (passkeys and hardware security keys) cannot be phished this way, because the browser binds every assertion to the origin that asked for it. Phishing-resistant factors are the expected baseline for admin and billing accounts.

This issue adds:

1. **WebAuthn registration and authentication** as a second factor, using [`webauthn-rs`](https://docs.rs/webauthn-rs).
2. **Recovery codes that hold up**: high-entropy, salted and slow-hashed, and single-use. The current backup codes do not meet this bar (see below).
3. **Mandatory 2FA for admins**, where a WebAuthn credential satisfies the requirement.
4. **UI flows** for enrolment, login and recovery.

The password step stays the first factor. This issue does not cover passwordless (passkey-only) login.

---

## Current State

| File | Notes |
|---|---|
| `backend/src/twofa.rs` | `TwoFAService`: TOTP secret generation/storage (AES-encrypted via `CryptoService`), RFC 6238 verification with ±1 step drift, and backup codes |
| `backend/src/api/twofa.rs` | `/api/auth/2fa/enroll/initiate`, `/enroll/confirm`, `/disable`, `/regenerate-backup`, all behind `auth_middleware` |
| `backend/src/auth.rs` | `login()` returns `LoginOutcome::TwoFaRequired { pending_token }` when `is_2fa_enabled`. `complete_2fa_login()` checks the single-use `pending_2fa` token, then `verify_login_code` |
| `backend/src/auth_middleware.rs` | `AdminUser` extractor checks `users.is_admin` and nothing else, so an admin with no 2FA has full admin access |
| `backend/migrations/037_create_2fa_tables.sql` | `user_2fa_secrets`, `user_2fa_backup_codes` |

### Gaps in the current implementation

| # | Gap | Impact |
|---|---|---|
| G1 | TOTP is the only factor | Phishable, which is not acceptable for admin and billing roles |
| G2 | Backup codes are 6 decimal digits (`Uuid::new_v4().as_u64_pair().0 % 1_000_000`) | About 20 bits of entropy. They share the TOTP code space, so every guess at `verify-2fa` is tested against 1 TOTP code plus up to 10 backup codes |
| G3 | Backup codes are hashed with **unsalted SHA-256** | Anyone with a DB dump can brute-force all 10⁶ values in milliseconds and recover every user's codes |
| G4 | `verify_backup_code` does a SELECT and then an UPDATE outside a transaction | Two concurrent requests can both redeem the same code |
| G5 | `is_2fa_enabled` reads only `user_2fa_secrets.is_enabled` | No way to represent "2FA via WebAuthn only" |
| G6 | `AdminUser` does not require 2FA | A stolen admin password is enough to reach admin routes. #2584 covers the `amr` claim; this doc defines which factors count |
| G7 | `confirm_enrollment` accepts `totp_secret` from the request body | The client can choose its own secret, and nothing binds it to the one returned by `initiate`. Low severity, but fix it while touching this code |
| G8 | No 2FA UI in the frontend (`frontend/src/app/[locale]/settings` has no security section) | Users cannot enrol without calling the API directly |

---

## Threat Model

| Threat | Today | After this issue |
|---|---|---|
| Real-time phishing proxy captures password + TOTP | Succeeds | Fails for WebAuthn users; the assertion is bound to the origin |
| DB dump leaks backup-code hashes | All codes recovered offline | Codes are 80-bit random and Argon2id-hashed, so offline recovery is infeasible |
| Online guessing at `/api/auth/verify-2fa` | ~11 hits per 10⁶ space per attempt (see #2582 for rate limits) | Recovery codes have their own format and endpoint path, so a guess cannot match both |
| Race to redeem one backup code twice | Possible | Atomic `UPDATE … WHERE used_at IS NULL` |
| Admin account with password only | Full admin access | `AdminUser` rejects until 2FA is enrolled and was used for this session |
| Lost authenticator / lost security key | Admin intervention only | Recovery code, or any other registered credential |

---

## Design

### 1. Factor model

2FA becomes "the user has at least one active second factor" instead of "the TOTP row is enabled".

```rust
pub enum SecondFactor {
    Totp,
    WebAuthn { credential_id: Vec<u8> },
    RecoveryCode,
}

impl TwoFAService {
    /// True if the user has an enabled TOTP secret OR at least one WebAuthn credential.
    pub async fn is_2fa_enabled(&self, user_id: &str) -> Result<bool>;
    /// Which factors the login UI should offer.
    pub async fn available_factors(&self, user_id: &str) -> Result<Vec<FactorKind>>;
}
```

`LoginOutcome::TwoFaRequired` gains a `factors` field so the client knows which prompt to show:

```json
{
  "status": "two_fa_required",
  "pending_token": "…",
  "expires_in": 300,
  "factors": ["webauthn", "totp", "recovery_code"]
}
```

### 2. WebAuthn (webauthn-rs)

**Dependency**: `webauthn-rs = { version = "0.5", features = ["danger-allow-state-serialisation"] }`. The feature flag lets the in-progress ceremony state be stored server-side between the two HTTP calls. Pin the exact version in `Cargo.toml`.

**Configuration** (new env vars, documented in `backend/.env.example`):

| Variable | Example | Notes |
|---|---|---|
| `WEBAUTHN_RP_ID` | `payraider.app` | Registrable domain. It must match the frontend origin's host or be a parent of it |
| `WEBAUTHN_RP_ORIGIN` | `https://payraider.app` | Exact origin. The server rejects assertions from any other origin |
| `WEBAUTHN_RP_NAME` | `PayRaider` | Shown by the authenticator |

The server must refuse to start with WebAuthn enabled if `RP_ORIGIN` is not `https://` (except `http://localhost` in dev).

**Schema** (new migration `NNN_create_webauthn_credentials.sql`):

```sql
CREATE TABLE user_webauthn_credentials (
    id              TEXT PRIMARY KEY,                -- uuid
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id   BLOB NOT NULL UNIQUE,            -- raw credential ID
    passkey_json    TEXT NOT NULL,                   -- serialized webauthn_rs::prelude::Passkey
    nickname        TEXT NOT NULL,                   -- "YubiKey 5C", "MacBook Touch ID"
    sign_count      INTEGER NOT NULL DEFAULT 0,
    backup_eligible BOOLEAN NOT NULL DEFAULT 0,      -- synced passkey vs device-bound
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at    TEXT
);
CREATE INDEX idx_webauthn_user ON user_webauthn_credentials(user_id);

-- Ceremony state between start/finish. Short TTL; rows deleted on finish or expiry.
CREATE TABLE webauthn_challenges (
    id          TEXT PRIMARY KEY,                    -- returned to client as challenge_id
    user_id     TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('register', 'authenticate')),
    state_json  TEXT NOT NULL,                       -- PasskeyRegistration / PasskeyAuthentication
    expires_at  TEXT NOT NULL
);
```

**Endpoints**:

| Method & path | Auth | Purpose |
|---|---|---|
| `POST /api/auth/2fa/webauthn/register/start` | Full access token **and** re-verification of an existing factor if one exists | Returns `CreationChallengeResponse` + `challenge_id`. Excludes already-registered credential IDs |
| `POST /api/auth/2fa/webauthn/register/finish` | Full access token | Body: `{ challenge_id, nickname, credential }`. Stores the passkey. If this is the user's first factor, the response includes freshly generated recovery codes |
| `POST /api/auth/2fa/webauthn/authenticate/start` | `pending_token` | Returns `RequestChallengeResponse` limited to the user's credentials + `challenge_id` |
| `POST /api/auth/2fa/webauthn/authenticate/finish` | `pending_token` | Body: `{ pending_token, challenge_id, credential }`. On success, behaves exactly like `complete_2fa_login` (consumes `pending_token`, issues tokens with `amr: ["pwd","hwk"]`) |
| `GET /api/auth/2fa/webauthn/credentials` | Full access token | List nicknames, created/last-used, backup-eligible |
| `DELETE /api/auth/2fa/webauthn/credentials/{id}` | Full access token + step-up (any factor) | Remove a credential. Refuse to delete the last factor of an admin |

**Verification rules**:
- Challenges expire after 5 minutes and are single-use. Delete the row on `finish` whether it succeeds or fails.
- `authenticate/finish` must check that `challenge.user_id == pending_token.sub`. Without this check, a challenge started for one user could complete another user's login.
- Require `userVerification: "preferred"` for the second-factor flow. Require `"required"` for admin step-up.
- Update `sign_count` and `last_used_at` on every successful assertion. If webauthn-rs reports a counter regression (`needs_update` / counter error), reject the assertion and write a `2fa_webauthn_counter_regression` audit event. A regression can mean the authenticator was cloned. Synced passkeys report `0`, which is allowed.

### 3. Recovery codes (replacing the current backup codes)

| Property | Current | New |
|---|---|---|
| Count | 10 | 10 |
| Format | `123456` | `xxxx-xxxx-xxxx-xxxx` (16 chars, Crockford base32, no ambiguous chars, 80 bits) |
| Source | `Uuid::new_v4() % 10^6` | `rand::rng().fill_bytes` (CSPRNG) |
| Hash | unsalted SHA-256 | Argon2id (the `argon2` crate is already used for passwords) with a per-code salt |
| Redemption | SELECT then UPDATE | `UPDATE … SET used_at = ? WHERE id = ? AND used_at IS NULL`, and treat it as success only if `rows_affected == 1` |
| Entry point | Shared with TOTP in `verify_login_code` | Separate `recovery_code` field in the verify request. Input is normalised (uppercased, dashes stripped) |

Argon2id output contains its own salt, so it cannot be looked up by equality. Store an 8-character non-secret **lookup prefix**, taken from a keyed HMAC of the code, next to the hash. Verification then fetches one row by `(user_id, lookup_prefix)` and checks only that row's Argon2 hash, instead of running 10 Argon2 verifications on every attempt.

```sql
ALTER TABLE user_2fa_backup_codes ADD COLUMN lookup_prefix TEXT;
ALTER TABLE user_2fa_backup_codes ADD COLUMN hash_alg TEXT NOT NULL DEFAULT 'sha256';
```

**Migration of existing codes**: existing rows keep `hash_alg = 'sha256'` and continue to verify until they are used or regenerated. After enrolment or regeneration, new rows are `argon2id`. Show a settings banner, "Your recovery codes use an old format, regenerate them", whenever any `sha256` rows remain. Once the oldest `sha256` row is more than 90 days old, a follow-up removes the legacy path.

The verify request becomes:

```json
POST /api/auth/verify-2fa
{ "pending_token": "…", "code": "123456" }            // TOTP
{ "pending_token": "…", "recovery_code": "K7QF-…" }   // recovery code
```

When a recovery code is used, the server marks the session `amr: ["pwd","rec"]` and returns `"recovery_codes_remaining": n`. The UI then nudges the user to register a new factor.

### 4. Require 2FA for admin roles

This builds on #2584, which adds `amr` to the JWT:

- Add `TwoFactorAdmin` (or extend `AdminUser`): it requires `is_admin` **and** that `amr` contains one of `otp`, `hwk` or `rec`.
- **Enrolment grace**: an admin who has no factor yet can log in and gets `403 { "error": "2FA_ENROLLMENT_REQUIRED" }` from every admin route. They can still reach the enrolment endpoints. No grace period by default; `ADMIN_2FA_GRACE_DAYS` (default `0`) exists for rollout only.
- **Policy option** `ADMIN_REQUIRE_PHISHING_RESISTANT=true` (default `false` for the first release): admin routes accept only `hwk`. Turn it on once all admins have registered a key.
- **Last-factor guard**: an admin cannot disable TOTP or delete their last WebAuthn credential while `is_admin = 1`.
- **Admin-assisted reset** (the only recovery path when every factor and code is lost): a second admin calls `POST /api/admin/users/{id}/2fa/reset`. It requires that admin's own `hwk`/`otp` step-up. It writes an audit event and emails the affected user. An admin cannot reset their own 2FA.

### 5. UI flows (frontend)

New `Security` section under `frontend/src/app/[locale]/settings/`:

1. **Overview**: lists the factors (TOTP on/off, each security key/passkey with nickname and last used) and recovery codes remaining (`n / 10`).
2. **Add security key / passkey**: calls `register/start`, then `navigator.credentials.create()` using `@simplewebauthn/browser` (handles base64url conversions), then `register/finish`, then prompts for a nickname. If this was the first factor, the recovery codes screen follows.
3. **Set up authenticator app** (existing API): show a QR code from `otpauth_uri`, confirm with a code, then show the recovery codes screen.
4. **Recovery codes screen**: shows the 10 codes once, with copy and download buttons and an "I've saved these" checkbox that must be ticked before closing. Regenerate requires step-up.
5. **Login, second step**: if `factors` includes `webauthn`, start the WebAuthn prompt automatically, with links to "Use authenticator app instead" and "Use a recovery code".
6. **Admin enrolment interstitial**: if an admin route returns `2FA_ENROLLMENT_REQUIRED`, redirect to the Security page with an explanation.

Browser compatibility: hide the "Add security key" option if `window.PublicKeyCredential` is undefined.

---

## Audit Events

Add to the audit log (see [AUDIT_LOG.md](./AUDIT_LOG.md)):

| Event | Trigger |
|---|---|
| `2fa_webauthn_registered` / `2fa_webauthn_removed` | Credential added or removed (include nickname, not credential ID) |
| `2fa_webauthn_login` / `2fa_webauthn_failed` | Assertion succeeded or failed |
| `2fa_webauthn_counter_regression` | Possible cloned authenticator |
| `2fa_recovery_code_used` | Include remaining count |
| `2fa_recovery_codes_regenerated` | |
| `2fa_admin_reset` | Actor admin ID, target user ID |
| `2fa_admin_enrollment_blocked` | Admin hit an admin route without a factor |

Never log raw recovery codes, TOTP secrets, or full WebAuthn client data.

---

## Test Plan

**Unit (`twofa.rs`, new `webauthn.rs`)**
- Recovery code generation: 10 codes, unique, match the format regex, 80 bits from the CSPRNG.
- The hash is Argon2id with a per-code salt; the same code hashes differently for two users.
- Redemption is single-use, including under two concurrent `tokio::spawn` redemptions (exactly one succeeds).
- Legacy `sha256` rows still verify; new rows are never written as `sha256`.
- Input normalisation: lowercase and missing dashes are accepted.

**Integration (axum test harness, `webauthn-rs` `SoftPasskey`/`SoftToken` for a virtual authenticator)**
- Register → login → `authenticate/finish` issues tokens with `amr` containing `hwk`.
- A challenge from user A used with user B's `pending_token` is rejected.
- An expired challenge (5 minutes or older) is rejected, and so is a replayed `challenge_id`.
- An assertion with the wrong origin or RP ID is rejected.
- A counter regression is rejected and audited.
- A recovery code is accepted only via the `recovery_code` field and never via `code`.
- An admin without 2FA gets `403 2FA_ENROLLMENT_REQUIRED` on admin routes, and `200` after enrolling.
- An admin cannot delete their last factor.
- With `ADMIN_REQUIRE_PHISHING_RESISTANT=true`, a TOTP-only admin session gets `403`.

**Frontend (Playwright with Chromium's virtual authenticator via CDP `WebAuthn.addVirtualAuthenticator`)**
- Full enrolment and login with a passkey.
- Fallback to TOTP, then to a recovery code.

---

## Acceptance Criteria

- [ ] Users can register multiple WebAuthn credentials and use any of them as the second factor.
- [ ] Recovery codes are 80-bit, Argon2id-hashed, single-use (atomically), and shown exactly once.
- [ ] Legacy backup codes keep working until regenerated; users are prompted to regenerate.
- [ ] Admin routes require a session established with a second factor; enrolment is reachable without one.
- [ ] `ADMIN_REQUIRE_PHISHING_RESISTANT` restricts admin access to WebAuthn sessions.
- [ ] Settings → Security UI covers enrolment, management, recovery codes, and the login second step.
- [ ] [TWO_FACTOR_AUTHENTICATION.md](./TWO_FACTOR_AUTHENTICATION.md) and `backend/.env.example` are updated.
- [ ] Tests from the plan above pass in CI.

---

## Out of Scope

- Passwordless (passkey-as-first-factor) login.
- SMS or email OTP.
- WebAuthn attestation verification and an authenticator allowlist (possible follow-up for regulated deployments).

## References

- [W3C Web Authentication Level 3](https://www.w3.org/TR/webauthn-3/)
- [webauthn-rs docs](https://docs.rs/webauthn-rs)
- [OWASP MFA Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Multifactor_Authentication_Cheat_Sheet.html)
- [RFC 8176: Authentication Method Reference values](https://www.rfc-editor.org/rfc/rfc8176) (`otp`, `hwk`, `pwd`)
