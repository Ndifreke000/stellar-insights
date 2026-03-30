use anyhow::Result;
use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::sdk::{trace as sdktrace, Resource};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

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
    // Register W3C TraceContext as the global propagator
    global::set_text_map_propagator(TraceContextPropagator::new());

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

    // Prepare layers
    let stdout_layer = if use_json {
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

    let file_layer = if let Some(writer) = file_writer {
        if use_json {
            Some(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(writer)
                    .with_target(true)
                    .with_level(true)
                    .boxed(),
            )
        } else {
            Some(
                tracing_subscriber::fmt::layer()
                    .with_writer(writer)
                    .with_target(true)
                    .with_level(true)
                    .boxed(),
            )
        }
    } else {
        None
    };

    let otel_layer = if otel_enabled {
        match init_otel_tracer(service_name) {
            Ok(tracer) => Some(tracing_opentelemetry::layer().with_tracer(tracer).boxed()),
            Err(e) => {
                eprintln!("Failed to initialize OTLP tracer: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Initialize the registry
    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .with(otel_layer)
        .init();

    if otel_enabled {
        tracing::info!("OpenTelemetry tracing enabled");
    }

    Ok(file_guard)
}

pub fn shutdown_tracing() {
    global::shutdown_tracer_provider();
}

/// Axum middleware that extracts W3C TraceContext headers
pub async fn trace_propagation_middleware(req: Request<Body>, next: Next) -> Response {
    let headers = req.headers();
    let carrier: std::collections::HashMap<String, String> = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.as_str().to_owned(), v.to_owned()))
        })
        .collect();

    let propagator = TraceContextPropagator::new();
    let parent_cx = propagator.extract(&carrier);

    let span = tracing::Span::current();
    span.set_parent(parent_cx);

    next.run(req).await
}

/// Inject the current trace context into an outbound RequestBuilder
pub fn inject_trace_context(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let mut carrier = std::collections::HashMap::new();
    let propagator = TraceContextPropagator::new();
    let cx = opentelemetry::Context::current();
    propagator.inject_context(&cx, &mut carrier);

    let mut builder = builder;
    for (key, value) in carrier {
        builder = builder.header(key, value);
    }
    builder
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn propagation_middleware_does_not_break_requests() {
        let app = Router::new()
            .route("/ping", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(trace_propagation_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

/// Re-export redaction utilities
pub use crate::logging::redaction::{
    redact_account, redact_amount, redact_email, redact_hash, redact_ip, redact_token,
    redact_user_id, Redacted,
};
