use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::asset_verification::{
    ListVerifiedAssetsQuery, ReportAssetRequest, VerifiedAssetResponse,
};
use crate::services::asset_verifier::AssetVerifier;

/// Create asset verification routes
pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/verify/:code/:issuer", get(verify_asset))
        .route("/:code/:issuer/verification", get(get_verification))
        .route("/verified", get(list_verified_assets))
        .route("/report", post(report_suspicious_asset))
        .with_state(Arc::new(pool))
}

/// Verify an asset and return its verification status
/// GET /api/assets/verify/:code/:issuer
async fn verify_asset(
    State(pool): State<Arc<SqlitePool>>,
    Path((code, issuer)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Input validation
    if code.is_empty() || code.len() > 12 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid asset code",
                "message": "Asset code must be 1-12 characters"
            })),
        ));
    }

    if !is_valid_stellar_public_key(&issuer) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid issuer",
                "message": "Issuer must be a valid Stellar public key"
            })),
        ));
    }

    let verifier = AssetVerifier::new((**pool).clone())
        .map_err(|e| {
            tracing::error!("Failed to create asset verifier: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error",
                    "message": "Failed to initialize verification service"
                })),
            )
        })?;

    match verifier.verify_asset(&code, &issuer).await {
        Ok(asset) => {
            let response: VerifiedAssetResponse = asset.into();
            Ok((StatusCode::OK, Json(response)))
        }
        Err(e) => {
            tracing::error!("Asset verification failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Verification failed",
                    "message": format!("Failed to verify asset: {}", e)
                })),
            ))
        }
    }
}

/// Get verification details for an asset
/// GET /api/assets/:code/:issuer/verification
async fn get_verification(
    State(pool): State<Arc<SqlitePool>>,
    Path((code, issuer)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Input validation
    if code.is_empty() || code.len() > 12 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid asset code",
                "message": "Asset code must be 1-12 characters"
            })),
        ));
    }

    if !is_valid_stellar_public_key(&issuer) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid issuer",
                "message": "Issuer must be a valid Stellar public key"
            })),
        ));
    }

    let verifier = AssetVerifier::new((**pool).clone())
        .map_err(|e| {
            tracing::error!("Failed to create asset verifier: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error",
                    "message": "Failed to initialize verification service"
                })),
            )
        })?;

    match verifier.get_verified_asset(&code, &issuer).await {
        Ok(Some(asset)) => {
            let response: VerifiedAssetResponse = asset.into();
            Ok((StatusCode::OK, Json(response)))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Not found",
                "message": "Asset verification not found. Use /verify endpoint to verify this asset."
            })),
        )),
        Err(e) => {
            tracing::error!("Failed to get verification: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error",
                    "message": format!("Failed to retrieve verification: {}", e)
                })),
            ))
        }
    }
}

/// List verified assets with optional filters
/// GET /api/assets/verified?status=verified&min_reputation=60&limit=50&offset=0
async fn list_verified_assets(
    State(pool): State<Arc<SqlitePool>>,
    Query(query): Query<ListVerifiedAssetsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Validate query parameters
    let limit = query.limit.unwrap_or(50).min(100).max(1);
    let offset = query.offset.unwrap_or(0).max(0);

    if let Some(min_rep) = query.min_reputation {
        if !(0.0..=100.0).contains(&min_rep) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid parameter",
                    "message": "min_reputation must be between 0 and 100"
                })),
            ));
        }
    }

    let verifier = AssetVerifier::new((**pool).clone())
        .map_err(|e| {
            tracing::error!("Failed to create asset verifier: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error",
                    "message": "Failed to initialize verification service"
                })),
            )
        })?;

    match verifier
        .list_verified_assets(query.status.as_ref(), query.min_reputation, limit, offset)
        .await
    {
        Ok(assets) => {
            let total = assets.len() as i64;
            let responses: Vec<VerifiedAssetResponse> =
                assets.into_iter().map(|a| a.into()).collect();

            Ok((
                StatusCode::OK,
                Json(json!({
                    "assets": responses,
                    "total": total,
                    "limit": limit,
                    "offset": offset
                })),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to list verified assets: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error",
                    "message": format!("Failed to list assets: {}", e)
                })),
            ))
        }
    }
}

/// Report a suspicious asset
/// POST /api/assets/report
async fn report_suspicious_asset(
    State(pool): State<Arc<SqlitePool>>,
    Json(request): Json<ReportAssetRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Input validation
    if request.asset_code.is_empty() || request.asset_code.len() > 12 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid asset code",
                "message": "Asset code must be 1-12 characters"
            })),
        ));
    }

    if !is_valid_stellar_public_key(&request.asset_issuer) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid issuer",
                "message": "Issuer must be a valid Stellar public key"
            })),
        ));
    }

    if request.description.is_empty() || request.description.len() > 1000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid description",
                "message": "Description must be 1-1000 characters"
            })),
        ));
    }

    if let Some(ref url) = request.evidence_url {
        if !is_valid_url(url) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid evidence URL",
                    "message": "Evidence URL must be a valid URL"
                })),
            ));
        }
    }

    if let Some(ref reporter) = request.reporter_account {
        if !is_valid_stellar_public_key(reporter) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid reporter account",
                    "message": "Reporter account must be a valid Stellar public key"
                })),
            ));
        }
    }

    let report_id = Uuid::new_v4().to_string();

    // Insert report into database
    let result = sqlx::query(
        r#"
        INSERT INTO asset_verification_reports (
            id, asset_code, asset_issuer, reporter_account,
            report_type, description, evidence_url, status
        ) VALUES (?, ?, ?, ?, ?, ?, ?, 'pending')
        "#,
    )
    .bind(&report_id)
    .bind(&request.asset_code)
    .bind(&request.asset_issuer)
    .bind(&request.reporter_account)
    .bind(request.report_type.as_str())
    .bind(&request.description)
    .bind(&request.evidence_url)
    .execute(&**pool)
    .await;

    match result {
        Ok(_) => {
            // Update suspicious reports count
            let _ = sqlx::query(
                r#"
                UPDATE verified_assets
                SET suspicious_reports_count = suspicious_reports_count + 1,
                    last_suspicious_report_at = CURRENT_TIMESTAMP,
                    updated_at = CURRENT_TIMESTAMP
                WHERE asset_code = ? AND asset_issuer = ?
                "#,
            )
            .bind(&request.asset_code)
            .bind(&request.asset_issuer)
            .execute(&**pool)
            .await;

            Ok((
                StatusCode::CREATED,
                Json(json!({
                    "id": report_id,
                    "status": "pending",
                    "message": "Report submitted successfully"
                })),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to create report: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Internal server error",
                    "message": "Failed to submit report"
                })),
            ))
        }
    }
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
        )); // Secret key
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
