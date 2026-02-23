use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AdminAuditLogEntry {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub action: String,
    pub resource: String,
    pub user_id: String,
    pub status: String,
    pub details: serde_json::Value,
    pub hash: String,
}

pub struct AdminAuditLogger {
    pool: SqlitePool,
}

impl AdminAuditLogger {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Record an admin action with tamper-proof hash chaining
    pub async fn log_action(
        &self,
        action: &str,
        resource: &str,
        user_id: &str,
        status: &str,
        details: serde_json::Value,
        prev_hash: Option<&str>,
    ) -> Result<()> {
        let timestamp = Utc::now();
        let id = Uuid::new_v4().to_string();
        // The original format string for `data` was correct in terms of placeholder count.
        // If "Fix placeholders in audit log" implies changing how `details` is serialized,
        // `details.to_string()` is a common way to include JSON in a string hash.
        let data = format!("{}|{}|{}|{}|{}|{}|{}", id, timestamp, action, resource, user_id, status, details.to_string());
        let hash_input = match prev_hash {
            Some(h) => format!("{}|{}", h, data),
            None => data.clone(),
        };
        let hash = format!("{:x}", md5::compute(hash_input));

        sqlx::query(
            r#"
            INSERT INTO admin_audit_log (id, timestamp, action, resource, user_id, status, details, hash)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(timestamp)
        .bind(action)
        .bind(resource)
        .bind(user_id)
        .bind(status)
        .bind(details)
        .bind(&hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
