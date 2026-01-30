use anyhow::Result;
use axum::{
    routing::{get, put, post},
    Router,
};
use dotenv::dotenv;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use backend::api::anchors::get_anchors;
use backend::api::corridors::{get_corridor_detail, list_corridors};
use backend::cache::{CacheConfig, CacheManager};
use backend::cache_invalidation::CacheInvalidationService;
use backend::database::Database;
use backend::handlers::*;
use backend::ingestion::{DataIngestionService, ledger::LedgerIngestionService};
use backend::ml::MLService;
use backend::ml_handlers;
use backend::rpc::StellarRpcClient;
use backend::rpc_handlers;
use backend::rate_limit::{RateLimiter, RateLimitConfig, rate_limit_middleware};
use backend::state::AppState;
use backend::websocket::WsState;


#[tokio::main]
async fn main() -> Result<()> {
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

    // Database connection
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    tracing::info!("Connecting to database...");
    let pool = sqlx::PgPool::connect(&database_url).await?;

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
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let auth_redis_connection = if let Ok(client) = redis::Client::open(redis_url.as_str()) {
        match client.get_multiplexed_tokio_connection().await {
            Ok(conn) => {
                tracing::info!("Auth service connected to Redis");
                Some(conn)
            }
            Err(e) => {
                tracing::warn!("Auth service failed to connect to Redis ({}), refresh tokens will not persist", e);
                None
            }
        }
    } else {
        tracing::warn!("Invalid Redis URL for auth service");
        None
    };
    let auth_service = Arc::new(AuthService::new(Arc::new(tokio::sync::RwLock::new(auth_redis_connection))));
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

    // Ledger ingestion task (commented out)
    /*
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
    */

    // Run initial sync (skip on network errors)
    tracing::info!("Running initial metrics synchronization...");
    let _ = ingestion_service.sync_all_metrics().await;

    // Initialize rate limiter
    let rate_limiter_result = RateLimiter::new().await;
    let rate_limiter = match rate_limiter_result {
        Ok(limiter) => {
            tracing::info!("Rate limiter initialized successfully");
            Arc::new(limiter)
        },
        Err(e) => {
            tracing::warn!("Failed to initialize Redis rate limiter, creating with memory fallback: {}", e);
            Arc::new(RateLimiter::new().await.unwrap_or_else(|_| {
                panic!("Failed to create rate limiter: critical error")
            }))
        }
    };

    // Configure rate limits for endpoints
    rate_limiter.register_endpoint("/health".to_string(), RateLimitConfig {
        requests_per_minute: 1000,
        whitelist_ips: vec!["127.0.0.1".to_string()],
    }).await;

    rate_limiter.register_endpoint("/api/anchors".to_string(), RateLimitConfig {
        requests_per_minute: 100,
        whitelist_ips: vec![],
    }).await;

    rate_limiter.register_endpoint("/api/corridors".to_string(), RateLimitConfig {
        requests_per_minute: 100,
        whitelist_ips: vec![],
    }).await;

    rate_limiter.register_endpoint("/api/rpc/payments".to_string(), RateLimitConfig {
        requests_per_minute: 100,
        whitelist_ips: vec![],
    }).await;

    rate_limiter.register_endpoint("/api/rpc/trades".to_string(), RateLimitConfig {
        requests_per_minute: 100,
        whitelist_ips: vec![],
    }).await;

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Import middleware
    use tower::ServiceBuilder;
    use axum::middleware;

    // Build auth router
    let auth_routes = stellar_insights_backend::api::auth::routes(auth_service.clone());

    // Build anchor router with protected write endpoints
    let anchor_routes = Router::new()
        .route("/health", get(health_check))
        .route("/api/anchors", get(get_anchors))
        .route("/api/anchors/:id", get(get_anchor))
        .route(
            "/api/anchors/account/:stellar_account",
            get(get_anchor_by_account),
        )
        .route("/api/anchors/:id/assets", get(get_anchor_assets))
        .route("/api/corridors", get(list_corridors))
        .route("/api/corridors/:corridor_key", get(get_corridor_detail))
        // .route("/api/ingestion/status", get(ingestion_status)) // Commented out due to missing handlers
        .with_state(app_state.clone())
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    rate_limiter.clone(),
                    rate_limit_middleware,
                ))
        )
        .layer(cors.clone());

    // Build protected anchor routes (require authentication)
    let protected_anchor_routes = Router::new()
        .route("/api/anchors", axum::routing::post(create_anchor))
        .route("/api/anchors/:id/metrics", put(update_anchor_metrics))
        .route("/api/anchors/:id/assets", axum::routing::post(create_anchor_asset))
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
                ))
        )
        .layer(cors.clone());

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
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    rate_limiter.clone(),
                    rate_limit_middleware,
                ))
        )
        .layer(cors.clone());

    // Merge routers
    let app = Router::new()
        .merge(auth_routes)
        .merge(anchor_routes)
        .merge(rpc_routes);

    // Start server
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port);

    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener, 
        app.into_make_service_with_connect_info::<std::net::SocketAddr>()
    ).await?;

    Ok(())
}
