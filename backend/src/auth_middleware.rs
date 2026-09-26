use axum::{
    extract::{Extension, Request},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::auth::Claims;

/// JWT secret shared via extension
#[derive(Clone)]
pub struct JwtSecret(pub Arc<str>);

/// SQLite pool for token revocation lookups, shared via extension
#[derive(Clone)]
pub struct TokenRevocationStore(pub Arc<SqlitePool>);

/// Extract user from authenticated request
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
    /// Session ID tied to the token that authenticated this request, if the
    /// token carries one (see Claims::session_id). Needed to distinguish
    /// "this session" from "this user's other sessions".
    pub session_id: Option<String>,
    pub is_admin: bool,
}

impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or(AuthError::MissingToken)
    }
}

/// Like AuthUser, but only resolves for a user with is_admin = true.
/// Rejects with AuthError::Forbidden for a valid, authenticated non-admin
/// -- distinct from AuthError::MissingToken/InvalidToken, which mean "we
/// don't know who this is" rather than "we know exactly who this is, and
/// they're not allowed here".
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthUser);

impl<S> axum::extract::FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        if auth_user.is_admin {
            Ok(Self(auth_user))
        } else {
            Err(AuthError::Forbidden)
        }
    }
}

/// Auth middleware - validates JWT and checks token revocation
pub async fn auth_middleware(
    Extension(JwtSecret(jwt_secret)): Extension<JwtSecret>,
    Extension(revocation_store): Extension<TokenRevocationStore>,
    mut req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidToken)?;

    let claims = validate_access_token(token, jwt_secret.as_ref())?;

    // Check revocation table if the token carries a jti
    if let Some(ref jti) = claims.jti {
        if is_token_revoked(&revocation_store, jti)
            .await
            .map_err(|_| AuthError::InvalidToken)?
        {
            return Err(AuthError::InvalidToken);
        }
    }

    let auth_user = AuthUser {
        user_id: claims.sub,
        username: claims.username,
        session_id: claims.session_id,
        is_admin: claims.is_admin,
    };
    req.extensions_mut().insert(auth_user);

    Ok(next.run(req).await)
}

/// Returns true when the jti is in the revocations table and has not yet expired.
///
/// pub(crate) so AuthService::complete_2fa_login (auth.rs) can also use it
/// to enforce that a pending-2FA token is single-use.
pub(crate) async fn is_token_revoked(
    store: &TokenRevocationStore,
    jti: &str,
) -> Result<bool, sqlx::Error> {
    let revoked: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM token_revocations
            WHERE jti = ? AND expires_at > strftime('%s', 'now')
        )",
    )
    .bind(jti)
    .fetch_one(store.0.as_ref())
    .await?;

    Ok(revoked)
}

/// Insert a revocation record so the given jti is rejected until expires_at.
/// Call this on password change or key rotation, passing the old token's jti
/// and the token's original exp timestamp as expires_at.
pub async fn revoke_token(
    store: &TokenRevocationStore,
    jti: &str,
    user_id: &str,
    expires_at: i64,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().timestamp();
    sqlx::query(
        "INSERT OR IGNORE INTO token_revocations (jti, user_id, revoked_at, expires_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(jti)
    .bind(user_id)
    .bind(now)
    .bind(expires_at)
    .execute(store.0.as_ref())
    .await?;

    Ok(())
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
    /// Authenticated, but not an admin (see AdminUser).
    Forbidden,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
            Self::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "Admin access required"),
        };

        let body = json!({
            "error": message,
        });

        (status, axum::Json(body)).into_response()
    }
}
