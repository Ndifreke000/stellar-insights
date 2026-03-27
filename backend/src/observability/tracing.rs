use anyhow::Result;
use opentelemetry::sdk::{trace as sdktrace, Resource};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Default number of rotated log files to retain (e.g. 30 days when using daily rotation).
const MAX_LOG_FILES: usize = 30;

fn init_otel_tracer(service_name: &str) -> Result<sdktrace::Tracer> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
            KeyValue::new("service.name", service_name.to_string()),
        ])))
        .install_batch(opentelemetry::runtime::Tokio)?;

    Ok(tracer)
}

/// Initialize tracing. When `LOG_DIR` is set, logs are also written to a rotating file
/// (daily rotation, up to 30 files retained). The returned guard must be held for the
/// process lifetime so that file logs are flushed; drop it only at shutdown.
pub fn init_tracing(service_name: &str) -> Result<Option<WorkerGuard>> {
    // Bridge log crate (e.g. sqlx statement logging) to tracing
    let _ = tracing_log::LogTracer::init();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "backend=info,tower_http=info".into());
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());
    let otel_enabled = std::env::var("OTEL_ENABLED")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(true); // Default to true now as we want tracing enabled

    // Optional rotating file appender when LOG_DIR is set
    let log_dir = std::env::var("LOG_DIR").ok();
    let file_guard = if let Some(ref dir) = log_dir {
        std::fs::create_dir_all(dir)?;
        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("stellar-insights")
            .filename_suffix("log")
            .max_log_files(MAX_LOG_FILES)
            .build(dir)?;
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        Some((non_blocking, guard))
    } else {
        None
    };

    let (file_writer, file_guard) = match file_guard {
        Some((w, g)) => (Some(w), Some(g)),
        None => (None, None),
    };

    let use_json = log_format.eq_ignore_ascii_case("json");
    let registry = tracing_subscriber::registry().with(env_filter);

    // Add formatting layer
    let fmt_layer = if use_json {
        tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .with_level(true)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_level(true)
            .boxed()
    };

    // Add optional file layer
    let file_layer = if let Some(writer) = file_writer {
        if use_json {
            Some(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(writer)
                    .with_target(true)
                    .with_level(true),
            )
        } else {
            Some(
                tracing_subscriber::fmt::layer()
                    .with_writer(writer)
                    .with_target(true)
                    .with_level(true),
            )
        }
    } else {
        None
    };

    // Add OpenTelemetry layer
    if otel_enabled {
        let tracer = init_otel_tracer(service_name)?;
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        if let Some(fl) = file_layer {
            registry.with(fmt_layer).with(otel_layer).with(fl).init();
        } else {
            registry.with(fmt_layer).with(otel_layer).init();
        }
        tracing::info!("OpenTelemetry tracing enabled");
    } else if let Some(fl) = file_layer {
        registry.with(fmt_layer).with(fl).init();
    } else {
        registry.with(fmt_layer).init();
    }


    Ok(file_guard)
}

pub fn shutdown_tracing() {
    opentelemetry::global::shutdown_tracer_provider();
}

/// Re-export redaction utilities for use throughout the application
pub use crate::logging::redaction::{
    redact_account, redact_amount, redact_email, redact_hash, redact_ip, redact_token,
    redact_user_id, Redacted,
};
