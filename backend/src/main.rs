use anyhow::Result;
use axum::{
    routing::{get, put},
    Router,
};
use dotenv::dotenv;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use backend::api::anchors::get_anchors;
use backend::api::corridors::{get_corridor_detail, list_corridors};
use backend::database::Database;
use backend::handlers::*;
use backend::ingestion::DataIngestionService;
use backend::rpc::StellarRpcClient;
use backend::rpc_handlers;
use backend::shutdown::{
    flush_caches, log_shutdown_summary, shutdown_background_tasks, shutdown_database,
    wait_for_signal, ShutdownConfig, ShutdownCoordinator,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Track shutdown start time for logging
    let shutdown_start = std::time::Instant::now();

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
    let shutdown_coordinator = Arc::new(ShutdownCoordinator::new(shutdown_config));

    // Database connection
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:stellar_insights.db".to_string());

    tracing::info!("Connecting to database...");
    let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;

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

    // Initialize Data Ingestion Service
    let ingestion_service = Arc::new(DataIngestionService::new(
        Arc::clone(&rpc_client),
        Arc::clone(&db),
    ));

    // Start background sync task with shutdown handling
    let ingestion_clone = Arc::clone(&ingestion_service);
    let mut shutdown_rx = shutdown_coordinator.subscribe();
    let sync_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = ingestion_clone.sync_all_metrics().await {
                        tracing::error!("Metrics synchronization failed: {}", e);
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Background sync task received shutdown signal");
                    break;
                }
            }
        }
        tracing::info!("Background sync task stopped");
    });

    // Run initial sync
    tracing::info!("Running initial metrics synchronization...");
    if let Err(e) = ingestion_service.sync_all_metrics().await {
        tracing::warn!("Initial sync failed: {}", e);
    }

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build anchor router
    let anchor_routes = Router::new()
        .route("/health", get(health_check))
        .route("/api/anchors", get(get_anchors).post(create_anchor))
        .route("/api/anchors/:id", get(get_anchor))
        .route(
            "/api/anchors/account/:stellar_account",
            get(get_anchor_by_account),
        )
        .route("/api/anchors/:id/metrics", put(update_anchor_metrics))
        .route(
            "/api/anchors/:id/assets",
            get(get_anchor_assets).post(create_anchor_asset),
        )
        .route("/api/corridors", get(list_corridors).post(create_corridor))
        .route(
            "/api/corridors/:id/metrics-from-transactions",
            put(update_corridor_metrics_from_transactions),
        )
        .route("/api/corridors/:corridor_key", get(get_corridor_detail))
        .with_state(db);

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
        .with_state(rpc_client);

    // Merge routers
    let app = Router::new()
        .merge(anchor_routes)
        .merge(rpc_routes)
        .layer(cors);

    // Start server
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port);

    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Spawn server with graceful shutdown
    let shutdown_coordinator_clone = Arc::clone(&shutdown_coordinator);
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let mut rx = shutdown_coordinator_clone.subscribe();
                let _ = rx.recv().await;
                tracing::info!("Server received shutdown signal, stopping accepting new connections");
            })
            .await
            .expect("Server error");
    });

    tracing::info!("Server is running. Press Ctrl+C to shutdown gracefully.");

    // Wait for shutdown signal
    wait_for_signal().await;

    // Trigger coordinated shutdown
    tracing::info!("Initiating graceful shutdown sequence...");
    shutdown_coordinator.trigger_shutdown();

    // Step 1: Wait for server to stop accepting new connections and finish in-flight requests
    tracing::info!("Step 1/4: Waiting for server to finish in-flight requests...");
    let server_shutdown = tokio::time::timeout(
        shutdown_coordinator.graceful_timeout(),
        server_handle,
    );
    
    match server_shutdown.await {
        Ok(Ok(_)) => tracing::info!("Server shutdown completed successfully"),
        Ok(Err(e)) => tracing::error!("Server task failed: {}", e),
        Err(_) => tracing::warn!(
            "Server did not shutdown within {:?}, proceeding anyway",
            shutdown_coordinator.graceful_timeout()
        ),
    }

    // Step 2: Shutdown background tasks
    tracing::info!("Step 2/4: Shutting down background tasks...");
    shutdown_background_tasks(
        vec![sync_task],
        shutdown_coordinator.background_task_timeout(),
    )
    .await;

    // Step 3: Flush caches
    tracing::info!("Step 3/4: Flushing caches...");
    flush_caches().await;

    // Step 4: Close database connections
    tracing::info!("Step 4/4: Closing database connections...");
    shutdown_database(pool, shutdown_coordinator.db_close_timeout()).await;

    // Log shutdown summary
    log_shutdown_summary(shutdown_start);
    tracing::info!("Graceful shutdown complete. Goodbye!");

    Ok(())
}
