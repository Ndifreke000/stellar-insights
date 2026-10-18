//! Integration tests for the preflight endpoint.
//!
//! These tests exercise the HTTP surface of `/api/v1/preflight`, including the
//! optional threshold overrides introduced for issue #2552, the short-window
//! (time-decayed) success-rate check introduced for issue #2554, and the Wilson
//! score lower bound introduced for issue #2555.

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

/// Synthetic recent-outage fixture: the long (24h) window looks healthy, but the
/// short window (last 30 min / last 50 payments) has collapsed. The preflight
/// must not return `proceed`; it should downgrade to `caution` or `hold`.
#[tokio::test]
async fn preflight_recent_outage_downgrades_verdict() {
    let query = format!("{}&fixture=recent_outage", BASE_QUERY);
    let (status, body) = get_preflight(&query).await;
    assert_eq!(status, StatusCode::OK);

    let verdict = body["verdict"].as_str().unwrap_or_default();
    assert_ne!(verdict, "proceed", "recent outage must not proceed");
    assert!(
        verdict == "caution" || verdict == "hold",
        "expected caution/hold, got {verdict}"
    );

    // The short-window rate should be surfaced and clearly below the long window.
    let short = body["short_window_success_rate"].as_f64();
    let long = body["long_window_success_rate"].as_f64();
    if let (Some(short), Some(long)) = (short, long) {
        assert!(short < long, "short window {short} should be below long {long}");
    }
}

/// Edge case: a healthy corridor (no recent failures) should still `proceed`.
#[tokio::test]
async fn preflight_healthy_corridor_proceeds() {
    let query = format!("{}&fixture=healthy", BASE_QUERY);
    let (status, body) = get_preflight(&query).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verdict"].as_str(), Some("proceed"));
}

/// Edge case: too few short-window samples must not trigger a false downgrade.
#[tokio::test]
async fn preflight_sparse_short_window_does_not_downgrade() {
    let query = format!("{}&fixture=sparse_short_window", BASE_QUERY);
    let (status, body) = get_preflight(&query).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["verdict"].as_str(), Some("proceed"));
}

/// Issue #2555: the success-rate check must use the Wilson score lower bound
/// (95% confidence) rather than the raw point estimate, so small samples are
/// not treated as confidently as large ones. The bound must be surfaced in the
/// check detail and never exceed the point estimate.
#[tokio::test]
async fn preflight_exposes_wilson_lower_bound() {
    let (status, body) = get_preflight(BASE_QUERY).await;
    assert_eq!(status, StatusCode::OK);

    let checks = body["checks"].as_array().expect("checks array");
    let success_check = checks
        .iter()
        .find(|c| c["name"].as_str() == Some("success_rate"))
        .expect("success_rate check present");

    let detail = &success_check["detail"];
    let lower = detail["wilson_lower_bound"]
        .as_f64()
        .expect("wilson_lower_bound surfaced in detail");
    let point = detail["success_rate"]
        .as_f64()
        .expect("success_rate surfaced in detail");

    assert!(
        lower <= point + 1e-9,
        "wilson lower bound {lower} must not exceed point estimate {point}"
    );
    assert!((0.0..=100.0).contains(&lower), "bound {lower} out of range");
}

/// Issue #2555: a small sample (19/20) and a large sample (950/1000) share the
/// same point estimate (95%), but the Wilson lower bound for the small sample is
/// materially lower. The small sample must therefore be judged less confidently.
#[tokio::test]
async fn preflight_small_sample_is_less_confident_than_large() {
    let small = format!("{}&fixture=small_sample", BASE_QUERY);
    let large = format!("{}&fixture=large_sample", BASE_QUERY);

    let (small_status, small_body) = get_preflight(&small).await;
    let (large_status, large_body) = get_preflight(&large).await;
    assert_eq!(small_status, StatusCode::OK);
    assert_eq!(large_status, StatusCode::OK);

    let small_lower = small_body["checks"]
        .as_array()
        .and_then(|c| c.iter().find(|c| c["name"].as_str() == Some("success_rate")))
        .and_then(|c| c["detail"]["wilson_lower_bound"].as_f64())
        .expect("small sample wilson bound");
    let large_lower = large_body["checks"]
        .as_array()
        .and_then(|c| c.iter().find(|c| c["name"].as_str() == Some("success_rate")))
        .and_then(|c| c["detail"]["wilson_lower_bound"].as_f64())
        .expect("large sample wilson bound");

    assert!(
        small_lower < large_lower,
        "small sample bound {small_lower} should be below large sample bound {large_lower}"
    );
}
