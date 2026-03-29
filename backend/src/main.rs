use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, Method,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    timeout::TimeoutLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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
use stellar_insights_backend::api::verification_rewards;
use stellar_insights_backend::api::webhooks;
use stellar_insights_backend::auth::AuthService;
use stellar_insights_backend::auth_middleware::auth_middleware;
use stellar_insights_backend::cache::{CacheConfig, CacheManager};
use stellar_insights_backend::cache_invalidation::CacheInvalidationService;
use stellar_insights_backend::database::Database;
use stellar_insights_backend::gdpr::{GdprService, handlers as gdpr_handlers};
use stellar_insights_backend::handlers::*;
use stellar_insights_backend::ingestion::ledger::LedgerIngestionService;
use stellar_insights_backend::ingestion::DataIngestionService;
use stellar_insights_backend::ip_whitelist_middleware::{ip_whitelist_middleware, IpWhitelistConfig};
use stellar_insights_backend::jobs::JobScheduler;
use stellar_insights_backend::network::NetworkConfig;
use stellar_insights_backend::openapi::ApiDoc;
use stellar_insights_backend::observability::{metrics as obs_metrics, tracing as obs_tracing};
use stellar_insights_backend::observability::tracing::trace_propagation_middleware;
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
use stellar_insights_backend::alerts::AlertManager;
use stellar_insights_backend::monitor::CorridorMonitor;
use stellar_insights_backend::telegram;
use stellar_insights_backend::shutdown::{
    flush_cache, log_shutdown_summary, shutdown_background_tasks, shutdown_database,
    shutdown_websockets, wait_for_signal, ShutdownConfig, ShutdownCoordinator,
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
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    let timeout_seconds: u64 = std::env::var("REQUEST_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

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
    .layer(TimeoutLayer::new(Duration::from_secs(timeout_seconds)));

        .layer(middleware::from_fn_with_state(
            db.clone(),
            stellar_insights_backend::api_analytics_middleware::api_analytics_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(trace_propagation_middleware))
        .layer(middleware::from_fn(obs_metrics::http_metrics_middleware))
        .layer(middleware::from_fn(request_id_middleware))
        .layer(compression); // Apply compression to all routes
    tracing::info!("Request timeout set to {} seconds", timeout_seconds);

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!(address = %addr, "Server listening");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    stellar_insights_backend::observability::tracing::shutdown_tracing();

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("Received Ctrl+C, shutting down"); },
        _ = terminate => { tracing::info!("Received SIGTERM, shutting down"); },
    }
}
