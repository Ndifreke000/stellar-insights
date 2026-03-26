use sqlx::SqlitePool;
use std::time::Duration;
use std::sync::Arc;
use axum::http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, Method,
};
use tower_http::cors::{AllowOrigin, CorsLayer};

use stellar_insights_backend::{
    api::v1::routes,
    cache::{CacheConfig, CacheManager},
    database::Database,
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
    openapi::ApiDoc,
    websocket::WsState,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    env_config::log_env_config();
    let _tracing_guard = stellar_insights_backend::observability::tracing::init_tracing(
        "stellar-insights-backend",
    )?;
    tracing::info!("Stellar Insights Backend - Initializing Server");

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://stellar_insights.db".to_string());
    let pool = SqlitePool::connect(&db_url).await?;
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

    let allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,https://stellar-insights.com".to_string());

    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|origin| origin.trim().parse::<HeaderValue>().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

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
    )
    .merge(
        SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()),
    );

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!(address = %addr, "Server listening");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    stellar_insights_backend::observability::tracing::shutdown_tracing();

    Ok(())
}

