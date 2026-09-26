use anyhow::{anyhow, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
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
    pub session_id: Option<String>,
    pub device_user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub event_type: Option<String>,
}

#[derive(Debug)]
pub struct IntegrityCheckResult {
    pub is_valid: bool,
    pub total_entries: usize,
    pub invalid_entries: Vec<String>,
    pub message: String,
}

pub struct AdminAuditLogger {
    pool: SqlitePool,
}

impl AdminAuditLogger {
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Record an admin action with tamper-proof hash chaining and optional session context
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
        let data = format!("{id}|{timestamp}|{action}|{resource}|{user_id}|{status}|{details}");
        let hash_input = match prev_hash {
            Some(h) => format!("{h}|{data}"),
            None => data.clone(),
        };
        let mut hasher = Sha256::new();
        hasher.update(hash_input.as_bytes());
        let hash = hex::encode(hasher.finalize());

        sqlx::query(
            r"
            INSERT INTO admin_audit_log (id, timestamp, action, resource, user_id, status, details, hash)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ",
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

    /// Record an admin action with full context (session, device, IP, event type)
    pub async fn log_action_with_context(
        &self,
        action: &str,
        resource: &str,
        user_id: &str,
        status: &str,
        details: serde_json::Value,
        session_id: Option<&str>,
        device_user_agent: Option<&str>,
        ip_address: Option<&str>,
        event_type: Option<&str>,
    ) -> Result<()> {
        let timestamp = Utc::now();
        let id = Uuid::new_v4().to_string();
        let data = format!("{id}|{timestamp}|{action}|{resource}|{user_id}|{status}|{details}");

        // Get previous hash for chaining
        let prev_hash_result = sqlx::query_scalar::<_, String>(
            "SELECT hash FROM admin_audit_log ORDER BY timestamp DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        let hash_input = match prev_hash_result {
            Some(h) => format!("{h}|{data}"),
            None => data.clone(),
        };

        let mut hasher = Sha256::new();
        hasher.update(hash_input.as_bytes());
        let hash = hex::encode(hasher.finalize());

        sqlx::query(
            r"
            INSERT INTO admin_audit_log
            (id, timestamp, action, resource, user_id, status, details, hash, session_id, device_user_agent, ip_address, event_type)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&id)
        .bind(timestamp)
        .bind(action)
        .bind(resource)
        .bind(user_id)
        .bind(status)
        .bind(details)
        .bind(&hash)
        .bind(session_id)
        .bind(device_user_agent)
        .bind(ip_address)
        .bind(event_type)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Verify integrity of audit log by checking hash chain
    pub async fn verify_integrity(&self) -> Result<IntegrityCheckResult> {
        let entries = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, hash, action FROM admin_audit_log ORDER BY timestamp ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut invalid_entries = Vec::new();
        let mut expected_prev_hash: Option<String> = None;

        for (id, stored_hash, action) in entries.iter() {
            // Reconstruct the data that was hashed
            let entry = sqlx::query_as::<_, (
                String,
                chrono::DateTime<chrono::Utc>,
                String,
                String,
                String,
                String,
                serde_json::Value,
            )>(
                "SELECT id, timestamp, action, resource, user_id, status, details FROM admin_audit_log WHERE id = ?",
            )
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

            let data = format!(
                "{}|{}|{}|{}|{}|{}|{}",
                entry.0, entry.1, entry.2, entry.3, entry.4, entry.5, entry.6
            );

            let hash_input = match &expected_prev_hash {
                Some(h) => format!("{h}|{data}"),
                None => data,
            };

            let mut hasher = Sha256::new();
            hasher.update(hash_input.as_bytes());
            let computed_hash = hex::encode(hasher.finalize());

            if computed_hash != *stored_hash {
                invalid_entries.push(id.clone());
            }

            expected_prev_hash = Some(stored_hash.clone());
        }

        let is_valid = invalid_entries.is_empty();
        let total_entries = entries.len();

        let message = if is_valid {
            format!("Audit log integrity verified: {} entries in valid hash chain", total_entries)
        } else {
            format!(
                "Audit log integrity check failed: {} invalid entries out of {}",
                invalid_entries.len(),
                total_entries
            )
        };

        Ok(IntegrityCheckResult {
            is_valid,
            total_entries,
            invalid_entries,
            message,
        })
    }

    /// Query audit log by filters
    pub async fn query_audit_log(
        &self,
        user_id: Option<&str>,
        action_type: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AdminAuditLogEntry>> {
        let mut query = "SELECT id, timestamp, action, resource, user_id, status, details, hash, session_id, device_user_agent, ip_address, event_type FROM admin_audit_log WHERE 1=1".to_string();

        if user_id.is_some() {
            query.push_str(" AND user_id = ?");
        }
        if action_type.is_some() {
            query.push_str(" AND action = ?");
        }
        if status.is_some() {
            query.push_str(" AND status = ?");
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, (
            String,
            chrono::DateTime<chrono::Utc>,
            String,
            String,
            String,
            String,
            serde_json::Value,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )>(&query);

        if let Some(uid) = user_id {
            q = q.bind(uid);
        }
        if let Some(act) = action_type {
            q = q.bind(act);
        }
        if let Some(st) = status {
            q = q.bind(st);
        }

        let results = q.bind(limit).bind(offset).fetch_all(&self.pool).await?;

        Ok(results
            .into_iter()
            .map(
                |(id, timestamp, action, resource, user_id, status, details, hash, session_id, device_user_agent, ip_address, event_type)| {
                    AdminAuditLogEntry {
                        id,
                        timestamp,
                        action,
                        resource,
                        user_id,
                        status,
                        details,
                        hash,
                        session_id,
                        device_user_agent,
                        ip_address,
                        event_type,
                    }
                },
            )
            .collect())
    }
}
