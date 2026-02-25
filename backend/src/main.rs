use anyhow::{Context, Result};
use async_graphql::http::{playground_source, GraphiQLSource};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::{
    http::Method,
    routing::{get, post, put},
    Router,
};
use dotenvy::dotenv;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tower_http::compression::{predicate::SizeAbove, CompressionLayer};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use stellar_insights_backend::alerts::AlertManager;
use stellar_insights_backend::api::account_merges;
use stellar_insights_backend::api::anchors_cached::get_anchors;
use stellar_insights_backend::api::api_analytics;
use stellar_insights_backend::api::api_keys;
use stellar_insights_backend::api::asset_verification;
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
use stellar_insights_backend::elk_health;
// use stellar_insights_backend::graphql::{build_schema, AppSchema};
// use stellar_insights_backend::gdpr::{GdprService, handlers as gdpr_handlers};
use stellar_insights_backend::handlers::*;
use stellar_insights_backend::ingestion::ledger::LedgerIngestionService;
use stellar_insights_backend::ingestion::DataIngestionService;
use stellar_insights_backend::ip_whitelist_middleware::{
    ip_whitelist_middleware, IpWhitelistConfig,
};
use stellar_insights_backend::jobs::JobScheduler;
use stellar_insights_backend::monitor::CorridorMonitor;
use stellar_insights_backend::network::NetworkConfig;
use stellar_insights_backend::observability::{metrics as obs_metrics, tracing as obs_tracing};
use stellar_insights_backend::openapi::ApiDoc;
use stellar_insights_backend::rate_limit::{
    rate_limit_middleware, ClientRateLimits, RateLimitConfig, RateLimiter,
};
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
use stellar_insights_backend::shutdown::{
    flush_cache, log_shutdown_summary, shutdown_background_tasks, shutdown_database,
    shutdown_websockets, wait_for_signal, ShutdownConfig, ShutdownCoordinator,
};
use stellar_insights_backend::state::AppState;
use stellar_insights_backend::telegram;
use stellar_insights_backend::vault;
use stellar_insights_backend::websocket::WsState;

#[tokio::main]
async fn main() -> Result<()> {
    // Track shutdown start time for logging
    let shutdown_start = std::time::Instant::now();

    // Load environment variables
    dotenv().ok();

    // Initialize tracing + optional OpenTelemetry exporter.
    // Hold the guard for process lifetime so rotating file logs are flushed.
    let _log_guard = obs_tracing::init_tracing("stellar-insights-backend")?;
    obs_metrics::init_metrics();

    tracing::info!("Starting Stellar Insights Backend");

    // Validate environment configuration
    stellar_insights_backend::env_config::validate_env()
        .context("Environment configuration validation failed")?;

    // Log sanitized environment configuration
    stellar_insights_backend::env_config::log_env_config();

    // Initialize shutdown coordinator
    let shutdown_config = ShutdownConfig::from_env();
    tracing::info!(
        "Shutdown configuration: graceful_timeout={:?}, background_timeout={:?}, db_timeout={:?}",
        shutdown_config.graceful_timeout,
        shutdown_config.background_task_timeout,
        shutdown_config.db_close_timeout
    );
    let shutdown_coordinator = Arc::new(ShutdownCoordinator::new(shutdown_config.clone()));

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./stellar_insights.db".to_string());

    // Log sanitized database URL to prevent credential leakage (SEC-016)
    let sanitized_db_url = if database_url.starts_with("sqlite:") {
        database_url.clone()
    } else if let Some(at_pos) = database_url.rfind('@') {
        if let Some(scheme_end) = database_url.find("://") {
            format!(
                "{}****@{}",
                &database_url[..scheme_end + 3],
                &database_url[at_pos + 1..]
            )
        } else {
            "[REDACTED]".to_string()
        }
    } else {
        database_url.clone()
    };
    tracing::info!("Connecting to database: {}", sanitized_db_url);

    // Load pool configuration from environment
    let pool_config = stellar_insights_backend::database::PoolConfig::from_env();
    tracing::info!(
        "Database pool configuration: max_connections={}, min_connections={}, \
         connect_timeout={}s, idle_timeout={}s, max_lifetime={}s",
        pool_config.max_connections,
        pool_config.min_connections,
        pool_config.connect_timeout_seconds,
        pool_config.idle_timeout_seconds,
        pool_config.max_lifetime_seconds
    );

    let pool = pool_config.create_pool(&database_url).await?;

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    let db = Arc::new(Database::new(pool.clone()));

    // Initialize Stellar RPC Client
    let mock_mode = std::env::var("RPC_MOCK_MODE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    // Initialize Stellar RPC Client with network configuration
    let network_config = NetworkConfig::from_env();
    tracing::info!(
        "Initializing Stellar RPC client for {} (mock_mode: {})",
        network_config.display_name(),
        mock_mode
    );

    let rpc_client = if mock_mode {
        Arc::new(StellarRpcClient::new_with_network(
            network_config.network,
            true,
        ))
    } else {
        Arc::new(StellarRpcClient::new(
            network_config.rpc_url.clone(),
            network_config.horizon_url.clone(),
            false,
        ))
    };

    // Initialize WebSocket state
    let ws_state = Arc::new(WsState::new());
    tracing::info!("WebSocket state initialized");

    // Initialize Data Ingestion Service
    let ingestion_service = Arc::new(DataIngestionService::new(
        Arc::clone(&rpc_client),
        Arc::clone(&db),
    ));

    // Initialize Fee Bump Tracker Service
    let fee_bump_tracker = Arc::new(FeeBumpTrackerService::new(pool.clone()));

    // Initialize Account Merge Detector Service
    let account_merge_detector = Arc::new(AccountMergeDetector::new(
        pool.clone(),
        Arc::clone(&rpc_client),
    ));

    // Initialize Liquidity Pool Analyzer
    let lp_analyzer = Arc::new(LiquidityPoolAnalyzer::new(
        pool.clone(),
        Arc::clone(&rpc_client),
    ));

    // Initialize Price Feed Client
    let price_feed_config = PriceFeedConfig::from_env();
    let asset_mapping = default_asset_mapping();
    let price_feed = Arc::new(PriceFeedClient::new(price_feed_config, asset_mapping));
    tracing::info!("Price feed client initialized");

    // Initialize Trustline Analyzer
    let trustline_analyzer = Arc::new(TrustlineAnalyzer::new(
        pool.clone(),
        Arc::clone(&rpc_client),
    ));

    // Initialize Ledger Ingestion Service
    let ledger_ingestion_service = Arc::new(LedgerIngestionService::new(
        Arc::clone(&rpc_client),
        Arc::clone(&fee_bump_tracker),
        Arc::clone(&account_merge_detector),
        pool.clone(),
    ));

    // Initialize Redis cache
    let cache_config = CacheConfig::default();
    let cache = Arc::new(CacheManager::new(cache_config).await?);
    tracing::info!("Cache manager initialized");

    // Initialize cache invalidation service
    let cache_invalidation = Arc::new(CacheInvalidationService::new(Arc::clone(&cache)));

    // Initialize AlertManager
    let (alert_manager, _initial_rx) = AlertManager::new();
    let alert_manager = Arc::new(alert_manager);

    // Initialize CorridorMonitor
    let corridor_monitor = Arc::new(CorridorMonitor::new(
        Arc::clone(&alert_manager),
        Arc::clone(&cache),
        Arc::clone(&rpc_client),
    ));

    // Initialize RealtimeBroadcaster
    let mut realtime_broadcaster = RealtimeBroadcaster::new(
        Arc::clone(&ws_state),
        Arc::clone(&db),
        Arc::clone(&rpc_client),
        Arc::clone(&cache),
    );
    tracing::info!("RealtimeBroadcaster initialized");

    // Initialize Webhook Dispatcher
    let webhook_dispatcher = WebhookDispatcher::new(pool.clone());
    tracing::info!("Webhook dispatcher initialized");

    // Create app state for handlers that need it
    let app_state = AppState::new(
        Arc::clone(&db),
        Arc::clone(&ws_state),
        Arc::clone(&ingestion_service),
    );

    // Create cached state tuple for cached API handlers
    let cached_state = (
        Arc::clone(&db),
        Arc::clone(&cache),
        Arc::clone(&rpc_client),
        Arc::clone(&price_feed),
    );

    // Track background tasks for graceful shutdown
    let mut background_tasks: Vec<JoinHandle<()>> = Vec::new();

    // Metrics synchronization task
    let ingestion_clone = Arc::clone(&ingestion_service);
    let cache_invalidation_clone = Arc::clone(&cache_invalidation);
    let shutdown_rx1 = shutdown_coordinator.subscribe();
    let task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
        let mut shutdown_rx = shutdown_rx1;
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = ingestion_clone.sync_all_metrics().await {
                        tracing::error!("Metrics synchronization failed: {}", e);
                        obs_metrics::record_background_job("metrics_sync", "error");
                    } else {
                        obs_metrics::record_background_job("metrics_sync", "success");
                        // Invalidate caches after successful sync
                        if let Err(e) = cache_invalidation_clone.invalidate_anchors().await {
                            tracing::warn!("Failed to invalidate anchor caches: {}", e);
                        }
                        if let Err(e) = cache_invalidation_clone.invalidate_corridors().await {
                            tracing::warn!("Failed to invalidate corridor caches: {}", e);
                        }
                        if let Err(e) = cache_invalidation_clone.invalidate_metrics().await {
                            tracing::warn!("Failed to invalidate metrics caches: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Metrics synchronization task shutting down");
                    break;
                }
            }
        }
    });
    background_tasks.push(task);

    // Initialize Auth Service with its own Redis connection
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let auth_redis_connection = if let Ok(client) = redis::Client::open(redis_url.as_str()) {
        match client.get_multiplexed_tokio_connection().await {
            Ok(conn) => {
                tracing::info!("Auth service connected to Redis");
                Some(conn)
            }
            Err(e) => {
                tracing::warn!(
                    "Auth service failed to connect to Redis ({}), refresh tokens will not persist",
                    e
                );
                None
            }
        }
    } else {
        tracing::warn!("Invalid Redis URL for auth service");
        None
    };
    let auth_service = Arc::new(AuthService::new(Arc::new(tokio::sync::RwLock::new(
        auth_redis_connection.clone(),
    ))));
    tracing::info!("Auth service initialized");

    // Initialize SEP-10 Service for Stellar authentication
    let sep10_redis_connection = Arc::new(tokio::sync::RwLock::new(auth_redis_connection));

    // Get and validate SEP-10 server public key (required for security)
    let sep10_server_key = std::env::var("SEP10_SERVER_PUBLIC_KEY")
        .context("SEP10_SERVER_PUBLIC_KEY environment variable is required for authentication")?;

    // Additional validation: ensure it's not the placeholder value
    if sep10_server_key == "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX" {
        anyhow::bail!(
            "SEP10_SERVER_PUBLIC_KEY is set to placeholder value. \
             Please generate a valid Stellar keypair using: stellar keys generate --network testnet"
        );
    }

    tracing::info!(
        "SEP-10 authentication enabled with server key: {}...",
        &sep10_server_key[..8]
    );

    let sep10_service = Arc::new(
        stellar_insights_backend::auth::sep10_simple::Sep10Service::new(
            std::env::var("SEP10_SERVER_PUBLIC_KEY").unwrap_or_else(|_| {
                "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX".to_string()
            }),
            network_config.network_passphrase.clone(),
            std::env::var("SEP10_HOME_DOMAIN")
                .unwrap_or_else(|_| "stellar-insights.local".to_string()),
            sep10_redis_connection,
        )
        .context("Failed to initialize SEP-10 service")?,
    );
    tracing::info!("SEP-10 service initialized successfully");

    // Initialize Verification Rewards Service
    let verification_rewards_service = Arc::new(
        stellar_insights_backend::services::verification_rewards::VerificationRewardsService::new(
            Arc::clone(&db),
        ),
    );
    tracing::info!("Verification rewards service initialized");

    // Initialize Governance Service
    let governance_service = Arc::new(
        stellar_insights_backend::services::governance::GovernanceService::new(Arc::clone(&db)),
    );
    tracing::info!("Governance service initialized");

    // Initialize GDPR Service
    // let gdpr_service = Arc::new(GdprService::new(pool.clone()));
    // tracing::info!("GDPR service initialized");

    // ML Retraining task (commented out)
    /*
    let ml_service_clone = ml_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(7 * 24 * 3600)); // 7 days
        loop {
            interval.tick().await;
            if let Ok(mut service) = ml_service_clone.try_write() {
                if let Err(e) = service.retrain_weekly().await {
                    tracing::error!("Weekly ML retraining failed: {}", e);
                }
            }
        }
    });
    */

    // Ledger ingestion task
    let ledger_ingestion_clone = Arc::clone(&ledger_ingestion_service);
    let shutdown_rx2 = shutdown_coordinator.subscribe();
    let task = tokio::spawn(async move {
        tracing::info!("Starting ledger ingestion background task");
        let mut shutdown_rx = shutdown_rx2;
        loop {
            tokio::select! {
                result = ledger_ingestion_clone.run_ingestion(5) => {
                    match result {
                        Ok(count) => {
                            obs_metrics::record_background_job("ledger_ingestion", "success");
                            if count == 0 {
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            } else {
                                tokio::task::yield_now().await;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Ledger ingestion failed: {}", e);
                            obs_metrics::record_background_job("ledger_ingestion", "error");
                            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Ledger ingestion task shutting down");
                    break;
                }
            }
        }
    });
    background_tasks.push(task);

    // Liquidity pool sync background task
    let lp_analyzer_clone = Arc::clone(&lp_analyzer);
    let shutdown_rx3 = shutdown_coordinator.subscribe();
    let task = tokio::spawn(async move {
        tracing::info!("Starting liquidity pool sync background task");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
        let mut shutdown_rx = shutdown_rx3;
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = lp_analyzer_clone.sync_pools().await {
                        tracing::error!("Liquidity pool sync failed: {}", e);
                        obs_metrics::record_background_job("liquidity_pool_sync", "error");
                    } else {
                        obs_metrics::record_background_job("liquidity_pool_sync", "success");
                    }
                    if let Err(e) = lp_analyzer_clone.take_snapshots().await {
                        tracing::error!("Liquidity pool snapshot failed: {}", e);
                        obs_metrics::record_background_job("liquidity_pool_snapshot", "error");
                    } else {
                        obs_metrics::record_background_job("liquidity_pool_snapshot", "success");
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Liquidity pool sync task shutting down");
                    break;
                }
            }
        }
    });
    background_tasks.push(task);

    // Trustline stats sync background task
    let trustline_analyzer_clone = Arc::clone(&trustline_analyzer);
    let shutdown_rx4 = shutdown_coordinator.subscribe();
    let task = tokio::spawn(async move {
        tracing::info!("Starting trustline stats sync background task");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(900)); // 15 minutes
        let mut shutdown_rx = shutdown_rx4;
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = trustline_analyzer_clone.sync_assets().await {
                        tracing::error!("Trustline sync failed: {}", e);
                        obs_metrics::record_background_job("trustline_sync", "error");
                    } else {
                        obs_metrics::record_background_job("trustline_sync", "success");
                    }
                    if let Err(e) = trustline_analyzer_clone.take_snapshots().await {
                        tracing::error!("Trustline snapshot failed: {}", e);
                        obs_metrics::record_background_job("trustline_snapshot", "error");
                    } else {
                        obs_metrics::record_background_job("trustline_snapshot", "success");
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Trustline stats sync task shutting down");
                    break;
                }
            }
        }
    });
    background_tasks.push(task);

    // Start RealtimeBroadcaster background task
    let shutdown_rx5 = shutdown_coordinator.subscribe();
    let task = tokio::spawn(async move {
        tracing::info!("Starting RealtimeBroadcaster background task");
        let mut shutdown_rx = shutdown_rx5;
        tokio::select! {
            _ = realtime_broadcaster.start() => {
                tracing::info!("RealtimeBroadcaster task completed");
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("RealtimeBroadcaster task shutting down");
            }
        }
    });
    background_tasks.push(task);

    // Initialize Alert Manager
    let (alert_manager_raw, alert_rx) = stellar_insights_backend::alerts::AlertManager::new();
    let alert_manager = Arc::new(alert_manager_raw);
    tracing::info!("Alert manager initialized");

    // Initialize Corridor Monitor
    let corridor_monitor = Arc::new(stellar_insights_backend::monitor::CorridorMonitor::new(
        Arc::clone(&alert_manager),
        Arc::clone(&cache),
        Arc::clone(&rpc_client),
    ));
    tracing::info!("Corridor monitor initialized");

    // Initialize Slack Bot Service
    let slack_webhook_url = std::env::var("SLACK_WEBHOOK_URL").ok();
    if let Some(url) = slack_webhook_url {
        let slack_bot = stellar_insights_backend::services::slack_bot::SlackBotService::new(
            url,
            alert_manager.subscribe(),
        );
        let task = tokio::spawn(async move {
            slack_bot.start().await;
        });
        background_tasks.push(task);
        tracing::info!("Slack bot service started as background task");
    } else {
        tracing::warn!("SLACK_WEBHOOK_URL not set, slack alerts disabled");
    }

    // Start Corridor Monitor background task
    let monitor_clone = Arc::clone(&corridor_monitor);
    let task = tokio::spawn(async move {
        monitor_clone.start().await;
    });
    background_tasks.push(task);
    tracing::info!("Corridor monitor task started");

    // Start Webhook Dispatcher background task
    let shutdown_rx6 = shutdown_coordinator.subscribe();
    let task = tokio::spawn(async move {
        let mut shutdown_rx = shutdown_rx6;
        tokio::select! {
            result = webhook_dispatcher.run() => {
                if let Err(e) = result {
                    tracing::error!("Webhook dispatcher encountered fatal error: {}", e);
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("Webhook dispatcher task shutting down");
            }
        }
    });
    background_tasks.push(task);

    // Start CorridorMonitor background task
    let monitor_clone = Arc::clone(&corridor_monitor);
    let shutdown_rx_monitor = shutdown_coordinator.subscribe();
    let task = tokio::spawn(async move {
        let mut shutdown_rx = shutdown_rx_monitor;
        tokio::select! {
            _ = monitor_clone.start() => {
                tracing::info!("CorridorMonitor task completed");
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("CorridorMonitor task shutting down");
            }
        }
    });
    background_tasks.push(task);

    // Start Telegram Bot (conditionally, when TELEGRAM_BOT_TOKEN is set)
    if let Ok(telegram_token) = std::env::var("TELEGRAM_BOT_TOKEN") {
        tracing::info!("Telegram bot token found, starting bot");
        let tg_subscriptions = Arc::new(telegram::SubscriptionService::new(pool.clone()));
        let tg_bot = telegram::TelegramBot::new(
            &telegram_token,
            Arc::clone(&db),
            Arc::clone(&cache),
            Arc::clone(&rpc_client),
            tg_subscriptions,
            &alert_manager,
        );
        let shutdown_rx_tg = shutdown_coordinator.subscribe();
        let task = tokio::spawn(async move {
            tg_bot.run(shutdown_rx_tg).await;
        });
        background_tasks.push(task);
        tracing::info!("Telegram bot started");
    } else {
        tracing::info!("TELEGRAM_BOT_TOKEN not set, Telegram bot disabled");
    }

    // Run initial sync (skip on network errors)
    tracing::info!("Running initial metrics synchronization...");
    let _ = ingestion_service.sync_all_metrics().await;

    // Start background job scheduler
    tracing::info!("Starting background job scheduler...");
    let _job_scheduler = JobScheduler::start(
        Arc::clone(&db),
        Arc::clone(&cache),
        Arc::clone(&rpc_client),
        Arc::clone(&ingestion_service),
        Arc::clone(&price_feed),
    )
    .await;
    tracing::info!("Background job scheduler started");

    // Initialize rate limiter with database support for API key validation
    let rate_limiter_result = RateLimiter::new_with_db(Some(pool.clone())).await;
    let rate_limiter = match rate_limiter_result {
        Ok(limiter) => {
            tracing::info!("Rate limiter initialized successfully with database support");
            Arc::new(limiter)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to initialize Redis rate limiter, creating with memory fallback: {}",
                e
            );
            Arc::new(
                RateLimiter::new_with_db(Some(pool.clone()))
                    .await
                    .unwrap_or_else(|_| panic!("Failed to create rate limiter: critical error")),
            )
        }
    };

    // Configure rate limits for endpoints with per-client tiers
    rate_limiter
        .register_endpoint(
            "/health".to_string(),
            RateLimitConfig {
                requests_per_minute: 1000,
                whitelist_ips: vec!["127.0.0.1".to_string()],
                client_limits: Some(ClientRateLimits {
                    authenticated: 1000,
                    premium: 5000,
                    anonymous: 1000,
                }),
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/anchors".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 60,
                }),
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/corridors".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 60,
                }),
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/rpc/payments".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 300,
                    premium: 2000,
                    anonymous: 50,
                }),
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/rpc/trades".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 300,
                    premium: 2000,
                    anonymous: 50,
                }),
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/liquidity-pools".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 60,
                }),
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/prices".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 300,
                    premium: 1500,
                    anonymous: 60,
                }),
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/account-merges".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 60,
                }),
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/achievements".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 60,
                }),
            },
        )
        .await;

    // Initialize IP whitelist configuration for admin endpoints
    let ip_whitelist_config = match IpWhitelistConfig::from_env() {
        Ok(config) => {
            tracing::info!(
                "IP whitelist initialized: {} network(s) configured, trust_proxy={}",
                config.allowed_networks.len(),
                config.trust_proxy
            );
            Arc::new(config)
        }
        Err(e) => {
            tracing::error!("Failed to initialize IP whitelist configuration: {}", e);
            tracing::error!("Admin endpoints will be INACCESSIBLE until ADMIN_IP_WHITELIST is properly configured");
            // Create a restrictive default that blocks everything
            Arc::new(IpWhitelistConfig {
                allowed_networks: Arc::new(vec![]),
                trust_proxy: false,
                max_forwarded_ips: 3,
            })
        }
    };

    // CORS configuration
    // Read comma-separated allowed origins from env.
    // Use "*" to allow all origins (development only).
    // Production example: CORS_ALLOWED_ORIGINS=https://stellar-insights.com
    let cors_allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://localhost:3001".to_string());

    tracing::info!(
        "Configuring CORS with allowed origins: {}",
        cors_allowed_origins
    );

    let cors_methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
        Method::PATCH,
        Method::HEAD,
    ];

    let cors = {
        let base = CorsLayer::new()
            .allow_methods(cors_methods)
            .allow_headers(Any)
            .max_age(Duration::from_secs(3600));

        if cors_allowed_origins.trim() == "*" {
            tracing::warn!(
                "CORS configured to allow ALL origins (*). \
                 This is insecure and should not be used in production."
            );
            base.allow_origin(Any)
        } else {
            let origins: Vec<axum::http::HeaderValue> = cors_allowed_origins
                .split(',')
                .filter_map(|o| {
                    let trimmed = o.trim();
                    trimmed
                        .parse::<axum::http::HeaderValue>()
                        .map_err(|e| {
                            tracing::warn!("Skipping invalid CORS origin '{}': {}", trimmed, e);
                        })
                        .ok()
                })
                .collect();

            if origins.is_empty() {
                panic!(
                    "CORS_ALLOWED_ORIGINS contains no valid origins. \
                     Set valid origins or use '*' explicitly for development. \
                     Refusing to fall back to allow-all. (SEC-011)"
                );
            } else {
                tracing::info!("CORS restricted to {} specific origin(s)", origins.len());
                base.allow_origin(origins)
            }
        }
    };

    // Compression configuration
    // Only compress responses larger than 1KB to avoid overhead on small responses
    let compression_min_size = std::env::var("COMPRESSION_MIN_SIZE")
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

    // Build OAuth routes
    let oauth_routes = oauth::routes(pool.clone());

    // Build webhook routes (require authentication)
    let webhook_routes = Router::new()
        .nest("/api/webhooks", webhooks::routes(pool.clone()))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(auth_middleware))
                .layer(middleware::from_fn_with_state(
                    rate_limiter.clone(),
                    rate_limit_middleware,
                )),
        )
        .layer(cors.clone());

    // Build metrics routes (public)
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

    // Build cost calculator routes
    let cost_calculator_routes = Router::new()
        .nest(
            "/api/cost-calculator",
            cost_calculator::routes(Arc::clone(&price_feed)),
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

    // Build GraphQL schema
    // let graphql_schema = build_schema(Arc::new(pool.clone()));
    // tracing::info!("GraphQL schema initialized");

    // GraphQL handler
    // async fn graphql_handler(
    //     State(schema): State<AppSchema>,
    //     req: GraphQLRequest,
    // ) -> GraphQLResponse {
    //     schema.execute(req.into_inner()).await.into()
    // }

    // GraphQL Playground handler
    // async fn graphql_playground() -> impl IntoResponse {
    //     Html(playground_source(
    //         async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    //     ))
    // }

    // Build GraphQL routes
    // let graphql_routes = Router::new()
    //     .route("/graphql", post(graphql_handler))
    //     .route("/graphql/playground", get(graphql_playground))
    //     .with_state(graphql_schema);

    // Build achievements / quests routes
    let achievements_routes = Router::new()
        .nest(
            "/api",
            stellar_insights_backend::api::achievements::routes(),
        )
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build API analytics routes (ADMIN - IP whitelisted)
    let api_analytics_routes = Router::new()
        .merge(api_analytics::routes(db.clone()))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    ip_whitelist_config.clone(),
                    ip_whitelist_middleware,
                ))
                .layer(middleware::from_fn_with_state(
                    rate_limiter.clone(),
                    rate_limit_middleware,
                )),
        )
        .layer(cors.clone());

    // Build cache stats routes (ADMIN - IP whitelisted)
    let cache_routes = Router::new()
        .merge(cache_stats::routes(Arc::clone(&cache)))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    ip_whitelist_config.clone(),
                    ip_whitelist_middleware,
                ))
                .layer(middleware::from_fn_with_state(
                    rate_limiter.clone(),
                    rate_limit_middleware,
                )),
        )
        .layer(cors.clone());

    // Build pool metrics route (ADMIN - IP whitelisted)
    let admin_db_routes = Router::new()
        .route("/api/db/pool-metrics", get(pool_metrics))
        .with_state(app_state.clone())
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    ip_whitelist_config.clone(),
                    ip_whitelist_middleware,
                ))
                .layer(middleware::from_fn_with_state(
                    rate_limiter.clone(),
                    rate_limit_middleware,
                )),
        )
        .layer(cors.clone());

    // Build governance routes
    let governance_routes = Router::new()
        .nest(
            "/api/governance",
            stellar_insights_backend::api::governance::routes(
                governance_service,
                Arc::clone(&sep10_service),
            ),
        )
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build API key management routes
    let api_key_routes = Router::new()
        .nest("/api/keys", api_keys::routes(Arc::clone(&db)))
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )))
        .layer(cors.clone());

    // Build verification rewards routes
    let verification_routes = Router::new()
        .nest(
            "/api/verifications",
            verification_rewards::routes(
                Arc::clone(&verification_rewards_service),
                Arc::clone(&sep10_service),
            ),
        )
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        )));

    // Build GDPR routes (temporarily disabled)
    /*
    let gdpr_routes = Router::new()
        .route("/api/gdpr/consents", get(gdpr_handlers::get_consents))
        .route("/api/gdpr/consents", put(gdpr_handlers::update_consent))
        .route(
            "/api/gdpr/consents/batch",
            put(gdpr_handlers::batch_update_consents),
        )
        .route("/api/gdpr/export", get(gdpr_handlers::get_export_requests))
        .route(
            "/api/gdpr/export",
            post(gdpr_handlers::create_export_request),
        )
        .route(
            "/api/gdpr/export/:id",
            get(gdpr_handlers::get_export_request),
        )
        .route(
            "/api/gdpr/export-types",
            get(gdpr_handlers::get_exportable_types),
        )
        .route(
            "/api/gdpr/deletion",
            get(gdpr_handlers::get_deletion_requests),
        )
        .route(
            "/api/gdpr/deletion",
            post(gdpr_handlers::create_deletion_request),
        )
        .route(
            "/api/gdpr/deletion/:id",
            get(gdpr_handlers::get_deletion_request),
        )
        .route(
            "/api/gdpr/deletion/:id/cancel",
            post(gdpr_handlers::cancel_deletion),
        )
        .route(
            "/api/gdpr/deletion/confirm",
            post(gdpr_handlers::confirm_deletion),
        )
        .route("/api/gdpr/summary", get(gdpr_handlers::get_gdpr_summary))
        .with_state(Arc::clone(&gdpr_service))
        .layer(cors.clone());
    */

    // Merge routers
    let swagger_routes =
        SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());

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

    let app = Router::new()
        .route("/metrics", get(obs_metrics::metrics_handler))
        .route("/api/elk/health", get(elk_health::elk_health_check))
        .route("/api/elk/metrics", get(elk_health::logging_metrics))
        .merge(swagger_routes)
        .merge(auth_routes)
        .merge(oauth_routes)
        .merge(webhook_routes)
        .merge(cached_routes)
        .merge(anchor_routes)
        .merge(protected_anchor_routes)
        .merge(rpc_routes)
        .merge(fee_bump_routes)
        .merge(account_merge_routes)
        .merge(lp_routes)
        .merge(price_routes)
        .merge(cost_calculator_routes)
        .merge(trustline_routes)
        .merge(achievements_routes)
        .merge(governance_routes)
        .merge(network_routes)
        .merge(api_analytics_routes)
        .merge(cache_routes)
        .merge(metrics_routes)
        // .merge(graphql_routes) // Add GraphQL routes
        .merge(admin_db_routes)
        .merge(verification_routes)
        .merge(asset_verification_routes)
        // .merge(gdpr_routes)
        .merge(api_key_routes)
        .merge(ws_routes)
        .merge(alert_ws_routes)
        .layer(middleware::from_fn_with_state(
            db.clone(),
            stellar_insights_backend::api_analytics_middleware::api_analytics_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(obs_metrics::http_metrics_middleware))
        .layer(middleware::from_fn(request_id_middleware))
        .layer(compression); // Apply compression to all routes

    // Start server
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port);

    tracing::info!("Server starting on {}", addr);
    tracing::info!(
        "GraphQL Playground available at http://{}/graphql/playground",
        addr
    );
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Clone resources needed for shutdown
    let pool_for_shutdown = pool.clone();
    let cache_for_shutdown = Arc::clone(&cache);
    let ws_state_for_shutdown = Arc::clone(&ws_state);
    let shutdown_coordinator_clone = Arc::clone(&shutdown_coordinator);

    // Spawn signal handler task
    tokio::spawn(async move {
        wait_for_signal().await;
        tracing::info!("Shutdown signal received, initiating graceful shutdown");
        shutdown_coordinator_clone.trigger_shutdown();
    });

    // Create shutdown signal future
    let shutdown_signal = async move {
        let mut rx = shutdown_coordinator.subscribe();
        let _ = rx.recv().await;
        tracing::info!("Server received shutdown signal, stopping to accept new connections");
    };

    // Start server with graceful shutdown
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal);

    tracing::info!("Server is ready to accept connections");

    // Run the server
    if let Err(e) = server.await {
        tracing::error!("Server error: {}", e);
    }

    tracing::info!("Server stopped accepting new connections, starting cleanup");

    // Graceful shutdown sequence
    tracing::info!("Step 1/4: Shutting down background tasks");
    shutdown_background_tasks(background_tasks, shutdown_config.background_task_timeout).await;

    tracing::info!("Step 2/4: Closing WebSocket connections");
    shutdown_websockets(ws_state_for_shutdown, Duration::from_secs(5)).await;

    tracing::info!("Step 3/4: Flushing cache and closing Redis connections");
    flush_cache(cache_for_shutdown, shutdown_config.db_close_timeout).await;

    tracing::info!("Step 4/4: Closing database connections");
    shutdown_database(pool_for_shutdown, shutdown_config.db_close_timeout).await;

    // Log final shutdown summary
    log_shutdown_summary(shutdown_start);

    tracing::info!("Graceful shutdown complete");

    obs_tracing::shutdown_tracing();

    Ok(())
}
