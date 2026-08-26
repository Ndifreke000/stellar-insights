use std::collections::BTreeMap;
use stellar_insights_backend::services::request_signing::RequestSigningService;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_valid_signature_verification() {
    let service = RequestSigningService::new(Arc::new(RwLock::new(None)));
    let signing_secret = "test-secret-key";

    let method = "POST";
    let path = "/api/v1/test";
    let query_params = BTreeMap::new();
    let body = b"test body";
    let timestamp = 1692374400i64;
    let nonce = "test-nonce-123";

    let body_hash = RequestSigningService::body_hash(body);
    let canonical =
        RequestSigningService::canonical_request(method, path, &query_params, &body_hash, timestamp, nonce);
    let signature = RequestSigningService::compute_signature(&canonical, signing_secret);

    let result = service
        .verify_signature(
            method,
            path,
            query_params,
            body,
            timestamp,
            nonce,
            &signature,
            signing_secret,
            300,
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_invalid_signature_rejected() {
    let service = RequestSigningService::new(Arc::new(RwLock::new(None)));
    let signing_secret = "test-secret-key";

    let method = "POST";
    let path = "/api/v1/test";
    let query_params = BTreeMap::new();
    let body = b"test body";
    let timestamp = 1692374400i64;
    let nonce = "test-nonce-123";

    let body_hash = RequestSigningService::body_hash(body);
    let canonical =
        RequestSigningService::canonical_request(method, path, &query_params, &body_hash, timestamp, nonce);
    let mut signature = RequestSigningService::compute_signature(&canonical, signing_secret);

    // Tamper with signature
    signature = signature.chars().rev().collect();

    let result = service
        .verify_signature(
            method,
            path,
            query_params,
            body,
            timestamp,
            nonce,
            &signature,
            signing_secret,
            300,
        )
        .await;

    assert!(result.is_ok_and(|v| !v));
}

#[tokio::test]
async fn test_tampered_body_rejected() {
    let service = RequestSigningService::new(Arc::new(RwLock::new(None)));
    let signing_secret = "test-secret-key";

    let method = "POST";
    let path = "/api/v1/test";
    let query_params = BTreeMap::new();
    let original_body = b"test body";
    let timestamp = 1692374400i64;
    let nonce = "test-nonce-123";

    // Sign with original body
    let body_hash = RequestSigningService::body_hash(original_body);
    let canonical = RequestSigningService::canonical_request(
        method,
        path,
        &query_params,
        &body_hash,
        timestamp,
        nonce,
    );
    let signature = RequestSigningService::compute_signature(&canonical, signing_secret);

    // Verify with tampered body
    let tampered_body = b"tampered body";
    let result = service
        .verify_signature(
            method,
            path,
            query_params,
            tampered_body,
            timestamp,
            nonce,
            &signature,
            signing_secret,
            300,
        )
        .await;

    assert!(result.is_ok_and(|v| !v));
}

#[tokio::test]
async fn test_expired_timestamp_rejected() {
    let service = RequestSigningService::new(Arc::new(RwLock::new(None)));
    let signing_secret = "test-secret-key";

    let method = "POST";
    let path = "/api/v1/test";
    let query_params = BTreeMap::new();
    let body = b"test body";
    let old_timestamp = 1000000000i64; // Very old timestamp
    let nonce = "test-nonce-123";

    let body_hash = RequestSigningService::body_hash(body);
    let canonical = RequestSigningService::canonical_request(
        method,
        path,
        &query_params,
        &body_hash,
        old_timestamp,
        nonce,
    );
    let signature = RequestSigningService::compute_signature(&canonical, signing_secret);

    let result = service
        .verify_signature(
            method,
            path,
            query_params,
            body,
            old_timestamp,
            nonce,
            &signature,
            signing_secret,
            300, // 5 minute clock skew
        )
        .await;

    assert!(result.is_ok_and(|v| !v));
}

#[tokio::test]
async fn test_query_params_included_in_signature() {
    let service = RequestSigningService::new(Arc::new(RwLock::new(None)));
    let signing_secret = "test-secret-key";

    let method = "GET";
    let path = "/api/v1/test";
    let mut query_params = BTreeMap::new();
    query_params.insert("key1".to_string(), "value1".to_string());
    query_params.insert("key2".to_string(), "value2".to_string());

    let body = b"";
    let timestamp = 1692374400i64;
    let nonce = "test-nonce-123";

    let body_hash = RequestSigningService::body_hash(body);
    let canonical = RequestSigningService::canonical_request(
        method,
        path,
        &query_params,
        &body_hash,
        timestamp,
        nonce,
    );
    let signature = RequestSigningService::compute_signature(&canonical, signing_secret);

    // Changing query params should invalidate signature
    let mut different_params = BTreeMap::new();
    different_params.insert("key1".to_string(), "different_value".to_string());
    different_params.insert("key2".to_string(), "value2".to_string());

    let result = service
        .verify_signature(
            method,
            path,
            different_params,
            body,
            timestamp,
            nonce,
            &signature,
            signing_secret,
            300,
        )
        .await;

    assert!(result.is_ok_and(|v| !v));
}

#[tokio::test]
async fn test_method_mismatch_rejected() {
    let service = RequestSigningService::new(Arc::new(RwLock::new(None)));
    let signing_secret = "test-secret-key";

    let method = "POST";
    let path = "/api/v1/test";
    let query_params = BTreeMap::new();
    let body = b"test body";
    let timestamp = 1692374400i64;
    let nonce = "test-nonce-123";

    let body_hash = RequestSigningService::body_hash(body);
    let canonical =
        RequestSigningService::canonical_request(method, path, &query_params, &body_hash, timestamp, nonce);
    let signature = RequestSigningService::compute_signature(&canonical, signing_secret);

    // Verify with different method
    let result = service
        .verify_signature(
            "GET",
            path,
            query_params,
            body,
            timestamp,
            nonce,
            &signature,
            signing_secret,
            300,
        )
        .await;

    assert!(result.is_ok_and(|v| !v));
}

#[test]
fn test_canonical_request_sorted_params() {
    let mut params = BTreeMap::new();
    params.insert("z_param".to_string(), "value_z".to_string());
    params.insert("a_param".to_string(), "value_a".to_string());
    params.insert("m_param".to_string(), "value_m".to_string());

    let canonical = RequestSigningService::canonical_request("GET", "/path", &params, "hash", 100, "nonce");

    // Verify parameters appear in sorted order
    let lines: Vec<&str> = canonical.lines().collect();
    assert_eq!(lines[2], "a_param=value_a");
    assert_eq!(lines[3], "m_param=value_m");
    assert_eq!(lines[4], "z_param=value_z");
}

#[test]
fn test_body_hash_consistency() {
    let body = b"consistent body content";
    let hash1 = RequestSigningService::body_hash(body);
    let hash2 = RequestSigningService::body_hash(body);
    assert_eq!(hash1, hash2);

    let different_body = b"different content";
    let hash3 = RequestSigningService::body_hash(different_body);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_signature_different_secrets() {
    let canonical = "POST\n/api/v1/test\n";
    let secret1 = "secret1";
    let secret2 = "secret2";

    let sig1 = RequestSigningService::compute_signature(canonical, secret1);
    let sig2 = RequestSigningService::compute_signature(canonical, secret2);

    assert_ne!(sig1, sig2);
}
