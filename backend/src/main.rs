use std::sync::Arc;
use std::time::Duration;
use tower_http::compression::{CompressionLayer, predicate::SizeAbove};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::timeout::TimeoutLayer;

use anyhow::Context;
use axum::http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, Method,
};

use stellar_insights_backend::{
    api::v1::routes,
    backup::{BackupConfig, BackupManager},
    cache::{CacheConfig, CacheManager},
    database::{Database, PoolConfig},
    env_config,
    ingestion::DataIngestionService,
    openapi::ApiDoc,
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
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

const DB_POOL_LOG_INTERVAL: Duration = Duration::from_secs(60);
const DB_POOL_IDLE_LOW_WATERMARK: usize = 2;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    env_config::log_env_config();
    let _tracing_guard =
        stellar_insights_backend::observability::tracing::init_tracing("stellar-insights-backend")?;
    tracing::info!("Stellar Insights Backend - Initializing Server");

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://stellar_insights.db".to_string());
    let pool = PoolConfig::from_env()
        .create_pool(&db_url)
        .await
        .context("Failed to create database pool")?;

    // Run migrations on startup
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;
    tracing::info!("Database migrations completed successfully");

    let db = Arc::new(Database::new(pool.clone()));

    let pool_metrics_db = Arc::clone(&db);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(DB_POOL_LOG_INTERVAL);
        loop {
            interval.tick().await;
            let metrics = pool_metrics_db.pool_metrics();
            tracing::info!(
                pool_size = metrics.size,
                pool_idle = metrics.idle,
                pool_active = metrics.active,
                "Database pool metrics"
            );

            if metrics.idle <= DB_POOL_IDLE_LOW_WATERMARK {
                tracing::warn!(
                    pool_size = metrics.size,
                    pool_idle = metrics.idle,
                    pool_active = metrics.active,
                    low_watermark = DB_POOL_IDLE_LOW_WATERMARK,
                    "Database pool idle connections are low"
                );
            }
        }
    });

    // Initialize Stellar RPC Client
    let mock_mode = std::env::var("RPC_MOCK_MODE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
    // Pool exhaustion monitoring: warn at >90% utilization, update Prometheus gauges
    {
        let monitor_pool = pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let size = monitor_pool.size();
                let idle = monitor_pool.num_idle() as u32;
                let active = size.saturating_sub(idle);
                if size > 0 && active as f64 / size as f64 > 0.9 {
                    tracing::warn!(
                        "Database pool nearly exhausted: {}/{} connections active",
                        active,
                        size
                    );
                }
                stellar_insights_backend::observability::metrics::set_pool_size(size as i64);
                stellar_insights_backend::observability::metrics::set_pool_idle(idle as i64);
                stellar_insights_backend::observability::metrics::set_pool_active(active as i64);
            }
        });
    }

    let cache = Arc::new(
        CacheManager::new(CacheConfig::default())
            .await
            .context("Failed to initialize cache manager - check Redis connection")?,
    );

    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));

    let price_feed_config = PriceFeedConfig::default();
    let price_feed = Arc::new(PriceFeedClient::new(
        price_feed_config,
        default_asset_mapping(),
    ));

    let ws_state = Arc::new(WsState::new());
    let ingestion = Arc::new(DataIngestionService::new(rpc_client.clone(), db.clone()));

    let app_state = AppState::new(
        db.clone(),
        ws_state,
        ingestion,
        cache.clone(),
        rpc_client.clone(),
    );
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

    let backup_config = BackupConfig::from_env();
    if backup_config.enabled {
        let backup_manager = Arc::new(BackupManager::new(backup_config));
        backup_manager.spawn_scheduler();
        tracing::info!("Backup scheduler enabled");
    }

    let rate_limiter = Arc::new(
        RateLimiter::new()
            .await
            .context("Failed to initialize rate limiter")?,
    );

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
        .filter_map(|origin| {
            let trimmed = origin.trim();
            match trimmed.parse::<HeaderValue>() {
                Ok(value) => {
                    tracing::info!("CORS: allowing origin '{}'", trimmed);
                    Some(value)
                }
                Err(_) => {
                    tracing::warn!(
                        "CORS: skipping invalid origin '{}' — check CORS_ALLOWED_ORIGINS",
                        trimmed
                    );
                    None
                }
            }
        })
        .collect();

    if origins.is_empty() {
        tracing::warn!(
            "CORS: no valid origins parsed from CORS_ALLOWED_ORIGINS='{}'. \
             All cross-origin requests will be rejected.",
            allowed_origins
        );
    }

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::PATCH,
        ])
        .allow_header(AUTHORIZATION)
        .allow_header(CONTENT_TYPE)
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    // Configure request timeout
    let timeout_seconds = std::env::var("REQUEST_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(1024);
    
    let compression = CompressionLayer::new()
        .gzip(true)
        .br(true)
        .compress_when(SizeAbove::new(compression_min_size));
    
    tracing::info!(
        "Compression enabled (gzip, brotli) for responses > {} bytes",
        compression_min_size
    );

    // Import middleware
    use axum::middleware;
    use tower::ServiceBuilder;

    // Build auth router
    let auth_routes = stellar_insights_backend::api::auth::routes(auth_service.clone());

    // Build cached routes (anchors list, corridors list/detail) with cache state
    let cached_routes = Router::new()
        .route("/api/anchors", get(get_anchors))
        .route("/api/corridors", get(list_corridors))
        .route("/api/corridors/:corridor_key", get(get_corridor_detail))
        .with_state(cached_state.clone())
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build non-cached anchor routes with app state
    let anchor_routes = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(get_prometheus_metrics))
        .route("/api/anchors/:id", get(get_anchor))
        .route(
            "/api/anchors/account/:stellar_account",
            get(get_anchor_by_account),
        )
        .route("/api/anchors/:id/assets", get(get_anchor_assets))
        .route("/api/analytics/muxed", get(get_muxed_analytics))
        .with_state(app_state.clone())
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build protected anchor routes (require authentication)
    let protected_anchor_routes = Router::new()
        .route("/api/admin/pool-metrics", get(get_pool_metrics))
        .route("/api/anchors", axum::routing::post(create_anchor))
        .route("/api/anchors/:id/metrics", put(update_anchor_metrics))
        .route(
            "/api/anchors/:id/assets",
            axum::routing::post(create_anchor_asset),
        )
        .route("/api/corridors", axum::routing::post(create_corridor))
        .route(
            "/api/corridors/:id/metrics-from-transactions",
            put(update_corridor_metrics_from_transactions),
        )
        .with_state(app_state.clone())
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(auth_middleware))
                .layer(middleware::from_fn_with_state(
                    rate_limiter.clone(),
                    rate_limit_middleware,
                )),
        )
        .layer(cors.clone());

    // Build cache stats and metrics routes
    let cache_routes = cache_stats::routes(Arc::clone(&cache));
    let metrics_routes = metrics_cached::routes(Arc::clone(&cache));

    // Build RPC router
    let rpc_routes = Router::new()
        .route("/api/rpc/health", get(rpc_handlers::rpc_health_check))
        .route(
            "/api/rpc/ledger/latest",
            get(rpc_handlers::get_latest_ledger),
        )
        .route("/api/rpc/payments", get(rpc_handlers::get_payments))
        .route(
            "/api/rpc/payments/account/:account_id",
            get(rpc_handlers::get_account_payments),
        )
        .route("/api/rpc/trades", get(rpc_handlers::get_trades))
        .route("/api/rpc/orderbook", get(rpc_handlers::get_order_book))
        .with_state(rpc_client)
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build fee bump routes
    let fee_bump_routes = Router::new()
        .nest(
            "/api/fee-bumps",
            fee_bump::routes(Arc::clone(&fee_bump_tracker)),
        )
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build account merge routes
    let account_merge_routes = Router::new()
        .nest(
            "/api/account-merges",
            account_merges::routes(Arc::clone(&account_merge_detector)),
        )
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build liquidity pool routes
    let lp_routes = Router::new()
        .nest(
            "/api/liquidity-pools",
            liquidity_pools::routes(Arc::clone(&lp_analyzer)),
        )
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build price feed routes
    let price_routes = Router::new()
        .nest(
            "/api/prices",
            stellar_insights_backend::api::price_feed::routes(Arc::clone(&price_feed)),
        )
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build network routes
    let network_routes = Router::new()
        .nest("/api/network", stellar_insights_backend::api::network::routes())
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build trustline routes
    let trustline_routes = Router::new()
        .nest(
            "/api/trustlines",
            stellar_insights_backend::api::trustlines::routes(Arc::clone(&trustline_analyzer)),
        )
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Merge routers
    let swagger_routes =
        SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());
    
    // Build WebSocket routes
    let ws_routes = Router::new()
        .route("/ws", get(stellar_insights_backend::websocket::ws_handler))
        .with_state(Arc::clone(&ws_state))
        .layer(cors.clone());
    
    let app = Router::new()
        .merge(swagger_routes)
        .merge(auth_routes)
        .merge(cached_routes)
        .merge(anchor_routes)
        .merge(protected_anchor_routes)
        .merge(rpc_routes)
        .merge(fee_bump_routes)
        .merge(account_merge_routes)
        .merge(lp_routes)
        .merge(price_routes)
        .merge(trustline_routes)
        .merge(network_routes)
        .merge(cache_routes)
        .merge(metrics_routes)
        .merge(ws_routes);
        .layer(compression); // Apply compression to all routes
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let timeout_duration = Duration::from_secs(timeout_seconds);
    tracing::info!("Request timeout set to {} seconds", timeout_seconds);

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
    .layer(TimeoutLayer::new(timeout_duration))
    .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!(address = %addr, "Server listening");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    stellar_insights_backend::observability::tracing::shutdown_tracing();

    Ok(())
}
