use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use stellar_insights_apm::{ApmManager, ApmConfig, ApmMiddleware};
use backend::database::Database;
use backend::handlers::*;
use backend::api::anchors::get_anchors;
use backend::api::corridors::{list_corridors, get_corridor_detail};
use backend::ingestion::DataIngestionService;
use backend::rpc::StellarRpcClient;
use backend::rpc_handlers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize APM
    let apm_config = ApmConfig::default();
    let apm = Arc::new(ApmManager::new(apm_config)?);
    
    // Set up graceful shutdown for APM
    let apm_shutdown = apm.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        if let Err(e) = apm_shutdown.shutdown().await {
            eprintln!("Error shutting down APM: {}", e);
        }
    });

    // Initialize tracing with APM context
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "stellar_insights=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!("Starting Stellar Insights backend with APM integration");

    // Database connection
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:password@localhost:5432/stellar_insights".to_string()
    });

    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    let db = Arc::new(Database::new(pool));

    // Initialize Stellar RPC Client
    let stellar_rpc_url = std::env::var("STELLAR_RPC_URL")
        .unwrap_or_else(|_| "https://horizon.stellar.org".to_string());
    
    let stellar_client = Arc::new(StellarRpcClient::new(&stellar_rpc_url)?);

    // Initialize data ingestion service
    let ingestion_service = Arc::new(DataIngestionService::new(
        db.clone(),
        stellar_client.clone(),
        apm.clone(),
    ));

    // Start background data ingestion
    let ingestion_service_clone = ingestion_service.clone();
    tokio::spawn(async move {
        if let Err(e) = ingestion_service_clone.start().await {
            tracing::error!("Data ingestion service error: {}", e);
        }
    });

    // Build the application
    let app = Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        
        // API routes
        .route("/api/anchors", get(get_anchors))
        .route("/api/corridors", get(list_corridors))
        .route("/api/corridors/:id", get(get_corridor_detail))
        
        // RPC routes
        .route("/rpc/stellar/*path", post(rpc_handlers::handle_stellar_rpc))
        
        // CORS layer
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        
        // APM middleware for HTTP request tracking
        .layer(middleware::from_fn_with_state(
            apm.clone(),
            ApmMiddleware::track_http_request,
        ))
        
        // General middleware
        .layer(
            ServiceBuilder::new()
                .timeout(std::time::Duration::from_secs(30))
                .compression(tower_http::Compression::new())
                .trace_http()
        )
        
        .with_state(db);

    // Get port from environment
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);

    tracing::info!("Starting server on port {}", port);

    // Start the server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(apm.clone()))
        .await?;

    Ok(())
}

/// Health check handler
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Metrics handler for Prometheus scraping
async fn metrics_handler() -> Result<String, axum::http::StatusCode> {
    // This would typically expose Prometheus metrics
    // For now, return a simple response
    Ok("# HELP stellar_insights_requests_total Total number of requests\n# TYPE stellar_insights_requests_total counter\nstellar_insights_requests_total 0\n".to_string())
}

/// Graceful shutdown signal
async fn shutdown_signal(apm: Arc<ApmManager>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            tracing::info!("Received terminate signal, shutting down...");
        }
    }

    // Shutdown APM
    if let Err(e) = apm.shutdown().await {
        tracing::error!("Error shutting down APM: {}", e);
    }

    tracing::info!("Shutdown complete");
}
