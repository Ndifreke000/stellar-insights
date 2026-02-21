/// Integration tests for CORS middleware (issue #207).
///
/// Covers:
/// - Allowed origin receives correct CORS response headers
/// - Preflight (OPTIONS) requests return the expected headers and 200/204 status
/// - Non-matching origin does NOT receive Access-Control-Allow-Origin
/// - Wildcard "*" origin configuration reflects properly
/// - max-age header is present on preflight responses
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    routing::get,
    Router,
};
use std::time::Duration;
use tower::util::ServiceExt;
use tower_http::cors::{Any, CorsLayer};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal router with a specific CORS layer for testing.
fn build_router_with_cors(cors: CorsLayer) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .layer(cors)
}

/// Build a CorsLayer that mirrors the real application logic for a given
/// `CORS_ALLOWED_ORIGINS` value (mirrors `main.rs`).
fn cors_layer_from_origins(cors_allowed_origins: &str) -> CorsLayer {
    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
        Method::PATCH,
        Method::HEAD,
    ];

    let base = CorsLayer::new()
        .allow_methods(methods)
        .allow_headers(Any)
        .max_age(Duration::from_secs(3600));

    if cors_allowed_origins.trim() == "*" {
        base.allow_origin(Any)
    } else {
        let origins: Vec<axum::http::HeaderValue> = cors_allowed_origins
            .split(',')
            .filter_map(|o| o.trim().parse::<axum::http::HeaderValue>().ok())
            .collect();

        if origins.is_empty() {
            base.allow_origin(Any)
        } else {
            base.allow_origin(origins)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests – Simple cross-origin GET
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cors_allowed_origin_receives_acao_header() {
    let cors = cors_layer_from_origins("http://localhost:3000,http://localhost:3001");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let acao = response
        .headers()
        .get("access-control-allow-origin")
        .expect("Access-Control-Allow-Origin header must be present for an allowed origin");

    assert_eq!(
        acao, "http://localhost:3000",
        "ACAO header should reflect the matching allowed origin"
    );
}

#[tokio::test]
async fn test_cors_second_allowed_origin_receives_acao_header() {
    let cors = cors_layer_from_origins("http://localhost:3000,http://localhost:3001");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let acao = response
        .headers()
        .get("access-control-allow-origin")
        .expect("Access-Control-Allow-Origin must be present for second allowed origin");

    assert_eq!(acao, "http://localhost:3001");
}

#[tokio::test]
async fn test_cors_disallowed_origin_does_not_receive_acao_header() {
    let cors = cors_layer_from_origins("http://localhost:3000");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .header(header::ORIGIN, "https://evil.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // The response may still be 200 but MUST NOT carry a permissive ACAO header.
    let acao = response.headers().get("access-control-allow-origin");
    if let Some(value) = acao {
        assert_ne!(
            value, "https://evil.example.com",
            "Disallowed origin must not be reflected in ACAO header"
        );
    }
    // If header is absent that is also acceptable – the browser will block the request.
}

// ---------------------------------------------------------------------------
// Tests – Preflight (OPTIONS)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cors_preflight_returns_allow_methods() {
    let cors = cors_layer_from_origins("http://localhost:3000");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Preflight should be answered with 200 or 204
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NO_CONTENT,
        "Preflight OPTIONS should return 200 or 204, got {}",
        response.status()
    );

    assert!(
        response
            .headers()
            .get("access-control-allow-methods")
            .is_some(),
        "Preflight response must contain Access-Control-Allow-Methods"
    );
}

#[tokio::test]
async fn test_cors_preflight_returns_max_age() {
    let cors = cors_layer_from_origins("http://localhost:3000");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let max_age = response
        .headers()
        .get("access-control-max-age")
        .expect("Preflight response should include Access-Control-Max-Age for caching");

    let secs: u64 = max_age
        .to_str()
        .unwrap()
        .parse()
        .expect("Access-Control-Max-Age should be a numeric value");

    assert!(secs > 0, "max-age should be positive");
}

#[tokio::test]
async fn test_cors_preflight_returns_allow_headers() {
    let cors = cors_layer_from_origins("http://localhost:3000");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .header("access-control-request-method", "POST")
                .header(
                    "access-control-request-headers",
                    "authorization, content-type",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response
            .headers()
            .get("access-control-allow-headers")
            .is_some(),
        "Preflight response must contain Access-Control-Allow-Headers"
    );
}

// ---------------------------------------------------------------------------
// Tests – Wildcard configuration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cors_wildcard_allows_any_origin() {
    let cors = cors_layer_from_origins("*");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .header(header::ORIGIN, "https://some-random-domain.io")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let acao = response
        .headers()
        .get("access-control-allow-origin")
        .expect("Wildcard CORS should set Access-Control-Allow-Origin");

    assert_eq!(acao, "*", "Wildcard config should respond with ACAO: *");
}

// ---------------------------------------------------------------------------
// Tests – No Origin header (same-origin / server-to-server)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cors_request_without_origin_still_succeeds() {
    let cors = cors_layer_from_origins("http://localhost:3000");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                // No Origin header – simulates same-origin or non-browser requests
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Requests without Origin header should pass through normally"
    );
}

// ---------------------------------------------------------------------------
// Tests – Production-like origin (HTTPS)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cors_production_origin_receives_acao_header() {
    let cors =
        cors_layer_from_origins("https://stellar-insights.com,https://www.stellar-insights.com");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .header(header::ORIGIN, "https://stellar-insights.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let acao = response
        .headers()
        .get("access-control-allow-origin")
        .expect("Production origin should receive ACAO header");

    assert_eq!(acao, "https://stellar-insights.com");
}
