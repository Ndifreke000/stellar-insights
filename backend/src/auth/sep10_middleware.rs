use axum::{
    async_trait,
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;

use super::sep10_simple::Sep10Service;

/// Extract SEP-10 authenticated user from request
#[derive(Debug, Clone)]
pub struct Sep10User {
    pub account: String,
    pub client_domain: Option<String>,
}

/// SEP-10 claims for extracting authenticated user in handlers
#[derive(Debug, Clone)]
pub struct Sep10Claims {
    pub account: String,
    pub client_domain: Option<String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for Sep10Claims
where
    S: Send + Sync,
{
    type Rejection = Sep10AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get Sep10User from extensions (set by middleware)
        parts
            .extensions
            .get::<Sep10User>()
            .map(|user| Sep10Claims {
                account: user.account.clone(),
                client_domain: user.client_domain.clone(),
            })
            .ok_or(Sep10AuthError::MissingToken)
    }
}

/// SEP-10 authentication middleware
pub async fn sep10_auth_middleware(
    State(sep10_service): State<Arc<Sep10Service>>,
    mut req: Request,
    next: Next,
) -> Result<Response, Sep10AuthError> {
    // Extract Authorization header and token before mutating req
    let token = {
        let auth_header = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(Sep10AuthError::MissingToken)?;

        // Extract Bearer token
        auth_header
            .strip_prefix("Bearer ")
            .ok_or(Sep10AuthError::InvalidToken)?
            .to_string()
    };

    // Validate session
    let session = sep10_service
        .validate_session(&token)
        .await
        .map_err(|_| Sep10AuthError::InvalidToken)?;

    // Attach user to request extensions
    let sep10_user = Sep10User {
        account: session.account,
        client_domain: session.client_domain,
    };
    req.extensions_mut().insert(sep10_user);
    req.extensions_mut().insert(token);

    Ok(next.run(req).await)
}

/// SEP-10 authentication errors
#[derive(Debug)]
pub enum Sep10AuthError {
    MissingToken,
    InvalidToken,
}

impl IntoResponse for Sep10AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Sep10AuthError::MissingToken => {
                (StatusCode::UNAUTHORIZED, "Missing authentication token")
            }
            Sep10AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
        };

        let body = json!({
            "error": message,
        });

        (status, axum::Json(body)).into_response()
    }
}
