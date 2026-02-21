use anyhow::{Context, Result};
use axum::{
    http::Method,
    routing::{get, put},
    Router,
};
use dotenv::dotenv;
use std::sync::Arc;
use std::time::Duration;
use tower_http::compression::{predicate::SizeAbove, CompressionLayer};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use stellar_insights_backend::api::account_merges;
use stellar_insights_backend::api::anchors_cached::get_anchors;
use stellar_insights_backend::api::cache_stats;
use stellar_insights_backend::api::corridors_cached::{get_corridor_detail, list_corridors};
use stellar_insights_backend::api::cost_calculator;
use stellar_insights_backend::api::fee_bump;
use stellar_insights_backend::api::liquidity_pools;
use stellar_insights_backend::api::metrics_cached;
use stellar_insights_backend::api::oauth;
use stellar_insights_backend::api::webhooks;
use stellar_insights_backend::auth::AuthService;
use stellar_insights_backend::auth_middleware::auth_middleware;
use stellar_insights_backend::cache::{CacheConfig, CacheManager};
use stellar_insights_backend::cache_invalidation::CacheInvalidationService;
use stellar_insights_backend::database::Database;
use stellar_insights_backend::handlers::*;
use stellar_insights_backend::ingestion::ledger::LedgerIngestionService;
use stellar_insights_backend::ingestion::DataIngestionService;
use stellar_insights_backend::network::NetworkConfig;
use stellar_insights_backend::openapi::ApiDoc;
use stellar_insights_backend::rate_limit::{rate_limit_middleware, RateLimitConfig, RateLimiter};
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
use stellar_insights_backend::shutdown::{ShutdownConfig, ShutdownCoordinator};
use stellar_insights_backend::state::AppState;
use stellar_insights_backend::vault;
use stellar_insights_backend::websocket::WsState;

#[tokio::main]
async fn main() -> Result<()> {
    // Track shutdown start time for logging
    let _shutdown_start = std::time::Instant::now();

    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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
    let _shutdown_coordinator = Arc::new(ShutdownCoordinator::new(shutdown_config));

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./stellar_insights.db".to_string());

    tracing::info!("Connecting to database: {}", database_url);

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

    // Initialize RealtimeBroadcaster
    let mut realtime_broadcaster = RealtimeBroadcaster::new(
        Arc::clone(&ws_state),
        Arc::clone(&db),
        Arc::clone(&rpc_client),
        Arc::clone(&cache),
    );
    tracing::info!("RealtimeBroadcaster initialized");

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

    let ingestion_clone = Arc::clone(&ingestion_service);
    let cache_invalidation_clone = Arc::clone(&cache_invalidation);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
        loop {
            interval.tick().await;
            if let Err(e) = ingestion_clone.sync_all_metrics().await {
                tracing::error!("Metrics synchronization failed: {}", e);
            } else {
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
    });

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
        .expect("Failed to initialize SEP-10 service"),
    );
    tracing::info!("SEP-10 service initialized");

    // Initialize Verification Rewards Service
    let verification_rewards_service = Arc::new(
        stellar_insights_backend::services::verification_rewards::VerificationRewardsService::new(
            Arc::clone(&db),
        ),
    );
    tracing::info!("Verification rewards service initialized");

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
    tokio::spawn(async move {
        tracing::info!("Starting ledger ingestion background task");
        loop {
            match ledger_ingestion_clone.run_ingestion(5).await {
                Ok(count) => {
                    if count == 0 {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    } else {
                        tokio::task::yield_now().await;
                    }
                }
                Err(e) => {
                    tracing::error!("Ledger ingestion failed: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            }
        }
    });

    // Liquidity pool sync background task
    let lp_analyzer_clone = Arc::clone(&lp_analyzer);
    tokio::spawn(async move {
        tracing::info!("Starting liquidity pool sync background task");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
        loop {
            interval.tick().await;
            if let Err(e) = lp_analyzer_clone.sync_pools().await {
                tracing::error!("Liquidity pool sync failed: {}", e);
            }
            if let Err(e) = lp_analyzer_clone.take_snapshots().await {
                tracing::error!("Liquidity pool snapshot failed: {}", e);
            }
        }
    });

    // Trustline stats sync background task
    let trustline_analyzer_clone = Arc::clone(&trustline_analyzer);
    tokio::spawn(async move {
        tracing::info!("Starting trustline stats sync background task");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(900)); // 15 minutes
        loop {
            interval.tick().await;
            if let Err(e) = trustline_analyzer_clone.sync_assets().await {
                tracing::error!("Trustline sync failed: {}", e);
            }
            if let Err(e) = trustline_analyzer_clone.take_snapshots().await {
                tracing::error!("Trustline snapshot failed: {}", e);
            }
        }
    });

    // Start RealtimeBroadcaster background task
    tokio::spawn(async move {
        tracing::info!("Starting RealtimeBroadcaster background task");
        realtime_broadcaster.start().await;
    });

    // Start Webhook Dispatcher background task
    let webhook_dispatcher = WebhookDispatcher::new(pool.clone());
    tokio::spawn(async move {
        if let Err(e) = webhook_dispatcher.run().await {
            tracing::error!("Webhook dispatcher encountered fatal error: {}", e);
        }
    });

    // Run initial sync (skip on network errors)
    tracing::info!("Running initial metrics synchronization...");
    let _ = ingestion_service.sync_all_metrics().await;

    // Initialize rate limiter
    let rate_limiter_result = RateLimiter::new().await;
    let rate_limiter = match rate_limiter_result {
        Ok(limiter) => {
            tracing::info!("Rate limiter initialized successfully");
            Arc::new(limiter)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to initialize Redis rate limiter, creating with memory fallback: {}",
                e
            );
            Arc::new(
                RateLimiter::new()
                    .await
                    .unwrap_or_else(|_| panic!("Failed to create rate limiter: critical error")),
            )
        }
    };

    // Configure rate limits for endpoints
    rate_limiter
        .register_endpoint(
            "/health".to_string(),
            RateLimitConfig {
                requests_per_minute: 1000,
                whitelist_ips: vec!["127.0.0.1".to_string()],
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/anchors".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/corridors".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/rpc/payments".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/rpc/trades".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/liquidity-pools".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/prices".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/account-merges".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
            },
        )
        .await;

    rate_limiter
        .register_endpoint(
            "/api/achievements".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
            },
        )
        .await;

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
                tracing::warn!(
                    "No valid CORS origins parsed from CORS_ALLOWED_ORIGINS; \
                     falling back to allow-all. Check your configuration."
                );
                base.allow_origin(Any)
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
        .route("/api/db/pool-metrics", get(pool_metrics))
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
        .merge(network_routes)
        .merge(cache_routes)
        .merge(metrics_routes)
        .merge(ws_routes)
        .layer(compression);

    // Start server
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port);

    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
