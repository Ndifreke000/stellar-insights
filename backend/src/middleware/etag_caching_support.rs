use anyhow::Result;
use axum::{
    body::Body,
    http::{
        header::{
            CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED,
            VARY,
        },
        HeaderMap, HeaderValue, StatusCode,
    },
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::models::{
    etag_caching_support::{ETagConfig, ETagResponse},
    network_context_middleware::NetworkContext,
};

#[derive(Clone, Default)]
struct CacheEntry {
    etag: String,
    last_modified: DateTime<Utc>,
}

#[derive(Clone)]
pub struct ETagCachingSupport {
    config: ETagConfig,
    state: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

/// Compute a weak ETag over the uncompressed representation of a payload.
///
/// The ETag is intentionally weak (`W/"..."`) because compression middleware
/// may transform the body after this point. A weak validator is semantically
/// equivalent across encodings, so conditional requests (gzip/br/identity)
/// all resolve to the same 304 decision.
pub fn compute_weak_etag(body: &[u8]) -> String {
    format!("W/\"{}\"", hex::encode(Sha256::digest(body)))
}

impl ETagCachingSupport {
    pub fn new(config: ETagConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Build a cache key, optionally scoped to the network.
    fn cache_key(&self, resource_key: &str, context: &NetworkContext) -> String {
        if self.config.network_scoped {
            format!("{:?}:{}", context.network, resource_key)
        } else {
            resource_key.to_string()
        }
    }

    /// Resolve (or update) the last-modified timestamp for a resource.
    fn resolve_last_modified(&self, key: &str, etag: &str) -> Result<DateTime<Utc>> {
        let now = Utc::now();
        let mut map = self
            .state
            .write()
            .map_err(|e| anyhow::anyhow!("Cache lock poisoned: {e}"))?;

        match map.get_mut(key) {
            Some(entry) if entry.etag == etag => Ok(entry.last_modified),
            Some(entry) => {
                entry.etag = etag.to_string();
                entry.last_modified = now;
                Ok(now)
            }
            None => {
                map.insert(
                    key.to_string(),
                    CacheEntry {
                        etag: etag.to_string(),
                        last_modified: now,
                    },
                );
                Ok(now)
            }
        }
    }

    /// Generate an ETag descriptor for a payload, given the network context.
    pub fn process<T: Serialize>(
        &self,
        context: &NetworkContext,
        resource_key: &str,
        payload: &T,
    ) -> Result<ETagResponse> {
        let body = serde_json::to_vec(payload)?;
        let etag = compute_weak_etag(&body);
        let cache_control = format!("public, max-age={}", self.config.default_ttl_seconds);
        let key = self.cache_key(resource_key, context);
        let _ = self.resolve_last_modified(&key, &etag)?;

        tracing::info!(
            network = ?context.network,
            resource_key,
            etag,
            "ETag computed"
        );

        Ok(ETagResponse {
            etag,
            cache_control,
            not_modified: false,
        })
    }

    /// Build a full HTTP response with ETag/Cache-Control headers, returning 304 when appropriate.
    pub fn cached_response<T: Serialize>(
        &self,
        request_headers: &HeaderMap,
        context: &NetworkContext,
        resource_key: &str,
        payload: &T,
    ) -> Result<Response> {
        let body = serde_json::to_vec(payload)?;
        let etag = compute_weak_etag(&body);
        let cache_control = format!("public, max-age={}", self.config.default_ttl_seconds);
        let key = self.cache_key(resource_key, context);
        let last_modified = self.resolve_last_modified(&key, &etag)?;

        tracing::info!(
            network = ?context.network,
            resource_key,
            etag,
            "ETag caching evaluated"
        );

        let not_modified = if_none_match_matches(request_headers, &etag)
            || if_modified_since_matches(request_headers, last_modified);

        if not_modified {
            let mut resp = Response::new(Body::empty());
            *resp.status_mut() = StatusCode::NOT_MODIFIED;
            set_cache_headers(resp.headers_mut(), &cache_control, &etag, last_modified);
            return Ok(resp);
        }

        let mut resp = Response::new(Body::from(body));
        resp.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        set_cache_headers(resp.headers_mut(), &cache_control, &etag, last_modified);
        Ok(resp)
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn format_http_date(dt: DateTime<Utc>) -> String {
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

fn parse_http_date(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc2822(value)
        .or_else(|_| DateTime::parse_from_str(value, "%a, %d %b %Y %H:%M:%S GMT"))
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

fn normalize_etag(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("W/")
        .trim()
        .trim_matches('"')
        .to_string()
}

fn if_none_match_matches(headers: &HeaderMap, etag: &str) -> bool {
    let Some(raw) = headers.get(IF_NONE_MATCH).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    if raw.trim() == "*" {
        return true;
    }
    let current = normalize_etag(etag);
    raw.split(',').map(normalize_etag).any(|c| c == current)
}

fn if_modified_since_matches(headers: &HeaderMap, last_modified: DateTime<Utc>) -> bool {
    let Some(raw) = headers.get(IF_MODIFIED_SINCE).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    parse_http_date(raw).is_some_and(|since| since.timestamp() >= last_modified.timestamp())
}

fn set_cache_headers(
    headers: &mut HeaderMap,
    cache_control: &str,
    etag: &str,
    last_modified: DateTime<Utc>,
) {
    if let Ok(v) = HeaderValue::from_str(cache_control) {
        headers.insert(CACHE_CONTROL, v);
    }
    if let Ok(v) = HeaderValue::from_str(etag) {
        headers.insert(ETAG, v);
    }
    if let Ok(v) = HeaderValue::from_str(&format_http_date(last_modified)) {
        headers.insert(LAST_MODIFIED, v);
    }
    // The representation may be compressed downstream; advertise that the
    // response varies by Accept-Encoding so caches key correctly.
    headers.insert(VARY, HeaderValue::from_static("Accept-Encoding"));
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Payload {
        value: &'static str,
    }

    fn instance() -> ETagCachingSupport {
        ETagCachingSupport::new(ETagConfig::default())
    }

    #[test]
    fn process_returns_etag_for_testnet() {
        let inst = instance();
        let ctx = NetworkContext::testnet();
        let result = inst.process(&ctx, "res:1", &Payload { value: "hello" });
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(!resp.etag.is_empty());
        assert!(resp.cache_control.contains("max-age="));
        assert!(!resp.not_modified);
    }

    #[test]
    fn process_returns_etag_for_mainnet() {
        let inst = instance();
        let ctx = NetworkContext::mainnet();
        let result = inst.process(&ctx, "res:1", &Payload { value: "hello" });
        assert!(result.is_ok());
    }

    #[test]
    fn network_scoped_keys_differ() {
        let inst = instance();
        let testnet_key = inst.cache_key("res", &NetworkContext::testnet());
        let mainnet_key = inst.cache_key("res", &NetworkContext::mainnet());
        assert_ne!(testnet_key, mainnet_key);
    }

    #[test]
    fn non_scoped_keys_are_equal() {
        let inst = ETagCachingSupport::new(ETagConfig {
            network_scoped: false,
            ..Default::default()
        });
        let testnet_key = inst.cache_key("res", &NetworkContext::testnet());
        let mainnet_key = inst.cache_key("res", &NetworkContext::mainnet());
        assert_eq!(testnet_key, mainnet_key);
    }

    #[test]
    fn etag_is_weak_and_stable_across_encodings() {
        let body = serde_json::to_vec(&Payload { value: "hello" }).unwrap();
        let etag = compute_weak_etag(&body);
        assert!(etag.starts_with("W/\""));
        // Same uncompressed representation yields the same ETag regardless of
        // the Accept-Encoding the client requested.
        assert_eq!(etag, compute_weak_etag(&body));
    }

    #[tokio::test]
    async fn cached_response_returns_304_with_vary_for_all_encodings() {
        let inst = instance();
        let ctx = NetworkContext::testnet();
        let payload = Payload { value: "hello" };

        let first = inst
            .cached_response(&HeaderMap::new(), &ctx, "res:enc", &payload)
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let etag = first
            .headers()
            .get(ETAG)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(etag.starts_with("W/\""));
        assert_eq!(
            first.headers().get(VARY).unwrap().to_str().unwrap(),
            "Accept-Encoding"
        );

        for encoding in ["gzip", "br", "identity"] {
            let mut headers = HeaderMap::new();
            headers.insert(IF_NONE_MATCH, HeaderValue::from_str(&etag).unwrap());
            headers.insert(
                axum::http::header::ACCEPT_ENCODING,
                HeaderValue::from_str(encoding).unwrap(),
            );
            let resp = inst
                .cached_response(&headers, &ctx, "res:enc", &payload)
                .unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_MODIFIED, "encoding={encoding}");
            assert_eq!(
                resp.headers().get(ETAG).unwrap().to_str().unwrap(),
                etag,
                "encoding={encoding}"
            );
            assert_eq!(
                resp.headers().get(VARY).unwrap().to_str().unwrap(),
                "Accept-Encoding"
            );
            let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
            assert!(body.is_empty());
        }
    }
}
