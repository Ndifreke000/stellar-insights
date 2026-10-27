//! Shared helpers for integration tests that are their own test crate root
//! (e.g. `tests/preflight.rs`). Not every test uses every helper.
#![allow(dead_code)]

use axum::routing::{get, post};
use axum::Router;
use sqlx::SqlitePool;
use std::sync::Arc;

use payraider_backend::api::preflight::{preflight_batch, preflight_get, preflight_post};
use payraider_backend::cache::{CacheConfig, CacheManager};
use payraider_backend::database::Database;
use payraider_backend::rpc::StellarRpcClient;
use payraider_backend::services::price_feed::{
    default_asset_mapping, PriceFeedClient, PriceFeedConfig,
};

/// Router exposing the preflight endpoints at the same paths as the v1 API
/// (`/api/v1/preflight`, `/api/v1/preflight/batch`), backed by an empty
/// in-memory database, an in-process cache (no Redis) and the RPC client in
/// mock mode, so the tests need no network.
pub async fn test_app() -> Router {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("in-memory SQLite pool");
    let db = Arc::new(Database::new(pool));
    let cache = Arc::new(CacheManager::new_in_memory_for_tests(CacheConfig::default()));
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let price_feed = Arc::new(PriceFeedClient::new(
        PriceFeedConfig::default(),
        default_asset_mapping(),
    ));

    Router::new()
        .route("/api/v1/preflight", get(preflight_get).post(preflight_post))
        .route("/api/v1/preflight/batch", post(preflight_batch))
        .with_state((db, cache, rpc_client, price_feed))
}
