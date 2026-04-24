use anyhow::Context;
use axum::{
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    middleware,
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tower_http::{
    compression::{predicate::{And, NotForContentType, SizeAbove}, CompressionLayer, CompressionLevel},
    cors::{AllowOrigin, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use stellar_insights_backend::{
    api::v1::routes,
    backup::{BackupConfig, BackupManager},
    cache::{CacheConfig, CacheManager},
    database::{Database, PoolConfig},
    env_config,
    ingestion::DataIngestionService,
    observability::metrics as obs_metrics,
    observability::tracing::trace_propagation_middleware,
    openapi::ApiDoc,
    rate_limit::RateLimiter,
    request_id::request_id_middleware,
    rpc::StellarRpcClient,
    services::{
        service_container::ServiceContainer,
        webhook_dispatcher::WebhookDispatcher,
    },
    shutdown::{
        flush_cache, log_shutdown_summary, shutdown_background_tasks, shutdown_database,
        shutdown_signal, shutdown_websockets, wait_for_signal, ShutdownConfig, ShutdownCoordinator,
    },
    state::AppState,
    websocket::WsState,
};

const DB_POOL_LOG_INTERVAL: Duration = Duration::from_secs(60);
const DB_POOL_IDLE_LOW_WATERMARK: usize = 2;

/// Default request timeout in seconds
const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 30;
/// Minimum allowed request timeout (prevents misconfiguration)
const MIN_REQUEST_TIMEOUT_SECONDS: u64 = 5;
/// Maximum allowed request timeout (prevents resource exhaustion)
const MAX_REQUEST_TIMEOUT_SECONDS: u64 = 300;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match dotenvy::dotenv() {
        Ok(path) => tracing::info!("Loaded environment from {}", path.display()),
        Err(dotenvy::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!(".env file not found, using environment variables only");
        }
        Err(e) => tracing::warn!("Failed to load .env file: {}", e),
    }
    env_config::log_env_config();

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

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;
    tracing::info!("Database migrations completed successfully");

    let db = Arc::new(Database::new(pool.clone()));

    // Database pool metrics logger
    let pool_metrics_handle: JoinHandle<()> = {
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
        })
    };

    // Pool exhaustion monitoring: warn at >90% utilization, update Prometheus gauges
    let pool_exhaustion_handle: JoinHandle<()> = {
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
        })
    };

    // Initialize cache manager
    let cache = Arc::new(
        CacheManager::new(CacheConfig::from_env())
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to initialize cache manager, using in-memory fallback: {}", e);
                CacheManager::new_in_memory_for_tests(CacheConfig::from_env())
            }),
    );

    // Initialize Stellar RPC Client
    let mock_mode = std::env::var("RPC_MOCK_MODE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(mock_mode));

    // Build all services via the container (dependency injection — issue #1123)
    let services = ServiceContainer::build(pool.clone(), rpc_client.clone());

    let ws_state = Arc::new(WsState::new());
    let ingestion = Arc::new(DataIngestionService::new(rpc_client.clone(), db.clone()));

    let app_state = AppState::new(
        db.clone(),
        cache.clone(),
        ws_state.clone(),
        ingestion,
        rpc_client.clone(),
    );

    let fee_bump_tracker = services.fee_bump_tracker;
    let account_merge_detector = services.account_merge_detector;
    let lp_analyzer = services.lp_analyzer;
    let price_feed = services.price_feed.clone();

    let cached_state = (
        db.clone(),
        cache.clone(),
        rpc_client.clone(),
        price_feed.clone(),
    );

    let backup_config = BackupConfig::from_env();
    if backup_config.enabled {
        let backup_manager = Arc::new(BackupManager::new(backup_config));
        std::mem::drop(backup_manager.spawn_scheduler());
        tracing::info!("Backup scheduler enabled");
    }

    let rate_limiter = Arc::new(
        RateLimiter::new()
            .await
            .context("Failed to initialize rate limiter")?,
    );

    // Configure rate limits for expensive operations
    use stellar_insights_backend::rate_limit::{ClientRateLimits, RateLimitConfig};
    
    // Export endpoints (CSV/Excel generation)
    rate_limiter.register_endpoint(
        "/api/export/csv".to_string(),
        RateLimitConfig {
            requests_per_minute: 5,
            whitelist_ips: vec![],
            client_limits: Some(ClientRateLimits {
                authenticated: 10,
                premium: 20,
                anonymous: 5,
            }),
        },
    ).await;
    
    rate_limiter.register_endpoint(
        "/api/export/excel".to_string(),
        RateLimitConfig {
            requests_per_minute: 5,
            whitelist_ips: vec![],
            client_limits: Some(ClientRateLimits {
                authenticated: 10,
                premium: 20,
                anonymous: 5,
            }),
        },
    ).await;
    
    // Analytics aggregation queries
    rate_limiter.register_endpoint(
        "/api/analytics".to_string(),
        RateLimitConfig {
            requests_per_minute: 20,
            whitelist_ips: vec![],
            client_limits: Some(ClientRateLimits {
                authenticated: 40,
                premium: 100,
                anonymous: 20,
            }),
        },
    ).await;
    
    // RPC proxy endpoints
    rate_limiter.register_endpoint(
        "/api/rpc".to_string(),
        RateLimitConfig {
            requests_per_minute: 100,
            whitelist_ips: vec![],
            client_limits: Some(ClientRateLimits {
                authenticated: 200,
                premium: 500,
                anonymous: 100,
            }),
        },
    ).await;

    let webhook_dispatcher_handle: JoinHandle<()> = {
        let webhook_pool = pool.clone();
        let max_restarts: u32 = std::env::var("WEBHOOK_DISPATCHER_MAX_RESTARTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        tokio::spawn(async move {
            let mut restarts: u32 = 0;
            loop {
                let dispatcher = WebhookDispatcher::new(webhook_pool.clone());
                match dispatcher.run().await {
                    Ok(()) => {
                        tracing::warn!("Webhook dispatcher exited cleanly; restarting");
                    }
                    Err(e) => {
                        restarts += 1;
                        tracing::error!(
                            restarts,
                            max_restarts,
                            error = %e,
                            "Webhook dispatcher failed"
                        );
                        if restarts >= max_restarts {
                            tracing::error!(
                                "Webhook dispatcher exceeded max restarts ({}); giving up",
                                max_restarts
                            );
                            break;
                        }
                    }
                }
                // Exponential back-off capped at 60 s before restarting.
                let backoff_secs = std::cmp::min(2u64.saturating_pow(restarts), 60);
                tracing::info!("Restarting webhook dispatcher in {}s", backoff_secs);
                tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
            }
        })
    };

    // CORS configuration
    let allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    let wildcard_origins = allowed_origins.trim() == "*";
    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|origin| {
            let trimmed = origin.trim();
            if trimmed == "*" {
                return None;
            }
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

    if origins.is_empty() && !wildcard_origins {
        tracing::warn!(
            "CORS: no valid origins parsed from CORS_ALLOWED_ORIGINS='{}'. \
             All cross-origin requests will be rejected.",
            allowed_origins
        );
    }

    let allow_origin = if wildcard_origins {
        tracing::info!("CORS: wildcard origin configured; mirroring request origin");
        AllowOrigin::mirror_request()
    } else {
        AllowOrigin::list(origins)
    };

    let cors = CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    // Compression configuration
    let compression_min_size: u16 = std::env::var("COMPRESSION_MIN_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024);

    // COMPRESSION_LEVEL: "fastest", "best", or a numeric quality (0-9). Default: "default".
    let compression_level = match std::env::var("COMPRESSION_LEVEL")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "fastest" => CompressionLevel::Fastest,
        "best" => CompressionLevel::Best,
        s => s
            .parse::<i32>()
            .map(CompressionLevel::Precise)
            .unwrap_or(CompressionLevel::Default),
    };

    tracing::info!(
        "Compression enabled (gzip, brotli) for responses > {} bytes, level={:?}",
        compression_min_size,
        compression_level
    );

    // Request timeout configuration — reads REQUEST_TIMEOUT_SECONDS from env,
    // defaults to 30s, clamped to [5, 300] to prevent misconfiguration.
    let request_timeout_seconds = std::env::var("REQUEST_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECONDS)
        .clamp(MIN_REQUEST_TIMEOUT_SECONDS, MAX_REQUEST_TIMEOUT_SECONDS);

    tracing::info!(
        "Request timeout configured: {}s (env REQUEST_TIMEOUT_SECONDS, range {}-{})",
        request_timeout_seconds,
        MIN_REQUEST_TIMEOUT_SECONDS,
        MAX_REQUEST_TIMEOUT_SECONDS,
    );

    // Request timeout: 408 on exceeded time. WebSocket routes are excluded (see `ws_routes`).
    let timeout_layer = TimeoutLayer::with_status_code(
        axum::http::StatusCode::REQUEST_TIMEOUT,
        Duration::from_secs(request_timeout_seconds),
    );

    // WebSocket routes are excluded from the timeout layer — WS connections
    // are long-lived and must not be killed by the HTTP request timeout.
    let ws_routes = Router::new()
        .route("/ws", stellar_insights_backend::websocket::ws_route())
        .with_state(Arc::clone(&ws_state))
        .layer(cors.clone());

    let base_routes = routes(
        app_state.clone(),
        cached_state,
        rpc_client.clone(),
        fee_bump_tracker,
        account_merge_detector,
        lp_analyzer,
        price_feed,
        rate_limiter,
        cors,
        pool.clone(),
        cache.clone(),
    );

    let app = base_routes
        .merge(ws_routes)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(middleware::from_fn(
            stellar_insights_backend::api_deprecation_middleware::deprecation_middleware,
            stellar_insights_backend::payload_limit::payload_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            db.clone(),
            stellar_insights_backend::api_analytics_middleware::api_analytics_middleware,
        ))
        // trace_propagation_middleware must be inside TraceLayer so a span already
        // exists when it calls set_parent(). In Axum's layer stack the last
        // .layer() is outermost (runs first), so placing trace_propagation_middleware
        // before TraceLayer here means it executes after TraceLayer at runtime.
        .layer(middleware::from_fn(trace_propagation_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(obs_metrics::http_metrics_middleware))
        .layer(middleware::from_fn(request_id_middleware))
        .layer(timeout_layer)
        .layer(
            CompressionLayer::new()
                .gzip(true)
                .br(true)
                .quality(compression_level)
                .compress_when(
                    SizeAbove::new(compression_min_size)
                        .and(NotForContentType::IMAGES)
                        .and(NotForContentType::SSE),
                ),
        );

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let start_shutdown = std::time::Instant::now();

    // Shutdown coordinator
    let shutdown_coordinator = Arc::new(ShutdownCoordinator::new(ShutdownConfig::from_env()));

    let mut background_tasks: Vec<JoinHandle<()>> = vec![
        pool_metrics_handle,
        pool_exhaustion_handle,
        webhook_dispatcher_handle,
    ];

    // Graceful shutdown handler
    let shutdown_handler: JoinHandle<()> = {
        let shutdown_pool = pool.clone();
        let shutdown_cache = cache.clone();
        let shutdown_ws_state = ws_state.clone();
        let coordinator = shutdown_coordinator.clone();
        tokio::spawn(async move {
            wait_for_signal().await;
            coordinator.trigger_shutdown();
            shutdown_websockets(shutdown_ws_state, coordinator.background_task_timeout()).await;
            flush_cache(shutdown_cache, coordinator.background_task_timeout()).await;
            shutdown_database(shutdown_pool, coordinator.db_close_timeout()).await;
        })
    };

    background_tasks.push(shutdown_handler);
    // Clone references needed inside the graceful shutdown future
    let shutdown_pool = pool.clone();
    let shutdown_cache = cache.clone();
    let shutdown_ws_state = ws_state.clone();

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            let coordinator = shutdown_coordinator.clone();
            coordinator.trigger_shutdown();
            shutdown_websockets(shutdown_ws_state, coordinator.background_task_timeout()).await;
            flush_cache(shutdown_cache, coordinator.background_task_timeout()).await;
            shutdown_database(shutdown_pool, coordinator.db_close_timeout()).await;
        })
        .await?;

    shutdown_background_tasks(
        background_tasks,
        ShutdownConfig::from_env().background_task_timeout,
    )
    .await;

    log_shutdown_summary(start_shutdown);
    tracing::info!("Server shutdown complete");
    stellar_insights_backend::observability::tracing::shutdown_tracing();

    Ok(())
}
