use anyhow::{Context, Result};
use axum::{
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    routing::{get, put},
    middleware,
    Router,
};
use dotenvy::dotenv;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::{
    compression::{predicate::SizeAbove, CompressionLayer},
    cors::{AllowOrigin, Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use stellar_insights_backend::{
    alerts::AlertManager,
    api::{
        account_merges,
        anchors_cached::get_anchors,
        api_analytics,
        api_keys,
        cache_stats,
        corridors_cached::{get_corridor_detail, list_corridors},
        cost_calculator,
        fee_bump,
        liquidity_pools,
        metrics_cached,
        oauth,
        verification_rewards,
        webhooks,
        v1::routes,
    },
    auth::AuthService,
    auth_middleware::auth_middleware,
    backup::{BackupConfig, BackupManager},
    cache::{CacheConfig, CacheManager},
    cache_invalidation::CacheInvalidationService,
    database::{Database, PoolConfig},
    env_config,
    gdpr::{handlers as gdpr_handlers, GdprService},
    handlers::*,
    ingestion::{ledger::LedgerIngestionService, DataIngestionService},
    ip_whitelist_middleware::{ip_whitelist_middleware, IpWhitelistConfig},
    jobs::JobScheduler,
    monitor::CorridorMonitor,
    network::NetworkConfig,
    observability::{
        metrics as obs_metrics,
        tracing::{self as obs_tracing, trace_propagation_middleware},
    },
    openapi::ApiDoc,
    rate_limit::{rate_limit_middleware, RateLimitConfig, RateLimiter},
    request_id::request_id_middleware,
    rpc::StellarRpcClient,
    rpc_handlers,
    services::{
        account_merge_detector::AccountMergeDetector,
        fee_bump_tracker::FeeBumpTrackerService,
        liquidity_pool_analyzer::LiquidityPoolAnalyzer,
        price_feed::{default_asset_mapping, PriceFeedClient, PriceFeedConfig},
        realtime_broadcaster::RealtimeBroadcaster,
        trustline_analyzer::TrustlineAnalyzer,
        webhook_dispatcher::WebhookDispatcher,
    },
    state::AppState,
    websocket::WsState,
    telegram,
};

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
    
    // Validate critical environment variables before proceeding
    env_config::validate_env()
        .context("Environment validation failed - please check your configuration")?;
    
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
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::PATCH,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    // Compression configuration
    let compression_min_size: usize = std::env::var("COMPRESSION_MIN_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024); // Default to 1KB

    let compression = CompressionLayer::new()
        .gzip(true)
        .br(true)
        .compress_when(SizeAbove::new(compression_min_size));

    // Request timeout configuration
    let request_timeout_seconds = std::env::var("REQUEST_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60)
        .clamp(5, 300); // Enforce 5s minimum, 300s maximum

    let swagger_routes =
        SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());

    // Build the main app using the v1 routes factory
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
        pool.clone(),
        cache.clone(),
    );

    // Add extra routes and layers
    let app = app
        .merge(swagger_routes)
        .route("/metrics", get(obs_metrics::metrics_handler))
        .route("/ws", get(stellar_insights_backend::websocket::ws_handler))
        .route("/ws/alerts", get(stellar_insights_backend::alert_handlers::alert_websocket_handler))
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
        .layer(TimeoutLayer::new(Duration::from_secs(request_timeout_seconds)))
        .layer(middleware::from_fn_with_state(
            db.clone(),
            stellar_insights_backend::api_analytics_middleware::api_analytics_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(trace_propagation_middleware))
        .layer(middleware::from_fn(obs_metrics::http_metrics_middleware))
        .layer(compression);

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
    let shutdown_pool = pool;
    let shutdown_cache = cache;
    let shutdown_ws_state = ws_state;
    let shutdown_coordinator_clone = shutdown_coordinator.clone();
    
    // Spawn graceful shutdown handler
    let shutdown_handler = tokio::spawn(async move {
        stellar_insights_backend::shutdown::wait_for_signal().await;
        shutdown_coordinator_clone.trigger_shutdown();
        
        stellar_insights_backend::shutdown::shutdown_websockets(
            shutdown_ws_state,
            shutdown_coordinator_clone.background_task_timeout(),
        ).await;
        
        stellar_insights_backend::shutdown::flush_cache(
            shutdown_cache,
            shutdown_coordinator_clone.background_task_timeout(),
        ).await;
        
        stellar_insights_backend::shutdown::shutdown_database(
            shutdown_pool,
            shutdown_coordinator_clone.db_close_timeout(),
        ).await;
    });
    
    background_tasks.push(shutdown_handler);
    
    // Start the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let mut shutdown_rx = shutdown_coordinator.subscribe();
            let _ = shutdown_rx.recv().await;
        })
        .await?;
    
    stellar_insights_backend::shutdown::shutdown_background_tasks(
        background_tasks,
        shutdown_coordinator.background_task_timeout(),
    ).await;
    
    stellar_insights_backend::shutdown::log_shutdown_summary(start_shutdown);
    tracing::info!("Server shutdown complete");
    stellar_insights_backend::observability::tracing::shutdown_tracing();

    Ok(())
}
