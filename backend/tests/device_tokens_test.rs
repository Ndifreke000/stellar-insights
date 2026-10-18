use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use payraider_backend::api::device_tokens::{self, RegisterDeviceTokenPayload};
use payraider_backend::models::push_notification_registration::NotificationProvider;
use sqlx::SqlitePool;
use tower::ServiceExt;

async fn create_test_db() -> Result<SqlitePool> {
    let pool = SqlitePool::connect(":memory:").await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS device_tokens (
            id TEXT PRIMARY KEY NOT NULL,
            account TEXT NOT NULL,
            device_token TEXT NOT NULL UNIQUE,
            provider TEXT NOT NULL DEFAULT 'fcm',
            device_label TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_device_tokens_account ON device_tokens(account);
        CREATE INDEX IF NOT EXISTS idx_device_tokens_token ON device_tokens(device_token);
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

#[tokio::test]
async fn test_register_and_list_device_tokens() -> Result<()> {
    let pool = create_test_db().await?;
    let router = device_tokens::routes(pool);

    let account = format!("G{}", "A".repeat(55));
    let token = "fcm_test_device_token_12345";

    let payload = RegisterDeviceTokenPayload {
        account: account.clone(),
        device_token: token.to_string(),
        provider: NotificationProvider::Fcm,
        device_label: Some("Android Simulator".to_string()),
    };

    let req = Request::builder()
        .method("POST")
        .uri("/devices")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload)?))?;

    let resp = router.clone().oneshot(req).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    assert_eq!(resp.status(), StatusCode::OK);

    // List devices
    let list_req = Request::builder()
        .method("GET")
        .uri(format!("/devices/{account}"))
        .body(Body::empty())?;

    let list_resp = router.clone().oneshot(list_req).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    assert_eq!(list_resp.status(), StatusCode::OK);

    // Deregister device
    let delete_req = Request::builder()
        .method("DELETE")
        .uri(format!("/devices/{token}"))
        .body(Body::empty())?;

    let del_resp = router.clone().oneshot(delete_req).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    assert_eq!(del_resp.status(), StatusCode::OK);

    Ok(())
}
