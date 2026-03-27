use anyhow::Result;
use opentelemetry::sdk::{trace as sdktrace, Resource};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const MAX_LOG_FILES: usize = 30;

fn init_otel_tracer(service_name: &str) -> Result<sdktrace::Tracer> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let tracer =
        opentelemetry_otlp::new_pipeline()
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
    let _ = tracing_log::LogTracer::init();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "backend=info,tower_http=info".into());

    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());
    let use_json = log_format.eq_ignore_ascii_case("json");

    let otel_enabled = std::env::var("OTEL_ENABLED")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    // Optional rotating file appender
    let log_dir = std::env::var("LOG_DIR").ok();
    let (file_writer, file_guard) = if let Some(ref dir) = log_dir {
        std::fs::create_dir_all(dir)?;
        let appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("stellar-insights")
            .filename_suffix("log")
            .max_log_files(MAX_LOG_FILES)
            .build(dir)?;
        let (nb, guard) = tracing_appender::non_blocking(appender);
        (Some(nb), Some(guard))
    } else {
        (None, None)
    };

    // otel_layer must be added directly on registry() (before other layers) so that
    // the S: LookupSpan bound is satisfied.
    if otel_enabled {
        let otel_layer = tracing_opentelemetry::layer()
            .with_tracer(init_otel_tracer(service_name)?);
        let base = tracing_subscriber::registry().with(otel_layer).with(env_filter);

        if use_json {
            let fmt = tracing_subscriber::fmt::layer()
                .json().with_target(true).with_level(true);
            match file_writer {
                Some(w) => {
                    let file = tracing_subscriber::fmt::layer()
                        .json().with_writer(w).with_target(true).with_level(true);
                    base.with(fmt).with(file).init();
                }
                None => base.with(fmt).init(),
            }
        } else {
            let fmt = tracing_subscriber::fmt::layer()
                .with_target(true).with_level(true);
            match file_writer {
                Some(w) => {
                    let file = tracing_subscriber::fmt::layer()
                        .with_writer(w).with_target(true).with_level(true);
                    base.with(fmt).with(file).init();
                }
                None => base.with(fmt).init(),
            }
        }
        tracing::info!("OpenTelemetry tracing enabled");
    } else {
        let base = tracing_subscriber::registry().with(env_filter);
        if use_json {
            let fmt = tracing_subscriber::fmt::layer()
                .json().with_target(true).with_level(true);
            match file_writer {
                Some(w) => {
                    let file = tracing_subscriber::fmt::layer()
                        .json().with_writer(w).with_target(true).with_level(true);
                    base.with(fmt).with(file).init();
                }
                None => base.with(fmt).init(),
            }
        } else {
            let fmt = tracing_subscriber::fmt::layer()
                .with_target(true).with_level(true);
            match file_writer {
                Some(w) => {
                    let file = tracing_subscriber::fmt::layer()
                        .with_writer(w).with_target(true).with_level(true);
                    base.with(fmt).with(file).init();
                }
                None => base.with(fmt).init(),
            }
        }
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
