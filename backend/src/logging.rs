pub mod redaction;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub use redaction::{
    redact_account, redact_amount, redact_email, redact_hash, redact_ip, redact_token,
    redact_user_id, Redacted,
};

/// Initializes logging to stdout only. No file output or rotation.
/// Initialize logging with Logstash integration
pub fn init_logging() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Create console layer for local development
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .json();

    // Build subscriber with console output.
    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .init();

    tracing::info!("Logging initialized with redaction support");

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

/// Log with automatic redaction of sensitive fields
///
/// Usage:
/// ```
/// log_secure!(info, "Processing payment",
///     account = redact_account(&stellar_account),
///     amount = redact_amount(payment_amount),
///     user_id = redact_user_id(&user_id)
/// );
/// ```
#[macro_export]
macro_rules! log_secure {
    ($level:ident, $msg:expr, $($key:ident = $value:expr),* $(,)?) => {
        tracing::$level!(
            $($key = $value,)*
            $msg
        );
    };
    ($level:ident, $msg:expr) => {
        tracing::$level!($msg);
    };
}
