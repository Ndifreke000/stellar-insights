#![cfg(feature = "sep-integration")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::{from_fn, Next};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::json;
use stellar_xdr::curr::{Limits, ReadXdr, TransactionEnvelope};

#[tokio::test]
async fn sep10_info_returns_server_metadata() {
    let app = common::sep10_router();
    let (status, body) = common::response_json(
        app,
        Request::builder()
            .uri("/api/sep10/info")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["authentication_endpoint"], "/api/sep10/auth");
    assert_eq!(body["version"], "1.0.0");
    assert!(body["signing_key"].is_string());
}

#[tokio::test]
async fn sep10_challenge_rejects_invalid_account() {
    let app = common::sep10_router();
    let payload = json!({
        "account": "INVALID",
        "home_domain": "testanchor.stellar.org"
    });

    let (status, _) = common::response_json(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/sep10/auth")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn sep10_verify_rejects_unsigned_challenge() {
    let app = common::sep10_router();
    let payload = json!({
        "transaction": "AAAAAgAAAABmNg8JT3WgXoyfd6lsQ32hW7jCIlJBYytJtMA70wPR0gAAAGQAET+nAAAABAAAAAEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAA="
    });

    let (status, _) = common::response_json(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/sep10/verify")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sep10_logout_accepts_token_extension() {
    let app = common::sep10_router().layer(from_fn(
        |mut req: axum::http::Request<Body>, next: Next| async move {
            req.extensions_mut()
                .insert("test-session-token".to_string());
            next.run(req).await
        },
    ));
    let (status, body) = common::response_json(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/sep10/logout")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Logged out successfully");
}

// Issue #2545: the challenge must be a base64 `TransactionEnvelope` signed by
// the server — what Freighter/xBull/LOBSTR sign — never a JSON blob.
#[tokio::test]
async fn sep10_challenge_returns_signed_transaction_xdr() {
    let app = common::sep10_router();
    let payload = json!({
        "account": "GDVEU3DD4KOFECV66VIHWEZOYX4ZKR3WV27L464SIIPOU2IUI3JCZA57",
        "home_domain": "testanchor.stellar.org"
    });

    let (status, body) = common::response_json(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/sep10/auth")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let transaction = body["transaction"]
        .as_str()
        .expect("challenge transaction field");
    let bytes = BASE64.decode(transaction).expect("challenge is base64");
    let envelope = TransactionEnvelope::from_xdr(&bytes, Limits::none()).expect(
        "challenge must decode as a Stellar transaction envelope, not JSON (issue #2545)",
    );
    match envelope {
        TransactionEnvelope::Tx(v1) => assert_eq!(
            v1.signatures.len(),
            1,
            "challenge must carry exactly one server signature"
        ),
        other => panic!("expected a V1 transaction envelope, got {other:?}"),
    }
    assert_eq!(body["network_passphrase"], "Test SDF Network ; September 2015");
}

// Regression: the pre-#2545 JSON challenge must no longer be accepted.
#[tokio::test]
async fn sep10_verify_rejects_legacy_json_challenge() {
    let app = common::sep10_router();
    let legacy = serde_json::json!({
        "type": "sep10_challenge",
        "server": "GBTL47RTFR5EKMZSXWOQU735WBK7LRPPDIDK3JTNTCZZ7NUBBRDTVSK2",
        "client": "GDVEU3DD4KOFECV66VIHWEZOYX4ZKR3WV27L464SIIPOU2IUI3JCZA57",
        "nonce": "bm9uY2U=",
        "expires_at": 4_102_444_800_i64,
    });
    let payload = json!({ "transaction": BASE64.encode(legacy.to_string()) });

    let (status, _) = common::response_json(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/sep10/verify")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// SEP-1: the home domain must publish SIGNING_KEY and WEB_AUTH_ENDPOINT so
// wallets and the anchor validation suite can find this SEP-10 server.
#[tokio::test]
async fn stellar_toml_publishes_signing_key_and_web_auth_endpoint() {
    let app = common::sep10_router();
    let (status, body) = common::response_json(
        app,
        Request::builder()
            .uri("/.well-known/stellar.toml")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let toml = body["raw"].as_str().expect("stellar.toml body");
    assert!(toml.contains(
        "SIGNING_KEY=\"GAMX62ZD4FWIKMWGVPEDR6WNL2TYTPQMO2ZJEAZUAON7VCZ5G2GWDF7W\""
    ));
    assert!(toml.contains(
        "WEB_AUTH_ENDPOINT=\"https://testanchor.stellar.org/api/sep10/auth\""
    ));
}
