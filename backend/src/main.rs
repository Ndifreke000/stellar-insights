use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use stellar_insights_backend::{
    api::v1::routes,
    cache::{CacheConfig, CacheManager},
    database::{Database, PoolConfig},
    env_config,
    ingestion::DataIngestionService,
    rate_limit::RateLimiter,
    rpc::StellarRpcClient,
    services::{
        account_merge_detector::AccountMergeDetector,
        fee_bump_tracker::FeeBumpTrackerService,
        liquidity_pool_analyzer::LiquidityPoolAnalyzer,
        price_feed::{default_asset_mapping, PriceFeedClient, PriceFeedConfig},
        webhook_dispatcher::WebhookDispatcher,
    },
    state::AppState,
    websocket::WsState,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Stellar Insights Backend - Initializing Server");

    let _ = dotenvy::dotenv();
    env_config::log_env_config();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://stellar_insights.db".to_string());
    let pool = PoolConfig::from_env().create_pool(&db_url).await?;
    let db = Arc::new(Database::new(pool.clone()));

    let cache = Arc::new(CacheManager::new(CacheConfig::default()).await.unwrap());

    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));

    let price_feed_config = PriceFeedConfig::default();
    let price_feed = Arc::new(PriceFeedClient::new(
        price_feed_config,
        default_asset_mapping(),
    ));

    let ws_state = Arc::new(WsState::new());
    let ingestion = Arc::new(DataIngestionService::new(rpc_client.clone(), db.clone()));

    let app_state = AppState::new(db.clone(), ws_state, ingestion);
    let cached_state = (
        db.clone(),
        cache.clone(),
        rpc_client.clone(),
        price_feed.clone(),
    );

    let fee_bump_tracker = Arc::new(FeeBumpTrackerService::new(pool.clone()));
    let account_merge_detector =
        Arc::new(AccountMergeDetector::new(pool.clone(), rpc_client.clone()));
    let lp_analyzer = Arc::new(LiquidityPoolAnalyzer::new(pool.clone(), rpc_client.clone()));

    let rate_limiter = Arc::new(RateLimiter::new().await.unwrap());

    // Start webhook dispatcher as a background task
    let webhook_pool = pool.clone();
    tokio::spawn(async move {
        let dispatcher = WebhookDispatcher::new(webhook_pool);
        if let Err(e) = dispatcher.run().await {
            tracing::error!("Webhook dispatcher stopped: {}", e);
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = routes(
        app_state,
        cached_state,
        rpc_client,
        fee_bump_tracker,
        account_merge_detector,
        lp_analyzer,
        price_feed,
        rate_limiter,
        cors,
        pool,
        cache,
    );

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    println!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
