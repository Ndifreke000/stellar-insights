use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::auth::{AuthService, LoginRequest, LogoutRequest, RefreshTokenRequest};

/// POST /api/auth/login - User login
pub async fn login(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<LoginRequest>,
) -> Result<Response, AuthApiError> {
    let response = auth_service
        .login(request)
        .await
        .map_err(|_| AuthApiError::InvalidCredentials)?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/auth/refresh - Refresh access token
pub async fn refresh(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Response, AuthApiError> {
    let response = auth_service
        .refresh(request)
        .await
        .map_err(|_| AuthApiError::InvalidToken)?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/auth/logout - Logout user
pub async fn logout(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<LogoutRequest>,
) -> Result<Response, AuthApiError> {
    auth_service
        .logout(request)
        .await
        .map_err(|_| AuthApiError::InvalidToken)?;

    let body = json!({
        "message": "Logged out successfully"
    });

    Ok((StatusCode::OK, Json(body)).into_response())
}

/// Auth API errors
#[derive(Debug)]
pub enum AuthApiError {
    InvalidCredentials,
    InvalidToken,
}

impl IntoResponse for AuthApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthApiError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "Invalid username or password",
            ),
            AuthApiError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "Invalid or expired token",
            ),
        };

        let body = json!({
            "error": message,
        });

        (status, Json(body)).into_response()
    }
}

/// Create auth routes
pub fn routes(auth_service: Arc<AuthService>) -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .with_state(auth_service)
}
