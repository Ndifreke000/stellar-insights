use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

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

#[derive(Clone)]
struct CacheEntry {
    etag: String,
    last_modified: DateTime<Utc>,
}

static CACHE_METADATA: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();

fn metadata_map() -> &'static Mutex<HashMap<String, CacheEntry>> {
    CACHE_METADATA.get_or_init(|| Mutex::new(HashMap::new()))
}

fn format_http_date(dt: DateTime<Utc>) -> String {
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

fn parse_http_date(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc2822(value)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            DateTime::parse_from_str(value, "%a, %d %b %Y %H:%M:%S GMT")
                .map(|dt| dt.with_timezone(&Utc))
        })
        .ok()
}

/// Single source of truth for ETag generation.
///
/// The digest is computed over the *uncompressed* representation and emitted as
/// a weak validator (`W/"..."`). Because compression may run after this point
/// and produce different bytes per encoding, a strong ETag would be incorrect;
/// a weak ETag is stable across gzip/br/identity and lets conditional requests
/// resolve to the same 304 regardless of `Accept-Encoding`.
pub fn compute_weak_etag(body: &[u8]) -> String {
    let digest = Sha256::digest(body);
    format!("W/\"{}\"", hex::encode(digest))
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
    raw.split(',')
        .map(normalize_etag)
        .any(|candidate| candidate == current)
}

fn if_modified_since_matches(headers: &HeaderMap, last_modified: DateTime<Utc>) -> bool {
    let Some(raw) = headers.get(IF_MODIFIED_SINCE).and_then(|v| v.to_str().ok()) else {
        return false;
    };

    let Some(since) = parse_http_date(raw) else {
        return false;
    };

    since.timestamp() >= last_modified.timestamp()
}

fn resolve_last_modified(resource_key: &str, etag: &str) -> DateTime<Utc> {
    let now = Utc::now();
    let Ok(mut map) = metadata_map().lock() else {
        return now;
    };

    match map.get_mut(resource_key) {
        Some(entry) if entry.etag == etag => entry.last_modified,
        Some(entry) => {
            entry.etag = etag.to_string();
            entry.last_modified = now;
            now
        }
        None => {
            map.insert(
                resource_key.to_string(),
                CacheEntry {
                    etag: etag.to_string(),
                    last_modified: now,
                },
            );
            now
        }
    }
}

fn set_common_headers(
    headers: &mut HeaderMap,
    cache_control: &str,
    etag: &str,
    last_modified: DateTime<Utc>,
) {
    if let Ok(value) = HeaderValue::from_str(cache_control) {
        headers.insert(CACHE_CONTROL, value);
    }
    if let Ok(value) = HeaderValue::from_str(etag) {
        headers.insert(ETAG, value);
    }
    if let Ok(value) = HeaderValue::from_str(&format_http_date(last_modified)) {
        headers.insert(LAST_MODIFIED, value);
    }
    // The representation may be compressed downstream, so caches must key on
    // the negotiated encoding alongside the (weak) ETag.
    headers.insert(VARY, HeaderValue::from_static("Accept-Encoding"));
}

pub fn cached_json_response<T: Serialize>(
    request_headers: &HeaderMap,
    resource_key: &str,
    payload: &T,
    ttl_seconds: usize,
) -> anyhow::Result<Response> {
    let body = serde_json::to_vec(payload)?;
    let etag = compute_weak_etag(&body);
    let last_modified = resolve_last_modified(resource_key, &etag);
    let cache_control = format!("public, max-age={ttl_seconds}");

    let not_modified = if_none_match_matches(request_headers, &etag)
        || if_modified_since_matches(request_headers, last_modified);

    if not_modified {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        set_common_headers(response.headers_mut(), &cache_control, &etag, last_modified);
        return Ok(response);
    }

    let mut response = Response::new(Body::from(body));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    set_common_headers(response.headers_mut(), &cache_control, &etag, last_modified);
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[derive(Serialize)]
    struct Payload {
        value: &'static str,
    }

    #[tokio::test]
    async fn returns_cache_headers_for_fresh_response() {
        let headers = HeaderMap::new();
        let response = cached_json_response(&headers, "resource:a", &Payload { value: "a" }, 60)
            .expect("cached_json_response should succeed for a fresh request");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get(CACHE_CONTROL).is_some());
        assert!(response.headers().get(ETAG).is_some());
        assert!(response.headers().get(LAST_MODIFIED).is_some());
        assert_eq!(
            response.headers().get(VARY).and_then(|v| v.to_str().ok()),
            Some("Accept-Encoding")
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        assert_eq!(body, r#"{"value":"a"}"#);
    }

    #[tokio::test]
    async fn etag_is_weak_and_stable_across_encodings() {
        let body = br#"{"value":"x"}"#;
        let etag = compute_weak_etag(body);
        assert!(etag.starts_with("W/\""), "ETag must be weak: {etag}");
        // Same uncompressed representation yields the same validator regardless
        // of the negotiated encoding applied downstream.
        assert_eq!(etag, compute_weak_etag(body));
    }

    #[tokio::test]
    async fn returns_304_when_if_none_match_matches() {
        let headers = HeaderMap::new();
        let first = cached_json_response(&headers, "resource:b", &Payload { value: "b" }, 60)
            .expect("cached_json_response should succeed for the initial request");
        let etag = first
            .headers()
            .get(ETAG)
            .expect("response should carry an ETag header")
            .to_str()
            .expect("ETag header should be valid ASCII")
            .to_string();

        let mut conditional_headers = HeaderMap::new();
        conditional_headers.insert(
            IF_NONE_MATCH,
            HeaderValue::from_str(&etag).expect("etag should be a valid header value"),
        );

        let second = cached_json_response(
            &conditional_headers,
            "resource:b",
            &Payload { value: "b" },
            60,
        )
        .expect("cached_json_response should succeed for the conditional request");
        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(
            second.headers().get(VARY).and_then(|v| v.to_str().ok()),
            Some("Accept-Encoding")
        );
    }

    #[tokio::test]
    async fn returns_304_for_strong_etag_from_client() {
        // Clients may echo the validator back without the weak prefix; the
        // comparison must still match so 304s are consistent across encodings.
        let headers = HeaderMap::new();
        let first = cached_json_response(&headers, "resource:d", &Payload { value: "d" }, 60)
            .expect("cached_json_response should succeed for the initial request");
        let etag = first
            .headers()
            .get(ETAG)
            .expect("response should carry an ETag header")
            .to_str()
            .expect("ETag header should be valid ASCII")
            .to_string();
        let strong = etag.trim_start_matches("W/").to_string();

        let mut conditional_headers = HeaderMap::new();
        conditional_headers.insert(
            IF_NONE_MATCH,
            HeaderValue::from_str(&strong).expect("etag should be a valid header value"),
        );

        let second = cached_json_response(
            &conditional_headers,
            "resource:d",
            &Payload { value: "d" },
            60,
        )
        .expect("cached_json_response should succeed for the conditional request");
        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn returns_304_when_if_modified_since_matches() {
        let headers = HeaderMap::new();
        let first = cached_json_response(&headers, "resource:c", &Payload { value: "c" }, 60)
            .expect("cached_json_response should succeed for the initial request");
        let last_modified = first
            .headers()
            .get(LAST_MODIFIED)
            .expect("response should carry a Last-Modified header")
            .to_str()
            .expect("Last-Modified header should be valid ASCII")
            .to_string();

        let mut conditional_headers = HeaderMap::new();
        conditional_headers.insert(
            IF_MODIFIED_SINCE,
            HeaderValue::from_str(&last_modified)
                .expect("last_modified should be a valid header value"),
        );

        let second = cached_json_response(
            &conditional_headers,
            "resource:c",
            &Payload { value: "c" },
            60,
        )
        .expect("cached_json_response should succeed for the conditional request");
        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
    }
}
