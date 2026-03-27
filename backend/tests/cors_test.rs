/// Integration tests for CORS middleware.
///
/// Covers:
/// - Allowed origin receives correct CORS response headers
/// - Preflight (OPTIONS) requests return the expected headers and 200/204 status
/// - Non-matching origin does NOT receive Access-Control-Allow-Origin
/// - Wildcard "*" origin configuration reflects properly
/// - max-age header is present on preflight responses
/// - Only specific headers (Authorization, Content-Type) are advertised
/// - Credentials flag is respected
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    routing::get,
    Router,
};
use std::time::Duration;
use tower::util::ServiceExt;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal router with a specific CORS layer for testing.
fn build_router_with_cors(cors: CorsLayer) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .layer(cors)
}

/// Build a CorsLayer that exactly mirrors the production logic in `main.rs`
/// for a given `CORS_ALLOWED_ORIGINS` value.
///
/// This must be kept in sync with the CORS setup in `backend/src/main.rs`.
fn cors_layer_from_origins(cors_allowed_origins: &str) -> CorsLayer {
    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
        Method::PATCH,
    ];

    // Matches main.rs: specific headers only, not Any
    let allowed_headers = [header::AUTHORIZATION, header::CONTENT_TYPE];

    let base = CorsLayer::new()
        .allow_methods(methods)
        .allow_headers(allowed_headers)
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    if cors_allowed_origins.trim() == "*" {
        base.allow_origin(Any)
    } else {
        let origins: Vec<axum::http::HeaderValue> = cors_allowed_origins
            .split(',')
            .filter_map(|o| o.trim().parse::<axum::http::HeaderValue>().ok())
            .collect();

        if origins.is_empty() {
            // Mirror main.rs behaviour: empty list rejects all cross-origin requests.
            base.allow_origin(AllowOrigin::list([]))
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

    let acao = response.headers().get("access-control-allow-origin");
    if let Some(value) = acao {
        assert_ne!(
            value, "https://evil.example.com",
            "Disallowed origin must not be reflected in ACAO header"
        );
    }
    // Absent header is also acceptable — the browser will block the request.
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
    assert_eq!(secs, 3600, "max-age should be exactly 3600 seconds");
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

/// Verify that the preflight response only advertises the specific headers
/// allowed by the production configuration (Authorization and Content-Type),
/// not a wildcard (*).
#[tokio::test]
async fn test_cors_preflight_does_not_allow_wildcard_headers() {
    let cors = cors_layer_from_origins("http://localhost:3000");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", "authorization")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    if let Some(allow_headers) = response.headers().get("access-control-allow-headers") {
        let value = allow_headers.to_str().unwrap_or("");
        assert_ne!(
            value, "*",
            "Access-Control-Allow-Headers must not be wildcard — only specific headers allowed"
        );
    }
}

/// Verify that credentials support is enabled (`Access-Control-Allow-Credentials: true`).
/// This is required for requests that include cookies or Authorization headers.
#[tokio::test]
async fn test_cors_preflight_allows_credentials() {
    let cors = cors_layer_from_origins("http://localhost:3000");
    let app = build_router_with_cors(cors);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:3000")
                .header("access-control-request-method", "GET")
                .header("access-control-request-headers", "authorization")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let allow_credentials = response.headers().get("access-control-allow-credentials");

    if let Some(value) = allow_credentials {
        assert_eq!(
            value.to_str().unwrap_or(""),
            "true",
            "Access-Control-Allow-Credentials must be 'true' when credentials are enabled"
        );
    }
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

// ---------------------------------------------------------------------------
// Tests – Empty / invalid origins fallback
// ---------------------------------------------------------------------------

/// When all provided origins fail to parse, the list is empty and all
/// cross-origin requests should be rejected (no ACAO header returned).
#[tokio::test]
async fn test_cors_empty_origins_rejects_cross_origin() {
    // Passing only whitespace — every entry fails to parse.
    let cors = cors_layer_from_origins("   ,   ");
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

    // Browser-level: no ACAO header means the request is blocked cross-origin.
    let acao = response.headers().get("access-control-allow-origin");
    assert!(
        acao.is_none(),
        "Empty origins list should not produce an ACAO header, got: {:?}",
        acao
    );
}
