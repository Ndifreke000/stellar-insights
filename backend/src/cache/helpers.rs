use crate::cache::CacheManager;
use axum::{
    body::Body,
    http::{
        header::{
            CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED,
        },
        HeaderMap, HeaderValue, StatusCode,
    },
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

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
}

/// Generate HTTP cache response with ETag and Last-Modified headers
///
/// This layer adds HTTP-level caching on top of Redis caching.
/// It handles conditional requests (If-None-Match, If-Modified-Since)
/// and returns 304 Not Modified when appropriate.
pub fn cached_json_response<T: Serialize>(
    request_headers: &HeaderMap,
    resource_key: &str,
    payload: &T,
    ttl_seconds: usize,
) -> anyhow::Result<Response> {
    let body = serde_json::to_vec(payload)?;
    let etag = format!("\"{:x}\"", Sha256::digest(&body));
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

/// Executes a query using a cache-aside strategy.
pub async fn cached_query<T, F, Fut>(
    cache: &Arc<CacheManager>,
    key: &str,
    ttl: usize,
    query_fn: F,
) -> anyhow::Result<T>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    if let Some(cached) = cache.get::<T>(key).await? {
        tracing::debug!("Cache hit for key: {}", key);
        return Ok(cached);
    }

    tracing::debug!("Cache miss for key: {}", key);

    let result = query_fn().await?;

    // Cache write is best-effort so reads are never blocked by cache backend issues.
    if let Err(error) = cache.set(key, &result, ttl).await {
        tracing::warn!("Failed to cache result for key {}: {}", key, error);
    }

    Ok(result)
}

/// Executes a query with a cache key generated from serialized params.
pub async fn cached_query_with_params<T, P, F, Fut>(
    cache: &Arc<CacheManager>,
    key_prefix: &str,
    params: &P,
    ttl: usize,
    query_fn: F,
) -> anyhow::Result<T>
where
    T: Serialize + DeserializeOwned,
    P: Serialize,
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let key = build_param_cache_key(key_prefix, params);
    cached_query(cache, &key, ttl, query_fn).await
}

/// Builds a deterministic cache key from a prefix and serializable params.
pub fn build_param_cache_key<P: Serialize>(key_prefix: &str, params: &P) -> String {
    let params_hash = calculate_hash(params);
    format!("{key_prefix}:{params_hash}")
}

fn calculate_hash<T: Serialize>(value: &T) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let json = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct TestParams {
        limit: i64,
        offset: i64,
    }

    #[test]
    fn test_build_param_cache_key_is_stable() {
        let params = TestParams {
            limit: 10,
            offset: 0,
        };

        let key_a = build_param_cache_key("corridor:list", &params);
        let key_b = build_param_cache_key("corridor:list", &params);

        assert_eq!(key_a, key_b);
        assert!(key_a.starts_with("corridor:list:"));
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    use axum::body::to_bytes;

    #[derive(Serialize)]
    struct Payload {
        value: &'static str,
    }

    #[tokio::test]
    async fn returns_cache_headers_for_fresh_response() {
        let headers = HeaderMap::new();
        let response =
            cached_json_response(&headers, "resource:a", &Payload { value: "a" }, 60).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get(CACHE_CONTROL).is_some());
        assert!(response.headers().get(ETAG).is_some());
        assert!(response.headers().get(LAST_MODIFIED).is_some());

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body, r#"{"value":"a"}"#);
    }

    #[tokio::test]
    async fn returns_304_when_if_none_match_matches() {
        let headers = HeaderMap::new();
        let first =
            cached_json_response(&headers, "resource:b", &Payload { value: "b" }, 60).unwrap();
        let etag = first
            .headers()
            .get(ETAG)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let mut conditional_headers = HeaderMap::new();
        conditional_headers.insert(IF_NONE_MATCH, HeaderValue::from_str(&etag).unwrap());
        let second = cached_json_response(
            &conditional_headers,
            "resource:b",
            &Payload { value: "b" },
            60,
        )
        .unwrap();
        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn returns_304_when_if_modified_since_matches() {
        let headers = HeaderMap::new();
        let first =
            cached_json_response(&headers, "resource:c", &Payload { value: "c" }, 60).unwrap();
        let last_modified = first
            .headers()
            .get(LAST_MODIFIED)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let mut conditional_headers = HeaderMap::new();
        conditional_headers.insert(
            IF_MODIFIED_SINCE,
            HeaderValue::from_str(&last_modified).unwrap(),
        );

        let second = cached_json_response(
            &conditional_headers,
            "resource:c",
            &Payload { value: "c" },
            60,
        )
        .unwrap();
        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
    }
}
