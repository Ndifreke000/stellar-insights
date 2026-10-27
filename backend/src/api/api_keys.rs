use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::auth::Sep10User;
use crate::database::Database;
use crate::models::api_key::CreateApiKeyRequest;
use crate::validation::ValidatedJson;

/// The key owner is the wallet proven by the caller's SEP-10 session, which
/// `sep10_auth_middleware` attaches to the request. Keys used to be owned by
/// whatever `X-Wallet-Address` header the client sent, so anyone could list,
/// rotate or revoke another wallet's keys by naming it.
fn wallet_address(user: Option<Extension<Sep10User>>) -> Result<String, ApiKeyError> {
    user.map(|Extension(user)| user.account)
        .filter(|account| !account.is_empty())
        .ok_or_else(|| ApiKeyError::Unauthorized("A SEP-10 wallet session is required".to_string()))
}

/// POST /api/api-keys - Create a new API key
#[utoipa::path(
    post,
    path = "/api/api-keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized - missing or invalid SEP-10 session"),
        (status = 500, description = "Internal server error")
    ),
    tag = "API Keys"
)]
pub async fn create_api_key(
    State(db): State<Arc<Database>>,
    user: Option<Extension<Sep10User>>,
    ValidatedJson(req): ValidatedJson<CreateApiKeyRequest>,
) -> Result<Response, ApiKeyError> {
    let wallet_address = wallet_address(user)?;

    let name = req.name.trim();
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
    {
        return Err(ApiKeyError::BadRequest(
            "Key name may only contain letters, digits, spaces, hyphens, and underscores"
                .to_string(),
        ));
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
        (status = 401, description = "Unauthorized - missing or invalid SEP-10 session"),
        (status = 500, description = "Internal server error")
    ),
    tag = "API Keys"
)]
pub async fn list_api_keys(
    State(db): State<Arc<Database>>,
    user: Option<Extension<Sep10User>>,
) -> Result<Response, ApiKeyError> {
    let wallet_address = wallet_address(user)?;

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
        (status = 401, description = "Unauthorized - missing or invalid SEP-10 session"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "API Keys"
)]
pub async fn get_api_key(
    State(db): State<Arc<Database>>,
    user: Option<Extension<Sep10User>>,
    Path(id): Path<String>,
) -> Result<Response, ApiKeyError> {
    let wallet_address = wallet_address(user)?;

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
        (status = 401, description = "Unauthorized - missing or invalid SEP-10 session"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "API Keys"
)]
pub async fn rotate_api_key(
    State(db): State<Arc<Database>>,
    user: Option<Extension<Sep10User>>,
    Path(id): Path<String>,
) -> Result<Response, ApiKeyError> {
    let wallet_address = wallet_address(user)?;

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
        (status = 401, description = "Unauthorized - missing or invalid SEP-10 session"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "API Keys"
)]
pub async fn revoke_api_key(
    State(db): State<Arc<Database>>,
    user: Option<Extension<Sep10User>>,
    Path(id): Path<String>,
) -> Result<Response, ApiKeyError> {
    let wallet_address = wallet_address(user)?;

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

/// API key management, owned by the caller's SEP-10 wallet.
pub fn routes(
    db: Arc<Database>,
    sep10_service: Arc<crate::auth::sep10_simple::Sep10Service>,
) -> axum::Router {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/", get(list_api_keys).post(create_api_key))
        .route("/{id}", get(get_api_key).delete(revoke_api_key))
        .route("/{id}/rotate", post(rotate_api_key))
        .layer(axum::middleware::from_fn_with_state(
            sep10_service,
            crate::auth::sep10_auth_middleware,
        ))
        .with_state(db)
}
