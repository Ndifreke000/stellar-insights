#![cfg(feature = "sep-integration")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;

#[tokio::test]
async fn sep31_info_proxies_mock_anchor() {
    let mock_url = common::mock_anchor_url().await;
    std::env::set_var("MOCK_ANCHOR_URL", &mock_url);

    let app = common::sep31_router();
    let uri = format!("/api/sep31/info?transfer_server={}", urlencoding::encode(&mock_url));

    let (status, body) = common::response_json(
        app,
        Request::builder().uri(&uri).body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["version"], "1.0.0");
}

#[tokio::test]
async fn sep31_quote_proxies_mock_anchor() {
    let mock_url = common::mock_anchor_url().await;
    let app = common::sep31_router();
    let payload = json!({
        "transfer_server": mock_url,
        "payload": {
            "amount": "100",
            "sell_asset": "USDC:GISSUER",
            "buy_asset": "iso4217:USD"
        }
    });

    let (status, body) = common::response_json(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/sep31/quote")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "quote-1");
}

#[tokio::test]
async fn sep31_transactions_create_and_list() {
    let mock_url = common::mock_anchor_url().await;
    let app = common::sep31_router();
    let create_payload = json!({
        "transfer_server": mock_url,
        "payload": {
            "amount": "100",
            "receiver_id": "receiver-1"
        }
    });

    let (status, body) = common::response_json(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/sep31/transactions")
            .header("content-type", "application/json")
            .body(Body::from(create_payload.to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "tx-1");

    let list_uri = format!(
        "/api/sep31/transactions?transfer_server={}&jwt=test-token",
        urlencoding::encode(&mock_url)
    );
    let (status, body) = common::response_json(
        app.clone(),
        Request::builder().uri(&list_uri).body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["transactions"].is_array());
}

#[tokio::test]
async fn sep31_transaction_detail_and_customer_endpoints_work() {
    let mock_url = common::mock_anchor_url().await;
    let app = common::sep31_router();

    let detail_uri = format!(
        "/api/sep31/transactions/tx-1?transfer_server={}&jwt=test-token",
        urlencoding::encode(&mock_url)
    );
    let (status, body) = common::response_json(
        app.clone(),
        Request::builder()
            .uri(&detail_uri)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "tx-1");

    let customer_get_uri = format!(
        "/api/sep31/customer?transfer_server={}&jwt=test-token&id=customer-1",
        urlencoding::encode(&mock_url)
    );
    let (status, body) = common::response_json(
        app.clone(),
        Request::builder()
            .uri(&customer_get_uri)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "customer-1");

    let customer_put_payload = json!({
        "transfer_server": mock_url,
        "payload": { "id": "customer-1", "status": "ACCEPTED" }
    });
    let (status, body) = common::response_json(
        app,
        Request::builder()
            .method("PUT")
            .uri("/api/sep31/customer")
            .header("content-type", "application/json")
            .body(Body::from(customer_put_payload.to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ACCEPTED");
}

#[tokio::test]
async fn sep31_anchors_list_returns_array() {
    let app = common::sep31_router();
    let (status, body) = common::response_json(
        app,
        Request::builder()
            .uri("/api/sep31/anchors")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
}
