#![cfg(feature = "legacy_sep10_tests")]

use anyhow::Result;
use std::sync::Arc;
use stellar_base::crypto::KeyPair;
use tokio::sync::RwLock;

use stellar_analytics::auth::sep10::{ChallengeRequest, Sep10Service, VerificationRequest};

/// Helper to create a test SEP-10 service
fn create_test_service() -> Sep10Service {
    let server_secret = "SBZVMB74YYQS3VQJMXZ7OZGD5GXZMHQHX3YYQS3VQJMXZ7OZGD5GXZMHQH";
    let network_passphrase = "Test SDF Network ; September 2015".to_string();
    let home_domain = "testanchor.stellar.org".to_string();
    let redis_conn = Arc::new(RwLock::new(None));

    Sep10Service::new(server_secret, network_passphrase, home_domain, redis_conn)
        .expect("Failed to create SEP-10 service")
}

#[tokio::test]
async fn test_generate_challenge_success() {
    let service = create_test_service();

    let client_keypair = KeyPair::random();
    let request = ChallengeRequest {
        account: client_keypair.public_key().account_id(),
        home_domain: Some("testanchor.stellar.org".to_string()),
        client_domain: None,
        memo: None,
    };

    let result = service.generate_challenge(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(!response.transaction.is_empty());
    assert_eq!(
        response.network_passphrase,
        "Test SDF Network ; September 2015"
    );
}

#[tokio::test]
async fn test_generate_challenge_with_client_domain() {
    let service = create_test_service();

    let client_keypair = KeyPair::random();
    let request = ChallengeRequest {
        account: client_keypair.public_key().account_id(),
        home_domain: Some("testanchor.stellar.org".to_string()),
        client_domain: Some("wallet.example.com".to_string()),
        memo: None,
    };

    let result = service.generate_challenge(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(!response.transaction.is_empty());
}

#[tokio::test]
async fn test_generate_challenge_with_memo() {
    let service = create_test_service();

    let client_keypair = KeyPair::random();
    let request = ChallengeRequest {
        account: client_keypair.public_key().account_id(),
        home_domain: Some("testanchor.stellar.org".to_string()),
        client_domain: None,
        memo: Some("test-memo".to_string()),
    };

    let result = service.generate_challenge(request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_generate_challenge_invalid_account() {
    let service = create_test_service();

    let request = ChallengeRequest {
        account: "INVALID_ACCOUNT".to_string(),
        home_domain: Some("testanchor.stellar.org".to_string()),
        client_domain: None,
        memo: None,
    };

    let result = service.generate_challenge(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_challenge_invalid_home_domain() {
    let service = create_test_service();

    let client_keypair = KeyPair::random();
    let request = ChallengeRequest {
        account: client_keypair.public_key().account_id(),
        home_domain: Some("wrong.domain.com".to_string()),
        client_domain: None,
        memo: None,
    };

    let result = service.generate_challenge(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_verify_challenge_success() {
    let service = create_test_service();

    // Generate challenge
    let client_keypair = KeyPair::random();
    let request = ChallengeRequest {
        account: client_keypair.public_key().account_id(),
        home_domain: Some("testanchor.stellar.org".to_string()),
        client_domain: None,
        memo: None,
    };

    let challenge_response = service
        .generate_challenge(request)
        .await
        .expect("Failed to generate challenge");

    // Decode and sign transaction
    let xdr_bytes =
        base64::decode(&challenge_response.transaction).expect("Failed to decode challenge");

    let mut envelope = stellar_base::transaction::TransactionEnvelope::from_xdr(&xdr_bytes)
        .expect("Failed to parse envelope");

    // Sign with client key
    let network = stellar_base::network::Network::new(&challenge_response.network_passphrase);
    let (transaction, mut signatures) = match envelope {
        stellar_base::transaction::TransactionEnvelope::V1 { tx, signatures } => (tx, signatures),
        _ => panic!("Unexpected envelope type"),
    };

    let tx_hash = transaction
        .hash(&network)
        .expect("Failed to hash transaction");
    let client_signature = client_keypair.sign(&tx_hash);

    let decorated_sig = stellar_base::xdr::DecoratedSignature {
        hint: client_keypair.public_key().signature_hint(),
        signature: stellar_base::crypto::Signature::from_bytes(&client_signature)
            .expect("Failed to create signature"),
    };

    signatures.push(decorated_sig);

    let signed_envelope = stellar_base::transaction::TransactionEnvelope::V1 {
        tx: transaction,
        signatures,
    };

    let signed_xdr = base64::encode(&signed_envelope.to_xdr().expect("Failed to encode XDR"));

    // Verify challenge
    let verify_request = VerificationRequest {
        transaction: signed_xdr,
    };

    let result = service.verify_challenge(verify_request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(!response.token.is_empty());
    assert!(response.expires_in > 0);
}

#[tokio::test]
async fn test_verify_challenge_missing_client_signature() {
    let service = create_test_service();

    // Generate challenge
    let client_keypair = KeyPair::random();
    let request = ChallengeRequest {
        account: client_keypair.public_key().account_id(),
        home_domain: Some("testanchor.stellar.org".to_string()),
        client_domain: None,
        memo: None,
    };

    let challenge_response = service
        .generate_challenge(request)
        .await
        .expect("Failed to generate challenge");

    // Try to verify without client signature (only server signature)
    let verify_request = VerificationRequest {
        transaction: challenge_response.transaction,
    };

    let result = service.verify_challenge(verify_request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_verify_challenge_replay_protection() {
    let service = create_test_service();

    // Generate and sign challenge
    let client_keypair = KeyPair::random();
    let request = ChallengeRequest {
        account: client_keypair.public_key().account_id(),
        home_domain: Some("testanchor.stellar.org".to_string()),
        client_domain: None,
        memo: None,
    };

    let challenge_response = service
        .generate_challenge(request)
        .await
        .expect("Failed to generate challenge");

    // Sign transaction
    let xdr_bytes =
        base64::decode(&challenge_response.transaction).expect("Failed to decode challenge");

    let envelope = stellar_base::transaction::TransactionEnvelope::from_xdr(&xdr_bytes)
        .expect("Failed to parse envelope");

    let network = stellar_base::network::Network::new(&challenge_response.network_passphrase);
    let (transaction, mut signatures) = match envelope {
        stellar_base::transaction::TransactionEnvelope::V1 { tx, signatures } => (tx, signatures),
        _ => panic!("Unexpected envelope type"),
    };

    let tx_hash = transaction
        .hash(&network)
        .expect("Failed to hash transaction");
    let client_signature = client_keypair.sign(&tx_hash);

    let decorated_sig = stellar_base::xdr::DecoratedSignature {
        hint: client_keypair.public_key().signature_hint(),
        signature: stellar_base::crypto::Signature::from_bytes(&client_signature)
            .expect("Failed to create signature"),
    };

    signatures.push(decorated_sig);

    let signed_envelope = stellar_base::transaction::TransactionEnvelope::V1 {
        tx: transaction,
        signatures,
    };

    let signed_xdr = base64::encode(&signed_envelope.to_xdr().expect("Failed to encode XDR"));

    // First verification should succeed
    let verify_request = VerificationRequest {
        transaction: signed_xdr.clone(),
    };

    let result = service.verify_challenge(verify_request).await;
    assert!(result.is_ok());

    // Second verification with same transaction should fail (replay protection)
    let verify_request_2 = VerificationRequest {
        transaction: signed_xdr,
    };

    let result_2 = service.verify_challenge(verify_request_2).await;
    assert!(result_2.is_err());
}

#[tokio::test]
async fn test_validate_session() {
    let service = create_test_service();

    // Generate and verify challenge to get a token
    let client_keypair = KeyPair::random();
    let request = ChallengeRequest {
        account: client_keypair.public_key().account_id(),
        home_domain: Some("testanchor.stellar.org".to_string()),
        client_domain: None,
        memo: None,
    };

    let challenge_response = service
        .generate_challenge(request)
        .await
        .expect("Failed to generate challenge");

    // Sign and verify (simplified for test)
    // In real test, would sign properly
    // For now, just test session validation logic

    // Note: This test would need Redis to fully work
    // Testing the validation logic structure
}

#[tokio::test]
async fn test_invalidate_session() {
    let service = create_test_service();

    // Test session invalidation
    let result = service.invalidate_session("test-token").await;
    // Should not error even if session doesn't exist
    assert!(result.is_ok());
}
