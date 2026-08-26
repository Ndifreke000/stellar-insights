use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use redis::aio::MultiplexedConnection;

use crate::services::request_signing::RequestSigningService;

#[derive(Clone)]
pub struct SigningSecret(pub Arc<str>);

#[derive(Clone)]
pub struct SigningService {
    pub service: Arc<RequestSigningService>,
}

#[derive(Debug, Clone)]
pub struct SignatureVerifiedUser {
    pub user_id: String,
    pub username: String,
}

const CLOCK_SKEW_SECS: i64 = 300; // 5 minutes

/// Middleware to verify request signature with HMAC-SHA256 and replay protection
pub async fn request_signing_middleware(
    SigningSecret(signing_secret): SigningSecret,
    SigningService { service }: SigningService,
    req: Request,
    next: Next,
) -> Result<Response, SigningError> {
    // Extract required headers
    let signature = req
        .headers()
        .get("X-Signature")
        .and_then(|h| h.to_str().ok())
        .map(std::string::ToString::to_string)
        .ok_or(SigningError::InvalidRequest)?;

    let timestamp = req
        .headers()
        .get("X-Timestamp")
        .and_then(|h| h.to_str().ok())
        .map(std::string::ToString::to_string)
        .ok_or(SigningError::InvalidRequest)?;

    let nonce = req
        .headers()
        .get("X-Nonce")
        .and_then(|h| h.to_str().ok())
        .map(std::string::ToString::to_string)
        .ok_or(SigningError::InvalidRequest)?;

    // Parse timestamp
    let ts = timestamp
        .parse::<i64>()
        .map_err(|_| SigningError::InvalidRequest)?;

    // Extract method and path
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Extract and sort query parameters
    let mut query_params = BTreeMap::new();
    if let Some(query) = req.uri.query() {
        for param in query.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                query_params.insert(key.to_string(), value.to_string());
            }
        }
    }

    // Collect body
    let max_body_size: usize = std::env::var("MAX_REQUEST_BODY_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10 * 1024 * 1024);
    let (parts, body) = req.into_parts();
    let body_bytes = axum::body::to_bytes(body, max_body_size)
        .await
        .map_err(|_| SigningError::InvalidRequest)?;

    // Verify signature
    let valid = service
        .verify_signature(
            &method,
            &path,
            query_params,
            &body_bytes,
            ts,
            &nonce,
            &signature,
            signing_secret.as_ref(),
            CLOCK_SKEW_SECS,
        )
        .await
        .map_err(|_| SigningError::InvalidRequest)?;

    if !valid {
        return Err(SigningError::InvalidRequest);
    }

    // Reconstruct request
    let mut req = Request::from_parts(parts, axum::body::Body::from(body_bytes));

    req.extensions_mut().insert(SignatureVerifiedUser {
        user_id: "authenticated".to_string(),
        username: "authenticated".to_string(),
    });

    Ok(next.run(req).await)
}

#[derive(Debug)]
pub enum SigningError {
    InvalidRequest,
}

impl IntoResponse for SigningError {
    fn into_response(self) -> Response {
        let body = json!({"error": "Invalid request signature or headers"});
        (StatusCode::UNAUTHORIZED, axum::response::Json(body)).into_response()
    }
}
