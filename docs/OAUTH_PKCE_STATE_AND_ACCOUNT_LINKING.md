# OAuth: PKCE, State Binding and Account-Linking Edge Cases

**GitHub Issue**: [#2853](https://github.com/Ndifreke000/stellar-insights/issues/2853)
**Related**: [security/THREAT_MODEL.md](./security/THREAT_MODEL.md), [SESSION_MANAGEMENT.md](./SESSION_MANAGEMENT.md)

---

## Overview

The issue asks for a review of OAuth login: PKCE enforcement, binding `state` to the browser session, and account-takeover through account linking.

**First, what the code actually does.** `backend/src/api/oauth.rs` is not "log in with Google/GitHub". It is an **OAuth 2.0 authorization server**: PayRaider issues tokens to third-party clients (today Zapier) so they can act on a user's behalf. No code anywhere in the backend consumes an external identity provider, and nothing links accounts by email (`users` has no `email` or `email_verified` column).

The three concerns therefore map onto the codebase as follows:

| Issue concern | Where it applies |
|---|---|
| PKCE S256 | **Our authorization server** must require it from clients (Part A) |
| `state` bound to an HttpOnly cookie | Mostly the **client's** job when we are the server. We bind the consent request to the user's session and return `state` unchanged (Part A). The cookie binding becomes ours once we act as a client (Part B) |
| Auto-link only on verified email | Not reachable today. Part B specifies the rule so it is designed in before any "Sign in with …" feature lands |

The review also found authorization-server defects that are more serious than the issue's scope. They are covered in Part A, because PKCE has nothing to attach to until they are fixed.

---

## Current State: Findings

Files: `backend/src/api/oauth.rs` (routes), `backend/src/auth/oauth.rs` (`OAuthService`), `backend/migrations/019_oauth_webhooks.sql`. Routes are mounted from `backend/src/api/v1/mod.rs`.

| # | Severity | Finding |
|---|---|---|
| F1 | **Critical** | `authorize` builds `authcode_<uuid>` and returns it, but **never stores it**. `token` (grant `authorization_code`) does not validate `code` at all: it only hashes it for a debug log. Any string is accepted as long as `client_id`/`client_secret` are valid |
| F2 | **Critical** | In `token`, `user_id` comes from `validate_client_credentials`, which returns the user who **registered the client**, not the user who authorised. Every token a client obtains therefore acts as the client's owner. If the owner and the authorising user differ, the result is either wrong-account access or impersonation of the client owner |
| F3 | High | `redirect_uri` is neither registered per client nor checked at `authorize` or `token`. Once codes are real (F1), this enables code theft through an attacker-controlled redirect |
| F4 | High | No PKCE (`code_challenge`, `code_verifier`) anywhere |
| F5 | High | `refresh_token` grant does not check that the refresh token's `client_id`/`sub` match the authenticating client. A client can redeem another client's refresh token. Refresh tokens are not rotated |
| F6 | High | `revoke_token` encrypts the token with AES-GCM (**random nonce**) and then looks it up by equality. The ciphertext never matches, so revocation silently does nothing while returning `200 "Token revoked successfully"` |
| F7 | Medium | `store_authorization` performs a plain `INSERT` against `UNIQUE(client_id, user_id)`. A user re-authorising the same app gets a 500 |
| F8 | Medium | `validate_client_credentials` compares secrets with `==` (not constant-time) |
| F9 | Medium | `authorize` is a `POST` that reads parameters from the query string and returns JSON. There is no consent screen and no redirect, so the request is not a standard authorization request, and a cross-site POST from any page can create authorisations for a logged-in user (auth here is a bearer header, which limits but does not remove the risk) |
| F10 | Low | `expires_in` is hard-coded to 7 days for access tokens; access tokens should be short-lived (≤ 1 hour) and renewed with refresh tokens |

---

## Threat Model

| Attack | Precondition | Today | After |
|---|---|---|---|
| Forge a token for any client by sending any `code` | Client secret (Zapier holds it) | Works (F1) | Code must exist, be unexpired and unused, and belong to the client |
| Act as the wrong user | Two users and one client | Works (F2) | Token subject = user who consented |
| Code interception (malicious app or redirect) | Leak of `code` | n/a until F1 is fixed, then works without PKCE | Fails: the attacker lacks the `code_verifier` |
| Open redirect / code exfiltration via `redirect_uri` | Attacker crafts an authorize URL | Works (F3) | Exact-match against registered URIs |
| CSRF login / consent fixation | Victim visits attacker page | Partially possible (F9) | Consent requires a same-site session plus an anti-CSRF token |
| Cross-client refresh token reuse | One client's refresh token | Works (F5) | Bound to `client_id`, rotated, reuse detection |
| User revokes a compromised token | – | No effect (F6) | Token deleted; JWT `jti` added to the revocation list |
| Account takeover via unverified email link (Part B) | Attacker controls an IdP account with the victim's email | Not reachable | Auto-link only if the IdP asserts the email is verified **and** the local account's email is verified |

---

## Part A: Authorization server hardening

### A1. Persist and validate authorization codes (fixes F1, F2)

```sql
CREATE TABLE oauth_authorization_codes (
    code_hash             TEXT PRIMARY KEY,          -- SHA-256(code); raw code never stored
    client_id             TEXT NOT NULL REFERENCES oauth_clients(client_id),
    user_id               TEXT NOT NULL REFERENCES users(id),   -- the CONSENTING user
    redirect_uri          TEXT NOT NULL,
    scopes                TEXT NOT NULL,
    code_challenge        TEXT NOT NULL,
    code_challenge_method TEXT NOT NULL CHECK (code_challenge_method = 'S256'),
    expires_at            TEXT NOT NULL,             -- now + 60s
    consumed_at           TEXT
);
```

- Generate the code with 32 bytes from the CSPRNG, base64url-encoded.
- At `token`, use one atomic statement: `UPDATE … SET consumed_at = now WHERE code_hash = ? AND client_id = ? AND consumed_at IS NULL AND expires_at > now RETURNING user_id, scopes, redirect_uri, code_challenge`. If zero rows come back, return `invalid_grant`.
- **Reuse detection** (RFC 6749 §4.1.2): if a code that is already consumed is presented, revoke every token issued from it.
- The token's `sub` comes from the code row, **never** from the client owner.

### A2. Require PKCE S256 (fixes F4)

- `authorize` requires `code_challenge` (43–128 chars, base64url) and `code_challenge_method=S256`. It rejects `plain` and requests missing the challenge with `invalid_request`. This applies to every client, confidential ones included, as OAuth 2.1 and RFC 9700 recommend.
- `token` requires `code_verifier` and checks `BASE64URL(SHA256(code_verifier)) == code_challenge` with a constant-time compare.
- Zapier supports PKCE. Set `"enablePkce": true` in the Zapier app's OAuth config.

### A3. Register and exact-match `redirect_uri` (fixes F3)

```sql
CREATE TABLE oauth_client_redirect_uris (
    client_id    TEXT NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    redirect_uri TEXT NOT NULL,
    PRIMARY KEY (client_id, redirect_uri)
);
```

- Exact string match: no prefix, wildcard or path normalisation.
- `https://` only, except `http://localhost` and `http://127.0.0.1` for development clients.
- `token` requires `redirect_uri` to match the value stored with the code.
- If `redirect_uri` is invalid, **do not redirect**. Render an error page (RFC 6749 §4.1.2.1).

### A4. Standard authorization endpoint, consent and state (fixes F7, F9)

Replace `POST /api/oauth/authorize` with the standard flow:

1. `GET /oauth/authorize?response_type=code&client_id=…&redirect_uri=…&scope=…&state=…&code_challenge=…&code_challenge_method=S256` renders a consent page in the frontend (`/[locale]/oauth/consent`).
2. The consent page requires a logged-in **browser session** (cookie), not a bearer token, and shows the client name, its registered redirect host, and the requested scopes.
3. The consent form posts back with an **anti-CSRF token** bound to the session cookie (`SameSite=Lax`, `HttpOnly`, `Secure`).
4. After approval, respond `302` to `redirect_uri?code=…&state=<unchanged>`. After denial, `302` to `redirect_uri?error=access_denied&state=…`.
5. Store consent with `INSERT … ON CONFLICT(client_id, user_id) DO UPDATE SET scopes = excluded.scopes`. If an existing consent already covers the requested scopes, skip the consent screen.

`state` is opaque to us: we return it byte-for-byte unchanged, and the client is responsible for checking it. Our part of the CSRF defence is step 3. Keep the old `POST` route for one release; it returns `410 Gone` with a pointer to this doc.

### A5. Refresh tokens: binding and rotation (fixes F5, F10)

- Access token lifetime: 1 hour. Refresh token: 30 days, sliding.
- The refresh token's `client_id` must equal the authenticating client, and it must exist in `oauth_tokens` (hashed) and not be revoked.
- **Rotation**: each refresh returns a new refresh token and marks the old one used. If a used refresh token is presented again, revoke the whole token family (RFC 9700 §4.14.2).
- Store tokens as `SHA-256(token)` rather than AES ciphertext. Tokens are only ever compared, never read back.

### A6. Revocation that works (fixes F6)

- Look up by `SHA-256(token)`. Delete the row and add the JWT `jti` to the existing token revocation store used by `auth_middleware`, so the token stops working before it expires.
- Implement RFC 7009 semantics: return `200` even for unknown tokens, but only after authenticating the client, and only revoke tokens that belong to that client.

### A7. Misc

- Constant-time compare for `client_secret` (`subtle::ConstantTimeEq`). Better: store client secrets hashed (Argon2id) instead of encrypted (F8).
- Per-client rate limiting on `/oauth/token`, reusing `check_rate_limit_for_account`.

---

## Part B: When we consume an external IdP ("Sign in with …")

Nothing in the codebase does this yet. These rules are **mandatory** for any future implementation, and the PR adding it must reference this section.

### B1. Client-side PKCE + state + nonce, bound to an HttpOnly cookie

1. `GET /api/auth/oidc/{provider}/start` generates `state` (32 random bytes), `nonce` (32 bytes) and a PKCE `code_verifier`.
2. Store them **server-side**, keyed by a random `login_txn` ID, with a 10-minute TTL. Set the cookie `__Host-oidc_txn=<login_txn>; HttpOnly; Secure; SameSite=Lax; Path=/`. `SameSite=Lax` is required because the IdP returns via a top-level GET.
3. Redirect to the IdP with `state`, `nonce`, `code_challenge` (S256).
4. At the callback, read the cookie and load the transaction. Require `query.state == txn.state` (constant-time). Exchange the code with `code_verifier`. Validate the ID token's signature, `iss`, `aud`, `exp`, and `nonce == txn.nonce`. Delete the transaction (single-use).
5. If the cookie is missing, or `state` does not match, reject. **Never** fall back to accepting `state` without the cookie.

### B2. Account-linking rules

Identities are stored separately from users and keyed by **`(issuer, subject)`**, never by email:

```sql
CREATE TABLE user_identities (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    issuer      TEXT NOT NULL,
    subject     TEXT NOT NULL,
    email       TEXT,
    linked_at   TEXT NOT NULL,
    UNIQUE (issuer, subject)
);
```

Decision table at the callback:

| `(iss, sub)` known? | Local account with same email? | IdP `email_verified` | Local email verified | Action |
|---|---|---|---|---|
| Yes | – | – | – | Log in as the linked user |
| No | No | – | – | Create a new account, link the identity |
| No | Yes | `true` (and IdP is on the trusted list) | Yes | **Auto-link**, log in, send a "new sign-in method linked" email |
| No | Yes | `false` or absent | – | **Do not link.** Ask the user to log in with their existing credentials (password + 2FA), then link from Settings |
| No | Yes | `true` | No | **Do not link** (an attacker may have registered the local account with the victim's unverified email). Same as above |

Additional rules:
- Keep a trusted IdP allowlist for auto-linking. Some providers set `email_verified=true` for addresses they do not control; for example, Microsoft multi-tenant accounts require the `xms_edov` claim check.
- If the local account has 2FA, OIDC login is still followed by the second factor unless the IdP's `amr` claim proves MFA and policy trusts it.
- Admin accounts (`is_admin = 1`) **never** auto-link.
- Manual linking from Settings requires a step-up (recent password or 2FA) and runs the full B1 flow.
- Unlinking requires the account to still have at least one login method left.

---

## Test Plan

Every attack in the threat model gets a named integration test under `backend/tests/oauth_security.rs`:

| Test | Asserts |
|---|---|
| `token_rejects_unknown_code` | Random `code` → `invalid_grant` (F1) |
| `token_subject_is_consenting_user` | User B consents to a client registered by user A; token `sub == B` (F2) |
| `code_is_single_use_and_reuse_revokes` | Second exchange fails, and tokens from the first are revoked |
| `code_expires_after_60s` | |
| `code_bound_to_client` | Client X cannot redeem client Y's code |
| `pkce_required` / `pkce_plain_rejected` / `pkce_wrong_verifier_rejected` | |
| `redirect_uri_exact_match` | Trailing slash, extra path, different port, `http` vs `https`, and userinfo tricks (`https://good@evil`) are all rejected |
| `invalid_redirect_does_not_redirect` | Response is an error page, not 302 |
| `consent_requires_csrf_token` | Cross-site POST without the token → 403 |
| `state_returned_verbatim` | Including characters that need URL-encoding |
| `refresh_bound_to_client` (F5) | |
| `refresh_rotation_and_reuse_detection` | |
| `revoke_actually_revokes` (F6) | The token fails on `/api/oauth/me` after revocation |
| `reauthorize_same_client_is_200` (F7) | |
| Part B (when built): `callback_without_cookie_rejected`, `state_mismatch_rejected`, `nonce_mismatch_rejected`, `unverified_email_does_not_link`, `unverified_local_email_does_not_link`, `admin_never_auto_links` | |

---

## Rollout

1. Ship A1, A3, A5, A6 and A7 behind no flag: they fix active vulnerabilities.
2. Ship A2 (PKCE) and A4 (consent flow) together, with the Zapier app config updated to enable PKCE and point at `GET /oauth/authorize`. Tell existing Zapier connections to reconnect: codes issued before this release are invalid by construction.
3. Document the flows in this file and in [API_DOCUMENTATION.md](./API_DOCUMENTATION.md).

## Acceptance Criteria

- [ ] F1–F10 are fixed, each with a regression test.
- [ ] PKCE S256 is mandatory; `plain` and missing challenges are rejected.
- [ ] Authorization codes are persisted, hashed, single-use, 60-second TTL, and bound to client, user, redirect URI and challenge.
- [ ] Consent uses a session cookie and CSRF token; `state` is round-tripped unchanged.
- [ ] Revocation is effective immediately.
- [ ] Part B rules are documented and referenced from `security/THREAT_MODEL.md`.

## References

- [RFC 6749: OAuth 2.0](https://www.rfc-editor.org/rfc/rfc6749)
- [RFC 7636: PKCE](https://www.rfc-editor.org/rfc/rfc7636)
- [RFC 9700: OAuth 2.0 Security Best Current Practice](https://www.rfc-editor.org/rfc/rfc9700)
- [RFC 7009: Token Revocation](https://www.rfc-editor.org/rfc/rfc7009)
- [OpenID Connect Core §3.1.2.1 (nonce)](https://openid.net/specs/openid-connect-core-1_0.html)
- [Zapier OAuth v2 with PKCE](https://docs.zapier.com/platform/build/oauth)
