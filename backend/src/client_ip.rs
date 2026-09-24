//! Resolve the real client IP when running behind load balancers / ingress.
//!
//! With several replicas behind an ingress, the TCP peer is always the proxy,
//! so per-IP limits and access logs need the forwarded client address instead.
//!
//! Configuration:
//! - `TRUST_PROXY_HEADERS=true` — honour `X-Forwarded-For` (default: false, so a
//!   directly exposed instance can't be spoofed).
//! - `TRUSTED_PROXY_HOPS` — number of proxies we control in front of the app
//!   (default: 1). The client IP is taken that many entries from the *right* of
//!   `X-Forwarded-For`; entries further left are client-supplied and untrusted.

use axum::http::HeaderMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;

struct ProxyConfig {
    trust_headers: bool,
    hops: usize,
}

fn config() -> &'static ProxyConfig {
    static CONFIG: OnceLock<ProxyConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let flag = |name: &str| {
            std::env::var(name)
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                .unwrap_or(false)
        };
        ProxyConfig {
            trust_headers: flag("TRUST_PROXY_HEADERS"),
            hops: std::env::var("TRUSTED_PROXY_HOPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .filter(|n| *n > 0)
                .unwrap_or(1),
        }
    })
}

/// Best-effort client IP: forwarded address when proxies are trusted, otherwise
/// the TCP peer.
#[must_use]
pub fn client_ip(headers: &HeaderMap, peer: Option<SocketAddr>) -> Option<IpAddr> {
    let cfg = config();
    if cfg.trust_headers {
        if let Some(ip) = forwarded_ip(headers, cfg.hops) {
            return Some(ip);
        }
    }
    peer.map(|p| p.ip())
}

fn forwarded_ip(headers: &HeaderMap, hops: usize) -> Option<IpAddr> {
    let chain: Vec<&str> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    // With N trusted proxies the Nth entry from the right was written by our
    // outermost proxy and is the address it saw.
    chain
        .len()
        .checked_sub(hops)
        .and_then(|i| chain.get(i))
        .and_then(|s| s.parse().ok())
}
