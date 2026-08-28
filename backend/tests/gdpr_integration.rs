use stellar_insights_backend::services::gdpr::{GdprService, UserInfo};
use sqlx::sqlite::SqlitePoolOptions;

async fn create_test_db() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory SQLite pool");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create users table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_consents (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            consent_type TEXT NOT NULL,
            consent_given BOOLEAN NOT NULL DEFAULT FALSE,
            consent_version TEXT NOT NULL,
            ip_address TEXT,
            user_agent TEXT,
            granted_at TEXT,
            revoked_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create user_consents table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS api_keys (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create api_keys table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS admin_audit_log (
            id TEXT PRIMARY KEY,
            timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            action VARCHAR(100) NOT NULL,
            resource VARCHAR(255) NOT NULL,
            user_id TEXT NOT NULL,
            status VARCHAR(20) NOT NULL,
            details TEXT,
            hash TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create admin_audit_log table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS vault_audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            operation TEXT NOT NULL,
            resource TEXT NOT NULL,
            user_id TEXT,
            status TEXT NOT NULL,
            details TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create vault_audit_log table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS data_export_requests (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            requested_data_types TEXT NOT NULL,
            export_format TEXT NOT NULL DEFAULT 'json',
            requested_at TEXT NOT NULL DEFAULT (datetime('now')),
            completed_at TEXT,
            expires_at TEXT,
            download_token TEXT UNIQUE,
            file_path TEXT,
            error_message TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create data_export_requests table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS data_deletion_requests (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            reason TEXT,
            delete_all_data BOOLEAN NOT NULL DEFAULT TRUE,
            data_types_to_delete TEXT,
            requested_at TEXT NOT NULL DEFAULT (datetime('now')),
            scheduled_deletion_at TEXT,
            completed_at TEXT,
            cancelled_at TEXT,
            error_message TEXT,
            confirmation_token TEXT UNIQUE,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create data_deletion_requests table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS data_processing_log (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            activity_type TEXT NOT NULL,
            data_category TEXT NOT NULL,
            purpose TEXT,
            legal_basis TEXT,
            processed_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create data_processing_log table");

    pool
}

#[tokio::test]
async fn test_create_export_request() {
    let pool = create_test_db().await;
    let service = GdprService::new(pool.clone());
    let user_id = "test_user_123";

    // Insert test user
    sqlx::query("INSERT INTO users (id, username) VALUES (?, ?)")
        .bind(user_id)
        .bind("testuser")
        .execute(&pool)
        .await
        .expect("Failed to insert test user");

    let result = service.create_export_request(user_id).await;
    assert!(result.is_ok());

    let export = result.unwrap();
    assert_eq!(export.user_id, user_id);
    assert_eq!(export.status, "pending");
}

#[tokio::test]
async fn test_create_deletion_request() {
    let pool = create_test_db().await;
    let service = GdprService::new(pool.clone());
    let user_id = "test_user_456";

    // Insert test user
    sqlx::query("INSERT INTO users (id, username) VALUES (?, ?)")
        .bind(user_id)
        .bind("testuser2")
        .execute(&pool)
        .await
        .expect("Failed to insert test user");

    let result = service.create_deletion_request(user_id).await;
    assert!(result.is_ok());

    let deletion = result.unwrap();
    assert_eq!(deletion.user_id, user_id);
    assert_eq!(deletion.status, "pending");
    assert!(deletion.scheduled_deletion_at.is_some());
}

#[tokio::test]
async fn test_set_and_get_consent() {
    let pool = create_test_db().await;
    let service = GdprService::new(pool.clone());
    let user_id = "test_user_789";

    // Insert test user
    sqlx::query("INSERT INTO users (id, username) VALUES (?, ?)")
        .bind(user_id)
        .bind("testuser3")
        .execute(&pool)
        .await
        .expect("Failed to insert test user");

    // Set consent
    let result = service.set_consent(user_id, "marketing", true).await;
    assert!(result.is_ok());

    // Get consents
    let consents = service.get_consents(user_id).await.expect("Failed to get consents");
    assert_eq!(consents.len(), 1);
    assert_eq!(consents[0].consent_type, "marketing");
    assert!(consents[0].consent_given);

    // Update consent
    let result = service.set_consent(user_id, "marketing", false).await;
    assert!(result.is_ok());

    let updated_consents = service.get_consents(user_id).await.expect("Failed to get updated consents");
    assert_eq!(updated_consents[0].consent_given, false);
}

#[tokio::test]
async fn test_confirm_deletion() {
    let pool = create_test_db().await;
    let service = GdprService::new(pool.clone());
    let user_id = "test_user_confirm";

    // Insert test user
    sqlx::query("INSERT INTO users (id, username) VALUES (?, ?)")
        .bind(user_id)
        .bind("testuser_confirm")
        .execute(&pool)
        .await
        .expect("Failed to insert test user");

    // Create deletion request
    let deletion = service
        .create_deletion_request(user_id)
        .await
        .expect("Failed to create deletion request");

    // Confirm deletion
    let result = service.confirm_deletion(&deletion.id, user_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cancel_deletion() {
    let pool = create_test_db().await;
    let service = GdprService::new(pool.clone());
    let user_id = "test_user_cancel";

    // Insert test user
    sqlx::query("INSERT INTO users (id, username) VALUES (?, ?)")
        .bind(user_id)
        .bind("testuser_cancel")
        .execute(&pool)
        .await
        .expect("Failed to insert test user");

    // Create deletion request
    let deletion = service
        .create_deletion_request(user_id)
        .await
        .expect("Failed to create deletion request");

    // Cancel deletion
    let result = service.cancel_deletion(&deletion.id, user_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_compile_user_data() {
    let pool = create_test_db().await;
    let service = GdprService::new(pool.clone());
    let user_id = "test_user_export";

    // Insert test user
    sqlx::query("INSERT INTO users (id, username) VALUES (?, ?)")
        .bind(user_id)
        .bind("testuser_export")
        .execute(&pool)
        .await
        .expect("Failed to insert test user");

    // Add consent
    service
        .set_consent(user_id, "marketing", true)
        .await
        .expect("Failed to set consent");

    // Compile user data
    let export = service
        .compile_user_data(user_id)
        .await
        .expect("Failed to compile user data");

    assert_eq!(export.user_info.id, user_id);
    assert_eq!(export.user_info.username, "testuser_export");
    assert_eq!(export.consents.len(), 1);
    assert_eq!(export.consents[0].consent_type, "marketing");
}

#[tokio::test]
async fn test_execute_deletion_anonymizes_audit_logs() {
    let pool = create_test_db().await;
    let service = GdprService::new(pool.clone());
    let user_id = "test_user_delete";

    // Insert test user
    sqlx::query("INSERT INTO users (id, username) VALUES (?, ?)")
        .bind(user_id)
        .bind("testuser_delete")
        .execute(&pool)
        .await
        .expect("Failed to insert test user");

    // Insert audit log entry for this user
    sqlx::query(
        "INSERT INTO admin_audit_log (id, action, resource, user_id, status, hash) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind("audit_123")
    .bind("user_login")
    .bind("user")
    .bind(user_id)
    .bind("success")
    .bind("hash_value")
    .execute(&pool)
    .await
    .expect("Failed to insert audit log");

    // Execute deletion
    let result = service.execute_deletion(user_id).await;
    assert!(result.is_ok());

    // Verify audit log is anonymized (not deleted)
    let audit_rows: Vec<(String,)> = sqlx::query_as(
        "SELECT user_id FROM admin_audit_log WHERE id = ?"
    )
    .bind("audit_123")
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch audit log");

    assert_eq!(audit_rows.len(), 1);
    assert_eq!(audit_rows[0].0, "anonymized");

    // Verify user is anonymized
    let user_rows: Vec<(String,)> = sqlx::query_as(
        "SELECT username FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch user");

    assert_eq!(user_rows.len(), 1);
    assert!(user_rows[0].0.starts_with("deleted_user_"));
}
