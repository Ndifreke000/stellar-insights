// GDPR API Handlers - HTTP endpoints for GDPR compliance

use crate::error::ApiError;
use crate::gdpr::models::*;
use crate::gdpr::service::GdprService;
use actix_web::{web, HttpRequest, Responder};
use serde_json::json;

/// Get all consents for the authenticated user
pub async fn get_consents(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
) -> Result<impl Responder, AppError> {
    // Extract user ID from request (assumes auth middleware sets this)
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let consents = gdpr_service.get_user_consents(user_id).await?;
    Ok(web::Json(consents))
}

/// Update a single consent
pub async fn update_consent(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
    body: web::Json<UpdateConsentRequest>,
) -> Result<impl Responder, AppError> {
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let ip_address = req
        .connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string());

    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = gdpr_service
        .update_consent(user_id, body.into_inner(), ip_address, user_agent)
        .await?;

    Ok(web::Json(response))
}

/// Batch update multiple consents
pub async fn batch_update_consents(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
    body: web::Json<BatchUpdateConsentsRequest>,
) -> Result<impl Responder, AppError> {
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let ip_address = req
        .connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string());

    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let responses = gdpr_service
        .batch_update_consents(user_id, body.consents, ip_address, user_agent)
        .await?;

    Ok(web::Json(responses))
}

/// Create a new data export request
pub async fn create_export_request(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
    body: web::Json<CreateExportRequest>,
) -> Result<impl Responder, AppError> {
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let response = gdpr_service
        .create_export_request(user_id, body.into_inner())
        .await?;

    Ok(web::Json(response))
}

/// Get export request status
pub async fn get_export_request(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
    path: web::Path<String>,
) -> Result<impl Responder, AppError> {
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let request_id = path.into_inner();
    let response = gdpr_service
        .get_export_request(user_id, &request_id)
        .await?;

    Ok(web::Json(response))
}

/// Get all export requests for the authenticated user
pub async fn get_export_requests(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
) -> Result<impl Responder, AppError> {
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let requests = gdpr_service.get_user_export_requests(user_id).await?;
    Ok(web::Json(requests))
}

/// Create a new data deletion request
pub async fn create_deletion_request(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
    body: web::Json<CreateDeletionRequest>,
) -> Result<impl Responder, AppError> {
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let response = gdpr_service
        .create_deletion_request(user_id, body.into_inner())
        .await?;

    Ok(web::Json(response))
}

/// Confirm a deletion request (via email link)
pub async fn confirm_deletion(
    gdpr_service: web::Data<GdprService>,
    body: web::Json<ConfirmDeletionRequest>,
) -> Result<impl Responder, AppError> {
    let response = gdpr_service
        .confirm_deletion(&body.confirmation_token)
        .await?;

    Ok(web::Json(response))
}

/// Cancel a deletion request
pub async fn cancel_deletion(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
    path: web::Path<String>,
) -> Result<impl Responder, AppError> {
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let request_id = path.into_inner();
    let response = gdpr_service.cancel_deletion(user_id, &request_id).await?;

    Ok(web::Json(response))
}

/// Get deletion request status
pub async fn get_deletion_request(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
    path: web::Path<String>,
) -> Result<impl Responder, AppError> {
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let request_id = path.into_inner();
    let response = gdpr_service
        .get_deletion_request(user_id, &request_id)
        .await?;

    Ok(web::Json(response))
}

/// Get all deletion requests for the authenticated user
pub async fn get_deletion_requests(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
) -> Result<impl Responder, AppError> {
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let requests = gdpr_service.get_user_deletion_requests(user_id).await?;
    Ok(web::Json(requests))
}

/// Get GDPR summary for the authenticated user
pub async fn get_gdpr_summary(
    req: HttpRequest,
    gdpr_service: web::Data<GdprService>,
) -> Result<impl Responder, AppError> {
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("demo-user-id-123");

    let summary = gdpr_service.get_gdpr_summary(user_id).await?;
    Ok(web::Json(summary))
}

/// Get available exportable data types
pub async fn get_exportable_types() -> Result<impl Responder, AppError> {
    let types = GdprService::get_exportable_data_types();
    Ok(web::Json(types))
}
