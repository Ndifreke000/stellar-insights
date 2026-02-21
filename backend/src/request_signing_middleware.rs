use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct SigningSecret(pub Arc<str>);

#[derive(Debug, Clone)]
pub struct SignatureVerifiedUser {
    pub user_id: String,
    pub username: String,
}

/// Middleware to verify request signature
pub async fn request_signing_middleware(
    SigningSecret(signing_secret): SigningSecret,
    mut req: Request,
    next: Next,
) -> Result<Response, SigningError> {
    // Extract signature header
    let signature = req
        .headers()
        .get("X-Signature")
        .and_then(|h| h.to_str().ok())
        .ok_or(SigningError::MissingSignature)?;
    let timestamp = req
        .headers()
        .get("X-Timestamp")
        .and_then(|h| h.to_str().ok())
        .ok_or(SigningError::MissingTimestamp)?;

    // Prevent replay: check timestamp is recent (within 5 min)
    let ts = timestamp
        .parse::<i64>()
        .map_err(|_| SigningError::InvalidTimestamp)?;
    let now = Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        return Err(SigningError::ReplayDetected);
    }

    // Compute expected signature
    let body = req.body().to_bytes().await.unwrap_or_default();
    let mut mac = HmacSha256::new_from_slice(signing_secret.as_ref().as_bytes())
        .map_err(|_| SigningError::Internal)?;
    mac.update(timestamp.as_bytes());
    mac.update(&body);
    let expected = hex::encode(mac.finalize().into_bytes());

    if signature != expected {
        return Err(SigningError::InvalidSignature);
    }

    // Attach verified user (stub, integrate with auth as needed)
    req.extensions_mut().insert(SignatureVerifiedUser {
        user_id: "stub-user-id".to_string(),
        username: "stub-username".to_string(),
    });

    Ok(next.run(req).await)
}

#[derive(Debug)]
pub enum SigningError {
    MissingSignature,
    MissingTimestamp,
    InvalidTimestamp,
    ReplayDetected,
    InvalidSignature,
    Internal,
}

impl IntoResponse for SigningError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            SigningError::MissingSignature => {
                (StatusCode::UNAUTHORIZED, "Missing X-Signature header")
            }
            SigningError::MissingTimestamp => {
                (StatusCode::UNAUTHORIZED, "Missing X-Timestamp header")
            }
            SigningError::InvalidTimestamp => (StatusCode::BAD_REQUEST, "Invalid timestamp"),
            SigningError::ReplayDetected => (StatusCode::UNAUTHORIZED, "Replay attack detected"),
            SigningError::InvalidSignature => {
                (StatusCode::UNAUTHORIZED, "Invalid request signature")
            }
            SigningError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
        };
        let body = json!({"error": message});
        (status, axum::response::Json(body)).into_response()
    }
}
