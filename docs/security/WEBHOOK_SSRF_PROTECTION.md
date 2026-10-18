# Webhook SSRF Protection

**Issue**: [#2579 — Block SSRF in user-supplied webhook URLs](https://github.com/Ndifreke000/stellar-insights/issues/2579)  
**Status**: Partial — registration-time checks exist; send-time re-validation and redirect blocking are missing  
**Estimated size**: M (2–4 days)  
**Affected file**: `backend/src/api/webhooks.rs` (271 lines), `backend/src/validation.rs`, `backend/src/services/webhook_dispatcher.rs`

---

## Overview

Webhook URLs are supplied by authenticated users and are fetched by the server on every delivery. Without adequate controls a malicious user can supply a URL that resolves to an internal service — the AWS EC2 metadata endpoint, a Redis or database port, a Kubernetes API server — and use the delivery mechanism as an SSRF (Server-Side Request Forgery) proxy.

This document describes the threat, what the current code does, what is still missing, and the complete implementation plan.

---

## Threat Model

**Who is the attacker?** An authenticated Stellar Insights user (could be a free-tier account) who registers a webhook URL they control or craft specifically to probe internal infrastructure.

**What can they do without full SSRF protection?**

| Attack | Example URL | Impact |
|---|---|---|
| Read EC2/cloud instance metadata | `http://169.254.169.254/latest/meta-data/iam/security-credentials/` | Exfiltrate cloud IAM credentials |
| Probe internal services | `http://10.0.0.5:6379/` (Redis), `http://172.17.0.1:2375/` (Docker socket) | Port-scan internal network, execute commands |
| Access Kubernetes API | `http://10.96.0.1:443/api/v1/secrets` | Read all cluster secrets |
| DNS rebinding | Register with public IP → DNS changes to `127.0.0.1` after validation | Bypass registration-time IP checks |
| Open redirect chain | `https://legit-site.com/redirect?to=http://169.254.169.254` | Bounce through a trusted redirect to a blocked target |

---

## Current State

### What exists today

`backend/src/validation.rs` — `validate_webhook_url()` is called at registration time in `register_webhook()`:

```rust
// validation.rs
pub fn validate_webhook_url(url: &str) -> ApiResult<()> {
    let parsed = url::Url::parse(url)
        .map_err(|_| ApiError::bad_request("INVALID_URL", "Invalid URL format"))?;

    // Must be HTTPS
    if parsed.scheme() != "https" {
        return Err(ApiError::bad_request("INVALID_URL", "Webhook URL must use HTTPS scheme"));
    }

    // Hostname-level blocklist (string matching only — no DNS resolution)
    if let Some(host) = parsed.host_str() {
        // Blocks: localhost, 127.0.0.1, ::1
        // Blocks: 10.x, 192.168.x, 172.16-31.x, 169.254.x (string prefix match)
    }
    Ok(())
}
```

**What this protects against**: supplying an obvious literal private IP or `localhost` in the URL hostname at registration time.

### What is missing

| Gap | Risk |
|---|---|
| **No DNS resolution at validation time** — only the literal hostname string is checked. A hostname like `internal.attacker.com` that resolves to `10.0.0.5` passes validation. | High |
| **No re-validation at dispatch time** — the URL stored in the database is fetched directly by `WebhookDispatcher::deliver_webhook()` without re-checking the IP the hostname resolves to at send time (DNS rebinding). | High |
| **Redirects are followed** — `reqwest::Client` follows up to 10 redirects by default. An attacker can register `https://attacker.com/hook` which issues a `301 Location: http://169.254.169.254/` redirect. | High |
| **IPv6 encoded addresses** — `http://[::ffff:10.0.0.1]/` (IPv4-mapped IPv6) is not blocked. | Medium |
| **Octal/hex IP encoding** — `http://0xa.0.0.1/` or `http://010.0.0.1/` is not blocked. | Medium |
| **DNS TTL expiry (rebinding)** — a URL validated at registration with a public IP can have its DNS TTL expire and re-resolve to a private IP before the next delivery. | Medium |

---

## Required Changes

### 1. Resolve DNS and validate the resolved IP at registration and send time

Add a `resolve_and_validate_url` function that:

1. Parses the URL.
2. Resolves the hostname to IP addresses via DNS (`tokio::net::lookup_host` or the system resolver).
3. Checks every resolved IP against the full blocked range list — including IPv4-mapped IPv6 (`::ffff:x.x.x.x`).
4. Returns an error if **any** resolved address falls in a blocked range.

This check must run in **two places**:
- `validate_webhook_url()` at registration (existing call site in `register_webhook()`).
- `deliver_webhook()` in `WebhookDispatcher` immediately before building the HTTP request.

Blocked ranges to enforce:

| Range | Description |
|---|---|
| `127.0.0.0/8` | IPv4 loopback |
| `10.0.0.0/8` | RFC-1918 private |
| `172.16.0.0/12` | RFC-1918 private |
| `192.168.0.0/16` | RFC-1918 private |
| `169.254.0.0/16` | Link-local / AWS metadata |
| `100.64.0.0/10` | Carrier-grade NAT (RFC-6598) |
| `0.0.0.0/8` | This network |
| `::1/128` | IPv6 loopback |
| `fc00::/7` | IPv6 unique-local |
| `fe80::/10` | IPv6 link-local |
| `::ffff:0:0/96` | IPv4-mapped IPv6 |

Reference implementation sketch (Rust):

```rust
use std::net::IpAddr;
use tokio::net::lookup_host;

pub async fn resolve_and_validate_url(url: &str) -> ApiResult<()> {
    let parsed = url::Url::parse(url)
        .map_err(|_| ApiError::bad_request("INVALID_URL", "Invalid URL format"))?;

    if parsed.scheme() != "https" {
        return Err(ApiError::bad_request("INVALID_URL", "Webhook URL must use HTTPS"));
    }

    let host = parsed.host_str()
        .ok_or_else(|| ApiError::bad_request("INVALID_URL", "URL has no host"))?;
    let port = parsed.port_or_known_default().unwrap_or(443);

    let addrs = lookup_host(format!("{host}:{port}"))
        .await
        .map_err(|_| ApiError::bad_request("INVALID_URL", "DNS resolution failed"))?;

    for addr in addrs {
        if is_blocked_ip(addr.ip()) {
            return Err(ApiError::bad_request(
                "INVALID_URL",
                "Webhook URL resolves to a disallowed address",
            ));
        }
    }
    Ok(())
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || octets[0] == 0          // 0.0.0.0/8
                || (octets[0] == 100 && (octets[1] & 0xC0) == 64)  // 100.64/10
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || matches!(v6.segments()[0], 0xfc00..=0xfdff)  // unique-local fc00::/7
                || (v6.segments()[0] & 0xffc0) == 0xfe80       // link-local fe80::/10
                || v6.to_ipv4_mapped().map_or(false, |v4| is_blocked_ip(IpAddr::V4(v4)))
        }
    }
}
```

### 2. Disable redirects on the dispatcher HTTP client

In `WebhookDispatcher::new()`, set redirect policy to `none`:

```rust
// webhook_dispatcher.rs
let http_client = Client::builder()
    .timeout(Duration::from_secs(10))
    .redirect(reqwest::redirect::Policy::none())  // add this
    .build()
    .unwrap_or_else(|_| Client::new());
```

Consumers' webhook endpoints should not redirect — if they do, treat it as a delivery failure and let the retry logic handle it. This eliminates the open-redirect bypass entirely.

### 3. Update `validate_webhook_url` to be async

The registration-time check currently runs synchronously (no DNS lookup). Changing it to `async` requires updating the call site in `register_webhook()`:

```rust
// webhooks.rs — register_webhook handler
crate::validation::resolve_and_validate_url(&request.url)
    .await
    .map_err(|e| WebhookApiError::BadRequest(e.to_string()))?;
```

Rename `validate_webhook_url` → `resolve_and_validate_url` (or keep the old synchronous function for non-network callers and add the async variant).

### 4. Document the behaviour change

Any client that currently registers `http://` webhook URLs will receive a new error. The error response shape is unchanged (`{ "error": "INVALID_URL", "message": "..." }`), but the message text will differ. No migration path is required because `http://` webhook URLs were already rejected.

---

## Acceptance Criteria

- [ ] `POST /api/v1/webhooks` with `url: "http://169.254.169.254/hook"` returns HTTP 400.
- [ ] `POST /api/v1/webhooks` with a URL whose hostname resolves to a private IP returns HTTP 400.
- [ ] `POST /api/v1/webhooks` with a URL returning a `3xx` redirect to a private IP delivers normally to the original URL (redirects not followed — no SSRF via redirect).
- [ ] The dispatcher HTTP client has `redirect::Policy::none()` set.
- [ ] `resolve_and_validate_url` is called at both registration and dispatch time.
- [ ] IPv4-mapped IPv6 addresses (`::ffff:10.x.x.x`) are blocked.
- [ ] CI passes (lint, type-check — no new tests added per contributor instructions).

---

## Implementation Guidelines

- Prefer deny-by-default: if DNS resolution fails, reject the URL rather than allowing it.
- Never log the resolved IPs in a way that leaks internal network topology to the caller; return a generic error message.
- Keep handlers thin — move the `resolve_and_validate_url` function into `validation.rs`, not into the handler.
- If this changes an API contract for existing webhooks with `http://` URLs, document the migration path in the PR description (none needed here as `http://` was already blocked).

---

## Related Documentation

- [Webhook Signature Verification](./WEBHOOK_SIGNATURE_VERIFICATION.md) — issue #2578
- [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)
- `backend/src/validation.rs` — current `validate_webhook_url()` implementation
- `backend/src/services/webhook_dispatcher.rs` — `deliver_webhook()`, HTTP client construction
- `backend/src/api/webhooks.rs` — `register_webhook()` call site
