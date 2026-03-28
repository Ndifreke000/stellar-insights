use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::database::Database;
use crate::models::api_key::CreateApiKeyRequest;

fn extract_wallet_address(headers: &HeaderMap) -> Result<String, ApiKeyError> {
    headers
        .get("X-Wallet-Address")
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiKeyError::Unauthorized("Missing X-Wallet-Address header".to_string()))
}

/// POST /api/api-keys - Create a new API key
#[utoipa::path(
    post,
    path = "/api/api-keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized - missing X-Wallet-Address header"),
        (status = 500, description = "Internal server error")
    ),
    tag = "API Keys"
)]
pub async fn create_api_key(
    State(db): State<Arc<Database>>,
    headers: HeaderMap,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Response, ApiKeyError> {
    let wallet_address = extract_wallet_address(&headers)?;

    if req.name.trim().is_empty() {
        return Err(ApiKeyError::BadRequest("Key name is required".to_string()));
    }

    let response = db
        .create_api_key(&wallet_address, req)
        .await
        .map_err(|e| ApiKeyError::ServerError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(json!(response))).into_response())
}

/// GET /api/api-keys - List all API keys for the authenticated user
#[utoipa::path(
    get,
    path = "/api/api-keys",
    responses(
        (status = 200, description = "List of API keys"),
        (status = 401, description = "Unauthorized - missing X-Wallet-Address header"),
        (status = 500, description = "Internal server error")
    ),
    tag = "API Keys"
)]
pub async fn list_api_keys(
    State(db): State<Arc<Database>>,
    headers: HeaderMap,
) -> Result<Response, ApiKeyError> {
    let wallet_address = extract_wallet_address(&headers)?;

    let keys = db
        .list_api_keys(&wallet_address)
        .await
        .map_err(|e| ApiKeyError::ServerError(e.to_string()))?;

    Ok((StatusCode::OK, Json(json!({ "keys": keys }))).into_response())
}

/// GET /api/api-keys/{id} - Get a specific API key by ID
#[utoipa::path(
    get,
    path = "/api/api-keys/{id}",
    params(
        ("id" = String, Path, description = "API key ID")
    ),
    responses(
        (status = 200, description = "API key details"),
        (status = 401, description = "Unauthorized - missing X-Wallet-Address header"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "API Keys"
)]
pub async fn get_api_key(
    State(db): State<Arc<Database>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ApiKeyError> {
    let wallet_address = extract_wallet_address(&headers)?;

    let key = db
        .get_api_key_by_id(&id, &wallet_address)
        .await
        .map_err(|e| ApiKeyError::ServerError(e.to_string()))?;

    match key {
        Some(k) => Ok((StatusCode::OK, Json(json!(k))).into_response()),
        None => Err(ApiKeyError::NotFound("API key not found".to_string())),
    }
}

/// POST /api/api-keys/{id}/rotate - Rotate an API key
#[utoipa::path(
    post,
    path = "/api/api-keys/{id}/rotate",
    params(
        ("id" = String, Path, description = "API key ID")
    ),
    responses(
        (status = 200, description = "API key rotated"),
        (status = 401, description = "Unauthorized - missing X-Wallet-Address header"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "API Keys"
)]
pub async fn rotate_api_key(
    State(db): State<Arc<Database>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ApiKeyError> {
    let wallet_address = extract_wallet_address(&headers)?;

    let response = db
        .rotate_api_key(&id, &wallet_address)
        .await
        .map_err(|e| ApiKeyError::ServerError(e.to_string()))?;

    match response {
        Some(r) => Ok((StatusCode::OK, Json(json!(r))).into_response()),
        None => Err(ApiKeyError::NotFound(
            "API key not found or already revoked".to_string(),
        )),
    }
}

/// DELETE /api/api-keys/{id} - Revoke an API key
#[utoipa::path(
    delete,
    path = "/api/api-keys/{id}",
    params(
        ("id" = String, Path, description = "API key ID")
    ),
    responses(
        (status = 200, description = "API key revoked"),
        (status = 401, description = "Unauthorized - missing X-Wallet-Address header"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "API Keys"
)]
pub async fn revoke_api_key(
    State(db): State<Arc<Database>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ApiKeyError> {
    let wallet_address = extract_wallet_address(&headers)?;

    let revoked = db
        .revoke_api_key(&id, &wallet_address)
        .await
        .map_err(|e| ApiKeyError::ServerError(e.to_string()))?;

    if revoked {
        Ok((
            StatusCode::OK,
            Json(json!({ "message": "API key revoked successfully" })),
        )
            .into_response())
    } else {
        Err(ApiKeyError::NotFound(
            "API key not found or already revoked".to_string(),
        ))
    }
}

#[derive(Debug)]
pub enum ApiKeyError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    ServerError(String),
}

impl IntoResponse for ApiKeyError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            Self::ServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
