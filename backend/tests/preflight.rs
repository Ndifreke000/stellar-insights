//! Integration tests for the preflight endpoint.
//!
//! These tests exercise the HTTP surface of `/api/v1/preflight`, including the
//! optional threshold overrides introduced for issue #2552.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

mod common;

use common::test_app;

async fn get_preflight(query: &str) -> (StatusCode, serde_json::Value) {
    let app = test_app().await;
    let uri = format!("/api/v1/preflight?{}", query);
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

const BASE_QUERY: &str = "source_asset=USDC&destination_asset=NGN&amount_usd=2500";

#[tokio::test]
async fn preflight_defaults_are_echoed() {
    let (status, body) = get_preflight(BASE_QUERY).await;
    assert_eq!(status, StatusCode::OK);

    let thresholds = &body["effective_thresholds"];
    assert_eq!(thresholds["min_success_rate"].as_f64(), Some(95.0));
    assert_eq!(thresholds["min_sample_size"].as_i64(), Some(20));
    assert_eq!(thresholds["max_liquidity_share"].as_f64(), Some(0.50));
}

#[tokio::test]
async fn preflight_accepts_valid_overrides() {
    let query = format!(
        "{}&min_success_rate=90&min_sample_size=50&max_liquidity_share=0.25",
        BASE_QUERY
    );
    let (status, body) = get_preflight(&query).await;
    assert_eq!(status, StatusCode::OK);

    let thresholds = &body["effective_thresholds"];
    assert_eq!(thresholds["min_success_rate"].as_f64(), Some(90.0));
    assert_eq!(thresholds["min_sample_size"].as_i64(), Some(50));
    assert_eq!(thresholds["max_liquidity_share"].as_f64(), Some(0.25));
}

#[tokio::test]
async fn preflight_rejects_out_of_range_success_rate() {
    let query = format!("{}&min_success_rate=150", BASE_QUERY);
    let (status, body) = get_preflight(&query).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn preflight_rejects_out_of_range_sample_size() {
    let query = format!("{}&min_sample_size=0", BASE_QUERY);
    let (status, body) = get_preflight(&query).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn preflight_rejects_out_of_range_liquidity_share() {
    let query = format!("{}&max_liquidity_share=1.5", BASE_QUERY);
    let (status, body) = get_preflight(&query).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].is_string());
}
