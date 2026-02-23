use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::database::Database;
use crate::models::api_key::CreateApiKeyRequest;

fn extract_wallet_address(headers: &HeaderMap) -> Result<String, ApiKeyError> {
    headers
        .get("X-Wallet-Address")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiKeyError::Unauthorized("Missing X-Wallet-Address header".to_string()))
}

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
            ApiKeyError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiKeyError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiKeyError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiKeyError::ServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub fn routes(db: Arc<Database>) -> Router {
    Router::new()
        .route("/", get(list_api_keys).post(create_api_key))
        .route("/:id", get(get_api_key).delete(revoke_api_key))
        .route("/:id/rotate", post(rotate_api_key))
        .with_state(db)
}
