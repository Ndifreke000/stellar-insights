use axum::{
    body::Body,
    extract::Request,
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::fmt;
use tracing::Instrument;
use uuid::Uuid;

/// Header carrying the per-request ID (one per HTTP request/response).
pub static REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");
/// Header carrying the correlation ID shared by every request in one
/// distributed flow (e.g. frontend → backend → webhook).
pub static CORRELATION_ID_HEADER: HeaderName = HeaderName::from_static("x-correlation-id");

/// Upstream-supplied IDs longer than this are replaced, so clients can't bloat logs.
const MAX_ID_LEN: usize = 128;

/// Request ID wrapper for storing in request extensions
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    /// Generate a new random request ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Get the request ID as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Correlation ID wrapper for storing in request extensions.
#[derive(Clone, Debug)]
pub struct CorrelationId(pub String);

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Accept an upstream ID only if it is short and made of safe characters —
/// anything else could inject fake fields/lines into structured logs.
fn sanitize_id(value: &HeaderValue) -> Option<String> {
    let id = value.to_str().ok()?.trim();
    let valid = !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'));
    valid.then(|| id.to_string())
}

/// Middleware to add request and correlation ID tracking
///
/// This middleware:
/// - Reuses a valid upstream `X-Request-ID` or generates a new one
/// - Reuses a valid upstream `X-Correlation-ID`, defaulting to the request ID
/// - Stores both in request extensions (and normalised request headers) for handlers
/// - Runs the request inside an `http_request` span carrying both IDs, so every
///   log line emitted while handling the request includes them
/// - Echoes both IDs in the response headers
pub async fn request_id_middleware(mut req: Request<Body>, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(sanitize_id)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let correlation_id = req
        .headers()
        .get(&CORRELATION_ID_HEADER)
        .and_then(sanitize_id)
        .unwrap_or_else(|| request_id.clone());

    // IDs are validated ASCII, so these conversions cannot fail.
    let request_id_value = HeaderValue::from_str(&request_id).ok();
    let correlation_id_value = HeaderValue::from_str(&correlation_id).ok();

    // Normalise the request headers so handlers that forward them downstream
    // propagate the sanitised values.
    if let Some(v) = &request_id_value {
        req.headers_mut().insert(REQUEST_ID_HEADER.clone(), v.clone());
    }
    if let Some(v) = &correlation_id_value {
        req.headers_mut().insert(CORRELATION_ID_HEADER.clone(), v.clone());
    }
    req.extensions_mut().insert(RequestId(request_id.clone()));
    req.extensions_mut()
        .insert(CorrelationId(correlation_id.clone()));

    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        correlation_id = %correlation_id
    );
    let mut response = next.run(req).instrument(span).await;

    if let Some(v) = request_id_value {
        response.headers_mut().insert(REQUEST_ID_HEADER.clone(), v);
    }
    if let Some(v) = correlation_id_value {
        response
            .headers_mut()
            .insert(CORRELATION_ID_HEADER.clone(), v);
    }

    response
}

/// Extract request ID from request extensions
///
/// Returns None if no request ID is found (shouldn't happen if middleware is applied)
pub fn get_request_id(req: &Request<Body>) -> Option<String> {
    req.extensions().get::<RequestId>().map(|id| id.0.clone())
}

/// Error response with request ID
#[must_use]
pub fn error_with_request_id(
    status: StatusCode,
    message: String,
    request_id: Option<String>,
) -> Response {
    let body = if let Some(id) = request_id {
        serde_json::json!({
            "error": message,
            "request_id": id
        })
    } else {
        serde_json::json!({
            "error": message
        })
    };

    (status, axum::Json(body)).into_response()
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

    #[test]
    fn test_request_id_creation() {
        let id1 = RequestId::new();
        let id2 = RequestId::new();

        // IDs should be different
        assert_ne!(id1.0, id2.0);

        // IDs should be valid UUIDs (36 characters with hyphens)
        assert_eq!(id1.0.len(), 36);
        assert_eq!(id2.0.len(), 36);
    }

    #[test]
    fn test_request_id_display() {
        let id = RequestId::new();
        let display = format!("{}", id);
        assert_eq!(display, id.0);
    }

    #[test]
    fn test_request_id_as_str() {
        let id = RequestId::new();
        assert_eq!(id.as_str(), &id.0);
    }

    #[test]
    fn test_request_id_clone() {
        let id1 = RequestId::new();
        let id2 = id1.clone();
        assert_eq!(id1.0, id2.0);
    }

    #[test]
    fn test_request_id_default() {
        let id = RequestId::default();
        assert_eq!(id.0.len(), 36);
    }

    #[tokio::test]
    async fn middleware_sets_response_request_id() {
        let app = Router::new()
            .route("/health", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(request_id_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("X-Request-ID").is_some());
    }

    #[tokio::test]
    async fn middleware_preserves_upstream_request_id() {
        let app = Router::new()
            .route("/health", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(request_id_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header("X-Request-ID", "upstream-request-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("X-Request-ID")
                .and_then(|h| h.to_str().ok()),
            Some("upstream-request-id")
        );
    }
}
