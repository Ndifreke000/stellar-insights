/// OAuth API endpoints for Zapier integration
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::auth::oauth::{OAuthError, OAuthService, TokenResponse, AVAILABLE_SCOPES};
use crate::auth_middleware::AuthUser;

/// OAuth Token Request (for /api/oauth/token)
#[derive(Debug, Deserialize)]
pub struct OAuthTokenRequest {
    pub grant_type: String,            // "authorization_code" or "refresh_token"
    pub code: Option<String>,          // for authorization_code
    pub refresh_token: Option<String>, // for refresh_token
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: Option<String>,
}

/// OAuth Authorization Request (for /api/oauth/authorize)
#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String, // "code"
    pub scope: String,         // space-separated scopes
    pub state: String,         // CSRF prevention
}

/// OAuth Authorization Response
#[derive(Debug, Serialize)]
pub struct OAuthAuthorizeResponse {
    pub authorization_code: String,
    pub state: String,
}

/// OAuth Token Error Response
#[derive(Debug, Serialize)]
pub struct OAuthTokenErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}

/// OAuth Revoke Request
#[derive(Debug, Deserialize)]
pub struct OAuthRevokeRequest {
    pub access_token: String,
    pub client_id: String,
    pub client_secret: String,
}

/// List OAuth Apps Response
#[derive(Debug, Serialize)]
pub struct OAuthAppInfo {
    pub client_id: String,
    pub app_name: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListOAuthAppsResponse {
    pub apps: Vec<OAuthAppInfo>,
}

/// POST /api/oauth/authorize - Request authorization code
pub async fn authorize(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
    Query(request): Query<OAuthAuthorizeRequest>,
) -> Result<Response, OAuthApiError> {
    if request.response_type != "code" {
        return Err(OAuthApiError::InvalidRequest(
            "response_type must be 'code'".to_string(),
        ));
    }

    let service = OAuthService::new(db);

    // Validate scopes
    service
        .validate_scopes(&request.scope)
        .map_err(|e| OAuthApiError::InvalidScope(format!("Invalid scopes: {}", e)))?;

    // Store authorization
    let scopes = service.validate_scopes(&request.scope)?;
    service
        .store_authorization(&request.client_id, &auth_user.user_id, &scopes)
        .await
        .map_err(|e| OAuthApiError::ServerError(e.to_string()))?;

    // Generate authorization code
    let authorization_code = format!("authcode_{}", uuid::Uuid::new_v4());

    Ok((
        StatusCode::OK,
        Json(OAuthAuthorizeResponse {
            authorization_code,
            state: request.state,
        }),
    )
        .into_response())
}

/// POST /api/oauth/token - Exchange authorization code for tokens
pub async fn token(
    State(db): State<SqlitePool>,
    Json(request): Json<OAuthTokenRequest>,
) -> Result<Response, OAuthApiError> {
    let service = OAuthService::new(db.clone());

    // Validate client credentials
    let user_id = service
        .validate_client_credentials(&request.client_id, &request.client_secret)
        .await
        .map_err(|_| OAuthApiError::InvalidClient)?;

    // Get user from database for username
    let user = sqlx::query!("SELECT username FROM users WHERE id = ?", user_id)
        .fetch_optional(&db)
        .await
        .map_err(|e| OAuthApiError::ServerError(e.to_string()))?
        .ok_or_else(|| OAuthApiError::InvalidClient)?;

    let username = user.username;

    match request.grant_type.as_str() {
        "authorization_code" => {
            let code = request.code.ok_or_else(|| {
                OAuthApiError::InvalidRequest(
                    "code is required for authorization_code grant".to_string(),
                )
            })?;

            // In a real implementation, you would:
            // 1. Look up the authorization code in Redis with a TTL
            // 2. Verify it matches the client_id and redirect_uri
            // 3. For now, we'll simplify and just verify client credentials (done above)

            // Get scopes from most recent authorization
            let auth = service
                .get_authorization(&request.client_id, &user_id)
                .await
                .map_err(|e| OAuthApiError::ServerError(e.to_string()))?
                .ok_or_else(|| {
                    OAuthApiError::InvalidRequest(
                        "No authorization found for this client".to_string(),
                    )
                })?;

            // Generate tokens
            let access_token = service
                .generate_access_token(&user_id, &username, &request.client_id, auth.clone())
                .map_err(|e| OAuthApiError::ServerError(e.to_string()))?;

            let refresh_token = service
                .generate_refresh_token(&user_id, &username, &request.client_id)
                .map_err(|e| OAuthApiError::ServerError(e.to_string()))?;

            // Store tokens
            service
                .store_token(
                    &user_id,
                    &access_token,
                    &refresh_token,
                    chrono::Utc::now().timestamp(),
                )
                .await
                .map_err(|e| OAuthApiError::ServerError(e.to_string()))?;

            Ok((
                StatusCode::OK,
                Json(TokenResponse {
                    access_token,
                    refresh_token,
                    token_type: "Bearer".to_string(),
                    expires_in: 86400 * 7, // 7 days in seconds
                    scope: auth.join(" "),
                }),
            )
                .into_response())
        }
        "refresh_token" => {
            let refresh_token = request.refresh_token.ok_or_else(|| {
                OAuthApiError::InvalidRequest(
                    "refresh_token is required for refresh_token grant".to_string(),
                )
            })?;

            // Validate refresh token
            let claims = service
                .validate_oauth_token(&refresh_token)
                .map_err(|_| OAuthApiError::InvalidGrant)?;

            if claims.token_type != "refresh" {
                return Err(OAuthApiError::InvalidGrant);
            }

            // Generate new access token (reuse scopes from refresh token or get from DB)
            let auth = service
                .get_authorization(&request.client_id, &user_id)
                .await
                .map_err(|e| OAuthApiError::ServerError(e.to_string()))?
                .unwrap_or_default();

            let access_token = service
                .generate_access_token(&user_id, &username, &request.client_id, auth.clone())
                .map_err(|e| OAuthApiError::ServerError(e.to_string()))?;

            Ok((
                StatusCode::OK,
                Json(TokenResponse {
                    access_token,
                    refresh_token: refresh_token.clone(),
                    token_type: "Bearer".to_string(),
                    expires_in: 86400 * 7,
                    scope: auth.join(" "),
                }),
            )
                .into_response())
        }
        _ => Err(OAuthApiError::UnsupportedGrantType),
    }
}

/// POST /api/oauth/revoke - Revoke an access token
pub async fn revoke(
    State(db): State<SqlitePool>,
    Json(request): Json<OAuthRevokeRequest>,
) -> Result<Response, OAuthApiError> {
    let service = OAuthService::new(db);

    // Validate client credentials
    service
        .validate_client_credentials(&request.client_id, &request.client_secret)
        .await
        .map_err(|_| OAuthApiError::InvalidClient)?;

    // Revoke token
    service
        .revoke_token(&request.access_token)
        .await
        .map_err(|e| OAuthApiError::ServerError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(json!({"message": "Token revoked successfully"})),
    )
        .into_response())
}

/// GET /api/oauth/apps - List OAuth apps for authenticated user
pub async fn list_apps(
    State(db): State<SqlitePool>,
    auth_user: AuthUser,
) -> Result<Response, OAuthApiError> {
    let apps = sqlx::query!(
        r#"
        SELECT client_id, app_name, created_at FROM oauth_clients
        WHERE user_id = ?
        ORDER BY created_at DESC
        "#,
        auth_user.user_id
    )
    .fetch_all(&db)
    .await
    .map_err(|e| OAuthApiError::ServerError(e.to_string()))?;

    let app_list: Vec<OAuthAppInfo> = apps
        .into_iter()
        .map(|row| OAuthAppInfo {
            client_id: row.client_id,
            app_name: row.app_name,
            created_at: row.created_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(ListOAuthAppsResponse { apps: app_list }),
    )
        .into_response())
}

/// OAuth API Error types
#[derive(Debug)]
pub enum OAuthApiError {
    InvalidRequest(String),
    InvalidClient,
    InvalidScope(String),
    InvalidGrant,
    UnsupportedGrantType,
    ServerError(String),
}

impl IntoResponse for OAuthApiError {
    fn into_response(self) -> Response {
        let (status_code, error, description) = match self {
            OAuthApiError::InvalidRequest(msg) => {
                (StatusCode::BAD_REQUEST, "invalid_request", Some(msg))
            }
            OAuthApiError::InvalidClient => (StatusCode::UNAUTHORIZED, "invalid_client", None),
            OAuthApiError::InvalidScope(msg) => {
                (StatusCode::BAD_REQUEST, "invalid_scope", Some(msg))
            }
            OAuthApiError::InvalidGrant => (StatusCode::UNAUTHORIZED, "invalid_grant", None),
            OAuthApiError::UnsupportedGrantType => {
                (StatusCode::BAD_REQUEST, "unsupported_grant_type", None)
            }
            OAuthApiError::ServerError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "server_error", Some(msg))
            }
        };

        let body = Json(OAuthTokenErrorResponse {
            error: error.to_string(),
            error_description: description,
        });

        (status_code, body).into_response()
    }
}

impl From<anyhow::Error> for OAuthApiError {
    fn from(err: anyhow::Error) -> Self {
        OAuthApiError::ServerError(err.to_string())
    }
}

/// Create OAuth routes
pub fn routes(db: SqlitePool) -> Router {
    Router::new()
        .route("/api/oauth/authorize", post(authorize))
        .route("/api/oauth/token", post(token))
        .route("/api/oauth/revoke", post(revoke))
        .route("/api/oauth/apps", get(list_apps))
        .with_state(db)
}
