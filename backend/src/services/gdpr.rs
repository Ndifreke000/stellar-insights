use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataExport {
    pub user_info: UserInfo,
    pub consents: Vec<ConsentRecord>,
    pub api_keys: Vec<ApiKeyInfo>,
    pub audit_logs: Vec<AuditLogRecord>,
    pub exported_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub consent_type: String,
    pub consent_given: bool,
    pub granted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogRecord {
    pub id: String,
    pub action: String,
    pub timestamp: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ExportRequest {
    pub id: String,
    pub user_id: String,
    pub status: String, // pending, processing, completed, failed
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DeletionRequest {
    pub id: String,
    pub user_id: String,
    pub status: String, // pending, confirmed, processing, completed, cancelled
    pub scheduled_deletion_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub struct GdprService {
    db_pool: SqlitePool,
}

impl GdprService {
    pub fn new(db_pool: SqlitePool) -> Self {
        Self { db_pool }
    }

    /// Create a data export request
    pub async fn create_export_request(&self, user_id: &str) -> Result<ExportRequest> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO data_export_requests (id, user_id, status, requested_data_types, export_format, requested_at)
            VALUES (?, ?, 'pending', 'all', 'json', ?)
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(now.to_rfc3339())
        .execute(&self.db_pool)
        .await
        .context("Failed to create export request")?;

        Ok(ExportRequest {
            id,
            user_id: user_id.to_string(),
            status: "pending".to_string(),
            created_at: now,
        })
    }

    /// Compile all personal data for a user
    pub async fn compile_user_data(&self, user_id: &str) -> Result<UserDataExport> {
        // Get user info
        let user_row = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, username, created_at, updated_at FROM users WHERE id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .context("Failed to fetch user")?
        .ok_or_else(|| anyhow!("User not found"))?;

        let user_info = UserInfo {
            id: user_row.0,
            username: user_row.1,
            created_at: DateTime::parse_from_rfc3339(&user_row.2)
                .ok()
                .and_then(|dt| Some(dt.with_timezone(&Utc)))
                .unwrap_or_else(|| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&user_row.3)
                .ok()
                .and_then(|dt| Some(dt.with_timezone(&Utc)))
                .unwrap_or_else(|| Utc::now()),
        };

        // Get consents
        let consent_rows = sqlx::query_as::<_, (String, bool, Option<String>, Option<String>)>(
            "SELECT consent_type, consent_given, granted_at, revoked_at FROM user_consents WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await
        .context("Failed to fetch consents")?;

        let consents = consent_rows
            .into_iter()
            .map(|row| ConsentRecord {
                consent_type: row.0,
                consent_given: row.1,
                granted_at: row.2.and_then(|dt| DateTime::parse_from_rfc3339(&dt).ok().map(|d| d.with_timezone(&Utc))),
                revoked_at: row.3.and_then(|dt| DateTime::parse_from_rfc3339(&dt).ok().map(|d| d.with_timezone(&Utc))),
            })
            .collect();

        // Get API keys (exclude secret/hash)
        let key_rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, name, created_at FROM api_keys WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await
        .context("Failed to fetch API keys")?;

        let api_keys = key_rows
            .into_iter()
            .map(|row| ApiKeyInfo {
                id: row.0,
                name: row.1,
                created_at: DateTime::parse_from_rfc3339(&row.2)
                    .ok()
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|| Utc::now()),
            })
            .collect();

        // Get audit logs (anonymize where user_id is logged)
        let audit_rows = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, action, timestamp, status FROM admin_audit_log WHERE user_id = ? ORDER BY timestamp DESC LIMIT 100",
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await
        .context("Failed to fetch audit logs")?;

        let audit_logs = audit_rows
            .into_iter()
            .map(|row| AuditLogRecord {
                id: row.0,
                action: row.1,
                timestamp: DateTime::parse_from_rfc3339(&row.2)
                    .ok()
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|| Utc::now()),
                status: row.3,
            })
            .collect();

        Ok(UserDataExport {
            user_info,
            consents,
            api_keys,
            audit_logs,
            exported_at: Utc::now(),
        })
    }

    /// Create a deletion request with confirmation token
    pub async fn create_deletion_request(&self, user_id: &str) -> Result<DeletionRequest> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        // Deletion is scheduled 7 days from now to allow cancellation
        let scheduled = now + Duration::days(7);

        sqlx::query(
            r#"
            INSERT INTO data_deletion_requests (id, user_id, status, delete_all_data, requested_at, scheduled_deletion_at)
            VALUES (?, ?, 'pending', TRUE, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(now.to_rfc3339())
        .bind(scheduled.to_rfc3339())
        .execute(&self.db_pool)
        .await
        .context("Failed to create deletion request")?;

        Ok(DeletionRequest {
            id,
            user_id: user_id.to_string(),
            status: "pending".to_string(),
            scheduled_deletion_at: Some(scheduled),
            created_at: now,
        })
    }

    /// Confirm a deletion request (after user confirms via email/2FA)
    pub async fn confirm_deletion(&self, deletion_id: &str, user_id: &str) -> Result<()> {
        sqlx::query("UPDATE data_deletion_requests SET status = 'confirmed' WHERE id = ? AND user_id = ?")
            .bind(deletion_id)
            .bind(user_id)
            .execute(&self.db_pool)
            .await
            .context("Failed to confirm deletion")?;

        Ok(())
    }

    /// Execute deletion: anonymize audit logs, delete personal data
    pub async fn execute_deletion(&self, user_id: &str) -> Result<()> {
        let mut tx = self.db_pool.begin().await.context("Failed to begin transaction")?;

        // Anonymize admin_audit_log entries (keep logs, anonymize user reference)
        sqlx::query("UPDATE admin_audit_log SET user_id = 'anonymized' WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("Failed to anonymize audit logs")?;

        // Anonymize vault_audit_log entries
        sqlx::query("UPDATE vault_audit_log SET user_id = 'anonymized' WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("Failed to anonymize vault logs")?;

        // Delete user consents
        sqlx::query("DELETE FROM user_consents WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete consents")?;

        // Delete export/deletion requests
        sqlx::query("DELETE FROM data_export_requests WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete export requests")?;

        sqlx::query("DELETE FROM data_deletion_requests WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete deletion requests")?;

        // Delete API keys
        sqlx::query("DELETE FROM api_keys WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete API keys")?;

        // Anonymize user account (keep record but remove identifying data)
        sqlx::query("UPDATE users SET username = ?, password_hash = NULL WHERE id = ?")
            .bind(format!("deleted_user_{}", Uuid::new_v4()))
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("Failed to anonymize user")?;

        tx.commit().await.context("Failed to commit deletion")?;

        Ok(())
    }

    /// Cancel a deletion request if still in pending state
    pub async fn cancel_deletion(&self, deletion_id: &str, user_id: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE data_deletion_requests SET status = 'cancelled', cancelled_at = ? WHERE id = ? AND user_id = ? AND status = 'pending'"
        )
        .bind(Utc::now().to_rfc3339())
        .bind(deletion_id)
        .bind(user_id)
        .execute(&self.db_pool)
        .await
        .context("Failed to cancel deletion")?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Deletion request not found or already confirmed"));
        }

        Ok(())
    }

    /// Set or update user consent
    pub async fn set_consent(&self, user_id: &str, consent_type: &str, consent_given: bool) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO user_consents (id, user_id, consent_type, consent_given, consent_version, granted_at)
            VALUES (?, ?, ?, ?, '1.0', ?)
            ON CONFLICT(user_id, consent_type) DO UPDATE SET
                consent_given = excluded.consent_given,
                granted_at = CASE WHEN excluded.consent_given THEN excluded.granted_at ELSE granted_at END,
                revoked_at = CASE WHEN NOT excluded.consent_given THEN ? ELSE revoked_at END,
                updated_at = ?
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(consent_type)
        .bind(consent_given)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db_pool)
        .await
        .context("Failed to set consent")?;

        Ok(())
    }

    /// Get all consents for a user
    pub async fn get_consents(&self, user_id: &str) -> Result<Vec<ConsentRecord>> {
        let rows = sqlx::query_as::<_, (String, bool, Option<String>, Option<String>)>(
            "SELECT consent_type, consent_given, granted_at, revoked_at FROM user_consents WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await
        .context("Failed to fetch consents")?;

        Ok(rows
            .into_iter()
            .map(|row| ConsentRecord {
                consent_type: row.0,
                consent_given: row.1,
                granted_at: row.2.and_then(|dt| DateTime::parse_from_rfc3339(&dt).ok().map(|d| d.with_timezone(&Utc))),
                revoked_at: row.3.and_then(|dt| DateTime::parse_from_rfc3339(&dt).ok().map(|d| d.with_timezone(&Utc))),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_data_export_serialization() {
        let export = UserDataExport {
            user_info: UserInfo {
                id: "user_123".to_string(),
                username: "testuser".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            consents: vec![],
            api_keys: vec![],
            audit_logs: vec![],
            exported_at: Utc::now(),
        };

        let json = serde_json::to_string(&export).unwrap();
        assert!(json.contains("user_123"));
        assert!(json.contains("testuser"));
    }
}
