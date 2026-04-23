use anyhow::Result;
use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::resource::Resource;
use opentelemetry_sdk::runtime;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

const MAX_LOG_FILES: usize = 30;

fn init_otel_tracer(service_name: &str) -> Result<opentelemetry_sdk::trace::Tracer> {
    // HTTP/protobuf OTLP on 4318; avoids pulling `tonic` into the crate graph.
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| {
        "http://localhost:4318/v1/traces".to_string()
    });

    let resource = Resource::new([KeyValue::new(
        "service.name",
        service_name.to_string(),
    )]);

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()?;

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_config(opentelemetry_sdk::trace::Config::default().with_resource(resource))
        .build();

    global::set_tracer_provider(provider.clone());
    Ok(provider.tracer("stellar-insights-backend"))
}

/// Initialize tracing. When `LOG_DIR` is set, logs are also written to a rotating file
/// (daily rotation, up to 30 files retained). The returned guard must be held for the
/// process lifetime so that file logs are flushed; drop it only at shutdown.
pub fn init_tracing(service_name: &str) -> Result<Option<WorkerGuard>> {
    // Register W3C TraceContext as the global propagator so that
    // `traceparent` / `tracestate` headers are used for context propagation.
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

    // OTel layer must be registered on `registry()` first so `LookupSpan` bounds are satisfied.
    let stdout = std::io::stdout;

    if otel_enabled {
        let otel_tracer = init_otel_tracer(service_name)?;
        let otel_layer = tracing_opentelemetry::layer().with_tracer(otel_tracer);
        let base = tracing_subscriber::registry()
            .with(otel_layer)
            .with(TraceIdLayer)
            .with(env_filter);

        match (use_json, file_writer) {
            (true, Some(w)) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                let file_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(w)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).with(file_layer).init();
            }
            (true, None) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).init();
            }
            (false, Some(w)) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                let file_layer = tracing_subscriber::fmt::layer()
                    .with_writer(w)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).with(file_layer).init();
            }
            (false, None) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).init();
            }
        }
        tracing::info!("OpenTelemetry tracing enabled");
    } else {
        let base = tracing_subscriber::registry().with(env_filter);
        match (use_json, file_writer) {
            (true, Some(w)) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                let file_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(w)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).with(file_layer).init();
            }
            (true, None) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).init();
            }
            (false, Some(w)) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                let file_layer = tracing_subscriber::fmt::layer()
                    .with_writer(w)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).with(file_layer).init();
            }
            (false, None) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).init();
            }
        }
    }

    Ok(file_guard)
}

pub fn shutdown_tracing() {
    global::shutdown_tracer_provider();
}

/// A [`tracing_subscriber::Layer`] that stamps `trace_id` and `span_id` onto
/// every span's extensions the moment it is created, so that all log events
/// emitted inside that span automatically carry those fields.
///
/// This is the idiomatic way to get W3C trace context into structured logs
/// without re-emitting events or touching the fmt layer.
pub struct TraceIdLayer;

impl<S> Layer<S> for TraceIdLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        _attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).expect("span must exist on new_span");
        // Seed with a placeholder; the real IDs are stamped in on_record once
        // the OTel layer has attached its context (see trace_propagation_middleware).
        span.extensions_mut().insert(TraceIds {
            trace_id: String::new(),
            span_id: String::new(),
        });
    }

    fn on_record(
        &self,
        id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).expect("span must exist on record");
        let mut exts = span.extensions_mut();
        if let Some(ids) = exts.get_mut::<TraceIds>() {
            let mut visitor = TraceIdVisitor(ids);
            values.record(&mut visitor);
        }
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Pull trace_id / span_id from the nearest enclosing span that has them.
        let ids = ctx.lookup_current().and_then(|span| {
            let exts = span.extensions();
            exts.get::<TraceIds>().cloned()
        });

        if let Some(TraceIds { trace_id, span_id }) = ids {
            if !trace_id.is_empty() {
                // Record onto the current span so the fmt layer picks them up.
                tracing::Span::current().record("trace_id", &trace_id.as_str());
                tracing::Span::current().record("span_id", &span_id.as_str());
            }
        }
        let _ = event;
    }
}

#[derive(Clone)]
struct TraceIds {
    trace_id: String,
    span_id: String,
}

struct TraceIdVisitor<'a>(&'a mut TraceIds);

impl tracing::field::Visit for TraceIdVisitor<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "trace_id" => self.0.trace_id = value.to_owned(),
            "span_id" => self.0.span_id = value.to_owned(),
            _ => {}
        }
    }
    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
}

/// Axum middleware that extracts W3C TraceContext headers (`traceparent`, `tracestate`)
/// from incoming requests, sets them as the parent context on the current span, and
/// records `trace_id` / `span_id` as structured span fields so every log event
/// emitted during the request carries those IDs.
///
/// Must be placed *inside* `TraceLayer` in the middleware stack (i.e. added before
/// `TraceLayer` in the `.layer()` chain) so a span already exists when this runs.
pub async fn trace_propagation_middleware(req: Request<Body>, next: Next) -> Response {
    let carrier: std::collections::HashMap<String, String> = req
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.as_str().to_owned(), v.to_owned()))
        })
        .collect();

    // Extract remote context via the globally registered W3C TraceContext propagator.
    let parent_cx =
        global::get_text_map_propagator(|propagator| propagator.extract(&carrier));

    let span = tracing::Span::current();
    span.set_parent(parent_cx.clone());

    // Stamp trace_id / span_id onto the span so structured logs carry them.
    use opentelemetry::trace::TraceContextExt as _;
    let span_ctx = parent_cx.span().span_context().clone();
    if span_ctx.is_valid() {
        span.record("trace_id", span_ctx.trace_id().to_string().as_str());
        span.record("span_id", span_ctx.span_id().to_string().as_str());
    }

    next.run(req).await
}

/// Inject the current trace context into an outbound `reqwest::RequestBuilder`.
///
/// Uses the globally registered propagator (W3C `TraceContext` by default) so
/// that `traceparent` / `tracestate` headers are forwarded to downstream
/// services, preserving the distributed trace across service boundaries.
///
/// # Example
/// ```rust
/// let response = inject_trace_context(client.get(&url)).send().await?;
/// ```
pub fn inject_trace_context(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let mut carrier = std::collections::HashMap::new();
    let cx = opentelemetry::Context::current();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut carrier);
    });

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
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn inject_trace_context_does_not_panic_without_active_span() {
        // Verify inject_trace_context is safe to call even when no OTel span is active.
        // Without a real OTLP exporter the carrier will simply be empty.
        global::set_text_map_propagator(TraceContextPropagator::new());
        let client = reqwest::Client::new();
        let builder = client.get("http://localhost");
        // Should not panic
        let _ = inject_trace_context(builder);
    }

    #[tokio::test]
    async fn propagation_middleware_accepts_traceparent_header() {
        let app = Router::new()
            .route("/ping", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(trace_propagation_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    // Valid W3C traceparent header
                    .header(
                        "traceparent",
                        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

/// Re-export redaction utilities for use throughout the application
pub use crate::logging::redaction::{
    redact_account, redact_amount, redact_email, redact_hash, redact_ip, redact_token,
    redact_user_id, Redacted,
};
