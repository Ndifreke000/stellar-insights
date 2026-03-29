use anyhow::{Context, Result};
use axum::{
    http::Method,
    routing::{get, put},
    Router,
    middleware,
};
use dotenvy::dotenv;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower::timeout::TimeoutLayer;
use tower_http::compression::{predicate::SizeAbove, CompressionLayer};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use anyhow::Context;
use axum::http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, Method,
};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    timeout::TimeoutLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use stellar_insights_backend::alerts::AlertManager;
use stellar_insights_backend::api::account_merges;
use stellar_insights_backend::api::anchors_cached::get_anchors;
use stellar_insights_backend::api::api_analytics;
use stellar_insights_backend::api::api_keys;
use stellar_insights_backend::api::cache_stats;
use stellar_insights_backend::api::corridors_cached::{get_corridor_detail, list_corridors};
use stellar_insights_backend::api::cost_calculator;
use stellar_insights_backend::api::fee_bump;
use stellar_insights_backend::api::liquidity_pools;
use stellar_insights_backend::api::metrics_cached;
use stellar_insights_backend::api::oauth;
use stellar_insights_backend::api::v1::routes;
use stellar_insights_backend::api::verification_rewards;
use stellar_insights_backend::api::webhooks;
use stellar_insights_backend::auth::AuthService;
use stellar_insights_backend::auth_middleware::auth_middleware;
use stellar_insights_backend::backup::{BackupConfig, BackupManager};
use stellar_insights_backend::cache::{CacheConfig, CacheManager};
use stellar_insights_backend::cache_invalidation::CacheInvalidationService;
use stellar_insights_backend::database::{Database, PoolConfig};
use stellar_insights_backend::gdpr::{handlers as gdpr_handlers, GdprService};
use stellar_insights_backend::handlers::*;
use stellar_insights_backend::ingestion::ledger::LedgerIngestionService;
use stellar_insights_backend::ingestion::DataIngestionService;
use stellar_insights_backend::ip_whitelist_middleware::{
    ip_whitelist_middleware, IpWhitelistConfig,
};
use stellar_insights_backend::jobs::JobScheduler;
use stellar_insights_backend::monitor::CorridorMonitor;
use stellar_insights_backend::network::NetworkConfig;
use stellar_insights_backend::observability::tracing::trace_propagation_middleware;
use stellar_insights_backend::observability::{metrics as obs_metrics, tracing as obs_tracing};
use stellar_insights_backend::openapi::ApiDoc;
use stellar_insights_backend::rate_limit::{rate_limit_middleware, RateLimitConfig, RateLimiter};
use stellar_insights_backend::request_id::request_id_middleware;
use stellar_insights_backend::rpc::StellarRpcClient;
use stellar_insights_backend::rpc_handlers;
use stellar_insights_backend::services::account_merge_detector::AccountMergeDetector;
use stellar_insights_backend::services::fee_bump_tracker::FeeBumpTrackerService;
use stellar_insights_backend::services::liquidity_pool_analyzer::LiquidityPoolAnalyzer;
use stellar_insights_backend::services::price_feed::{
    default_asset_mapping, PriceFeedClient, PriceFeedConfig,
};
use stellar_insights_backend::services::realtime_broadcaster::RealtimeBroadcaster;
use stellar_insights_backend::services::trustline_analyzer::TrustlineAnalyzer;
use stellar_insights_backend::services::webhook_dispatcher::WebhookDispatcher;
use stellar_insights_backend::state::AppState;
use stellar_insights_backend::telegram;
use stellar_insights_backend::websocket::WsState;

const DB_POOL_LOG_INTERVAL: Duration = Duration::from_secs(60);
const DB_POOL_IDLE_LOW_WATERMARK: usize = 2;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present. A missing file is fine (production/CI uses real env vars).
    // Any other error — malformed syntax, permission denied — is logged as a warning
    // so it doesn't silently corrupt configuration.
    match dotenvy::dotenv() {
        Ok(path) => tracing::debug!("Loaded environment from {}", path.display()),
        Err(dotenvy::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(".env file not found, using environment variables only");
        }
        Err(e) => tracing::warn!("Failed to load .env file: {}", e),
    }
    env_config::log_env_config();
    let _tracing_guard =
        stellar_insights_backend::observability::tracing::init_tracing("stellar-insights-backend")?;
    stellar_insights_backend::observability::metrics::init_metrics();
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
        cache.clone(),
        ws_state,
        ingestion,
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

    // Initialize auth service
    let auth_service = Arc::new(AuthService::new(db.clone()));

    // Initialize alert manager
    let alert_manager = Arc::new(AlertManager::new(db.clone(), cache.clone()));

    // Start webhook dispatcher as a background task
    let webhook_pool = pool.clone();
    tokio::spawn(async move {
        let dispatcher = WebhookDispatcher::new(webhook_pool);
        if let Err(e) = dispatcher.run().await {
            tracing::error!("Webhook dispatcher stopped: {}", e);
        }
    });
    let allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

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
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    let timeout_seconds: u64 = std::env::var("REQUEST_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(1024);

    // Compression configuration
    let compression_min_size: usize = std::env::var("COMPRESSION_MIN_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024); // Default to 1KB

    let compression = CompressionLayer::new()
        .gzip(true)
        .br(true)
        .compress_when(SizeAbove::new(compression_min_size));

    tracing::info!(
        "Compression enabled (gzip, brotli) for responses > {} bytes",
        compression_min_size
    );

    // Request timeout configuration
    let request_timeout_seconds = std::env::var("REQUEST_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60)
        .clamp(5, 300); // Enforce 5s minimum, 300s maximum

    tracing::info!(
        "Request timeout configured: {} seconds",
        request_timeout_seconds
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
        .nest(
            "/api/network",
            stellar_insights_backend::api::network::routes(),
        )
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

    // Build WebSocket routes (excluded from request timeout — long-lived connections)

    // Build WebSocket routes
    let ws_routes = Router::new()
        .route("/ws", get(stellar_insights_backend::websocket::ws_handler))
        .with_state(Arc::clone(&ws_state))
        .layer(cors.clone());

    let alert_ws_routes = Router::new()
        .route(
            "/ws/alerts",
            get(stellar_insights_backend::alert_handlers::alert_websocket_handler),
        )
        .with_state(Arc::clone(&alert_manager))
        .layer(cors.clone());

    // Timeout + JSON error handler for non-WebSocket routes
    let timeout_layer = tower::ServiceBuilder::new()
        .layer(axum::error_handling::HandleErrorLayer::new(
            |_: tower::BoxError| async {
                (
                    axum::http::StatusCode::REQUEST_TIMEOUT,
                    axum::Json(serde_json::json!({
                        "error": "REQUEST_TIMEOUT",
                        "message": "Request exceeded the maximum allowed time"
                    })),
                )
            },
        ))
        .layer(TimeoutLayer::new(Duration::from_secs(
            request_timeout_seconds,
        )));

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
        .merge(ws_routes)
        .layer(compression); // Apply compression to all routes

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
    .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
    .layer(TimeoutLayer::new(Duration::from_secs(timeout_seconds)))
    .layer(middleware::from_fn_with_state(
        db.clone(),
        stellar_insights_backend::api_analytics_middleware::api_analytics_middleware,
    ))
    .layer(TraceLayer::new_for_http())
    .layer(middleware::from_fn(trace_propagation_middleware))
    .layer(middleware::from_fn(obs_metrics::http_metrics_middleware))
    .layer(middleware::from_fn(request_id_middleware))
    .layer(timeout_layer) // Apply request timeout to all non-WS routes
    .layer(compression); // Apply compression to all routes
    tracing::info!("Request timeout set to {} seconds", timeout_seconds);

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let start_shutdown = std::time::Instant::now();
    
    // Setup shutdown coordinator
    let shutdown_config = stellar_insights_backend::shutdown::ShutdownConfig::from_env();
    let shutdown_coordinator = stellar_insights_backend::shutdown::ShutdownCoordinator::new(shutdown_config);
    
    // Track background tasks for graceful shutdown
    let mut background_tasks = Vec::<JoinHandle<()>>::new();
    
    // Clone references for shutdown tasks
    let shutdown_pool = pool.clone();
    let shutdown_cache = cache.clone();
    let shutdown_ws_state = ws_state.clone();
    let shutdown_coordinator_clone = shutdown_coordinator.clone();
    
    // Spawn graceful shutdown handler
    let shutdown_handler = tokio::spawn(async move {
        // Wait for shutdown signal
        stellar_insights_backend::shutdown::wait_for_signal().await;
        
        // Trigger shutdown notification
        shutdown_coordinator_clone.trigger_shutdown();
        
        // Graceful shutdown sequence
        
        // 1. Shutdown WebSocket connections
        stellar_insights_backend::shutdown::shutdown_websockets(
            shutdown_ws_state,
            shutdown_coordinator_clone.background_task_timeout(),
        ).await;
        
        // 2. Flush cache
        stellar_insights_backend::shutdown::flush_cache(
            shutdown_cache,
            shutdown_coordinator_clone.background_task_timeout(),
        ).await;
        
        // 3. Close database connections
        stellar_insights_backend::shutdown::shutdown_database(
            shutdown_pool,
            shutdown_coordinator_clone.db_close_timeout(),
        ).await;
    });
    
    background_tasks.push(shutdown_handler);
    
    // Start the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            // Wait for any shutdown signal
            let mut shutdown_rx = shutdown_coordinator.subscribe();
            let _ = shutdown_rx.recv().await;
        })
        .await?;
    
    // Wait for all background tasks to complete
    stellar_insights_backend::shutdown::shutdown_background_tasks(
        background_tasks,
        shutdown_coordinator.background_task_timeout(),
    ).await;
    
    stellar_insights_backend::shutdown::log_shutdown_summary(start_shutdown);
    tracing::info!("Server shutdown complete");
    stellar_insights_backend::observability::tracing::shutdown_tracing();

    Ok(())
