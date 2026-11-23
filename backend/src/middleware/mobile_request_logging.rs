//! Mobile request logging middleware.
//!
//! Logs a small, allowlisted set of request fields so that device
//! identifiers, bearer tokens and request bodies never reach the logs.
//! Device ids are hashed before being emitted, and log volume is bounded by
//! sampling plus a per-device rate limit.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};

/// Header carrying the mobile device identifier. Never logged raw.
const DEVICE_ID_HEADER: &str = "x-device-id";

/// Salt used when hashing device ids. Overridable via env so hashes are not
/// portable across deployments.
const DEVICE_ID_SALT_ENV: &str = "MOBILE_LOG_DEVICE_SALT";
const DEFAULT_DEVICE_ID_SALT: &str = "stellar-insights-mobile-log";

/// Fraction of requests that are logged (1 in N).
const SAMPLE_EVERY: u64 = 10;

/// Minimum interval between logged requests for a single device.
const PER_DEVICE_MIN_INTERVAL: Duration = Duration::from_secs(5);

/// Upper bound on tracked devices to keep the rate-limit map bounded.
const MAX_TRACKED_DEVICES: usize = 10_000;

/// Allowlisted fields emitted for every sampled request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileLogRecord {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub latency_ms: u64,
    /// Truncated, salted hash of the device id (never the raw value).
    pub device_hash: Option<String>,
}

impl MobileLogRecord {
    /// Renders the record as a single structured log line.
    pub fn to_log_line(&self) -> String {
        format!(
            "mobile_request method={} path={} status={} latency_ms={} device_hash={}",
            self.method,
            self.path,
            self.status,
            self.latency_ms,
            self.device_hash.as_deref().unwrap_or("-"),
        )
    }
}

/// Hashes a device id with a deployment salt, returning a short hex digest.
/// The raw identifier is never returned or logged.
pub fn hash_device_id(device_id: &str) -> String {
    let salt =
        std::env::var(DEVICE_ID_SALT_ENV).unwrap_or_else(|_| DEFAULT_DEVICE_ID_SALT.to_string());
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(device_id.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..8])
}

/// Decides whether a request should be logged, applying sampling and a
/// per-device rate limit. Pure enough to unit test without HTTP.
pub struct LogSampler {
    counter: u64,
    last_seen: Mutex<HashMap<String, Instant>>,
}

impl Default for LogSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl LogSampler {
    pub fn new() -> Self {
        Self {
            counter: 0,
            last_seen: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true when the request should be logged. `device_hash` is the
    /// already-hashed identifier (or `None` when absent).
    pub fn should_log(&mut self, device_hash: Option<&str>) -> bool {
        self.counter = self.counter.wrapping_add(1);
        if self.counter % SAMPLE_EVERY != 0 {
            return false;
        }

        let Some(device_hash) = device_hash else {
            return true;
        };

        let now = Instant::now();
        let mut last_seen = match self.last_seen.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if last_seen.len() >= MAX_TRACKED_DEVICES {
            last_seen.clear();
        }

        match last_seen.get(device_hash) {
            Some(prev) if now.duration_since(*prev) < PER_DEVICE_MIN_INTERVAL => false,
            _ => {
                last_seen.insert(device_hash.to_string(), now);
                true
            }
        }
    }
}

/// Axum middleware that logs an allowlisted, PII-safe record per request.
pub async fn mobile_request_logging(request: Request, next: Next) -> Response {
    let started = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let device_hash = request
        .headers()
        .get(DEVICE_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(hash_device_id);

    let response = next.run(request).await;

    let mut sampler = LogSampler::new();
    if sampler.should_log(device_hash.as_deref()) {
        let record = MobileLogRecord {
            method,
            path,
            status: response.status().as_u16(),
            latency_ms: started.elapsed().as_millis() as u64,
            device_hash,
        };
        tracing::info!(target: "mobile_request", "{}", record.to_log_line());
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_is_hashed_not_raw() {
        let raw = "device-abc-123";
        let hashed = hash_device_id(raw);
        assert_ne!(hashed, raw);
        assert!(!hashed.contains(raw));
        assert_eq!(hashed.len(), 16);
        // Deterministic for the same input.
        assert_eq!(hashed, hash_device_id(raw));
    }

    #[test]
    fn snapshot_contains_no_sensitive_fields() {
        let record = MobileLogRecord {
            method: "POST".to_string(),
            path: "/api/v1/mobile/sync".to_string(),
            status: 200,
            latency_ms: 12,
            device_hash: Some(hash_device_id("device-abc-123")),
        };
        let line = record.to_log_line();

        assert!(line.contains("method=POST"));
        assert!(line.contains("path=/api/v1/mobile/sync"));
        assert!(line.contains("status=200"));
        assert!(line.contains("latency_ms=12"));

        // No raw device id, tokens, or auth headers.
        assert!(!line.contains("device-abc-123"));
        assert!(!line.contains("authorization"));
        assert!(!line.contains("bearer"));
        assert!(!line.contains("token"));
        assert!(!line.contains("x-device-id"));
    }

    #[test]
    fn sampling_skips_most_requests() {
        let mut sampler = LogSampler::new();
        let logged = (0..SAMPLE_EVERY)
            .filter(|_| sampler.should_log(None))
            .count();
        assert_eq!(logged, 1);
    }

    #[test]
    fn rate_limit_suppresses_repeat_device() {
        let mut sampler = LogSampler::new();
        let hash = hash_device_id("device-abc-123");
        // Advance to the first sampled slot.
        for _ in 0..SAMPLE_EVERY - 1 {
            assert!(!sampler.should_log(Some(&hash)));
        }
        assert!(sampler.should_log(Some(&hash)));

        // Next sampled slot for the same device is suppressed by the rate limit.
        for _ in 0..SAMPLE_EVERY - 1 {
            assert!(!sampler.should_log(Some(&hash)));
        }
        assert!(!sampler.should_log(Some(&hash)));
    }
}
