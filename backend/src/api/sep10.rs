use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::auth::sep10_simple::{ChallengeRequest, Sep10Service, VerificationRequest};

/// GET /api/sep10/info - Get SEP-10 server information
pub async fn get_info(
    State(sep10_service): State<Arc<Sep10Service>>,
) -> Result<Response, Sep10ApiError> {
    let info = json!({
        "authentication_endpoint": "/api/sep10/auth",
        "network_passphrase": sep10_service.network_passphrase,
        "signing_key": sep10_service.server_public_key,
        "version": "1.0.0"
    });

    Ok((StatusCode::OK, Json(info)).into_response())
}

/// POST /api/sep10/auth - Request SEP-10 challenge transaction
pub async fn request_challenge(
    State(sep10_service): State<Arc<Sep10Service>>,
    Json(request): Json<ChallengeRequest>,
) -> Result<Response, Sep10ApiError> {
    let response = sep10_service
        .generate_challenge(request)
        .await
        .map_err(|e| Sep10ApiError::ChallengeGenerationFailed(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/sep10/verify - Verify signed challenge transaction
pub async fn verify_challenge(
    State(sep10_service): State<Arc<Sep10Service>>,
    Json(request): Json<VerificationRequest>,
) -> Result<Response, Sep10ApiError> {
    let response = sep10_service
        .verify_challenge(request)
        .await
        .map_err(|e| Sep10ApiError::VerificationFailed(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/sep10/logout - Invalidate SEP-10 session
pub async fn logout(
    State(sep10_service): State<Arc<Sep10Service>>,
    axum::extract::Extension(token): axum::extract::Extension<String>,
) -> Result<Response, Sep10ApiError> {
    sep10_service
        .invalidate_session(&token)
        .await
        .map_err(|e| Sep10ApiError::LogoutFailed(e.to_string()))?;

    let body = json!({
        "message": "Logged out successfully"
    });

    Ok((StatusCode::OK, Json(body)).into_response())
}

/// SEP-10 API errors
#[derive(Debug)]
pub enum Sep10ApiError {
    ChallengeGenerationFailed(String),
    VerificationFailed(String),
    LogoutFailed(String),
}

impl IntoResponse for Sep10ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Sep10ApiError::ChallengeGenerationFailed(msg) => (
                StatusCode::BAD_REQUEST,
                format!("Challenge generation failed: {}", msg),
            ),
            Sep10ApiError::VerificationFailed(msg) => (
                StatusCode::UNAUTHORIZED,
                format!("Verification failed: {}", msg),
            ),
            Sep10ApiError::LogoutFailed(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Logout failed: {}", msg),
            ),
        };

        let body = json!({
            "error": message,
        });

        (status, Json(body)).into_response()
    }
}

/// Create SEP-10 routes
pub fn routes(sep10_service: Arc<Sep10Service>) -> Router {
    Router::new()
        .route("/api/sep10/info", get(get_info))
        .route("/api/sep10/auth", post(request_challenge))
        .route("/api/sep10/verify", post(verify_challenge))
        .route("/api/sep10/logout", post(logout))
        .with_state(sep10_service)
}
