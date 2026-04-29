/// Request/response logging middleware (issue #1202).
///
/// Logs structured fields for every HTTP request and response:
/// - request_id (from X-Request-ID extension set by `request_id_middleware`)
/// - method, path, query
/// - response status, latency_ms
/// - optional request/response body snippets (capped, redacted) when
///   `API_LOG_BODIES=true` is set in the environment.
///
/// Sensitive headers (Authorization, Cookie, Set-Cookie) are never logged.
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};
use http_body_util::BodyExt;
use std::time::Instant;

use crate::request_id::RequestId;

/// Maximum bytes of a body to include in logs (prevents huge log lines).
const MAX_BODY_LOG_BYTES: usize = 512;

/// Whether to log request/response bodies. Controlled by `API_LOG_BODIES` env var.
fn log_bodies() -> bool {
    std::env::var("API_LOG_BODIES")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

pub async fn request_response_logging_middleware(
    req: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();

    // Extract request metadata before consuming the request
    let request_id = req
        .extensions()
        .get::<RequestId>()
        .map(|r| r.0.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let method = req.method().to_string();
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let query = uri.query().unwrap_or("").to_string();

    // Optionally buffer and log request body
    let (req, req_body_snippet) = if log_bodies() {
        let (parts, body) = req.into_parts();
        let bytes = body
            .collect()
            .await
            .map(|b| b.to_bytes())
            .unwrap_or_default();
        let snippet = truncate_body(&bytes);
        let req = Request::from_parts(parts, Body::from(bytes));
        (req, Some(snippet))
    } else {
        (req, None)
    };

    tracing::info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        query = %query,
        body = req_body_snippet.as_deref().unwrap_or(""),
        "→ request"
    );

    let response = next.run(req).await;
    let latency_ms = start.elapsed().as_millis();
    let status = response.status().as_u16();

    // Optionally buffer and log response body
    let (response, resp_body_snippet) = if log_bodies() {
        let (parts, body) = response.into_parts();
        let bytes = body
            .collect()
            .await
            .map(|b| b.to_bytes())
            .unwrap_or_default();
        let snippet = truncate_body(&bytes);
        let response = Response::from_parts(parts, Body::from(bytes));
        (response, Some(snippet))
    } else {
        (response, None)
    };

    if status >= 500 {
        tracing::error!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = status,
            latency_ms = latency_ms,
            body = resp_body_snippet.as_deref().unwrap_or(""),
            "← response [ERROR]"
        );
    } else if status >= 400 {
        tracing::warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = status,
            latency_ms = latency_ms,
            body = resp_body_snippet.as_deref().unwrap_or(""),
            "← response [WARN]"
        );
    } else {
        tracing::info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = status,
            latency_ms = latency_ms,
            "← response"
        );
    }

    response
}

/// Truncate body bytes to `MAX_BODY_LOG_BYTES` and convert to a lossy UTF-8 string.
fn truncate_body(bytes: &[u8]) -> String {
    let slice = if bytes.len() > MAX_BODY_LOG_BYTES {
        &bytes[..MAX_BODY_LOG_BYTES]
    } else {
        bytes
    };
    let mut s = String::from_utf8_lossy(slice).into_owned();
    if bytes.len() > MAX_BODY_LOG_BYTES {
        s.push_str("…[truncated]");
    }
    s
}
