use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::auth::sep10_simple::{ChallengeRequest, Sep10Service, VerificationRequest};

const SEP10_CHALLENGE_LIMIT_PER_MINUTE: usize = 10;
static SEP10_CHALLENGE_WINDOWS: LazyLock<Mutex<HashMap<String, VecDeque<Instant>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn extract_client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|ip| !ip.is_empty())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|ip| !ip.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

async fn check_challenge_rate_limit(ip: &str) -> Option<u64> {
    let now = Instant::now();
    let window = Duration::from_secs(60);
    let mut buckets = SEP10_CHALLENGE_WINDOWS.lock().await;
    let entries = buckets
        .entry(ip.to_string())
        .or_insert_with(VecDeque::new);

    while let Some(oldest) = entries.front().copied() {
        if now.duration_since(oldest) >= window {
            entries.pop_front();
        } else {
            break;
        }
    }

    if entries.len() >= SEP10_CHALLENGE_LIMIT_PER_MINUTE {
        let retry_after_seconds = entries
            .front()
            .copied()
            .map(|first| {
                window
                    .saturating_sub(now.duration_since(first))
                    .as_secs()
                    .max(1)
            })
            .unwrap_or(60);
        return Some(retry_after_seconds);
    }

    entries.push_back(now);
    None
}

/// GET /api/sep10/info - Get SEP-10 server information
#[utoipa::path(
    get,
    path = "/api/sep10/info",
    responses(
        (status = 200, description = "SEP-10 server information")
    ),
    tag = "SEP-10"
)]
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
#[utoipa::path(
    post,
    path = "/api/sep10/auth",
    request_body = ChallengeRequest,
    responses(
        (status = 200, description = "Challenge transaction generated"),
        (status = 400, description = "Challenge generation failed")
    ),
    tag = "SEP-10"
)]
pub async fn request_challenge(
    State(sep10_service): State<Arc<Sep10Service>>,
    headers: HeaderMap,
    Json(request): Json<ChallengeRequest>,
) -> Result<Response, Sep10ApiError> {
    let client_ip = extract_client_ip(&headers);
    if let Some(retry_after_seconds) = check_challenge_rate_limit(&client_ip).await {
        tracing::warn!(
            client_ip = %client_ip,
            retry_after_seconds,
            "SEP-10 challenge rate limit exceeded"
        );
        return Err(Sep10ApiError::RateLimited { retry_after_seconds });
    }

    let response = sep10_service
        .generate_challenge(request)
        .await
        .map_err(|e| Sep10ApiError::ChallengeGenerationFailed(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/sep10/verify - Verify signed challenge transaction
#[utoipa::path(
    post,
    path = "/api/sep10/verify",
    request_body = VerificationRequest,
    responses(
        (status = 200, description = "Verification successful"),
        (status = 401, description = "Verification failed")
    ),
    tag = "SEP-10"
)]
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
#[utoipa::path(
    post,
    path = "/api/sep10/logout",
    responses(
        (status = 200, description = "Logged out successfully"),
        (status = 500, description = "Logout failed")
    ),
    tag = "SEP-10"
)]
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
    RateLimited { retry_after_seconds: u64 },
}

impl IntoResponse for Sep10ApiError {
    fn into_response(self) -> Response {
        let (status, message, retry_after) = match self {
            Self::ChallengeGenerationFailed(msg) => (
                StatusCode::BAD_REQUEST,
                format!("Challenge generation failed: {msg}"),
                None,
            ),
            Self::VerificationFailed(msg) => (
                StatusCode::UNAUTHORIZED,
                format!("Verification failed: {msg}"),
                None,
            ),
            Self::LogoutFailed(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Logout failed: {msg}"),
                None,
            ),
            Self::RateLimited { retry_after_seconds } => (
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "Too many SEP-10 challenge requests. Retry after {retry_after_seconds} seconds"
                ),
                Some(retry_after_seconds),
            ),
        };

        let body = json!({
            "error": message,
        });
        let mut response = (status, Json(body)).into_response();
        if let Some(retry_after_seconds) = retry_after {
            if let Ok(value) = axum::http::HeaderValue::from_str(&retry_after_seconds.to_string()) {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, value);
            }
        }
        response
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
