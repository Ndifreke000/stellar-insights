use anyhow::Result;
use axum::{
    routing::{get, put},
    Router,
};
use dotenv::dotenv;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use stellar_insights_backend::api::anchors_cached::get_anchors;
use stellar_insights_backend::api::cache_stats;
use stellar_insights_backend::api::corridors_cached::{get_corridor_detail, list_corridors};
use stellar_insights_backend::api::fee_bump;
use stellar_insights_backend::api::liquidity_pools;
use stellar_insights_backend::api::metrics_cached;
use stellar_insights_backend::auth::AuthService;
use stellar_insights_backend::auth_middleware::auth_middleware;
use stellar_insights_backend::cache::{CacheConfig, CacheManager};
use stellar_insights_backend::cache_invalidation::CacheInvalidationService;
use stellar_insights_backend::database::Database;
use stellar_insights_backend::handlers::*;
use stellar_insights_backend::ingestion::ledger::LedgerIngestionService;
use stellar_insights_backend::ingestion::DataIngestionService;
use stellar_insights_backend::openapi::ApiDoc;
use stellar_insights_backend::rate_limit::{rate_limit_middleware, RateLimitConfig, RateLimiter};
use stellar_insights_backend::rpc::StellarRpcClient;
use stellar_insights_backend::rpc_handlers;
use stellar_insights_backend::services::fee_bump_tracker::FeeBumpTrackerService;
use stellar_insights_backend::services::liquidity_pool_analyzer::LiquidityPoolAnalyzer;
use stellar_insights_backend::services::trustline_analyzer::TrustlineAnalyzer;
use stellar_insights_backend::shutdown::{ShutdownConfig, ShutdownCoordinator};
use stellar_insights_backend::state::AppState;
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
    let pool = sqlx::SqlitePool::connect(&database_url).await?;

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    let db = Arc::new(Database::new(pool.clone()));

    // Initialize Stellar RPC Client
    let mock_mode = std::env::var("RPC_MOCK_MODE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    let rpc_url = std::env::var("STELLAR_RPC_URL")
        .unwrap_or_else(|_| "https://stellar.api.onfinality.io/public".to_string());

    let horizon_url = std::env::var("STELLAR_HORIZON_URL")
        .unwrap_or_else(|_| "https://horizon.stellar.org".to_string());

    tracing::info!(
        "Initializing Stellar RPC client (mock_mode: {}, rpc: {}, horizon: {})",
        mock_mode,
        rpc_url,
        horizon_url
    );

    let rpc_client = Arc::new(StellarRpcClient::new(rpc_url, horizon_url, mock_mode));

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

    // Initialize Liquidity Pool Analyzer
    let lp_analyzer = Arc::new(LiquidityPoolAnalyzer::new(
        pool.clone(),
        Arc::clone(&rpc_client),
    ));

    // Initialize Trustline Analyzer
    let trustline_analyzer = Arc::new(TrustlineAnalyzer::new(
        pool.clone(),
        Arc::clone(&rpc_client),
    ));

    // Initialize Ledger Ingestion Service
    let ledger_ingestion_service = Arc::new(LedgerIngestionService::new(
        Arc::clone(&rpc_client),
        Arc::clone(&fee_bump_tracker),
        pool.clone(),
    ));

    // Initialize Redis cache
    let cache_config = CacheConfig::default();
    let cache = Arc::new(CacheManager::new(cache_config).await?);
    tracing::info!("Cache manager initialized");

    // Initialize cache invalidation service
    let cache_invalidation = Arc::new(CacheInvalidationService::new(Arc::clone(&cache)));

    // Create app state for handlers that need it
    let app_state = AppState::new(
        Arc::clone(&db),
        Arc::clone(&ws_state),
        Arc::clone(&ingestion_service),
    );

    // Create cached state tuple for cached API handlers
    let cached_state = (Arc::clone(&db), Arc::clone(&cache), Arc::clone(&rpc_client));

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
        auth_redis_connection,
    ))));
    tracing::info!("Auth service initialized");

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

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

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
    let app = Router::new()
        .merge(swagger_routes)
        .merge(auth_routes)
        .merge(cached_routes)
        .merge(anchor_routes)
        .merge(protected_anchor_routes)
        .merge(rpc_routes)
        .merge(fee_bump_routes)
        .merge(lp_routes)
        .merge(trustline_routes)
        .merge(cache_routes)
        .merge(metrics_routes);

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
