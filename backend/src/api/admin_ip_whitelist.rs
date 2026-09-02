use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::admin_ip_whitelist::IpWhitelistService;
use crate::auth_middleware::AdminUser;

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct AddWhitelistRequest {
    pub ip_or_cidr: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WhitelistEntryResponse {
    pub id: String,
    pub ip_or_cidr: String,
    pub description: Option<String>,
    pub added_by_user_id: Option<String>,
    pub added_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub enum IpWhitelistApiError {
    InvalidIp,
    NotFound,
    AlreadyExists,
    FailedClosed,
    Unauthorized,
    ServerError,
}

impl IntoResponse for IpWhitelistApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidIp => (
                StatusCode::BAD_REQUEST,
                "INVALID_IP",
                "Invalid IP address or CIDR notation",
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Whitelist entry not found",
            ),
            Self::AlreadyExists => (
                StatusCode::CONFLICT,
                "ALREADY_EXISTS",
                "IP or CIDR already in whitelist",
            ),
            Self::FailedClosed => (
                StatusCode::FORBIDDEN,
                "ADMIN_ACCESS_DENIED",
                "Admin access denied: IP not whitelisted",
            ),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Unauthorized: admin authentication required",
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

/// GET /admin/ip-whitelist - List all whitelisted IPs
#[utoipa::path(
    get,
    path = "/admin/ip-whitelist",
    responses(
        (status = 200, description = "Whitelist entries returned"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Access denied (IP not whitelisted)")
    ),
    tag = "Admin"
)]
pub async fn list_whitelist(
    State(service): State<Arc<IpWhitelistService>>,
    _admin: AdminUser,
) -> Result<Response, IpWhitelistApiError> {
    let entries = service
        .get_all_entries()
        .await
        .map_err(|_| IpWhitelistApiError::ServerError)?;

    let response = json!({
        "entries": entries
            .into_iter()
            .map(|e| json!({
                "id": e.id,
                "ip_or_cidr": e.ip_or_cidr,
                "description": e.description,
                "added_by_user_id": e.added_by_user_id,
                "added_at": e.added_at,
            }))
            .collect::<Vec<_>>()
    });

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /admin/ip-whitelist - Add IP to whitelist
#[utoipa::path(
    post,
    path = "/admin/ip-whitelist",
    request_body = AddWhitelistRequest,
    responses(
        (status = 201, description = "Entry added to whitelist"),
        (status = 400, description = "Invalid IP or CIDR"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Access denied (IP not whitelisted)")
    ),
    tag = "Admin"
)]
pub async fn add_to_whitelist(
    State(service): State<Arc<IpWhitelistService>>,
    admin: AdminUser,
    Json(request): Json<AddWhitelistRequest>,
) -> Result<Response, IpWhitelistApiError> {
    // This previously didn't call the service at all -- it returned a
    // "success" response built straight from the request body, without
    // ever writing to admin_ip_whitelist. IpWhitelistService::add_to_whitelist
    // (admin_ip_whitelist.rs) already existed, validated, and worked; it
    // just wasn't being called.
    let entry = service
        .add_to_whitelist(
            &request.ip_or_cidr,
            request.description,
            Some(&admin.0.user_id),
        )
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Invalid IP") {
                IpWhitelistApiError::InvalidIp
            } else if msg.contains("UNIQUE constraint") {
                IpWhitelistApiError::AlreadyExists
            } else {
                IpWhitelistApiError::ServerError
            }
        })?;

    let response = json!({
        "message": "IP added to whitelist",
        "entry": {
            "id": entry.id,
            "ip_or_cidr": entry.ip_or_cidr,
            "description": entry.description,
            "added_by_user_id": entry.added_by_user_id,
            "added_at": entry.added_at,
        }
    });

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// DELETE /admin/ip-whitelist/:ip_or_cidr - Remove IP from whitelist
#[utoipa::path(
    delete,
    path = "/admin/ip-whitelist/:ip_or_cidr",
    responses(
        (status = 204, description = "Entry removed from whitelist"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Access denied (IP not whitelisted)"),
        (status = 404, description = "Entry not found")
    ),
    tag = "Admin"
)]
pub async fn remove_from_whitelist(
    State(service): State<Arc<IpWhitelistService>>,
    _admin: AdminUser,
    axum::extract::Path(ip_or_cidr): axum::extract::Path<String>,
) -> Result<StatusCode, IpWhitelistApiError> {
    // This previously discarded ip_or_cidr and always returned 204 without
    // touching the table at all -- IpWhitelistService::remove_from_whitelist
    // already existed and worked, it just wasn't called.
    //
    // Lockout avoidance: refuse to remove the last remaining entry. This is
    // a coarse version of the TODO's intent ("keep at least one entry or
    // current IP") -- it doesn't verify the *caller's* IP specifically
    // stays whitelisted (this handler has no access to the connecting IP
    // today), just that the whitelist can't be emptied outright.
    let count = service
        .count_entries()
        .await
        .map_err(|_| IpWhitelistApiError::ServerError)?;
    if count <= 1 {
        return Err(IpWhitelistApiError::FailedClosed);
    }

    service
        .remove_from_whitelist(&ip_or_cidr)
        .await
        .map_err(|_| IpWhitelistApiError::ServerError)?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /admin/ip-whitelist/check - Check if an IP is whitelisted
#[utoipa::path(
    post,
    path = "/admin/ip-whitelist/check",
    request_body(content = serde_json::Value, description = "IP address to check, e.g. {\"ip\": \"192.168.1.1\"}"),
    responses(
        (status = 200, description = "Check result returned"),
        (status = 400, description = "Invalid IP format"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Admin"
)]
pub async fn check_whitelist(
    State(service): State<Arc<IpWhitelistService>>,
    Json(request): Json<serde_json::Value>,
) -> Result<Response, IpWhitelistApiError> {
    let ip = request
        .get("ip")
        .and_then(|v| v.as_str())
        .ok_or(IpWhitelistApiError::InvalidIp)?;

    let is_whitelisted = service
        .is_whitelisted(ip)
        .await
        .map_err(|_| IpWhitelistApiError::InvalidIp)?;

    let response = json!({
        "ip": ip,
        "is_whitelisted": is_whitelisted,
        "access": if is_whitelisted {
            "allowed"
        } else {
            "denied"
        }
    });

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Create admin IP whitelist routes
pub fn routes(service: Arc<IpWhitelistService>) -> Router {
    Router::new()
        .route("/", get(list_whitelist).post(add_to_whitelist))
        .route("/:ip_or_cidr", delete(remove_from_whitelist))
        .route("/check", post(check_whitelist))
        .route("/admin/ip-whitelist", get(list_whitelist).post(add_to_whitelist))
        .route("/admin/ip-whitelist/:ip_or_cidr", delete(remove_from_whitelist))
        .route("/admin/ip-whitelist/check", post(check_whitelist))
        .with_state(service)
}
