//! GDPR compliance API handlers.
//!
//! Implements the Right to Access (data export), Right to be Forgotten (data
//! deletion), and consent management endpoints required by issue #1827.
//!
//! All endpoints require a valid JWT (`AuthUser` extractor). The underlying
//! tables (`user_consents`, `data_export_requests`, `data_deletion_requests`,
//! `data_processing_log`) were created by migration 015.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth_middleware::AuthUser;
use crate::error::{ApiError, ApiResult};

// ── Route registration ────────────────────────────────────────────────────────

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/summary", get(get_summary))
        .route("/consents", get(get_consents).put(update_consent))
        .route("/consents/batch", put(batch_update_consents))
        .route("/export", get(list_export_requests).post(create_export_request))
        .route("/export/{id}", get(get_export_request))
        .route("/export-types", get(get_export_types))
        .route("/deletion", get(list_deletion_requests).post(create_deletion_request))
        .route("/deletion/{id}", get(get_deletion_request))
        .route("/deletion/{id}/cancel", post(cancel_deletion_request))
        .route("/deletion/confirm", post(confirm_deletion))
        .with_state(pool)
}

// ── Response / request types ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ConsentResponse {
    pub consent_type: String,
    pub consent_given: bool,
    pub consent_version: String,
    pub granted_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConsentRequest {
    pub consent_type: String,
    pub consent_given: bool,
    pub consent_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchUpdateConsentsRequest {
    pub consents: Vec<UpdateConsentRequest>,
}

#[derive(Debug, Serialize)]
pub struct ExportRequestResponse {
    pub id: String,
    pub status: String,
    pub requested_at: String,
    pub expires_at: Option<String>,
    /// Constructed as `/api/gdpr/export/{id}/download` when status is completed.
    pub download_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateExportRequest {
    pub data_types: Vec<String>,
    pub export_format: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeletionRequestResponse {
    pub id: String,
    pub status: String,
    pub requested_at: String,
    pub scheduled_deletion_at: Option<String>,
    pub confirmation_required: bool,
    pub confirmation_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDeletionRequest {
    pub reason: Option<String>,
    pub delete_all_data: Option<bool>,
    pub data_types: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmDeletionRequest {
    pub confirmation_token: String,
}

#[derive(Debug, Serialize)]
pub struct DataTypeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
}

#[derive(Debug, Serialize)]
pub struct ExportableDataTypes {
    pub types: Vec<DataTypeInfo>,
}

#[derive(Debug, Serialize)]
pub struct GdprSummary {
    pub user_id: String,
    pub consents: Vec<ConsentResponse>,
    pub pending_export_requests: i64,
    pub pending_deletion_requests: i64,
    pub data_processing_activities_count: i64,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn export_download_url(id: &str) -> String {
    format!("/api/gdpr/export/{id}/download")
}

// ── Summary ───────────────────────────────────────────────────────────────────

/// GET /api/gdpr/summary
async fn get_summary(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    let uid = &auth_user.user_id;

    // Consents
    let consents = fetch_consents(&pool, uid).await?;

    // Pending export count
    let pending_exports: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM data_export_requests WHERE user_id = ? AND status IN ('pending','processing')"
    )
    .bind(uid)
    .fetch_one(&pool)
    .await
    .map_err(ApiError::from)?;

    // Pending deletion count
    let pending_deletions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM data_deletion_requests WHERE user_id = ? AND status IN ('pending','scheduled')"
    )
    .bind(uid)
    .fetch_one(&pool)
    .await
    .map_err(ApiError::from)?;

    // Processing activities count
    let activities: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM data_processing_log WHERE user_id = ?"
    )
    .bind(uid)
    .fetch_one(&pool)
    .await
    .map_err(ApiError::from)?;

    Ok(Json(GdprSummary {
        user_id: uid.clone(),
        consents,
        pending_export_requests: pending_exports,
        pending_deletion_requests: pending_deletions,
        data_processing_activities_count: activities,
    }))
}

// ── Consent management ────────────────────────────────────────────────────────

async fn fetch_consents(pool: &SqlitePool, user_id: &str) -> ApiResult<Vec<ConsentResponse>> {
    struct Row {
        consent_type: String,
        consent_given: bool,
        consent_version: String,
        granted_at: Option<String>,
        revoked_at: Option<String>,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"SELECT consent_type, consent_given as "consent_given: bool",
                  consent_version, granted_at, revoked_at
           FROM user_consents WHERE user_id = ?"#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(ApiError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| ConsentResponse {
            consent_type: r.consent_type,
            consent_given: r.consent_given,
            consent_version: r.consent_version,
            granted_at: r.granted_at,
            revoked_at: r.revoked_at,
        })
        .collect())
}

/// GET /api/gdpr/consents
async fn get_consents(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    let consents = fetch_consents(&pool, &auth_user.user_id).await?;
    Ok(Json(consents))
}

/// PUT /api/gdpr/consents  (single update)
async fn update_consent(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Json(req): Json<UpdateConsentRequest>,
) -> ApiResult<impl IntoResponse> {
    upsert_consent(&pool, &auth_user.user_id, &req).await?;
    let updated = fetch_consents(&pool, &auth_user.user_id)
        .await?
        .into_iter()
        .find(|c| c.consent_type == req.consent_type)
        .ok_or_else(|| ApiError::internal("GDPR_CONSENT_ERROR", "Consent not found after upsert"))?;
    Ok(Json(updated))
}

/// PUT /api/gdpr/consents/batch
async fn batch_update_consents(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Json(req): Json<BatchUpdateConsentsRequest>,
) -> ApiResult<impl IntoResponse> {
    for consent in &req.consents {
        upsert_consent(&pool, &auth_user.user_id, consent).await?;
    }
    let consents = fetch_consents(&pool, &auth_user.user_id).await?;
    Ok(Json(consents))
}

async fn upsert_consent(
    pool: &SqlitePool,
    user_id: &str,
    req: &UpdateConsentRequest,
) -> ApiResult<()> {
    let version = req.consent_version.as_deref().unwrap_or("1.0");
    let now = now_iso();
    let granted_at: Option<String> = if req.consent_given { Some(now.clone()) } else { None };
    let revoked_at: Option<String> = if !req.consent_given { Some(now.clone()) } else { None };

    // Check whether a row already exists for this (user_id, consent_type)
    let existing_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM user_consents WHERE user_id = ? AND consent_type = ? LIMIT 1",
    )
    .bind(user_id)
    .bind(&req.consent_type)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::from)?;

    if let Some(id) = existing_id {
        // Update existing row
        sqlx::query!(
            r#"UPDATE user_consents
               SET consent_given   = ?,
                   consent_version = ?,
                   granted_at      = ?,
                   revoked_at      = ?,
                   updated_at      = ?
               WHERE id = ?"#,
            req.consent_given,
            version,
            granted_at,
            revoked_at,
            now,
            id,
        )
        .execute(pool)
        .await
        .map_err(ApiError::from)?;
    } else {
        // Insert new row
        let id = Uuid::new_v4().to_string();
        sqlx::query!(
            r#"INSERT INTO user_consents
                   (id, user_id, consent_type, consent_given, consent_version,
                    granted_at, revoked_at, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            id,
            user_id,
            req.consent_type,
            req.consent_given,
            version,
            granted_at,
            revoked_at,
            now,
            now,
        )
        .execute(pool)
        .await
        .map_err(ApiError::from)?;
    }

    Ok(())
}

// ── Data export ───────────────────────────────────────────────────────────────

/// GET /api/gdpr/export
async fn list_export_requests(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    struct Row {
        id: String,
        status: String,
        requested_at: String,
        expires_at: Option<String>,
    }

    let rows = sqlx::query_as!(
        Row,
        "SELECT id, status, requested_at, expires_at FROM data_export_requests WHERE user_id = ? ORDER BY requested_at DESC",
        auth_user.user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(ApiError::from)?;

    let list: Vec<ExportRequestResponse> = rows
        .into_iter()
        .map(|r| {
            let download_url = if r.status == "completed" {
                Some(export_download_url(&r.id))
            } else {
                None
            };
            ExportRequestResponse {
                id: r.id,
                status: r.status,
                requested_at: r.requested_at,
                expires_at: r.expires_at,
                download_url,
            }
        })
        .collect();

    Ok(Json(list))
}

/// POST /api/gdpr/export
async fn create_export_request(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Json(req): Json<CreateExportRequest>,
) -> ApiResult<impl IntoResponse> {
    if req.data_types.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_EXPORT_REQUEST",
            "At least one data type must be selected",
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = now_iso();
    let expires_at = (Utc::now() + chrono::Duration::days(7)).to_rfc3339();
    let format = req.export_format.as_deref().unwrap_or("json");
    let types_json = serde_json::to_string(&req.data_types)
        .unwrap_or_else(|_| "[]".to_string());

    sqlx::query!(
        r#"INSERT INTO data_export_requests
               (id, user_id, status, requested_data_types, export_format, requested_at, expires_at)
           VALUES (?, ?, 'pending', ?, ?, ?, ?)"#,
        id,
        auth_user.user_id,
        types_json,
        format,
        now,
        expires_at,
    )
    .execute(&pool)
    .await
    .map_err(ApiError::from)?;

    Ok((
        StatusCode::CREATED,
        Json(ExportRequestResponse {
            id,
            status: "pending".to_string(),
            requested_at: now,
            expires_at: Some(expires_at),
            download_url: None,
        }),
    ))
}

/// GET /api/gdpr/export/:id
async fn get_export_request(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    struct Row {
        id: String,
        status: String,
        requested_at: String,
        expires_at: Option<String>,
    }

    let row = sqlx::query_as!(
        Row,
        "SELECT id, status, requested_at, expires_at FROM data_export_requests WHERE id = ? AND user_id = ?",
        id,
        auth_user.user_id,
    )
    .fetch_optional(&pool)
    .await
    .map_err(ApiError::from)?
    .ok_or_else(|| ApiError::not_found("EXPORT_REQUEST_NOT_FOUND", "Export request not found"))?;

    let download_url = if row.status == "completed" {
        Some(export_download_url(&row.id))
    } else {
        None
    };

    Ok(Json(ExportRequestResponse {
        id: row.id,
        status: row.status,
        requested_at: row.requested_at,
        expires_at: row.expires_at,
        download_url,
    }))
}

/// GET /api/gdpr/export-types
async fn get_export_types() -> ApiResult<impl IntoResponse> {
    Ok(Json(ExportableDataTypes {
        types: vec![
            DataTypeInfo {
                id: "profile".to_string(),
                name: "Profile Information".to_string(),
                description: "Your account profile data including name and email".to_string(),
                category: "Account".to_string(),
            },
            DataTypeInfo {
                id: "consents".to_string(),
                name: "Privacy Consents".to_string(),
                description: "History of your privacy consent choices".to_string(),
                category: "Privacy".to_string(),
            },
            DataTypeInfo {
                id: "api_keys".to_string(),
                name: "API Keys".to_string(),
                description: "Your generated API key metadata (secrets are not included)".to_string(),
                category: "Security".to_string(),
            },
            DataTypeInfo {
                id: "alert_rules".to_string(),
                name: "Alert Rules".to_string(),
                description: "Your configured threshold alert rules".to_string(),
                category: "Alerts".to_string(),
            },
            DataTypeInfo {
                id: "webhooks".to_string(),
                name: "Webhooks".to_string(),
                description: "Your registered webhook endpoints".to_string(),
                category: "Integrations".to_string(),
            },
            DataTypeInfo {
                id: "activity_log".to_string(),
                name: "Activity Log".to_string(),
                description: "Records of data processing activities on your account".to_string(),
                category: "Audit".to_string(),
            },
        ],
    }))
}

// ── Data deletion ─────────────────────────────────────────────────────────────

/// GET /api/gdpr/deletion
async fn list_deletion_requests(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    struct Row {
        id: String,
        status: String,
        requested_at: String,
        scheduled_deletion_at: Option<String>,
        confirmation_token: Option<String>,
    }

    let rows = sqlx::query_as!(
        Row,
        "SELECT id, status, requested_at, scheduled_deletion_at, confirmation_token FROM data_deletion_requests WHERE user_id = ? ORDER BY requested_at DESC",
        auth_user.user_id,
    )
    .fetch_all(&pool)
    .await
    .map_err(ApiError::from)?;

    let list: Vec<DeletionRequestResponse> = rows
        .into_iter()
        .map(|r| {
            let confirmation_required = r.confirmation_token.is_some();
            DeletionRequestResponse {
                id: r.id,
                status: r.status,
                requested_at: r.requested_at,
                scheduled_deletion_at: r.scheduled_deletion_at,
                confirmation_required,
                confirmation_token: r.confirmation_token,
            }
        })
        .collect();

    Ok(Json(list))
}

/// POST /api/gdpr/deletion
async fn create_deletion_request(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Json(req): Json<CreateDeletionRequest>,
) -> ApiResult<impl IntoResponse> {
    let id = Uuid::new_v4().to_string();
    let now = now_iso();
    // Schedule deletion 30 days out to allow a cancellation window
    let scheduled = (Utc::now() + chrono::Duration::days(30)).to_rfc3339();
    let confirmation_token = Uuid::new_v4().to_string();
    let delete_all = req.delete_all_data.unwrap_or(true);
    let data_types_json: Option<String> = req
        .data_types
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default());

    sqlx::query!(
        r#"INSERT INTO data_deletion_requests
               (id, user_id, status, reason, delete_all_data, data_types_to_delete,
                requested_at, scheduled_deletion_at, confirmation_token)
           VALUES (?, ?, 'pending', ?, ?, ?, ?, ?, ?)"#,
        id,
        auth_user.user_id,
        req.reason,
        delete_all,
        data_types_json,
        now,
        scheduled,
        confirmation_token,
    )
    .execute(&pool)
    .await
    .map_err(ApiError::from)?;

    Ok((
        StatusCode::CREATED,
        Json(DeletionRequestResponse {
            id,
            status: "pending".to_string(),
            requested_at: now,
            scheduled_deletion_at: Some(scheduled),
            confirmation_required: true,
            confirmation_token: Some(confirmation_token),
        }),
    ))
}

/// GET /api/gdpr/deletion/:id
async fn get_deletion_request(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    struct Row {
        id: String,
        status: String,
        requested_at: String,
        scheduled_deletion_at: Option<String>,
        confirmation_token: Option<String>,
    }

    let row = sqlx::query_as!(
        Row,
        "SELECT id, status, requested_at, scheduled_deletion_at, confirmation_token FROM data_deletion_requests WHERE id = ? AND user_id = ?",
        id,
        auth_user.user_id,
    )
    .fetch_optional(&pool)
    .await
    .map_err(ApiError::from)?
    .ok_or_else(|| ApiError::not_found("DELETION_REQUEST_NOT_FOUND", "Deletion request not found"))?;

    let confirmation_required = row.confirmation_token.is_some();
    Ok(Json(DeletionRequestResponse {
        id: row.id,
        status: row.status,
        requested_at: row.requested_at,
        scheduled_deletion_at: row.scheduled_deletion_at,
        confirmation_required,
        confirmation_token: row.confirmation_token,
    }))
}

/// POST /api/gdpr/deletion/:id/cancel
async fn cancel_deletion_request(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let now = now_iso();
    let result = sqlx::query!(
        "UPDATE data_deletion_requests SET status = 'cancelled', cancelled_at = ? WHERE id = ? AND user_id = ? AND status IN ('pending', 'scheduled')",
        now,
        id,
        auth_user.user_id,
    )
    .execute(&pool)
    .await
    .map_err(ApiError::from)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found(
            "DELETION_REQUEST_NOT_FOUND",
            "Deletion request not found or cannot be cancelled",
        ));
    }

    struct Row {
        id: String,
        status: String,
        requested_at: String,
        scheduled_deletion_at: Option<String>,
        confirmation_token: Option<String>,
    }

    let row = sqlx::query_as!(
        Row,
        "SELECT id, status, requested_at, scheduled_deletion_at, confirmation_token FROM data_deletion_requests WHERE id = ? AND user_id = ?",
        id,
        auth_user.user_id,
    )
    .fetch_optional(&pool)
    .await
    .map_err(ApiError::from)?
    .ok_or_else(|| ApiError::internal("GDPR_ERROR", "Failed to retrieve updated request"))?;

    Ok(Json(DeletionRequestResponse {
        id: row.id,
        status: row.status,
        requested_at: row.requested_at,
        scheduled_deletion_at: row.scheduled_deletion_at,
        confirmation_required: false,
        confirmation_token: None,
    }))
}

/// POST /api/gdpr/deletion/confirm
async fn confirm_deletion(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Json(req): Json<ConfirmDeletionRequest>,
) -> ApiResult<impl IntoResponse> {
    // Fetch the request first to get its id
    let existing_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM data_deletion_requests WHERE confirmation_token = ? AND user_id = ? AND status = 'pending'"
    )
    .bind(&req.confirmation_token)
    .bind(&auth_user.user_id)
    .fetch_optional(&pool)
    .await
    .map_err(ApiError::from)?;

    let request_id = existing_id.ok_or_else(|| {
        ApiError::bad_request(
            "INVALID_CONFIRMATION_TOKEN",
            "Invalid or already-used confirmation token",
        )
    })?;

    sqlx::query!(
        "UPDATE data_deletion_requests SET status = 'scheduled' WHERE id = ?",
        request_id,
    )
    .execute(&pool)
    .await
    .map_err(ApiError::from)?;

    struct Row {
        id: String,
        status: String,
        requested_at: String,
        scheduled_deletion_at: Option<String>,
        confirmation_token: Option<String>,
    }

    let row = sqlx::query_as!(
        Row,
        "SELECT id, status, requested_at, scheduled_deletion_at, confirmation_token FROM data_deletion_requests WHERE id = ?",
        request_id,
    )
    .fetch_optional(&pool)
    .await
    .map_err(ApiError::from)?
    .ok_or_else(|| ApiError::internal("GDPR_ERROR", "Failed to retrieve updated request"))?;

    Ok(Json(DeletionRequestResponse {
        id: row.id,
        status: row.status,
        requested_at: row.requested_at,
        scheduled_deletion_at: row.scheduled_deletion_at,
        confirmation_required: false,
        confirmation_token: row.confirmation_token,
    }))
}
