# stellar.toml Fetch Sandboxing

**Issue**: [#2580 — Validate and sandbox `stellar.toml` fetching](https://github.com/Ndifreke000/stellar-insights/issues/2580)  
**Status**: Partial — timeout and size limits exist; HTTPS-only enforcement and SSRF protection have gaps  
**Estimated size**: S (half a day to a day)  
**Affected file**: `backend/src/services/stellar_toml.rs` (793 lines)

---

## Overview

`StellarTomlClient` fetches remote `stellar.toml` files from anchor domains supplied by callers or discovered through the anchor directory. Because the domain comes from attacker-controlled input the fetch must be fully sandboxed: HTTPS-only, hard timeout, size cap, and SSRF protection against private/internal addresses. This document describes what is already in place, what gaps remain, and the precise changes required to close them.

---

## Threat Model

**Who is the attacker?** Anyone who can influence which domain is passed to `fetch_toml()` — a malicious anchor operator, an unauthenticated API caller reaching the anchor-directory endpoint, or an attacker who manages to submit a crafted domain through the frontend.

**What can they do today?**

| Attack | How | Impact |
|---|---|---|
| SSRF to internal services | Pass a domain whose DNS resolves to `10.x`, `172.x`, or `169.254.169.254` | Server probes internal network, potentially reads cloud metadata credentials |
| Slow-loris / resource exhaustion | Serve a `stellar.toml` response that sends headers immediately but streams bytes slowly past the 10-second timeout | Worker goroutines tied up indefinitely (timeout only applies to connection, not full response body on some reqwest versions) |
| Oversized response | Serve a multi-GB `Content-Length: 0` body (bypassing the header check) that exceeds the 1 MB body limit | OOM on body accumulation if the Content-Length check is skipped |
| HTTP fallback abuse | HTTPS fetch fails → HTTP fallback fires → request goes to a plain-text internal HTTP service | Downgrade to cleartext, SSRF via HTTP |
| Redirect chain to private IP | HTTPS fetch returns `301 Location: http://169.254.169.254/` | SSRF via redirect (up to 3 hops allowed today) |
| DNS rebinding | Domain validates on first fetch (public IP) → DNS TTL expires → re-resolves to `127.0.0.1` before cache expires | SSRF bypasses registration-time validation |

---

## Current Implementation

### Key constants (`stellar_toml.rs`, lines 11–20)

```rust
const SUCCESS_CACHE_TTL: u64 = 24 * 60 * 60;   // 24 hours
const FAILURE_CACHE_TTL: u64 = 60 * 60;          // 1 hour
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RESPONSE_SIZE: usize = 1024 * 1024;     // 1 MB
```

### HTTP client construction (`StellarTomlClient::new`)

```rust
let http_client = Client::builder()
    .timeout(REQUEST_TIMEOUT)          // 10 s total request timeout
    .user_agent("PayRaider/1.0")
    .redirect(reqwest::redirect::Policy::limited(3))  // 3 redirect hops
    .build()?;
```

### Domain validation (`validate_domain`)

Applied before every fetch. Current checks:

| Check | Implemented? | Notes |
|---|---|---|
| Empty domain | ✅ | Returns error |
| `..` or `//` in domain | ✅ | Returns error |
| Bare IP address (via `IpAddr::parse`) | ✅ | Blocks literal IPs |
| `localhost`, `127.0.0.1`, `0.0.0.0` substring | ✅ | String match |
| `10.`, `192.168.`, `172.` prefix | ✅ | String prefix match — **over-broad**: blocks all `172.x` not just RFC-1918 `172.16–31.x` |
| DNS resolution + IP range check | ❌ | Not performed |
| `169.254.x` (AWS metadata) | ❌ | Not blocked |
| Link-local IPv6 `fe80::/10` | ❌ | Not blocked |
| Unique-local IPv6 `fc00::/7` | ❌ | Not blocked |
| IPv4-mapped IPv6 `::ffff:10.x.x.x` | ❌ | Not blocked |
| Domain length > 253 bytes | ✅ | Returns error |

### Fetch flow (`fetch_toml_from_network`)

```
1. Try https://<domain>/.well-known/stellar.toml
2. On ANY error → fall back to http://<domain>/.well-known/stellar.toml
```

The HTTP fallback is unconditional — a transient HTTPS failure (network hiccup, certificate expiry) silently downgrades to cleartext.

### Response size checks (`fetch_url`)

1. Checks `Content-Length` header against `MAX_RESPONSE_SIZE` — but the header is optional; a server can omit it.
2. Reads entire body then checks `bytes.len() > MAX_RESPONSE_SIZE` — protects against the missing-header case.

Both checks are correct. The gap is that `reqwest`'s `.bytes()` buffers the entire body in memory before the check fires; a sufficiently slow response can hold a worker for the full 10-second timeout while accumulating data.

---

## Required Changes

### 1. Enforce HTTPS-only — remove the HTTP fallback

The HTTP fallback silently downgrades all failed HTTPS fetches to cleartext. Per [SEP-1](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0001.md), `stellar.toml` **must** be served over HTTPS. Remove the fallback entirely:

```rust
// Before (fetch_toml_from_network)
match self.fetch_url(&https_url).await {
    Ok(content) => return self.parse_toml(&content, domain),
    Err(_) => { /* fall through to HTTP */ }
}
let fallback = format!("http://{domain}/.well-known/stellar.toml");
self.fetch_url(&fallback).await

// After
let url = format!("https://{domain}/.well-known/stellar.toml");
self.fetch_url(&url).await
```

If an anchor's `stellar.toml` is only available over HTTP, that anchor does not meet SEP-1 requirements and should not be used.

**Migration path**: any anchor that was previously reachable only via HTTP will now fail to load. Log the domain and the error so operators can identify affected anchors. No API contract change — callers already see an error when a fetch fails.

### 2. Block DNS-resolved private/internal addresses

Replace the string-prefix checks in `validate_domain` with an async DNS-resolution check that validates every returned IP against the full blocked range list (same set as defined in [Webhook SSRF Protection](./WEBHOOK_SSRF_PROTECTION.md)):

```rust
pub async fn validate_domain(&self, domain: &str) -> Result<()> {
    // ... existing string checks ...

    // Resolve and validate every returned address
    let addrs = tokio::net::lookup_host(format!("{domain}:443"))
        .await
        .map_err(|_| anyhow!("DNS resolution failed for domain: {domain}"))?;

    for addr in addrs {
        if is_blocked_ip(addr.ip()) {
            return Err(anyhow!("Domain resolves to a disallowed address"));
        }
    }
    Ok(())
}
```

Keep the error message generic — do not echo back which IP was resolved (that would leak internal topology).

Also fix the `172.x` over-blocking: replace `lowercase.starts_with("172.")` with the precise RFC-1918 range check `172.16.0.0/12` in the IP validation function rather than the string check.

### 3. Disable redirects entirely

A `stellar.toml` served at a redirect chain is unusual and carries risk. Set redirect policy to `none`:

```rust
// StellarTomlClient::new
let http_client = Client::builder()
    .timeout(REQUEST_TIMEOUT)
    .user_agent("PayRaider/1.0")
    .redirect(reqwest::redirect::Policy::none())   // was: limited(3)
    .build()?;
```

If a redirect is required (e.g. `www.` → apex), let the caller resolve the canonical domain before passing it to `fetch_toml()`.

### 4. Add a per-read streaming size limit

Replace the buffered `.bytes()` call with a streaming read that aborts as soon as the accumulated byte count exceeds `MAX_RESPONSE_SIZE`. This prevents a slow server from holding a worker for the full timeout while filling memory:

```rust
// fetch_url — replace response.bytes().await
use futures::StreamExt;

let mut stream = response.bytes_stream();
let mut body = Vec::with_capacity(MAX_RESPONSE_SIZE);
while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|e| anyhow!("Stream read error: {e}"))?;
    if body.len() + chunk.len() > MAX_RESPONSE_SIZE {
        return Err(anyhow!("Response exceeds size limit of {} bytes", MAX_RESPONSE_SIZE));
    }
    body.extend_from_slice(&chunk);
}
```

This aborts the connection as soon as the limit is hit rather than buffering the entire body first.

### 5. Reduce the size cap to 100 KB

The SEP-1 specification does not define a maximum `stellar.toml` size, but real-world files are well under 10 KB. The current 1 MB cap is 100× larger than any observed production file. Reduce it to 100 KB to limit the blast radius of a slow-loris or memory-pressure attack:

```rust
const MAX_RESPONSE_SIZE: usize = 100 * 1024;   // 100 KB (was 1 MB)
```

Adjust the constant comment to explain the rationale.

---

## Summary of Changes

| Change | Severity of gap closed | File |
|---|---|---|
| Remove HTTP fallback — HTTPS only | High | `stellar_toml.rs` |
| DNS-resolution SSRF check | High | `stellar_toml.rs` |
| Disable redirects (`Policy::none`) | High | `stellar_toml.rs` |
| Streaming size limit | Medium | `stellar_toml.rs` |
| Reduce cap 1 MB → 100 KB | Low | `stellar_toml.rs` |
| Fix `172.x` over-blocking (use `/12` range) | Low | `stellar_toml.rs` |

---

## Acceptance Criteria

- [ ] Fetching `stellar.toml` for a domain whose DNS resolves to a private IP returns an error without making the outbound request.
- [ ] An anchor serving only HTTP (no HTTPS) fails to load with a clear log message.
- [ ] A response that starts with valid content then exceeds 100 KB is aborted mid-stream, not buffered.
- [ ] A `stellar.toml` served at a redirect chain returns an error.
- [ ] Workers are not blocked longer than `REQUEST_TIMEOUT` (10 s) on any single fetch.
- [ ] CI is green (lint, type-check).

---

## Configuration Reference

| Constant | Current value | Proposed value | Location |
|---|---|---|---|
| `REQUEST_TIMEOUT` | 10 s | 10 s (unchanged) | `stellar_toml.rs:18` |
| `MAX_RESPONSE_SIZE` | 1 048 576 B (1 MB) | 102 400 B (100 KB) | `stellar_toml.rs:21` |
| `SUCCESS_CACHE_TTL` | 86 400 s (24 h) | 86 400 s (unchanged) | `stellar_toml.rs:13` |
| `FAILURE_CACHE_TTL` | 3 600 s (1 h) | 3 600 s (unchanged) | `stellar_toml.rs:16` |
| Redirect policy | `limited(3)` | `none()` | `StellarTomlClient::new` |

---

## Related Documentation

- [Webhook SSRF Protection](./WEBHOOK_SSRF_PROTECTION.md) — issue #2579 (shared `is_blocked_ip` helper)
- [SEP-1 stellar.toml](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0001.md)
- [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)
- `backend/src/services/stellar_toml.rs` — full implementation
- Related issues: [#2545](https://github.com/Ndifreke000/stellar-insights/issues/2545), [#2606](https://github.com/Ndifreke000/stellar-insights/issues/2606)
