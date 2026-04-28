use axum::{
    body::Body,
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Middleware to add deprecation headers to v1 endpoints
pub async fn deprecation_middleware(req: Request<Body>, next: Next) -> Response {
    let uri = req.uri().to_string();
    let mut response = next.run(req).await;

    // Add deprecation headers to v1 endpoints
    if uri.contains("/api/v1/") {
        response.headers_mut().insert("Deprecation", HeaderValue::from_static("true"));
        response.headers_mut().insert("Sunset", HeaderValue::from_static("Wed, 01 Jan 2025 00:00:00 GMT"));
        response.headers_mut().insert("Link", HeaderValue::from_static("</api/v2/>; rel=\"successor-version\""));
        
        if let Ok(value) = HeaderValue::from_str("API v1 is deprecated. Please migrate to v2. See https://docs.stellar-insights.com/api-versioning") {
            response.headers_mut().insert("Warning", value);
        }
    }

    response
}
