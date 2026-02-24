use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use opentelemetry::global;
use opentelemetry::trace::{Span, SpanKind, Tracer};
use opentelemetry::{Context, KeyValue};
use tracing::{error, info, warn};

use crate::ApmManager;

/// APM middleware for Axum
pub struct ApmMiddleware {
    apm: Arc<ApmManager>,
}

impl ApmMiddleware {
    pub fn new(apm: Arc<ApmManager>) -> Self {
        Self { apm }
    }

    /// Middleware function for HTTP request tracking
    pub async fn track_http_request(
        State(apm): State<Arc<ApmManager>>,
        request: Request,
        next: Next,
    ) -> Response {
        if !apm.config.enabled {
            return next.run(request).await;
        }

        let start_time = Instant::now();
        let method = request.method().to_string();
        let uri = request.uri().to_string();
        let user_agent = request
            .headers()
            .get("user-agent")
            .map(|h| h.to_str().unwrap_or("unknown"))
            .unwrap_or("unknown");

        // Extract trace context from headers
        let trace_context = extract_trace_context(request.headers());

        // Create span for this request
        let tracer = global::tracer("stellar-insights");
        let span = tracer
            .span_builder(format!("{} {}", method, uri))
            .with_kind(SpanKind::Server)
            .with_attributes(vec![
                KeyValue::new("http.method", method.clone()),
                KeyValue::new("http.url", uri.clone()),
                KeyValue::new("http.user_agent", user_agent),
                KeyValue::new("net.host.name", get_host_name()),
            ])
            .start(&tracer);

        let _guard = span.enter();

        // Record request size if available
        if let Some(content_length) = request.headers().get("content-length") {
            if let Ok(size) = content_length.to_str() {
                if let Ok(bytes) = size.parse::<u64>() {
                    apm.metrics.http_request_size.record(
                        bytes as f64,
                        &[
                            KeyValue::new("http.method", method.clone()),
                            KeyValue::new("http.url", uri.clone()),
                        ],
                    );
                }
            }
        }

        // Process the request
        let response = next.run(request).await;

        // Calculate duration
        let duration = start_time.elapsed();

        // Extract response information
        let status_code = response.status();
        let status_code_value = status_code.as_u16();

        // Record metrics
        apm.metrics.http_requests_total.add(
            1,
            &[
                KeyValue::new("http.method", method),
                KeyValue::new("http.status_code", status_code_value.to_string()),
                KeyValue::new("http.url", uri),
            ],
        );

        apm.metrics.http_request_duration.record(
            duration.as_secs_f64(),
            &[
                KeyValue::new("http.method", method),
                KeyValue::new("http.status_code", status_code_value.to_string()),
                KeyValue::new("http.url", uri),
            ],
        );

        // Record response size if available
        if let Some(content_length) = response.headers().get("content-length") {
            if let Ok(size) = content_length.to_str() {
                if let Ok(bytes) = size.parse::<u64>() {
                    apm.metrics.http_response_size.record(
                        bytes as f64,
                        &[
                            KeyValue::new("http.method", method),
                            KeyValue::new("http.status_code", status_code_value.to_string()),
                            KeyValue::new("http.url", uri),
                        ],
                    );
                }
            }
        }

        // Add attributes to span
        span.set_attributes(vec![
            KeyValue::new("http.status_code", status_code_value.to_string()),
            KeyValue::new(
                "http.status_text",
                status_code.canonical_reason().unwrap_or("unknown"),
            ),
            KeyValue::new("http.response_time_ms", duration.as_millis() as i64),
        ]);

        // Set span status based on HTTP status
        if status_code.is_server_error() {
            span.set_status(opentelemetry::trace::Status::error(format!(
                "HTTP {} error",
                status_code_value
            )));
        } else if status_code.is_client_error() {
            span.set_status(opentelemetry::trace::Status::error(format!(
                "HTTP {} client error",
                status_code_value
            )));
        }

        // Log request completion
        if status_code.is_server_error() {
            error!(
                method = %method,
                uri = %uri,
                status = %status_code,
                duration_ms = duration.as_millis(),
                "HTTP request completed with server error"
            );
        } else if status_code.is_client_error() {
            warn!(
                method = %method,
                uri = %uri,
                status = %status_code,
                duration_ms = duration.as_millis(),
                "HTTP request completed with client error"
            );
        } else {
            info!(
                method = %method,
                uri = %uri,
                status = %status_code,
                duration_ms = duration.as_millis(),
                "HTTP request completed successfully"
            );
        }

        response
    }

    /// Middleware for database operation tracking
    pub async fn track_database_operation<F, R>(
        apm: &ApmManager,
        operation: &str,
        table: Option<&str>,
        f: F,
    ) -> Result<R, anyhow::Error>
    where
        F: std::future::Future<Output = Result<R, anyhow::Error>>,
    {
        if !apm.config.enabled {
            return f.await;
        }

        let start_time = Instant::now();
        let tracer = global::tracer("stellar-insights");

        let mut span_builder = tracer.span_builder(format!("db.{}", operation));
        span_builder = span_builder.with_kind(SpanKind::Client);

        if let Some(table_name) = table {
            span_builder = span_builder.with_attribute(KeyValue::new("db.table", table_name));
        }

        let span = span_builder.start(&tracer);
        let _guard = span.enter();

        let result = f.await;
        let duration = start_time.elapsed();

        // Record metrics
        apm.metrics.db_queries_total.add(
            1,
            &[
                KeyValue::new("db.operation", operation),
                KeyValue::new("db.table", table.unwrap_or("unknown")),
            ],
        );

        apm.metrics.db_query_duration.record(
            duration.as_secs_f64(),
            &[
                KeyValue::new("db.operation", operation),
                KeyValue::new("db.table", table.unwrap_or("unknown")),
            ],
        );

        // Add attributes to span
        span.set_attributes(vec![
            KeyValue::new("db.operation", operation),
            KeyValue::new("db.duration_ms", duration.as_millis() as i64),
        ]);

        match &result {
            Ok(_) => {
                info!(
                    operation = operation,
                    table = table.unwrap_or("unknown"),
                    duration_ms = duration.as_millis(),
                    "Database operation completed successfully"
                );
            }
            Err(e) => {
                error!(
                    operation = operation,
                    table = table.unwrap_or("unknown"),
                    duration_ms = duration.as_millis(),
                    error = %e,
                    "Database operation failed"
                );
                span.set_status(opentelemetry::trace::Status::error(e.to_string()));
                apm.record_error(
                    e,
                    std::collections::HashMap::from([
                        ("operation".to_string(), operation.to_string()),
                        ("table".to_string(), table.unwrap_or("unknown").to_string()),
                    ]),
                );
            }
        }

        result
    }

    /// Track Stellar RPC operations
    pub async fn track_stellar_operation<F, R>(
        apm: &ApmManager,
        operation: &str,
        endpoint: &str,
        f: F,
    ) -> Result<R, anyhow::Error>
    where
        F: std::future::Future<Output = Result<R, anyhow::Error>>,
    {
        if !apm.config.enabled {
            return f.await;
        }

        let start_time = Instant::now();
        let tracer = global::tracer("stellar-insights");

        let span = tracer
            .span_builder(format!("stellar.{}", operation))
            .with_kind(SpanKind::Client)
            .with_attributes(vec![
                KeyValue::new("stellar.operation", operation),
                KeyValue::new("stellar.endpoint", endpoint),
                KeyValue::new("stellar.network", "public"),
            ])
            .start(&tracer);

        let _guard = span.enter();

        let result = f.await;
        let duration = start_time.elapsed();

        // Record metrics
        apm.metrics.stellar_requests_total.add(
            1,
            &[
                KeyValue::new("stellar.operation", operation),
                KeyValue::new("stellar.endpoint", endpoint),
            ],
        );

        // Add attributes to span
        span.set_attributes(vec![
            KeyValue::new("stellar.operation", operation),
            KeyValue::new("stellar.endpoint", endpoint),
            KeyValue::new("stellar.duration_ms", duration.as_millis() as i64),
        ]);

        match &result {
            Ok(_) => {
                info!(
                    operation = operation,
                    endpoint = endpoint,
                    duration_ms = duration.as_millis(),
                    "Stellar RPC operation completed successfully"
                );
            }
            Err(e) => {
                error!(
                    operation = operation,
                    endpoint = endpoint,
                    duration_ms = duration.as_millis(),
                    error = %e,
                    "Stellar RPC operation failed"
                );
                span.set_status(opentelemetry::trace::Status::error(e.to_string()));
                apm.record_error(
                    e,
                    std::collections::HashMap::from([
                        ("operation".to_string(), operation.to_string()),
                        ("endpoint".to_string(), endpoint.to_string()),
                    ]),
                );
            }
        }

        result
    }

    /// Track background job execution
    pub async fn track_background_job<F, R>(
        apm: &ApmManager,
        job_name: &str,
        job_type: &str,
        f: F,
    ) -> Result<R, anyhow::Error>
    where
        F: std::future::Future<Output = Result<R, anyhow::Error>>,
    {
        if !apm.config.enabled {
            return f.await;
        }

        let start_time = Instant::now();
        let tracer = global::tracer("stellar-insights");

        let span = tracer
            .span_builder(format!("job.{}", job_name))
            .with_kind(SpanKind::Server)
            .with_attributes(vec![
                KeyValue::new("job.name", job_name),
                KeyValue::new("job.type", job_type),
            ])
            .start(&tracer);

        let _guard = span.enter();

        let result = f.await;
        let duration = start_time.elapsed();

        // Add attributes to span
        span.set_attributes(vec![
            KeyValue::new("job.name", job_name),
            KeyValue::new("job.type", job_type),
            KeyValue::new("job.duration_ms", duration.as_millis() as i64),
        ]);

        match &result {
            Ok(_) => {
                info!(
                    job_name = job_name,
                    job_type = job_type,
                    duration_ms = duration.as_millis(),
                    "Background job completed successfully"
                );
            }
            Err(e) => {
                error!(
                    job_name = job_name,
                    job_type = job_type,
                    duration_ms = duration.as_millis(),
                    error = %e,
                    "Background job failed"
                );
                span.set_status(opentelemetry::trace::Status::error(e.to_string()));
                apm.record_error(
                    e,
                    std::collections::HashMap::from([
                        ("job_name".to_string(), job_name.to_string()),
                        ("job_type".to_string(), job_type.to_string()),
                    ]),
                );
            }
        }

        result
    }
}

/// Extract trace context from HTTP headers
fn extract_trace_context(headers: &HeaderMap) -> Option<String> {
    headers
        .get("traceparent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            headers
                .get("x-trace-id")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
}

/// Get host name for tracing
fn get_host_name() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| {
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "localhost".to_string())
    })
}

/// Helper trait for adding APM context to requests
pub trait ApmContextExt {
    fn with_apm_context(self, context: Context) -> Self;
}

impl ApmContextExt for Request {
    fn with_apm_context(mut self, context: Context) -> Self {
        // Store context in request extensions for later use
        self.extensions_mut().insert(context);
        self
    }
}

/// Macro for easy database operation tracking
#[macro_export]
macro_rules! track_db_operation {
    ($apm:expr, $operation:expr, $table:expr, $async_expr:expr) => {
        $crate::middleware::ApmMiddleware::track_database_operation(
            $apm,
            $operation,
            $table,
            $async_expr,
        )
        .await
    };
}

/// Macro for easy Stellar RPC operation tracking
#[macro_export]
macro_rules! track_stellar_operation {
    ($apm:expr, $operation:expr, $endpoint:expr, $async_expr:expr) => {
        $crate::middleware::ApmMiddleware::track_stellar_operation(
            $apm,
            $operation,
            $endpoint,
            $async_expr,
        )
        .await
    };
}

/// Macro for easy background job tracking
#[macro_export]
macro_rules! track_background_job {
    ($apm:expr, $job_name:expr, $job_type:expr, $async_expr:expr) => {
        $crate::middleware::ApmMiddleware::track_background_job(
            $apm,
            $job_name,
            $job_type,
            $async_expr,
        )
        .await
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Method, Router};
    use tower::ServiceBuilder;

    #[tokio::test]
    async fn test_http_request_tracking() {
        let config = crate::ApmConfig::default();
        let apm = Arc::new(crate::ApmManager::new(config).unwrap());

        let app = Router::new()
            .layer(axum::middleware::from_fn_with_state(
                apm.clone(),
                crate::middleware::ApmMiddleware::track_http_request,
            ))
            .route("/test", axum::routing::get(|| async { "Hello, World!" }));

        // Test request
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("user-agent", "test-agent")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
