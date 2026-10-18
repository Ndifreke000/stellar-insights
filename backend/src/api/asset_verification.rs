use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::asset_verification::{
    ListVerifiedAssetsQuery, ReportAssetRequest, VerifiedAssetResponse,
};
use crate::services::asset_verifier::AssetVerifier;

/// Create asset verification routes
pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/verify/{code}/{issuer}", get(verify_asset))
        .route("/{code}/{issuer}/verification", get(get_verification))
        .route("/verified", get(list_verified_assets))
        .route("/report", post(report_suspicious_asset))
        .with_state(Arc::new(pool))
}

/// GET /api/assets/verify/:code/:issuer - Verify an asset and return its verification status
#[utoipa::path(
    get,
    path = "/api/assets/verify/{code}/{issuer}",
    params(
        ("code" = String, Path, description = "Asset code"),
        ("issuer" = String, Path, description = "Asset issuer public key")
    ),
    responses(
        (status = 200, description = "Asset verification result"),
        (status = 400, description = "Invalid asset code or issuer"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Asset Verification"
)]
async fn verify_asset(
    State(pool): State<Arc<SqlitePool>>,
    Path((code, issuer)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    if code.is_empty() || code.len() > 12 {
        return Err(ApiError::bad_request(
            "INVALID_ASSET_CODE",
            "Asset code must be 1-12 characters",
        ));
    }

    if !is_valid_stellar_public_key(&issuer) {
        return Err(ApiError::bad_request(
            "INVALID_ISSUER",
            "Issuer must be a valid Stellar public key",
        ));
    }

    let verifier = AssetVerifier::new(pool.as_ref().clone()).map_err(|e| {
        tracing::error!("Failed to create asset verifier: {}", e);
        ApiError::internal("SERVICE_INIT_ERROR", "Failed to initialize verification service")
    })?;

    let result = verifier.verify_asset(&code, &issuer).await.map_err(|e| {
        tracing::error!("Asset verification failed: {}", e);
        ApiError::internal("VERIFICATION_FAILED", "Failed to verify asset")
    })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "asset_code": code,
            "asset_issuer": issuer,
            "verified": result.stellar_expert_verified
                || result.stellar_toml_verified
                || result.anchor_registry_verified,
            "trust_indicators": {
                "stellar_expert_verified": result.stellar_expert_verified,
                "stellar_toml_verified": result.stellar_toml_verified,
                "anchor_registry_verified": result.anchor_registry_verified
            },
            "metrics": {
                "trustline_count": result.trustline_count,
                "transaction_count": result.transaction_count,
                "total_volume_usd": result.total_volume_usd
            }
        })),
    ))
}

/// GET /api/assets/:code/:issuer/verification - Get verification details for an asset
#[utoipa::path(
    get,
    path = "/api/assets/{code}/{issuer}/verification",
    params(
        ("code" = String, Path, description = "Asset code"),
        ("issuer" = String, Path, description = "Asset issuer public key")
    ),
    responses(
        (status = 200, description = "Asset verification details"),
        (status = 400, description = "Invalid asset code or issuer"),
        (status = 404, description = "Asset verification not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Asset Verification"
)]
async fn get_verification(
    State(pool): State<Arc<SqlitePool>>,
    Path((code, issuer)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    if code.is_empty() || code.len() > 12 {
        return Err(ApiError::bad_request(
            "INVALID_ASSET_CODE",
            "Asset code must be 1-12 characters",
        ));
    }

    if !is_valid_stellar_public_key(&issuer) {
        return Err(ApiError::bad_request(
            "INVALID_ISSUER",
            "Issuer must be a valid Stellar public key",
        ));
    }

    let verifier = AssetVerifier::new(pool.as_ref().clone()).map_err(|e| {
        tracing::error!("Failed to create asset verifier: {}", e);
        ApiError::internal("SERVICE_INIT_ERROR", "Failed to initialize verification service")
    })?;

    match verifier.get_verified_asset(&code, &issuer).await {
        Ok(Some(asset)) => {
            let response: VerifiedAssetResponse = asset.into();
            Ok((StatusCode::OK, Json(response)).into_response())
        }
        Ok(None) => Err(ApiError::not_found(
            "ASSET_VERIFICATION_NOT_FOUND",
            "Asset verification not found. Use /verify endpoint to verify this asset.",
        )),
        Err(e) => {
            tracing::error!("Failed to get verification: {}", e);
            Err(ApiError::internal(
                "VERIFICATION_LOOKUP_FAILED",
                "Failed to retrieve verification",
            ))
        }
    }
}

/// GET /api/assets/verified - List verified assets with optional filters
#[utoipa::path(
    get,
    path = "/api/assets/verified",
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("min_reputation" = Option<f64>, Query, description = "Minimum reputation score (0-100)"),
        ("limit" = Option<i64>, Query, description = "Maximum results (1-100, default 50)"),
        ("offset" = Option<i64>, Query, description = "Results offset (default 0)")
    ),
    responses(
        (status = 200, description = "List of verified assets"),
        (status = 400, description = "Invalid parameters"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Asset Verification"
)]
async fn list_verified_assets(
    State(pool): State<Arc<SqlitePool>>,
    Query(query): Query<ListVerifiedAssetsQuery>,
) -> ApiResult<impl IntoResponse> {
    let limit = query.limit.unwrap_or(50).min(100).max(1);
    let offset = query.offset.unwrap_or(0).max(0);

    if let Some(min_rep) = query.min_reputation {
        if !(0.0..=100.0).contains(&min_rep) {
            return Err(ApiError::bad_request(
                "INVALID_MIN_REPUTATION",
                "min_reputation must be between 0 and 100",
            ));
        }
    }

    let verifier = AssetVerifier::new(pool.as_ref().clone()).map_err(|e| {
        tracing::error!("Failed to create asset verifier: {}", e);
        ApiError::internal("SERVICE_INIT_ERROR", "Failed to initialize verification service")
    })?;

    let assets = verifier
        .list_verified_assets(query.status.clone(), query.min_reputation, limit, offset)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list verified assets: {}", e);
            ApiError::internal("LIST_ASSETS_FAILED", "Failed to list assets")
        })?;

    let total = assets.len() as i64;
    let responses: Vec<VerifiedAssetResponse> =
        assets.into_iter().map(std::convert::Into::into).collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "assets": responses,
            "total": total,
            "limit": limit,
            "offset": offset
        })),
    ))
}

/// POST /api/assets/report - Report a suspicious asset
#[utoipa::path(
    post,
    path = "/api/assets/report",
    request_body = ReportAssetRequest,
    responses(
        (status = 201, description = "Report submitted successfully"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Asset Verification"
)]
async fn report_suspicious_asset(
    State(pool): State<Arc<SqlitePool>>,
    Json(request): Json<ReportAssetRequest>,
) -> ApiResult<impl IntoResponse> {
    if request.asset_code.is_empty() || request.asset_code.len() > 12 {
        return Err(ApiError::bad_request(
            "INVALID_ASSET_CODE",
            "Asset code must be 1-12 characters",
        ));
    }

    if !is_valid_stellar_public_key(&request.asset_issuer) {
        return Err(ApiError::bad_request(
            "INVALID_ISSUER",
            "Issuer must be a valid Stellar public key",
        ));
    }

    if request.description.is_empty() || request.description.len() > 1000 {
        return Err(ApiError::bad_request(
            "INVALID_DESCRIPTION",
            "Description must be 1-1000 characters",
        ));
    }

    if let Some(ref url) = request.evidence_url {
        if !is_valid_url(url) {
            return Err(ApiError::bad_request(
                "INVALID_EVIDENCE_URL",
                "Evidence URL must be a valid URL",
            ));
        }
    }

    if let Some(ref reporter) = request.reporter_account {
        if !is_valid_stellar_public_key(reporter) {
            return Err(ApiError::bad_request(
                "INVALID_REPORTER_ACCOUNT",
                "Reporter account must be a valid Stellar public key",
            ));
        }
    }

    let report_id = Uuid::new_v4().to_string();

    sqlx::query(
        r"
        INSERT INTO asset_verification_reports (
            id, asset_code, asset_issuer, reporter_account,
            report_type, description, evidence_url, status
        ) VALUES (?, ?, ?, ?, ?, ?, ?, 'pending')
        ",
    )
    .bind(&report_id)
    .bind(&request.asset_code)
    .bind(&request.asset_issuer)
    .bind(&request.reporter_account)
    .bind(request.report_type.as_str())
    .bind(&request.description)
    .bind(&request.evidence_url)
    .execute(&*pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create report: {}", e);
        ApiError::internal("REPORT_CREATION_FAILED", "Failed to submit report")
    })?;

    // Best-effort: increment suspicious reports count on the asset record.
    let _ = sqlx::query(
        r"
        UPDATE verified_assets
        SET suspicious_reports_count = suspicious_reports_count + 1,
            last_suspicious_report_at = CURRENT_TIMESTAMP,
            updated_at = CURRENT_TIMESTAMP
        WHERE asset_code = ? AND asset_issuer = ?
        ",
    )
    .bind(&request.asset_code)
    .bind(&request.asset_issuer)
    .execute(&*pool)
    .await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": report_id,
            "status": "pending",
            "message": "Report submitted successfully"
        })),
    ))
}

/// Validate Stellar public key format
fn is_valid_stellar_public_key(key: &str) -> bool {
    key.len() == 56 && key.starts_with('G')
}

/// Validate URL format
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_stellar_public_key() {
        assert!(is_valid_stellar_public_key(
            "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        ));
        assert!(!is_valid_stellar_public_key("INVALID"));
        assert!(!is_valid_stellar_public_key(""));
        assert!(!is_valid_stellar_public_key(
            "SA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"
        ));
    }

    #[test]
    fn test_is_valid_url() {
        assert!(is_valid_url("https://example.com"));
        assert!(is_valid_url("http://example.com"));
        assert!(!is_valid_url("ftp://example.com"));
        assert!(!is_valid_url("example.com"));
        assert!(!is_valid_url(""));
    }
}
