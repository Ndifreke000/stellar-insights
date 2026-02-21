use std::collections::HashMap;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header::IF_NONE_MATCH, HeaderValue, Request, StatusCode};
use stellar_insights_backend::services::price_feed::{PriceFeedClient, PriceFeedConfig};
use tower::util::ServiceExt;

fn test_app() -> axum::Router {
    let price_feed = Arc::new(PriceFeedClient::new(
        PriceFeedConfig::default(),
        HashMap::new(),
    ));
    stellar_insights_backend::api::cost_calculator::routes(price_feed)
}

#[tokio::test]
async fn estimate_returns_cost_breakdown_and_comparison() {
    let app = test_app();

    let request_body = serde_json::json!({
        "source_currency": "USD",
        "destination_currency": "NGN",
        "source_amount": 1000.0,
        "destination_amount": 1500000.0,
        "routes": ["stellar_dex", "anchor_direct", "liquidity_pool"]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/estimate")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("cache-control").is_some());
    assert!(response.headers().get("etag").is_some());
    assert!(response.headers().get("last-modified").is_some());

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(payload["best_route"].is_object());
    assert!(payload["routes"].is_array());
    assert_eq!(payload["routes"].as_array().unwrap().len(), 3);
    assert!(payload["routes"][0]["breakdown"]["total_fees_source"].is_number());
}

#[tokio::test]
async fn estimate_supports_conditional_etag_requests() {
    let app = test_app();

    let request_body = serde_json::json!({
        "source_currency": "USDC",
        "destination_currency": "PHP",
        "source_amount": 750.0
    })
    .to_string();

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/estimate")
                .header("content-type", "application/json")
                .body(Body::from(request_body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(first_response.status(), StatusCode::OK);

    let etag = first_response
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let second_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/estimate")
                .header("content-type", "application/json")
                .header(IF_NONE_MATCH, HeaderValue::from_str(&etag).unwrap())
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(second_response.status(), StatusCode::NOT_MODIFIED);
}
