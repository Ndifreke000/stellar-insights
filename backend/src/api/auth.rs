use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::auth::{AuthService, LoginRequest, LogoutRequest, RefreshTokenRequest};

const TOKEN_ENDPOINT_LIMIT_PER_MINUTE: usize = 5;
const ACCOUNT_LOCKOUT_THRESHOLD: u32 = 5;
const CAPTCHA_THRESHOLD: u32 = 3;
const ACCOUNT_LOCKOUT_DURATION: Duration = Duration::from_secs(15 * 60);
const MAX_BACKOFF_SECONDS: u64 = 300;

#[derive(Debug, Clone)]
struct FailedAuthState {
    failed_attempts: u32,
    lockout_until: Option<Instant>,
    next_allowed_attempt_at: Option<Instant>,
}

static TOKEN_RATE_LIMIT_WINDOWS: LazyLock<Mutex<HashMap<String, VecDeque<Instant>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static FAILED_LOGIN_STATE: LazyLock<Mutex<HashMap<String, FailedAuthState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub enum AuthApiError {
    InvalidCredentials,
    InvalidToken,
    AccountLocked { retry_after_seconds: u64 },
    CaptchaRequired,
    RateLimited { retry_after_seconds: u64 },
}

impl IntoResponse for AuthApiError {
    fn into_response(self) -> Response {
        let (status, code, message, retry_after) = match self {
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "INVALID_CREDENTIALS",
                "Invalid username or password".to_string(),
                None,
            ),
            Self::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "INVALID_TOKEN",
                "Invalid or expired token".to_string(),
                None,
            ),
            Self::AccountLocked { retry_after_seconds } => (
                StatusCode::TOO_MANY_REQUESTS,
                "ACCOUNT_LOCKED",
                "Account temporarily locked due to repeated failed login attempts".to_string(),
                Some(retry_after_seconds),
            ),
            Self::CaptchaRequired => (
                StatusCode::TOO_MANY_REQUESTS,
                "CAPTCHA_REQUIRED",
                "CAPTCHA verification required after repeated failed login attempts".to_string(),
                None,
            ),
            Self::RateLimited { retry_after_seconds } => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                "Too many authentication attempts. Please try again later.".to_string(),
                Some(retry_after_seconds),
            ),
        };

        let body = json!({
            "error": {
                "code": code,
                "message": message,
            }
        });

        let mut response = (status, Json(body)).into_response();
        if let Some(seconds) = retry_after {
            if let Ok(value) = axum::http::HeaderValue::from_str(&seconds.to_string()) {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, value);
            }
        }
        response
    }
}

async fn check_rate_limit_for_account(account_key: &str) -> Option<u64> {
    let now = Instant::now();
    let mut windows = TOKEN_RATE_LIMIT_WINDOWS.lock().await;
    let entries = windows
        .entry(account_key.to_string())
        .or_insert_with(VecDeque::new);
    let window = Duration::from_secs(60);

    while let Some(oldest) = entries.front().copied() {
        if now.duration_since(oldest) >= window {
            entries.pop_front();
        } else {
            break;
        }
    }

    if entries.len() >= TOKEN_ENDPOINT_LIMIT_PER_MINUTE {
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

async fn preflight_login_guards(username: &str, headers: &HeaderMap) -> Result<(), AuthApiError> {
    let account = username.to_lowercase();
    let now = Instant::now();
    let mut states = FAILED_LOGIN_STATE.lock().await;
    if let Some(state) = states.get_mut(&account) {
        if let Some(lock_until) = state.lockout_until {
            if now < lock_until {
                return Err(AuthApiError::AccountLocked {
                    retry_after_seconds: lock_until.duration_since(now).as_secs().max(1),
                });
            }
            state.lockout_until = None;
        }

        if let Some(next_allowed) = state.next_allowed_attempt_at {
            if now < next_allowed {
                return Err(AuthApiError::RateLimited {
                    retry_after_seconds: next_allowed.duration_since(now).as_secs().max(1),
                });
            }
        }

        if state.failed_attempts >= CAPTCHA_THRESHOLD {
            let captcha = headers
                .get("x-captcha-token")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .unwrap_or_default();
            if captcha.is_empty() {
                tracing::warn!(
                    username = %account,
                    failed_attempts = state.failed_attempts,
                    "Suspicious auth attempt blocked: missing CAPTCHA token"
                );
                return Err(AuthApiError::CaptchaRequired);
            }
        }
    }

    Ok(())
}

async fn record_failed_login(username: &str) {
    let account = username.to_lowercase();
    let now = Instant::now();
    let mut states = FAILED_LOGIN_STATE.lock().await;
    let state = states.entry(account.clone()).or_insert(FailedAuthState {
        failed_attempts: 0,
        lockout_until: None,
        next_allowed_attempt_at: None,
    });

    state.failed_attempts = state.failed_attempts.saturating_add(1);
    let backoff_seconds = 2_u64
        .saturating_pow(state.failed_attempts.saturating_sub(1))
        .min(MAX_BACKOFF_SECONDS);
    state.next_allowed_attempt_at = Some(now + Duration::from_secs(backoff_seconds));

    if state.failed_attempts >= ACCOUNT_LOCKOUT_THRESHOLD {
        state.lockout_until = Some(now + ACCOUNT_LOCKOUT_DURATION);
        tracing::warn!(
            username = %account,
            failed_attempts = state.failed_attempts,
            "Brute-force pattern detected: account lockout enforced"
        );
    } else {
        tracing::warn!(
            username = %account,
            failed_attempts = state.failed_attempts,
            backoff_seconds,
            "Suspicious authentication failure recorded"
        );
    }
}

async fn clear_failed_login_state(username: &str) {
    let account = username.to_lowercase();
    let mut states = FAILED_LOGIN_STATE.lock().await;
    states.remove(&account);
}

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
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Response, AuthApiError> {
    let account_key = request.username.to_lowercase();
    if let Some(retry_after_seconds) = check_rate_limit_for_account(&account_key).await {
        tracing::warn!(
            username = %account_key,
            retry_after_seconds,
            "Token endpoint rate limit exceeded for account"
        );
        return Err(AuthApiError::RateLimited { retry_after_seconds });
    }

    preflight_login_guards(&request.username, &headers).await?;

    let response = auth_service.login(request).await.map_err(|_| AuthApiError::InvalidCredentials);
    match response {
        Ok(login_response) => {
            clear_failed_login_state(&account_key).await;
            Ok((StatusCode::OK, Json(login_response)).into_response())
        }
        Err(e) => {
            record_failed_login(&account_key).await;
            Err(e)
        }
    }
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
) -> Result<Response, AuthApiError> {
    let response = auth_service
        .refresh(request)
        .await
        .map_err(|_| AuthApiError::InvalidToken)?;

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

/// Create auth routes
pub fn routes(auth_service: Arc<AuthService>) -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .with_state(auth_service)
}
