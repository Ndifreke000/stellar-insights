use std::net::SocketAddr;
use tracing_logstash::Layer as LogstashLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging with Logstash integration
pub fn init_logging() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let logstash_host =
        std::env::var("LOGSTASH_HOST").unwrap_or_else(|_| "localhost:5000".to_string());

    // Parse Logstash address
    let logstash_addr: SocketAddr = logstash_host
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid LOGSTASH_HOST: {}", e))?;

    // Create Logstash layer
    let logstash_layer = LogstashLayer::new(logstash_addr)
        .map_err(|e| anyhow::anyhow!("Failed to create Logstash layer: {}", e))?;

    // Create console layer for local development
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .json();

    // Build subscriber with both layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(logstash_layer)
        .init();

    tracing::info!(
        logstash_host = %logstash_host,
        "Logging initialized with Logstash integration"
    );

    Ok(())
}

/// Log HTTP request with structured fields
#[macro_export]
macro_rules! log_request {
    ($method:expr, $path:expr, $status:expr, $duration:expr, $request_id:expr) => {
        tracing::info!(
            http_method = %$method,
            http_path = %$path,
            http_status = $status,
            response_time_ms = $duration,
            request_id = %$request_id,
            "HTTP request completed"
        );
    };
}

/// Log RPC call with structured fields
#[macro_export]
macro_rules! log_rpc_call {
    ($method:expr, $duration:expr, $success:expr) => {
        tracing::info!(
            rpc_method = %$method,
            response_time_ms = $duration,
            success = $success,
            "RPC call completed"
        );
    };
}

/// Log database query with structured fields
#[macro_export]
macro_rules! log_query {
    ($query:expr, $duration:expr) => {
        tracing::debug!(
            query = %$query,
            query_time_ms = $duration,
            "Database query executed"
        );
    };
}

/// Log error with context
#[macro_export]
macro_rules! log_error {
    ($err:expr, $context:expr) => {
        tracing::error!(
            error = %$err,
            context = $context,
            "Error occurred"
        );
    };
}
