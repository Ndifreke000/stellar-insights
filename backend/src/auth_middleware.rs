use axum::{
    extract::{Extension, Request},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;

use crate::auth::Claims;

/// JWT secret shared via extension
#[derive(Clone)]
pub struct JwtSecret(pub Arc<str>);

/// Extract user from authenticated request
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
}

/// Auth middleware - validates JWT from Authorization header
pub async fn auth_middleware(
    Extension(JwtSecret(jwt_secret)): Extension<JwtSecret>,
    mut req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    // Extract Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidToken)?;

    // Validate token
    let claims = validate_access_token(token, jwt_secret.as_ref())?;

    // Attach user to request extensions
    let auth_user = AuthUser {
        user_id: claims.sub,
        username: claims.username,
    };
    req.extensions_mut().insert(auth_user);

    Ok(next.run(req).await)
}

/// Validate access token
fn validate_access_token(token: &str, secret: &str) -> Result<Claims, AuthError> {
    use jsonwebtoken::{decode, DecodingKey, Validation};

    let validation = Validation::default();

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| {
        // Verify it's an access token
        if data.claims.token_type != "access" {
            return Err(AuthError::InvalidToken);
        }
        Ok(data.claims)
    })
    .map_err(|_| AuthError::InvalidToken)?
}

/// Authentication errors
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
        };

        let body = json!({
            "error": message,
        });

        (status, axum::Json(body)).into_response()
    }
}
