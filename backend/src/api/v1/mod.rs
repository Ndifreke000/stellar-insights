use crate::api::{
    account_merges, anchors, cache_stats, corridors, cost_calculator, fee_bump, liquidity_pools,
    metrics, oauth, price_feed as price_feed_api, rpc, webhooks,
};
use crate::auth_middleware::auth_middleware;
use crate::cache::CacheManager;
use crate::database::Database;
use crate::rate_limit::{rate_limit_middleware, RateLimiter};
use crate::rpc::StellarRpcClient;
use crate::services::account_merge_detector::AccountMergeDetector;
use crate::services::fee_bump_tracker::FeeBumpTrackerService;
use crate::services::liquidity_pool_analyzer::LiquidityPoolAnalyzer;
use crate::services::price_feed::PriceFeedClient;
use crate::state::AppState;
use axum::{
    middleware,
    routing::{get, put},
    Json, Router,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Serialize)]
struct ApiVersion {
    current: String,
    supported: Vec<String>,
    deprecated: Vec<String>,
    sunset_dates: HashMap<String, String>,
}

async fn get_api_version() -> Json<ApiVersion> {
    Json(ApiVersion {
        current: "v1".to_string(),
        supported: vec!["v1".to_string(), "v2".to_string()],
        deprecated: vec![],
        sunset_dates: HashMap::new(),
    })
}

async fn v2_not_implemented() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "API v2 is reserved for future releases",
        "status": "not_implemented"
    }))
}

fn v2_routes() -> Router {
    Router::new().route("/status", get(v2_not_implemented))
}

pub fn routes(
    app_state: AppState,
    cached_state: (
        Arc<Database>,
        Arc<CacheManager>,
        Arc<StellarRpcClient>,
        Arc<PriceFeedClient>,
    ),
    rpc_client: Arc<StellarRpcClient>,
    fee_bump_tracker: Arc<FeeBumpTrackerService>,
    account_merge_detector: Arc<AccountMergeDetector>,
    lp_analyzer: Arc<LiquidityPoolAnalyzer>,
    price_feed: Arc<PriceFeedClient>,
    rate_limiter: Arc<RateLimiter>,
    cors: CorsLayer,
    pool: sqlx::SqlitePool,
    cache: Arc<CacheManager>,
) -> Router {
    // 1. Cached routes
    let cached_routes = Router::new()
        .route("/anchors", get(anchors::get_anchors))
        .route("/corridors", get(corridors::list_corridors))
        .route(
            "/corridors/:corridor_key",
            get(corridors::get_corridor_detail),
        )
        .with_state(cached_state);

    // 2. Public anchor routes
    let public_anchor_routes = Router::new()
        .route("/health", get(crate::handlers::health_check))
        .route("/db/pool-metrics", get(crate::handlers::pool_metrics))
        .route("/anchors/:id", get(anchors::get_anchor))
        .route(
            "/anchors/account/:stellar_account",
            get(anchors::get_anchor_by_account),
        )
        .route("/anchors/:id/assets", get(anchors::get_anchor_assets))
        .route("/analytics/muxed", get(anchors::get_muxed_analytics))
        .with_state(app_state.clone());

    // 3. Protected anchor routes
    let protected_routes = Router::new()
        .route("/anchors", axum::routing::post(anchors::create_anchor))
        .route("/anchors/:id/metrics", put(anchors::update_anchor_metrics))
        .route(
            "/anchors/:id/assets",
            axum::routing::post(anchors::create_anchor_asset),
        )
        .route(
            "/corridors",
            axum::routing::post(corridors::create_corridor),
        )
        .route(
            "/corridors/:id/metrics-from-transactions",
            put(corridors::update_corridor_metrics_from_transactions),
        )
        .with_state(app_state)
        .layer(middleware::from_fn(auth_middleware));

    let protected_webhook_routes = Router::new()
        .nest("/webhooks", webhooks::routes(pool.clone()))
        .layer(middleware::from_fn(auth_middleware));

    // 4. RPC routes
    let rpc_routes = Router::new()
        .route("/rpc/health", get(rpc::rpc_health_check))
        .route("/rpc/ledger/latest", get(rpc::get_latest_ledger))
        .route("/rpc/payments", get(rpc::get_payments))
        .route(
            "/rpc/payments/account/:account_id",
            get(rpc::get_account_payments),
        )
        .route("/rpc/trades", get(rpc::get_trades))
        .route("/rpc/orderbook", get(rpc::get_order_book))
        .with_state(rpc_client);

    // 5. Special service routes
    let service_routes = Router::new()
        .nest("/fee-bumps", fee_bump::routes(fee_bump_tracker))
        .nest(
            "/account-merges",
            account_merges::routes(account_merge_detector),
        )
        .nest("/liquidity-pools", liquidity_pools::routes(lp_analyzer))
        .nest("/prices", price_feed_api::routes(price_feed.clone()))
        .nest("/cost-calculator", cost_calculator::routes(price_feed))
        .nest("/cache/stats", cache_stats::routes(cache.clone()))
        .nest("/metrics", metrics::routes(cache));

    // 6. OAuth routes
    let oauth_routes = oauth::routes(pool);

    // V1 router (mounted at /api/v1 and also preserved at root for compatibility)
    let v1_router = Router::new()
        .merge(cached_routes)
        .merge(public_anchor_routes)
        .merge(protected_routes)
        .merge(protected_webhook_routes)
        .merge(rpc_routes)
        .merge(service_routes)
        .merge(oauth_routes);

    // Combine all routes
    Router::new()
        .nest("/api/v1", v1_router.clone())
        .nest("/api/v2", v2_routes())
        .route("/api/version", get(get_api_version))
        // Preserve existing unversioned endpoints for backward compatibility.
        .merge(v1_router)
        .layer(cors)
        .layer(middleware::from_fn(
            crate::request_id::request_id_middleware,
        ))
        .layer(middleware::from_fn(
            crate::api_v1_middleware::version_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware,
        ))
}
