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
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful"),
        (status = 401, description = "Invalid credentials")
    ),
    tag = "Auth"
)]
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
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed"),
        (status = 401, description = "Invalid or expired token")
    ),
    tag = "Auth"
)]
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
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logged out successfully"),
        (status = 401, description = "Invalid or expired token")
    ),
    tag = "Auth"
)]
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
