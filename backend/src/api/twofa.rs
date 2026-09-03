use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::auth_middleware::AuthUser;
use crate::twofa::TwoFAService;

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct EnrollRequest {
    pub totp_secret: String,
    pub verification_code: String,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct DisableTwoFaRequest {
    /// Current TOTP or backup code, proving the caller still controls the
    /// second factor before it's removed.
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct EnrollResponse {
    pub backup_codes: Vec<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct EnrollmentQRResponse {
    pub otpauth_uri: String,
    pub secret: String,
}

#[derive(Debug)]
pub enum TwoFAApiError {
    InvalidCode,
    EnrollmentFailed,
    VerificationFailed,
    NotEnrolled,
    ServerError,
}

impl IntoResponse for TwoFAApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidCode => (
                StatusCode::UNAUTHORIZED,
                "INVALID_CODE",
                "Invalid or expired 2FA code",
            ),
            Self::EnrollmentFailed => (
                StatusCode::BAD_REQUEST,
                "ENROLLMENT_FAILED",
                "Failed to enroll in 2FA",
            ),
            Self::VerificationFailed => (
                StatusCode::UNAUTHORIZED,
                "VERIFICATION_FAILED",
                "2FA verification failed",
            ),
            Self::NotEnrolled => (
                StatusCode::BAD_REQUEST,
                "NOT_ENROLLED",
                "User has not enrolled in 2FA",
            ),
            Self::ServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERVER_ERROR",
                "Internal server error",
            ),
        };

        let body = json!({
            "error": {
                "code": code,
                "message": message,
            }
        });

        (status, Json(body)).into_response()
    }
}

/// POST /api/auth/2fa/enroll/initiate - Start 2FA enrollment
#[utoipa::path(
    post,
    path = "/api/auth/2fa/enroll/initiate",
    responses(
        (status = 200, description = "Enrollment initiated - return QR code"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Auth"
)]
pub async fn initiate_enrollment(
    State(twofa_service): State<Arc<TwoFAService>>,
    auth_user: AuthUser,
) -> Result<Response, TwoFAApiError> {
    // Generates a fresh per-user secret. Not persisted yet -- persistence
    // happens in confirm_enrollment, only after the caller proves they can
    // produce a valid code from it (see enroll_2fa there). If they never
    // confirm, nothing was ever stored for a secret nobody's app has.
    let (otpauth_uri, secret) = twofa_service
        .generate_totp_secret(&auth_user.user_id, &auth_user.username)
        .map_err(|_| TwoFAApiError::ServerError)?;

    let response = EnrollmentQRResponse { otpauth_uri, secret };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/auth/2fa/enroll/confirm - Confirm 2FA enrollment
#[utoipa::path(
    post,
    path = "/api/auth/2fa/enroll/confirm",
    request_body = EnrollRequest,
    responses(
        (status = 200, description = "Enrollment confirmed - return backup codes"),
        (status = 400, description = "Invalid enrollment request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Auth"
)]
pub async fn confirm_enrollment(
    State(twofa_service): State<Arc<TwoFAService>>,
    auth_user: AuthUser,
    Json(request): Json<EnrollRequest>,
) -> Result<Response, TwoFAApiError> {
    // Store the secret (disabled) first, so verify_totp_code below reads
    // against the same secret the client's authenticator app was just
    // given -- not some other stale one from a prior attempt.
    twofa_service
        .enroll_2fa(&auth_user.user_id, &request.totp_secret)
        .await
        .map_err(|_| TwoFAApiError::EnrollmentFailed)?;

    let verified = twofa_service
        .verify_totp_code(&auth_user.user_id, &request.verification_code)
        .await
        .map_err(|_| TwoFAApiError::ServerError)?;

    if !verified {
        // Left as a disabled, unconfirmed row -- INSERT OR REPLACE on the
        // next attempt overwrites it. It grants nothing while is_enabled
        // is false, so there's no need to delete it here.
        return Err(TwoFAApiError::InvalidCode);
    }

    twofa_service
        .activate_2fa(&auth_user.user_id)
        .await
        .map_err(|_| TwoFAApiError::ServerError)?;

    let backup_codes = twofa_service
        .generate_backup_codes(&auth_user.user_id)
        .await
        .map_err(|_| TwoFAApiError::ServerError)?;

    let response = EnrollResponse {
        backup_codes,
        message: "2FA enrollment confirmed. Save your backup codes in a secure location.".to_string(),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/auth/2fa/disable - Disable 2FA
#[utoipa::path(
    post,
    path = "/api/auth/2fa/disable",
    request_body = DisableTwoFaRequest,
    responses(
        (status = 200, description = "2FA disabled"),
        (status = 401, description = "Unauthorized or invalid confirmation code")
    ),
    tag = "Auth"
)]
pub async fn disable_2fa(
    State(twofa_service): State<Arc<TwoFAService>>,
    auth_user: AuthUser,
    Json(request): Json<DisableTwoFaRequest>,
) -> Result<Response, TwoFAApiError> {
    // Require a still-valid TOTP or backup code before disabling -- without
    // this, anyone with just a stolen access token could strip 2FA off an
    // account, exactly the case 2FA exists to raise the bar against.
    let totp_ok = twofa_service
        .verify_totp_code(&auth_user.user_id, &request.code)
        .await
        .map_err(|_| TwoFAApiError::ServerError)?;
    let backup_ok = if totp_ok {
        false // short-circuit: don't consume a backup code if TOTP already matched
    } else {
        twofa_service
            .verify_backup_code(&auth_user.user_id, &request.code)
            .await
            .map_err(|_| TwoFAApiError::ServerError)?
    };

    if !totp_ok && !backup_ok {
        return Err(TwoFAApiError::InvalidCode);
    }

    twofa_service
        .disable_2fa(&auth_user.user_id)
        .await
        .map_err(|_| TwoFAApiError::ServerError)?;

    let response = json!({
        "message": "2FA disabled"
    });

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/auth/2fa/regenerate-backup - Regenerate backup codes
#[utoipa::path(
    post,
    path = "/api/auth/2fa/regenerate-backup",
    responses(
        (status = 200, description = "Backup codes regenerated"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Auth"
)]
pub async fn regenerate_backup_codes(
    State(twofa_service): State<Arc<TwoFAService>>,
    auth_user: AuthUser,
) -> Result<Response, TwoFAApiError> {
    if !twofa_service
        .is_2fa_enabled(&auth_user.user_id)
        .await
        .map_err(|_| TwoFAApiError::ServerError)?
    {
        return Err(TwoFAApiError::NotEnrolled);
    }

    // generate_backup_codes deletes existing codes first (see twofa.rs),
    // so this does invalidate the old set as the endpoint promises.
    let backup_codes = twofa_service
        .generate_backup_codes(&auth_user.user_id)
        .await
        .map_err(|_| TwoFAApiError::ServerError)?;

    let response = json!({
        "backup_codes": backup_codes,
        "message": "Backup codes regenerated"
    });

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Create 2FA routes
pub fn routes(twofa_service: Arc<TwoFAService>) -> Router {
    // Protected: act on the caller's own enrollment, so they need a
    // verified identity (AuthUser, populated by auth_middleware).
    let protected = Router::new()
        .route("/enroll/initiate", post(initiate_enrollment))
        .route("/enroll/confirm", post(confirm_enrollment))
        .route("/disable", post(disable_2fa))
        .route("/regenerate-backup", post(regenerate_backup_codes))
        .route("/api/auth/2fa/enroll/initiate", post(initiate_enrollment))
        .route("/api/auth/2fa/enroll/confirm", post(confirm_enrollment))
        .route("/api/auth/2fa/disable", post(disable_2fa))
        .route("/api/auth/2fa/regenerate-backup", post(regenerate_backup_codes))
        .layer(axum::middleware::from_fn(
            crate::auth_middleware::auth_middleware,
        ));

    // The former public (no-auth) group here -- /verify and /backup-code --
    // was two permanently-fake stub handlers that always returned
    // "access_token": "..." without checking anything, because login() had
    // no 2FA integration to hook them into. That integration now exists
    // (AuthService::login/complete_2fa_login, api/auth.rs's real
    // POST /api/auth/verify-2fa), so the stubs were removed rather than
    // left as a second, non-functional "verify 2FA" contract.
    protected.with_state(twofa_service)
}
