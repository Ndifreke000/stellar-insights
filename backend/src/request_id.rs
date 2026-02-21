use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::fmt;
use uuid::Uuid;

/// Request ID wrapper for storing in request extensions
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    /// Generate a new random request ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Get the request ID as a string slice
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

/// Middleware to add request ID tracking
///
/// This middleware:
/// - Generates a unique request ID for each request
/// - Adds it to request extensions for use in handlers
/// - Includes it in response headers as X-Request-ID
/// - Logs the request ID for tracing
pub async fn request_id_middleware(mut req: Request<Body>, next: Next) -> Response {
    // Check if request already has an X-Request-ID header (from upstream)
    let request_id = if let Some(existing_id) = req.headers().get("X-Request-ID") {
        existing_id
            .to_str()
            .ok()
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string())
    } else {
        Uuid::new_v4().to_string()
    };

    // Store request ID in extensions for handlers to access
    req.extensions_mut().insert(RequestId(request_id.clone()));

    // Log the request with ID
    let method = req.method().clone();
    let uri = req.uri().clone();
    tracing::info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        "Incoming request"
    );

    // Process the request
    let response = next.run(req).await;

    // Add request ID to response headers
    let (mut parts, body) = response.into_parts();

    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        parts.headers.insert("X-Request-ID", header_value);
    }

    Response::from_parts(parts, body)
}

/// Extract request ID from request extensions
///
/// Returns None if no request ID is found (shouldn't happen if middleware is applied)
pub fn get_request_id(req: &Request<Body>) -> Option<String> {
    req.extensions().get::<RequestId>().map(|id| id.0.clone())
}

/// Error response with request ID
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
}
