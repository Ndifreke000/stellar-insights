use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware, Router,
};
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::util::ServiceExt;

// Use correct handlers from the updated API
use stellar_insights_backend::api::corridors::{get_corridor_detail, list_corridors};
use stellar_insights_backend::cache::{CacheConfig, CacheManager};
use stellar_insights_backend::database::Database;
use stellar_insights_backend::request_id::request_id_middleware;
use stellar_insights_backend::rpc::StellarRpcClient;
use stellar_insights_backend::services::price_feed::{
    default_asset_mapping, PriceFeedClient, PriceFeedConfig,
};

async fn setup_test_db() -> SqlitePool {
    SqlitePool::connect(":memory:").await.unwrap()
}

async fn create_test_router(db: Arc<Database>) -> Router {
    let cache = Arc::new(CacheManager::new_in_memory_for_tests(CacheConfig::default()));
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let price_feed = Arc::new(PriceFeedClient::new(
        PriceFeedConfig::default(),
        default_asset_mapping(),
    ));

    let state = (db, cache, rpc_client, price_feed);

    Router::new()
        .route("/api/corridors", axum::routing::get(list_corridors))
        .route(
            "/api/corridors/:corridor_key",
            axum::routing::get(get_corridor_detail),
        )
        .with_state(state)
        .layer(middleware::from_fn(request_id_middleware))
}

#[tokio::test]
async fn test_list_corridors_success() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));

    let app = create_test_router(db).await;

    let request = Request::builder()
        .uri("/api/corridors")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json.is_array());
    let corridors = json.as_array().unwrap();
    assert!(!corridors.is_empty());
    assert!(corridors[0].get("id").is_some());
}

#[tokio::test]
async fn test_get_corridor_detail_success() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));

    let app = create_test_router(db).await;

    // This corridor exists in the mock Stellar RPC payment stream.
    let corridor_key = "XLM%3Anative-%3EXLM%3Anative";
    let request = Request::builder()
        .uri(format!("/api/corridors/{corridor_key}"))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["corridor"]["id"], "XLM:native->XLM:native");
}

#[tokio::test]
async fn test_get_corridor_detail_not_found() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));

    let app = create_test_router(db).await;

    // Use URL encoded corridor key
    let corridor_key = "NONEXISTENT%3Aissuer-%3EFAKE%3Aissuer";
    let request = Request::builder()
        .uri(format!("/api/corridors/{corridor_key}"))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_corridor_detail_invalid_format() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    let app = create_test_router(db).await;

    let invalid_key = "INVALID_FORMAT";
    let request = Request::builder()
        .uri(format!("/api/corridors/{invalid_key}"))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Handler should return BadRequest for invalid format
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
