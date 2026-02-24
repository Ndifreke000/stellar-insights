use crate::api::{
    account_merges, anchors_cached, cache_stats, corridors_cached, cost_calculator, fee_bump,
    liquidity_pools, metrics_cached, oauth, price_feed as price_feed_api, webhooks,
};
use crate::auth_middleware::auth_middleware;
use crate::cache::CacheManager;
use crate::database::Database;
use crate::handlers::*;
use crate::rate_limit::{rate_limit_middleware, RateLimiter};
use crate::rpc::StellarRpcClient;
use crate::rpc_handlers;
use crate::services::account_merge_detector::AccountMergeDetector;
use crate::services::fee_bump_tracker::FeeBumpTrackerService;
use crate::services::liquidity_pool_analyzer::LiquidityPoolAnalyzer;
use crate::services::price_feed::PriceFeedClient;
use crate::state::AppState;
use axum::{
    middleware,
    routing::{get, put},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

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
        .route("/anchors", get(anchors_cached::get_anchors))
        .route("/corridors", get(corridors_cached::list_corridors))
        .route(
            "/corridors/:corridor_key",
            get(corridors_cached::get_corridor_detail),
        )
        .with_state(cached_state);

    // 2. Public anchor routes
    let public_anchor_routes = Router::new()
        .route("/db/pool-metrics", get(pool_metrics))
        .route("/anchors/:id", get(get_anchor))
        .route(
            "/anchors/account/:stellar_account",
            get(get_anchor_by_account),
        )
        .route("/anchors/:id/assets", get(get_anchor_assets))
        .route("/analytics/muxed", get(get_muxed_analytics))
        .with_state(app_state.clone());

    // 3. Protected anchor routes
    let protected_routes = Router::new()
        .route("/anchors", axum::routing::post(create_anchor))
        .route("/anchors/:id/metrics", put(update_anchor_metrics))
        .route(
            "/anchors/:id/assets",
            axum::routing::post(create_anchor_asset),
        )
        .route("/corridors", axum::routing::post(create_corridor))
        .route(
            "/corridors/:id/metrics-from-transactions",
            put(update_corridor_metrics_from_transactions),
        )
        .with_state(app_state)
        .layer(middleware::from_fn(auth_middleware));

    let protected_webhook_routes = Router::new()
        .nest("/webhooks", webhooks::routes(pool.clone()))
        .layer(middleware::from_fn(auth_middleware));

    // 4. RPC routes
    let rpc_routes = Router::new()
        .route("/rpc/health", get(rpc_handlers::rpc_health_check))
        .route("/rpc/ledger/latest", get(rpc_handlers::get_latest_ledger))
        .route("/rpc/payments", get(rpc_handlers::get_payments))
        .route(
            "/rpc/payments/account/:account_id",
            get(rpc_handlers::get_account_payments),
        )
        .route("/rpc/trades", get(rpc_handlers::get_trades))
        .route("/rpc/orderbook", get(rpc_handlers::get_order_book))
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
        .nest("/metrics", metrics_cached::routes(cache));

    // 6. OAuth routes
    let oauth_routes = oauth::routes(pool);

    // Combine all routes
    Router::new()
        .merge(cached_routes)
        .merge(public_anchor_routes)
        .merge(protected_routes)
        .merge(protected_webhook_routes)
        .merge(rpc_routes)
        .merge(service_routes)
        .merge(oauth_routes)
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    rate_limiter,
                    rate_limit_middleware,
                ))
                .layer(middleware::from_fn(
                    crate::api_v1_middleware::version_middleware,
                ))
                .layer(cors),
        )
}
