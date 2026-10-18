//! Device tokens API endpoints for mobile/web push notification targeting.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::push_notification_registration::PushNotificationRegistration;
use crate::models::network_context_middleware::NetworkContext;
use crate::models::push_notification_registration::{
    Config as RegistrationConfig, NotificationProvider, RegistrationRequest, RegistrationResponse,
};

/// Payload sent by the client when registering a device token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDeviceTokenPayload {
    /// Stellar account address (G-address, 56 characters)
    pub account: String,
    /// Device token issued by FCM / APNs / WebPush
    pub device_token: String,
    /// Provider (fcm, apns, web_push; default: fcm)
    #[serde(default)]
    pub provider: NotificationProvider,
    /// Optional human-readable device label
    pub device_label: Option<String>,
}

/// Device record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeviceTokenRecord {
    pub id: String,
    pub account: String,
    pub device_token: String,
    pub provider: String,
    pub device_label: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceListResponse {
    pub devices: Vec<DeviceTokenRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeregisterResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Clone)]
pub struct DeviceTokenApiState {
    pub pool: SqlitePool,
    pub service: PushNotificationRegistration,
}

/// Router for device token management.
pub fn routes(pool: SqlitePool) -> Router {
    let state = Arc::new(DeviceTokenApiState {
        pool,
        service: PushNotificationRegistration::new(RegistrationConfig::default()),
    });

    Router::new()
        .route("/devices", post(register_device))
        .route("/devices/{device_token}", delete(deregister_device))
        .route("/devices/{account}", get(list_account_devices))
        .with_state(state)
}

/// POST /notifications/devices - Register or update a device token
pub async fn register_device(
    State(state): State<Arc<DeviceTokenApiState>>,
    Json(payload): Json<RegisterDeviceTokenPayload>,
) -> ApiResult<impl IntoResponse> {
    let reg_request = RegistrationRequest {
        account: payload.account.clone(),
        device_token: payload.device_token.clone(),
        provider: payload.provider.clone(),
        device_label: payload.device_label.clone(),
    };

    let context = NetworkContext::testnet();
    state
        .service
        .register(&reg_request, &context)
        .await
        .map_err(|err| ApiError::bad_request("VALIDATION_ERROR", err.to_string()))?;

    let registration_id = Uuid::new_v4().to_string();
    let provider_str = match payload.provider {
        NotificationProvider::Fcm => "fcm",
        NotificationProvider::Apns => "apns",
        NotificationProvider::WebPush => "web_push",
    };

    sqlx::query(
        r#"
        INSERT INTO device_tokens (id, account, device_token, provider, device_label, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        ON CONFLICT(device_token) DO UPDATE SET
            account = excluded.account,
            provider = excluded.provider,
            device_label = excluded.device_label,
            updated_at = datetime('now')
        "#,
    )
    .bind(&registration_id)
    .bind(&payload.account)
    .bind(&payload.device_token)
    .bind(provider_str)
    .bind(&payload.device_label)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal("DB_ERROR", format!("Failed to register device token: {e}")))?;

    let response = RegistrationResponse {
        success: true,
        message: "Device registered for push notifications".to_string(),
        network: format!("{:?}", context.network),
        registration_id: Some(registration_id),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// DELETE /notifications/devices/:device_token - Deregister a device token
pub async fn deregister_device(
    State(state): State<Arc<DeviceTokenApiState>>,
    Path(device_token): Path<String>,
) -> ApiResult<impl IntoResponse> {
    if device_token.trim().is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_TOKEN",
            "Device token must not be empty",
        ));
    }

    sqlx::query("DELETE FROM device_tokens WHERE device_token = ?")
        .bind(&device_token)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal("DB_ERROR", format!("Failed to deregister device token: {e}")))?;

    let _ = state
        .service
        .deregister(&device_token, &NetworkContext::testnet())
        .await;

    Ok((
        StatusCode::OK,
        Json(DeregisterResponse {
            success: true,
            message: "Device token deregistered successfully".to_string(),
        }),
    ))
}

/// GET /notifications/devices/:account - List registered devices for a Stellar account
pub async fn list_account_devices(
    State(state): State<Arc<DeviceTokenApiState>>,
    Path(account): Path<String>,
) -> ApiResult<impl IntoResponse> {
    if !account.starts_with('G') || account.len() != 56 {
        return Err(ApiError::bad_request(
            "INVALID_ACCOUNT",
            "Account must be a 56-character Stellar public key starting with G",
        ));
    }

    let devices = sqlx::query_as::<_, DeviceTokenRecord>(
        r#"
        SELECT id, account, device_token, provider, device_label, created_at, updated_at
        FROM device_tokens
        WHERE account = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(&account)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal("DB_ERROR", format!("Failed to fetch devices: {e}")))?;

    Ok((StatusCode::OK, Json(DeviceListResponse { devices })))
}
