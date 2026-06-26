#![cfg(feature = "sep-integration")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;

#[tokio::test]
async fn sep24_info_proxies_mock_anchor() {
    let mock_url = common::mock_anchor_url().await;
    std::env::set_var("MOCK_ANCHOR_URL", &mock_url);

    let app = common::sep24_router();
    let uri = format!("/api/sep24/info?transfer_server={}", urlencoding::encode(&mock_url));

    let (status, body) = common::response_json(
        app,
        Request::builder().uri(&uri).body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["version"], "1.0.0");
}

#[tokio::test]
async fn sep24_deposit_interactive_proxies_mock_anchor() {
    let mock_url = common::mock_anchor_url().await;
    let app = common::sep24_router();
    let payload = json!({
        "transfer_server": mock_url,
        "asset_code": "USDC",
        "account": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"
    });

    let (status, body) = common::response_json(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/sep24/deposit/interactive")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_object());
}

#[tokio::test]
async fn sep24_withdraw_interactive_proxies_mock_anchor() {
    let mock_url = common::mock_anchor_url().await;
    let app = common::sep24_router();
    let payload = json!({
        "transfer_server": mock_url,
        "asset_code": "USDC",
        "account": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"
    });

    let (status, _) = common::response_json(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/sep24/withdraw/interactive")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn sep24_transactions_and_transaction_endpoints_work() {
    let mock_url = common::mock_anchor_url().await;
    let app = common::sep24_router();

    let list_uri = format!(
        "/api/sep24/transactions?transfer_server={}&jwt=test-token",
        urlencoding::encode(&mock_url)
    );
    let (status, body) = common::response_json(
        app.clone(),
        Request::builder().uri(&list_uri).body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["transactions"].is_array());

    let detail_uri = format!(
        "/api/sep24/transaction?transfer_server={}&id=tx-1&jwt=test-token",
        urlencoding::encode(&mock_url)
    );
    let (status, body) = common::response_json(
        app,
        Request::builder()
            .uri(&detail_uri)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "tx-1");
}

#[tokio::test]
async fn sep24_anchors_list_returns_array() {
    let app = common::sep24_router();
    let (status, body) = common::response_json(
        app,
        Request::builder()
            .uri("/api/sep24/anchors")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
}
