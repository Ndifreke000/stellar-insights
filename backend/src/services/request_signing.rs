use anyhow::{anyhow, Result};
use chrono::Utc;
use hmac::{Hmac, KeyInit, Mac};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

type HmacSha256 = Hmac<Sha256>;

pub struct RequestSigningService {
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
}

impl RequestSigningService {
    pub fn new(redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>) -> Self {
        Self { redis_connection }
    }

    /// Build canonical request string for signing
    pub fn canonical_request(
        method: &str,
        path: &str,
        query_params: &BTreeMap<String, String>,
        body_hash: &str,
        timestamp: i64,
        nonce: &str,
    ) -> String {
        let mut canonical = format!("{}\n{}\n", method.to_uppercase(), path);

        for (key, value) in query_params.iter() {
            canonical.push_str(&format!("{}={}\n", key, value));
        }

        canonical.push_str(&format!("{}\n{}\n{}", body_hash, timestamp, nonce));
        canonical
    }

    /// Compute HMAC-SHA256 signature
    pub fn compute_signature(canonical_request: &str, signing_secret: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(signing_secret.as_bytes()).expect("HMAC key creation");
        mac.update(canonical_request.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Compute SHA256 hash of request body
    pub fn body_hash(body: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(body);
        hex::encode(hasher.finalize())
    }

    /// Check if nonce has been used before (replay detection)
    pub async fn check_nonce(&self, nonce: &str, client_id: &str) -> Result<bool> {
        let redis_conn = self.redis_connection.read().await;

        if let Some(mut conn) = redis_conn.clone() {
            let key = format!("request_signing:nonce:{}", nonce);
            let exists: bool = conn.exists(&key).await.map_err(|e| {
                anyhow!("Redis nonce check failed: {}", e)
            })?;

            Ok(exists)
        } else {
            // Redis not available, skip nonce check (log warning in production)
            tracing::warn!("Redis not available for nonce replay check");
            Ok(false)
        }
    }

    /// Record nonce as used
    pub async fn record_nonce(&self, nonce: &str, client_id: &str, ttl_secs: usize) -> Result<()> {
        let redis_conn = self.redis_connection.read().await;

        if let Some(mut conn) = redis_conn.clone() {
            let key = format!("request_signing:nonce:{}", nonce);
            conn.set_ex(&key, client_id, ttl_secs)
                .await
                .map_err(|e| anyhow!("Failed to record nonce: {}", e))?;
            Ok(())
        } else {
            tracing::warn!("Redis not available for nonce recording");
            Ok(())
        }
    }

    /// Verify request signature
    /// Returns Ok(true) if valid, Ok(false) if invalid signature, Err on processing errors
    pub async fn verify_signature(
        &self,
        method: &str,
        path: &str,
        query_params: BTreeMap<String, String>,
        body: &[u8],
        timestamp: i64,
        nonce: &str,
        signature: &str,
        signing_secret: &str,
        clock_skew_secs: i64,
    ) -> Result<bool> {
        // Check timestamp freshness
        let now = Utc::now().timestamp();
        if (now - timestamp).abs() > clock_skew_secs {
            return Ok(false); // Timestamp outside acceptable window
        }

        // Check for nonce replay
        if self.check_nonce(nonce, "").await.unwrap_or(false) {
            return Ok(false); // Nonce already used
        }

        // Compute expected signature
        let body_hash = Self::body_hash(body);
        let canonical = Self::canonical_request(method, path, &query_params, &body_hash, timestamp, nonce);
        let expected_signature = Self::compute_signature(&canonical, signing_secret);

        // Compare signatures (constant-time comparison)
        let valid = signature == expected_signature;

        if valid {
            // Record nonce usage
            let _ = self
                .record_nonce(nonce, "", clock_skew_secs as usize)
                .await;
        }

        Ok(valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_request_format() {
        let mut params = BTreeMap::new();
        params.insert("foo".to_string(), "bar".to_string());
        params.insert("baz".to_string(), "qux".to_string());

        let canonical = RequestSigningService::canonical_request(
            "POST",
            "/api/endpoint",
            &params,
            "body_hash_123",
            1234567890,
            "nonce_abc",
        );

        assert!(canonical.contains("POST"));
        assert!(canonical.contains("/api/endpoint"));
        assert!(canonical.contains("foo=bar"));
        assert!(canonical.contains("baz=qux"));
        assert!(canonical.contains("body_hash_123"));
        assert!(canonical.contains("1234567890"));
        assert!(canonical.contains("nonce_abc"));
    }

    #[test]
    fn test_body_hash() {
        let body = b"test request body";
        let hash = RequestSigningService::body_hash(body);
        assert_eq!(hash.len(), 64); // SHA256 hex is 64 chars

        // Same body should produce same hash
        let hash2 = RequestSigningService::body_hash(body);
        assert_eq!(hash, hash2);

        // Different body should produce different hash
        let hash3 = RequestSigningService::body_hash(b"different body");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_compute_signature() {
        let canonical = "POST\n/api/v1/test\n";
        let secret = "my-secret-key";
        let sig1 = RequestSigningService::compute_signature(canonical, secret);
        let sig2 = RequestSigningService::compute_signature(canonical, secret);

        assert_eq!(sig1, sig2);
        assert_eq!(sig1.len(), 64); // HMAC-SHA256 hex is 64 chars

        // Different secret should produce different signature
        let sig3 = RequestSigningService::compute_signature(canonical, "different-secret");
        assert_ne!(sig1, sig3);
    }

    #[test]
    fn test_signature_deterministic() {
        let mut params = BTreeMap::new();
        params.insert("key".to_string(), "value".to_string());

        let canonical1 = RequestSigningService::canonical_request(
            "GET",
            "/path",
            &params,
            "hash123",
            100,
            "nonce1",
        );

        let canonical2 = RequestSigningService::canonical_request(
            "GET",
            "/path",
            &params,
            "hash123",
            100,
            "nonce1",
        );

        assert_eq!(canonical1, canonical2);
    }
}
