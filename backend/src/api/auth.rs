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
use crate::error::ApiError;

/// POST /api/auth/login - User login
pub async fn login(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let response = auth_service.login(request).await.map_err(|_| {
        ApiError::unauthorized("INVALID_CREDENTIALS", "Invalid username or password")
    })?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/auth/refresh - Refresh access token
pub async fn refresh(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Response, ApiError> {
    let response = auth_service
        .refresh(request)
        .await
        .map_err(|_| ApiError::unauthorized("INVALID_TOKEN", "Invalid or expired token"))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/auth/logout - Logout user
pub async fn logout(
    State(auth_service): State<Arc<AuthService>>,
    Json(request): Json<LogoutRequest>,
) -> Result<Response, ApiError> {
    auth_service
        .logout(request)
        .await
        .map_err(|_| ApiError::unauthorized("INVALID_TOKEN", "Invalid or expired token"))?;

    let body = json!({
        "message": "Logged out successfully"
    });

    Ok((StatusCode::OK, Json(body)).into_response())
}

/// Create auth routes
pub fn routes(auth_service: Arc<AuthService>) -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .with_state(auth_service)
}
