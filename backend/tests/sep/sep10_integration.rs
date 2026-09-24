#![cfg(feature = "sep-integration")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::{from_fn, Next};
use serde_json::json;

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
