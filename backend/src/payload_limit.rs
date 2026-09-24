//! Middleware to limit request payload size and prevent DoS attacks

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Maximum payload size in bytes (10MB)
const MAX_PAYLOAD_SIZE: usize = 10 * 1024 * 1024;

/// Middleware to enforce maximum payload size
pub async fn payload_limit_middleware(req: Request, next: Next) -> Response {
    // Check Content-Length header if present
    if let Some(content_length) = req.headers().get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                if length > MAX_PAYLOAD_SIZE {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        format!(
                            "Payload size {} exceeds maximum of {} bytes",
                            length, MAX_PAYLOAD_SIZE
                        ),
                    )
                        .into_response();
                }
            }
        }
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::post,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_payload_within_limit() {
        let app = Router::new()
            .route("/test", post(test_handler))
            .layer(middleware::from_fn(payload_limit_middleware));

        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-length", "1000")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_payload_exceeds_limit() {
        let app = Router::new()
            .route("/test", post(test_handler))
            .layer(middleware::from_fn(payload_limit_middleware));

        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-length", "11000000") // 11MB
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
