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

use crate::twofa::TwoFAService;

#[derive(Debug, Deserialize)]
pub struct EnrollRequest {
    pub totp_secret: String,
    pub verification_code: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyCodeRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct BackupCodeRequest {
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
    State(_twofa_service): State<Arc<TwoFAService>>,
) -> Result<Response, TwoFAApiError> {
    // TODO: Extract user_id from JWT claims via auth middleware
    // For now, return placeholder
    let response = json!({
        "otpauth_uri": "otpauth://totp/stellar-insights:user?secret=JBSWY3DPEBLW64TMMQ======&issuer=stellar-insights&digits=6",
        "secret": "JBSWY3DPEBLW64TMMQ"
    });

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
    State(_twofa_service): State<Arc<TwoFAService>>,
    Json(_request): Json<EnrollRequest>,
) -> Result<Response, TwoFAApiError> {
    // TODO: Extract user_id from JWT claims
    // Verify the provided TOTP code against the secret
    // If valid, activate 2FA and generate backup codes
    // Return backup codes to user

    let backup_codes = vec![
        "123456".to_string(),
        "234567".to_string(),
        "345678".to_string(),
    ];

    let response = EnrollResponse {
        backup_codes,
        message: "2FA enrollment confirmed. Save your backup codes in a secure location.".to_string(),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/auth/2fa/verify - Verify TOTP code during login
#[utoipa::path(
    post,
    path = "/api/auth/2fa/verify",
    request_body = VerifyCodeRequest,
    responses(
        (status = 200, description = "2FA verification successful"),
        (status = 401, description = "Invalid code")
    ),
    tag = "Auth"
)]
pub async fn verify_totp(
    State(_twofa_service): State<Arc<TwoFAService>>,
    Json(_request): Json<VerifyCodeRequest>,
) -> Result<Response, TwoFAApiError> {
    // TODO: Extract session_id from context (pending 2FA state)
    // Verify the TOTP code
    // If valid, upgrade session to fully authenticated
    // Return new access token

    let response = json!({
        "message": "2FA verification successful",
        "access_token": "..."
    });

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/auth/2fa/backup-code - Verify backup code during login
#[utoipa::path(
    post,
    path = "/api/auth/2fa/backup-code",
    request_body = BackupCodeRequest,
    responses(
        (status = 200, description = "Backup code verification successful"),
        (status = 401, description = "Invalid or expired backup code")
    ),
    tag = "Auth"
)]
pub async fn verify_backup_code(
    State(_twofa_service): State<Arc<TwoFAService>>,
    Json(_request): Json<BackupCodeRequest>,
) -> Result<Response, TwoFAApiError> {
    // TODO: Extract session_id from context
    // Verify the backup code (one-time use)
    // If valid, mark as used and upgrade session
    // Return new access token

    let response = json!({
        "message": "Backup code verified. Consider regenerating your backup codes.",
        "access_token": "..."
    });

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/auth/2fa/disable - Disable 2FA
#[utoipa::path(
    post,
    path = "/api/auth/2fa/disable",
    responses(
        (status = 200, description = "2FA disabled"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Auth"
)]
pub async fn disable_2fa(
    State(_twofa_service): State<Arc<TwoFAService>>,
) -> Result<Response, TwoFAApiError> {
    // TODO: Extract user_id from JWT claims
    // Require current TOTP code or backup code as confirmation
    // Disable 2FA
    // Log the action in audit log

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
    State(_twofa_service): State<Arc<TwoFAService>>,
) -> Result<Response, TwoFAApiError> {
    // TODO: Extract user_id from JWT claims
    // Generate new backup codes (invalidates old ones)
    // Return new backup codes

    let backup_codes = vec![
        "111111".to_string(),
        "222222".to_string(),
        "333333".to_string(),
    ];

    let response = json!({
        "backup_codes": backup_codes,
        "message": "Backup codes regenerated"
    });

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Create 2FA routes
pub fn routes(twofa_service: Arc<TwoFAService>) -> Router {
    Router::new()
        .route("/api/auth/2fa/enroll/initiate", post(initiate_enrollment))
        .route("/api/auth/2fa/enroll/confirm", post(confirm_enrollment))
        .route("/api/auth/2fa/verify", post(verify_totp))
        .route("/api/auth/2fa/backup-code", post(verify_backup_code))
        .route("/api/auth/2fa/disable", post(disable_2fa))
        .route("/api/auth/2fa/regenerate-backup", post(regenerate_backup_codes))
        .with_state(twofa_service)
}
