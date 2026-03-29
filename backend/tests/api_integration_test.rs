//! Integration tests for HTTP API endpoints.
//!
//! Each test spins up a lightweight in-memory SQLite database,
//! constructs the appropriate Axum router, and fires one-shot
//! HTTP requests – no external services required.

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::util::ServiceExt;

use stellar_insights_backend::api::anchors::get_anchors;
use stellar_insights_backend::cache::{CacheConfig, CacheManager};
use stellar_insights_backend::database::Database;
use stellar_insights_backend::handlers::{health_check, pool_metrics};
use stellar_insights_backend::ingestion::DataIngestionService;
use stellar_insights_backend::rpc::StellarRpcClient;
use stellar_insights_backend::services::price_feed::{
    default_asset_mapping, PriceFeedClient, PriceFeedConfig,
};
use stellar_insights_backend::state::AppState;
use stellar_insights_backend::websocket::WsState;

// ── helpers ─────────────────────────────────────────────────────────────────

/// Minimal DDL for the tables the API handlers actually read.
/// We avoid `sqlx::migrate!` because the migration directory contains
/// three files with the same version prefix (017_*), which triggers a
/// UNIQUE constraint violation in `_sqlx_migrations` on every fresh
/// in-memory database.
const MINIMAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS anchors (
    id                      TEXT PRIMARY KEY,
    name                    TEXT NOT NULL,
    stellar_account         TEXT NOT NULL UNIQUE,
    home_domain             TEXT,
    total_transactions      INTEGER DEFAULT 0,
    successful_transactions INTEGER DEFAULT 0,
    failed_transactions     INTEGER DEFAULT 0,
    total_volume_usd        REAL    DEFAULT 0,
    avg_settlement_time_ms  INTEGER DEFAULT 0,
    reliability_score       REAL    DEFAULT 0,
    status                  TEXT    DEFAULT 'green',
    created_at              TEXT    DEFAULT CURRENT_TIMESTAMP,
    updated_at              TEXT    DEFAULT CURRENT_TIMESTAMP
);
"#;

async fn setup_db() -> Arc<Database> {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("in-memory pool");
    sqlx::query(MINIMAL_SCHEMA)
        .execute(&pool)
        .await
        .expect("minimal schema");
    Arc::new(Database::new(pool))
}

async fn make_app_state(db: Arc<Database>) -> AppState {
    let ws_state = Arc::new(WsState::new());
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let ingestion = Arc::new(DataIngestionService::new(
        rpc_client.clone(),
        Arc::clone(&db),
    ));
    let cache = Arc::new(CacheManager::new(CacheConfig::default()).await.unwrap());
    AppState::new(db, cache, ws_state, ingestion, rpc_client)
}

async fn app_state_router(db: Arc<Database>) -> Router {
    let state = make_app_state(db).await;
    Router::new()
        .route("/health", get(health_check))
        .route("/api/pool-metrics", get(pool_metrics))
        .with_state(state)
}

async fn cached_anchor_router(db: Arc<Database>) -> Router {
    let cache = Arc::new(CacheManager::new(CacheConfig::default()).await.unwrap());
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let price_feed = Arc::new(PriceFeedClient::new(
        PriceFeedConfig::default(),
        default_asset_mapping(),
    ));

    Router::new()
        .route("/api/anchors", get(get_anchors))
        .with_state((db, cache, rpc_client, price_feed))
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).expect("valid JSON")
}

// ── /health ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_health_check_returns_200() {
    let db = setup_db().await;
    let app = app_state_router(db).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_check_body_has_status_and_checks() {
    let db = setup_db().await;
    let app = app_state_router(db).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = json_body(resp).await;
    assert!(body["status"].is_string());
    assert!(body["timestamp"].is_string());

    // Check database health details
    assert!(body["checks"]["database"]["healthy"].as_bool().unwrap());
    assert!(body["checks"]["database"]["response_time_ms"].is_number());
    assert!(body["checks"]["database"]["message"].is_null());

    // Check cache health details
    assert!(body["checks"]["cache"]["healthy"].is_boolean());
    assert!(body["checks"]["cache"]["response_time_ms"].is_number());
    assert!(
        body["checks"]["cache"]["message"].is_string()
            || body["checks"]["cache"]["message"].is_null()
    );

    // Check rpc health details
    assert!(body["checks"]["rpc"]["healthy"].is_boolean());
    assert!(body["checks"]["rpc"]["response_time_ms"].is_number());
    assert!(
        body["checks"]["rpc"]["message"].is_string() || body["checks"]["rpc"]["message"].is_null()
    );
}

// ── GET /api/anchors ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_anchors_empty_database_returns_200() {
    let db = setup_db().await;
    let app = cached_anchor_router(db).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/anchors")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_anchors_returns_json_object_with_anchors_array() {
    let db = setup_db().await;
    let app = cached_anchor_router(db).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/anchors")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = json_body(resp).await;
    // Response should be an object with an `anchors` array and `total` count
    assert!(body["anchors"].is_array(), "expected anchors array");
    assert!(body["total"].is_number(), "expected total count");
    assert_eq!(body["anchors"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn test_list_anchors_pagination_params_accepted() {
    let db = setup_db().await;
    let app = cached_anchor_router(db).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/anchors?limit=5&offset=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_anchors_zero_limit_param() {
    let db = setup_db().await;
    let app = cached_anchor_router(db).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/anchors?limit=0&offset=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Should not panic; response may be 200 with empty results or a 400
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::BAD_REQUEST,
        "unexpected status: {}",
        resp.status()
    );
}

// ── GET /api/pool-metrics ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_pool_metrics_returns_200() {
    let db = setup_db().await;
    let app = app_state_router(db).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/pool-metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_pool_metrics_response_is_json() {
    let db = setup_db().await;
    let app = app_state_router(db).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/pool-metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.headers()
            .get("content-type")
            .map(|v| v.to_str().unwrap_or("")),
        Some("application/json")
    );
}

// ── 404 for unknown routes ────────────────────────────────────────────────────

#[tokio::test]
async fn test_unknown_route_returns_404() {
    let db = setup_db().await;
    let app = app_state_router(db).await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/nonexistent-endpoint")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── POST not allowed on GET-only routes ──────────────────────────────────────

#[tokio::test]
async fn test_post_to_get_only_route_returns_405() {
    let db = setup_db().await;
    let app = app_state_router(db).await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/health")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}
