# SEP-24 / SEP-31 Proxy Open-Proxy Audit

**Issue**: [#2581 — Audit SEP-24/SEP-31 proxies for open-proxy abuse](https://github.com/Ndifreke000/stellar-insights/issues/2581)  
**Status**: Partial — allowlist exists but defaults to open; no backend auth required on any proxy endpoint  
**Estimated size**: M (2–4 days)  
**Affected files**: `backend/src/api/sep24_proxy.rs` (815 lines), `backend/src/api/sep31_proxy.rs` (573 lines)

---

## Overview

The SEP-24 and SEP-31 proxies forward requests from the Stellar Insights frontend to external anchor servers. Because the `transfer_server` URL parameter is caller-controlled, and because neither proxy currently requires backend authentication, any unauthenticated caller can use these endpoints as an open HTTP relay to any URL that starts with an entry in the allowlist — or, when the allowlist is empty (the default when the env var is unset), to **any URL on the internet**.

This document describes the current allowlist mechanism, all identified gaps, and the required changes to close them before production deployment.

---

## Threat Model

**Who is the attacker?** Any unauthenticated actor that can reach the Stellar Insights API — a public internet user, a bot, or an authenticated user attempting lateral movement.

**What can they do today?**

| Attack | Condition | Impact |
|---|---|---|
| **Open relay** | `SEP24_ALLOWED_ORIGINS` / `SEP31_ALLOWED_ORIGINS` env vars are unset (the default) | Proxy any HTTP request to any server on the internet; used for spam, DDoS amplification, or SSRF |
| **Allowlist bypass via broad prefix** | Allowlist contains `https://anchor.com` and attacker passes `https://anchor.com.evil.com/` | Prefix-match passes; request goes to the attacker's server |
| **Allowlist wildcard** | Allowlist contains `*` | All restrictions bypassed entirely |
| **Unauthenticated access** | No `AuthUser` extractor on any proxy handler | Any unauthenticated user can trigger outbound requests to all registered anchors |
| **Anchor response forwarding** | Anchor response is returned verbatim, no content-type or size enforcement | A compromised anchor can serve a large or malformed body that the proxy returns as-is |
| **SSRF via anchor URL** | Allowlist anchor resolves to or redirects to a private IP | Internal services reachable via the proxy |
| **No path restriction** | Only the base URL is checked; any path can be appended | Callers can reach unexpected paths on an allowlisted anchor host |

---

## Current Implementation

### SEP-24 (`sep24_proxy.rs`)

**Allowlist env var**: `SEP24_ALLOWED_ORIGINS` — comma-separated URL prefixes.

```rust
fn allowed_origins() -> Vec<String> {
    std::env::var("SEP24_ALLOWED_ORIGINS")
        .ok()
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
        .unwrap_or_default()   // empty Vec if unset → all origins allowed
}

fn is_origin_allowed(transfer_server: &str) -> bool {
    let allowed = allowed_origins();
    if allowed.is_empty() {
        return true;   // ← open proxy if env var unset
    }
    let url = transfer_server.trim().trim_end_matches('/');
    allowed.iter().any(|o| url.starts_with(o) || o == "*")
}
```

**Proxy handlers**: `get_info`, `post_deposit_interactive`, `post_withdraw_interactive`, `get_transactions`, `get_transaction`, `list_anchors`.  
**Callback handlers**: `get_callback`, `post_callback`, `options_callback` — use a separate CORS-based origin check (`SEP24_ANCHORS` JSON env var).  
**Auth**: none — no `AuthUser` extractor on any handler.  
**HTTP client**: 30-second timeout, default redirect policy (up to 10 hops).

### SEP-31 (`sep31_proxy.rs`)

**Allowlist env var**: `SEP31_ALLOWED_ORIGINS` — identical mechanism.

```rust
fn allowed_origins() -> Vec<String> {
    std::env::var("SEP31_ALLOWED_ORIGINS")
        .ok()
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
        .unwrap_or_default()   // open proxy if unset
}
```

**Proxy handlers**: `get_info`, `post_quote`, `post_transaction`, `get_transactions`, `get_transaction`, `get_customer`, `put_customer`, `list_anchors`.  
**Auth**: none.  
**HTTP client**: 30-second timeout, default redirect policy.

---

## Gap Analysis

### Gap 1 — Allowlist defaults to open (Critical)

When `SEP24_ALLOWED_ORIGINS` or `SEP31_ALLOWED_ORIGINS` is unset, `allowed_origins()` returns an empty `Vec`. `is_origin_allowed` then returns `true` unconditionally. This is documented as "dev only" in a comment but there is nothing preventing the same default from reaching production.

**Fix**: invert the default — treat an empty allowlist as "deny all" and return `false`. If an operator genuinely wants to allow all origins (dev environment), they should explicitly set the env var to `*`.

```rust
fn is_origin_allowed(transfer_server: &str) -> bool {
    let allowed = allowed_origins();
    if allowed.is_empty() {
        return false;   // deny-by-default; set SEP24_ALLOWED_ORIGINS=* for dev
    }
    // ...
}
```

Add a startup warning log when `*` is detected in the allowlist.

### Gap 2 — Prefix match allows subdomain takeover (High)

`url.starts_with(o)` checks raw string prefixes. If the allowlist contains `https://anchor.com`, then `https://anchor.com.evil.com/` also passes because the string starts with that prefix.

**Fix**: parse both the allowlist entry and the incoming `transfer_server` as URLs and compare scheme + host exactly (not as a string prefix):

```rust
fn is_origin_allowed(transfer_server: &str) -> bool {
    let allowed = allowed_origins();
    if allowed.is_empty() {
        return false;
    }
    if allowed.iter().any(|o| o == "*") {
        tracing::warn!("SEP proxy wildcard allowlist active — all anchors permitted");
        return true;
    }
    let incoming = match url::Url::parse(transfer_server.trim()) {
        Ok(u) => u,
        Err(_) => return false,
    };
    allowed.iter().any(|entry| {
        let entry_url = match url::Url::parse(entry.trim()) {
            Ok(u) => u,
            Err(_) => return false,
        };
        incoming.scheme() == entry_url.scheme()
            && incoming.host() == entry_url.host()
            && incoming.port_or_known_default() == entry_url.port_or_known_default()
    })
}
```

This compares scheme + host + port exactly, eliminating both subdomain and path injection.

### Gap 3 — No backend authentication required (High)

None of the proxy handler functions use the `AuthUser` extractor. Any unauthenticated caller with network access to the API can trigger outbound requests to all registered anchors.

**Fix**: add `auth_user: AuthUser` to every proxy handler. The user does not need any special role — authentication alone (a valid JWT) is sufficient to confirm the request is from a legitimate user:

```rust
// Example: get_info
pub async fn get_info(
    State(state): State<Sep24State>,
    auth_user: AuthUser,                           // add this
    Query(q): Query<InfoQuery>,
) -> Result<Json<Value>, Sep24Error> {
    // auth_user ensures the caller holds a valid session
    let _ = auth_user;
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep24Error::Forbidden("Transfer server not in allowed list".to_string()));
    }
    // ...
}
```

Apply consistently across all handlers in both files.

### Gap 4 — Redirects not restricted (Medium)

Both proxy HTTP clients use the `reqwest` default of 10 redirect hops. A redirect from a legitimate anchor host to an internal IP constitutes SSRF.

**Fix**: set `redirect::Policy::none()` on both `Sep24State` and `Sep31State` clients:

```rust
let client = Client::builder()
    .timeout(Duration::from_secs(30))
    .redirect(reqwest::redirect::Policy::none())   // add this
    .build()
    .unwrap_or_else(|_| Client::new());
```

If an anchor legitimately redirects (e.g. HTTP → HTTPS), the canonical HTTPS URL should be registered in the allowlist directly.

### Gap 5 — No path restriction (Low)

The allowlist checks the base URL only. The handler appends paths such as `/info`, `/transactions`, `/customer` directly. There is no mechanism to prevent a caller from constructing an anchor URL with path components pre-included (e.g. `https://anchor.com/../../admin`) that survives the `trim_end_matches('/')` normalisation.

**Fix**: after parsing the `transfer_server` URL, enforce that it has no path component beyond an optional trailing slash:

```rust
fn base_url(transfer_server: &str) -> Result<String, Sep24Error> {
    let parsed = url::Url::parse(transfer_server.trim())
        .map_err(|_| Sep24Error::Forbidden("Invalid transfer server URL".to_string()))?;
    if parsed.path() != "/" && !parsed.path().is_empty() {
        return Err(Sep24Error::Forbidden("transfer_server must not include a path".to_string()));
    }
    Ok(format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or("")))
}
```

### Gap 6 — Anchor response not size-capped (Low)

Both proxies deserialise the anchor response with `.json::<Value>()` before returning it. There is no content-length guard; a misbehaving anchor could serve a large body that gets buffered entirely in memory.

**Fix**: add a `Content-Length` pre-check before consuming the response body, and consider limiting response reads to a reasonable ceiling (e.g. 1 MB):

```rust
let resp = state.client.get(&url).send().await
    .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

const MAX_BODY: u64 = 1024 * 1024;
if resp.content_length().map_or(false, |l| l > MAX_BODY) {
    return Err(Sep24Error::Proxy("Anchor response too large".to_string()));
}
```

---

## Summary of Required Changes

| Change | Severity | File |
|---|---|---|
| Invert allowlist default to deny-by-default | Critical | `sep24_proxy.rs`, `sep31_proxy.rs` |
| Replace string prefix match with URL host/scheme equality check | High | `sep24_proxy.rs`, `sep31_proxy.rs` |
| Require backend auth (`AuthUser`) on all proxy handlers | High | `sep24_proxy.rs`, `sep31_proxy.rs` |
| Disable redirects (`Policy::none()`) | Medium | `sep24_proxy.rs`, `sep31_proxy.rs` |
| Reject `transfer_server` URLs with path components | Low | `sep24_proxy.rs`, `sep31_proxy.rs` |
| Cap anchor response body size | Low | `sep24_proxy.rs`, `sep31_proxy.rs` |

---

## Configuration

### `SEP24_ALLOWED_ORIGINS`

Comma-separated list of allowed SEP-24 anchor base URLs. After this fix, **if unset the proxy will deny all requests**.

```bash
# Production — explicit allowlist
SEP24_ALLOWED_ORIGINS=https://anchor1.example.com,https://anchor2.example.com

# Development only — allow any anchor
SEP24_ALLOWED_ORIGINS=*
```

### `SEP31_ALLOWED_ORIGINS`

Same format as `SEP24_ALLOWED_ORIGINS` but for the SEP-31 proxy.

```bash
SEP31_ALLOWED_ORIGINS=https://xfer.anchor.example.com
```

Both env vars must be present and non-empty in production. Add a startup assertion that logs a `WARN` if either is `*` and exits with an error if either is unset.

---

## Acceptance Criteria

- [ ] A request with no `SEP24_ALLOWED_ORIGINS` / `SEP31_ALLOWED_ORIGINS` set returns HTTP 403 on all proxy endpoints.
- [ ] `https://anchor.com.evil.com/` is rejected when only `https://anchor.com` is in the allowlist.
- [ ] Unauthenticated callers (no JWT) receive HTTP 401 on all proxy endpoints.
- [ ] A `3xx` redirect from an allowlisted anchor returns an error rather than following the redirect.
- [ ] Arbitrary URL proxying is impossible with any allowlist configuration other than `*`.
- [ ] CI is green (lint, type-check).

---

## Implementation Guidelines

- Prefer deny-by-default: an unset allowlist must block all requests.
- The exact-match URL comparison must normalise scheme to lowercase and strip trailing slashes before comparing.
- Add a startup log at `WARN` level if `*` is detected in either allowlist; add a startup log at `ERROR` level (but do not panic) if either env var is absent in a non-dev build.
- Keep handlers thin — move allowlist validation into a shared `validate_transfer_server()` function called from each handler rather than inlining the logic.
- Never forward internal error details (e.g. resolved IPs, internal hostnames) to the caller in error responses.

---

## Related Documentation

- [Webhook SSRF Protection](./WEBHOOK_SSRF_PROTECTION.md) — issue #2579 (shared `is_blocked_ip` helper for DNS-resolution checks)
- [stellar.toml Fetch Sandboxing](./STELLAR_TOML_FETCH_SANDBOX.md) — issue #2580 (same redirect and SSRF patterns)
- [SEP-24 Hosted Deposit and Withdrawal](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0024.md)
- [SEP-31 Cross-Border Payments](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0031.md)
- [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)
- `backend/src/api/sep24_proxy.rs` — `is_origin_allowed`, all proxy handlers, callback CORS
- `backend/src/api/sep31_proxy.rs` — `is_origin_allowed`, all proxy handlers
