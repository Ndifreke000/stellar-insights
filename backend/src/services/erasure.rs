//! GDPR erasure orchestration.
//!
//! A confirmed `data_deletion_requests` row is executed by [`ErasureService`]
//! once its `scheduled_deletion_at` passes. Erasure is split into per-store
//! steps so that every personal-data store is accounted for individually and
//! partial failures stay visible and retryable:
//!
//! - `sqlite`    – deletes/anonymizes rows in every user-linked table.
//! - `redis`     – removes cached/session keys that embed the user id.
//! - `keys`      – crypto-shreds the per-user DEK, rendering all ciphertext
//!   copies (live rows *and* backup snapshots) unrecoverable.
//! - `notifications` – deletes rows in any email/notification tables found
//!   at runtime (the schema has no dedicated email-log table today).
//!
//! Each attempt writes `erasure_jobs` + `erasure_steps` rows and, on
//! completion, an erasure certificate (JSON) listing every store, the action
//! taken, rows affected, and any deferred retention.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::services::user_keys::UserKeyService;

/// How often the scheduler polls for due deletion requests.
const SCHEDULER_INTERVAL: Duration = Duration::seconds(60);
/// Max requests processed per tick.
const SCHEDULER_BATCH: i64 = 10;

// ── Certificate types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ErasureStepRecord {
    pub store: String,
    pub status: String,
    pub rows_affected: i64,
    pub detail: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ErasureCertificate {
    pub certificate_version: u32,
    pub request_id: String,
    pub user_id: String,
    /// SHA-256 of the user id — stable correlation handle that does not
    /// itself identify the subject once the account row is anonymized.
    pub subject_hash: String,
    pub started_at: String,
    pub completed_at: String,
    pub status: String,
    pub steps: Vec<ErasureStepRecord>,
    pub crypto_shredded: bool,
    pub retention_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErasureJobView {
    pub id: String,
    pub deletion_request_id: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error: Option<String>,
    pub certificate: Option<ErasureCertificate>,
    pub steps: Vec<ErasureStepRecord>,
}

// ── Store handlers ───────────────────────────────────────────────────────────

struct StoreOutcome {
    rows_affected: i64,
    detail: Option<String>,
}

/// Tables whose rows are personal data and are deleted outright.
const DELETE_TABLES: &[(&str, &str)] = &[
    // (table, user-keyed column)
    ("sessions", "user_id"),
    ("user_2fa_backup_codes", "user_id"),
    ("user_2fa_secrets", "user_id"),
    ("user_consents", "user_id"),
    ("data_export_requests", "user_id"),
    ("data_processing_log", "user_id"),
    ("alert_rules", "user_id"),
    ("alert_history", "user_id"),
    ("corridor_alert_configs", "user_id"),
    ("corridor_alert_events", "user_id"),
    ("api_usage_stats", "user_id"),
    ("oauth_tokens", "user_id"),
    ("oauth_authorizations", "user_id"),
    ("oauth_clients", "user_id"),
    ("webhooks", "user_id"),
    ("token_revocations", "user_id"),
    ("snapshot_verifications", "user_id"),
    ("user_rewards", "user_id"),
    // device_tokens keys on `account`, not `user_id`.
    ("device_tokens", "account"),
];

/// Audit/ledger tables kept for accountability but anonymized instead of
/// deleted. A missing table or column is skipped, not fatal.
const ANONYMIZE_TABLES: &[&str] = &["admin_audit_log", "vault_audit_log", "audit_log"];

async fn erase_sqlite(pool: &SqlitePool, user_id: &str, request_id: &str) -> Result<StoreOutcome> {
    let mut tx = pool.begin().await.context("begin erasure tx")?;
    let mut total: i64 = 0;
    let mut notes: Vec<String> = Vec::new();

    for &(table, column) in DELETE_TABLES {
        // Table/column names are compile-time constants; user_id is bound.
        let sql = format!("DELETE FROM {table} WHERE {column} = ?");
        match sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(user_id)
            .execute(&mut *tx)
            .await
        {
            Ok(r) => total += r.rows_affected() as i64,
            Err(e) => {
                notes.push(format!("{table}: skipped ({e})"));
                tracing::warn!(table, error = %e, "erasure: table not present or incompatible");
            }
        }
    }

    for &table in ANONYMIZE_TABLES {
        let sql = format!("UPDATE {table} SET user_id = 'erased' WHERE user_id = ?");
        match sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(user_id)
            .execute(&mut *tx)
            .await
        {
            Ok(r) => total += r.rows_affected() as i64,
            Err(e) => notes.push(format!("{table}: skipped ({e})")),
        }
    }

    // Anonymize the account row itself: keep the id for FK integrity of
    // retained audit rows, strip all identifying data.
    sqlx::query("UPDATE users SET username = ?, password_hash = NULL WHERE id = ?")
        .bind(format!("erased_{}", Uuid::new_v4().simple()))
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .context("anonymize users row")?;

    // The deletion request row itself is the legal record of the erasure;
    // mark it completed rather than deleting it.
    sqlx::query(
        "UPDATE data_deletion_requests SET status = 'completed', completed_at = datetime('now') \
         WHERE id = ? AND status = 'scheduled'",
    )
    .bind(request_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await.context("commit erasure tx")?;

    Ok(StoreOutcome {
        rows_affected: total,
        detail: if notes.is_empty() {
            None
        } else {
            Some(notes.join("; "))
        },
    })
}

/// Scans sqlite_master for email/notification tables and deletes user rows.
/// The schema currently has no dedicated email-log table; this adapts if one
/// is added later so erasure coverage does not silently rot.
async fn erase_notification_stores(pool: &SqlitePool, user_id: &str) -> Result<StoreOutcome> {
    let tables = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND (\
            name LIKE '%email%' OR name LIKE '%mail_log%' OR name LIKE '%notification%')",
    )
    .fetch_all(pool)
    .await?;

    let mut total: i64 = 0;
    let mut seen: Vec<String> = Vec::new();
    for row in tables {
        let table: String = row.get("name");
        if table.starts_with("sqlite_") {
            continue;
        }
        // Only touch tables that carry a user-identifying column; build the
        // DELETE from whichever of the candidate columns actually exist.
        let cols = sqlx::query(
            "SELECT name FROM pragma_table_info(?) \
             WHERE name IN ('user_id','account','email')",
        )
        .bind(&table)
        .fetch_all(pool)
        .await
        .unwrap_or_default();
        if cols.is_empty() {
            continue;
        }
        let predicates: Vec<String> = cols
            .iter()
            .map(|c| format!("{} = ?", c.get::<String, _>("name")))
            .collect();
        let sql = format!("DELETE FROM {table} WHERE {}", predicates.join(" OR "));
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        for _ in &predicates {
            q = q.bind(user_id);
        }
        let result = q
            .execute(pool)
            .await
            .map(|r| r.rows_affected() as i64)
            .unwrap_or(0);
        total += result;
        seen.push(table);
    }

    Ok(StoreOutcome {
        rows_affected: total,
        detail: if seen.is_empty() {
            Some("no email/notification tables present".to_string())
        } else {
            Some(format!("tables: {}", seen.join(", ")))
        },
    })
}

/// Removes Redis keys that embed the user id. Cache keys are mostly
/// content-addressed, but rate-limit/session-style keys may carry identifiers;
/// a SCAN for the user id catches every variant without a key registry.
async fn erase_redis(user_id: &str) -> Result<StoreOutcome> {
    let redis_configured = std::env::var("REDIS_URL").is_ok();
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let client = redis::Client::open(url.as_str()).context("invalid REDIS_URL")?;
    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            if !redis_configured {
                // No Redis configured for this deployment: nothing to erase.
                // Recorded on the certificate rather than failing the job.
                return Ok(StoreOutcome {
                    rows_affected: 0,
                    detail: Some("skipped: REDIS_URL not set".to_string()),
                });
            }
            return Err(anyhow::anyhow!("connect redis: {e}"));
        }
    };

    let pattern = format!("*{user_id}*");
    let mut cursor: u64 = 0;
    let mut deleted: i64 = 0;
    loop {
        let (next, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(500)
            .query_async(&mut conn)
            .await
            .context("redis SCAN")?;
        for key in keys {
            deleted += redis::cmd("UNLINK")
                .arg(&key)
                .query_async::<i64>(&mut conn)
                .await
                .unwrap_or(0);
        }
        cursor = next;
        if cursor == 0 {
            break;
        }
    }

    Ok(StoreOutcome {
        rows_affected: deleted,
        detail: Some(format!("pattern {pattern}")),
    })
}

// ── Service ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ErasureService {
    pool: SqlitePool,
    user_keys: UserKeyService,
}

impl ErasureService {
    pub fn new(pool: SqlitePool) -> Self {
        let user_keys = UserKeyService::from_env(pool.clone());
        Self { pool, user_keys }
    }

    #[cfg(test)]
    pub fn with_user_keys(pool: SqlitePool, user_keys: UserKeyService) -> Self {
        Self { pool, user_keys }
    }

    /// Execute a full erasure for one deletion request. Idempotent: every
    /// store operation is a delete/anonymize, so retries are safe.
    pub async fn run_erasure(&self, request_id: &str) -> Result<ErasureJobView> {
        let request =
            sqlx::query("SELECT id, user_id, status FROM data_deletion_requests WHERE id = ?")
                .bind(request_id)
                .fetch_optional(&self.pool)
                .await?
                .context("deletion request not found")?;

        let status: String = request.get("status");
        if status != "scheduled" {
            anyhow::bail!("deletion request {request_id} is '{status}', expected 'scheduled'");
        }
        let user_id: String = request.get("user_id");
        let job_id = Uuid::new_v4().to_string();
        let started = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO erasure_jobs (id, deletion_request_id, user_id, status, started_at) \
             VALUES (?, ?, ?, 'running', ?)",
        )
        .bind(&job_id)
        .bind(request_id)
        .bind(&user_id)
        .bind(&started)
        .execute(&self.pool)
        .await?;

        // Run each store independently so one failure does not hide the
        // others; the job is marked failed if any store failed.
        let mut steps: Vec<ErasureStepRecord> = Vec::new();
        let mut failures: Vec<String> = Vec::new();

        let uid_keys = user_id.clone();
        let keys_svc = self.user_keys.clone();
        let stores: Vec<(
            &str,
            std::pin::Pin<Box<dyn std::future::Future<Output = Result<StoreOutcome>> + Send + '_>>,
        )> = vec![
            (
                "sqlite",
                Box::pin(erase_sqlite(&self.pool, &user_id, request_id)),
            ),
            (
                "notifications",
                Box::pin(erase_notification_stores(&self.pool, &user_id)),
            ),
            ("redis", Box::pin(erase_redis(&user_id))),
            // Crypto-shred runs last: nothing afterwards needs the DEK.
            (
                "keys",
                Box::pin(async move {
                    let shredded = keys_svc.destroy(&uid_keys).await?;
                    Ok(StoreOutcome {
                        rows_affected: i64::from(shredded),
                        detail: Some(if shredded {
                            "per-user data key destroyed".to_string()
                        } else {
                            "no per-user data key present".to_string()
                        }),
                    })
                }),
            ),
        ];

        for (name, fut) in stores {
            let step_id = Uuid::new_v4().to_string();
            let step_started = Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO erasure_steps (id, job_id, store, status, started_at) \
                 VALUES (?, ?, ?, 'running', ?)",
            )
            .bind(&step_id)
            .bind(&job_id)
            .bind(name)
            .bind(&step_started)
            .execute(&self.pool)
            .await?;

            match fut.await {
                Ok(outcome) => {
                    sqlx::query(
                        "UPDATE erasure_steps SET status = 'completed', rows_affected = ?, \
                         detail = ?, completed_at = datetime('now') WHERE id = ?",
                    )
                    .bind(outcome.rows_affected)
                    .bind(&outcome.detail)
                    .bind(&step_id)
                    .execute(&self.pool)
                    .await?;
                    steps.push(ErasureStepRecord {
                        store: name.to_string(),
                        status: "completed".to_string(),
                        rows_affected: outcome.rows_affected,
                        detail: outcome.detail,
                        error: None,
                    });
                }
                Err(e) => {
                    let msg = format!("{e:#}");
                    tracing::error!(store = name, error = %msg, "erasure step failed");
                    sqlx::query(
                        "UPDATE erasure_steps SET status = 'failed', error = ?, \
                         completed_at = datetime('now') WHERE id = ?",
                    )
                    .bind(&msg)
                    .bind(&step_id)
                    .execute(&self.pool)
                    .await?;
                    steps.push(ErasureStepRecord {
                        store: name.to_string(),
                        status: "failed".to_string(),
                        rows_affected: 0,
                        detail: None,
                        error: Some(msg.clone()),
                    });
                    failures.push(format!("{name}: {msg}"));
                }
            }
        }

        let completed = Utc::now();
        let all_ok = failures.is_empty();
        let job_status = if all_ok { "completed" } else { "failed" };
        let crypto_shredded = steps
            .iter()
            .any(|s| s.store == "keys" && s.status == "completed" && s.rows_affected > 0);

        let cert = ErasureCertificate {
            certificate_version: 1,
            request_id: request_id.to_string(),
            user_id: user_id.clone(),
            subject_hash: {
                use sha2::{Digest, Sha256};
                hex::encode(Sha256::digest(user_id.as_bytes()))
            },
            started_at: started,
            completed_at: completed.to_rfc3339(),
            status: job_status.to_string(),
            steps: steps.clone(),
            crypto_shredded,
            retention_notes: vec![
                "Audit tables (admin_audit_log, vault_audit_log, audit_log) are anonymized, \
                 not deleted, to preserve accountability records."
                    .to_string(),
                "data_deletion_requests rows are retained as the legal record of erasure."
                    .to_string(),
                "api_keys are keyed by wallet_address, not user_id; they are not covered \
                 by user-id erasure."
                    .to_string(),
            ],
        };
        let cert_json = serde_json::to_string(&cert)?;

        sqlx::query(
            "UPDATE erasure_jobs SET status = ?, completed_at = ?, certificate_json = ?, error = ? \
             WHERE id = ?",
        )
        .bind(job_status)
        .bind(completed.to_rfc3339())
        .bind(&cert_json)
        .bind(if failures.is_empty() {
            None
        } else {
            Some(failures.join("; "))
        })
        .bind(&job_id)
        .execute(&self.pool)
        .await?;

        // Only mark the request completed when every store succeeded; failed
        // jobs leave the request 'scheduled' so the next tick retries.
        // (erase_sqlite already flipped status for the success path.)
        if !all_ok {
            sqlx::query(
                "UPDATE data_deletion_requests SET status = 'scheduled', error_message = ? \
                 WHERE id = ?",
            )
            .bind(failures.join("; "))
            .bind(request_id)
            .execute(&self.pool)
            .await?;
        }

        tracing::info!(
            request_id,
            job_id = %job_id,
            status = job_status,
            "GDPR erasure job finished"
        );

        self.job_view(&job_id).await
    }

    /// Fetch the latest job (with steps + certificate) for a request.
    pub async fn latest_job(&self, request_id: &str) -> Result<Option<ErasureJobView>> {
        let job_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM erasure_jobs WHERE deletion_request_id = ? \
             ORDER BY started_at DESC LIMIT 1",
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await?;
        match job_id {
            Some(id) => self.job_view(&id).await.map(Some),
            None => Ok(None),
        }
    }

    async fn job_view(&self, job_id: &str) -> Result<ErasureJobView> {
        let row = sqlx::query(
            "SELECT id, deletion_request_id, status, started_at, completed_at, certificate_json, error \
             FROM erasure_jobs WHERE id = ?",
        )
        .bind(job_id)
        .fetch_one(&self.pool)
        .await?;

        let steps = sqlx::query(
            "SELECT store, status, rows_affected, detail, error FROM erasure_steps \
             WHERE job_id = ? ORDER BY started_at",
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| ErasureStepRecord {
            store: r.get("store"),
            status: r.get("status"),
            rows_affected: r.get("rows_affected"),
            detail: r.get("detail"),
            error: r.get("error"),
        })
        .collect();

        let cert_json: Option<String> = row.get("certificate_json");
        Ok(ErasureJobView {
            id: row.get("id"),
            deletion_request_id: row.get("deletion_request_id"),
            status: row.get("status"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
            error: row.get("error"),
            certificate: cert_json.and_then(|j| serde_json::from_str(&j).ok()),
            steps,
        })
    }

    /// Poll for due scheduled requests and run erasure for each.
    pub async fn run_due(&self) -> Result<usize> {
        let rows = sqlx::query(
            "SELECT id, scheduled_deletion_at FROM data_deletion_requests \
             WHERE status = 'scheduled' ORDER BY scheduled_deletion_at LIMIT ?",
        )
        .bind(SCHEDULER_BATCH)
        .fetch_all(&self.pool)
        .await?;

        let now = Utc::now();
        let mut ran = 0usize;
        for row in rows {
            let id: String = row.get("id");
            let scheduled: Option<String> = row.get("scheduled_deletion_at");
            let due = scheduled
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&Utc) <= now)
                .unwrap_or(false);
            // Also accept SQLite datetime('now') format as a fallback.
            let due = due
                || scheduled
                    .as_deref()
                    .and_then(|s| {
                        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok()
                    })
                    .map(|d| d.and_utc() <= now)
                    .unwrap_or(false);
            if !due {
                continue;
            }
            if let Err(e) = self.run_erasure(&id).await {
                tracing::error!(request_id = %id, error = %e, "erasure run failed");
            }
            ran += 1;
        }
        Ok(ran)
    }

    /// Background task: run due erasures every minute. Mirrors the backup
    /// scheduler's fire-and-forget style.
    pub fn spawn_scheduler(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                SCHEDULER_INTERVAL
                    .to_std()
                    .unwrap_or_else(|_| std::time::Duration::from_secs(60)),
            );
            loop {
                interval.tick().await;
                match self.run_due().await {
                    Ok(0) => {}
                    Ok(n) => tracing::info!(count = n, "erasure scheduler processed requests"),
                    Err(e) => tracing::error!(error = %e, "erasure scheduler tick failed"),
                }
            }
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::crypto::CryptoService;

    async fn test_pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for stmt in [
            "CREATE TABLE users (id TEXT PRIMARY KEY, username TEXT, password_hash TEXT)",
            "CREATE TABLE sessions (id TEXT PRIMARY KEY, user_id TEXT, refresh_token_jti TEXT, device_user_agent TEXT, ip_address TEXT)",
            "CREATE TABLE user_consents (id TEXT PRIMARY KEY, user_id TEXT, consent_type TEXT)",
            "CREATE TABLE data_deletion_requests (id TEXT PRIMARY KEY, user_id TEXT, status TEXT, scheduled_deletion_at TEXT, completed_at TEXT, cancelled_at TEXT, error_message TEXT)",
            "CREATE TABLE admin_audit_log (id TEXT PRIMARY KEY, user_id TEXT)",
        ] {
            sqlx::query(stmt).execute(&pool).await.unwrap();
        }
        sqlx::raw_sql(include_str!("../../migrations/060_gdpr_erasure.sql"))
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    async fn seed_user(pool: &SqlitePool, user_id: &str) {
        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind("alice")
            .bind("hash")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO sessions (id, user_id, refresh_token_jti, device_user_agent, ip_address) \
             VALUES ('s1', ?, 'j1', 'UA', '203.0.113.1')",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_consents (id, user_id, consent_type) VALUES ('c1', ?, 'analytics')",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO admin_audit_log (id, user_id) VALUES ('a1', ?)")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn seed_request(pool: &SqlitePool, request_id: &str, user_id: &str, scheduled: &str) {
        sqlx::query(
            "INSERT INTO data_deletion_requests (id, user_id, status, scheduled_deletion_at) \
             VALUES (?, ?, 'scheduled', ?)",
        )
        .bind(request_id)
        .bind(user_id)
        .bind(scheduled)
        .execute(pool)
        .await
        .unwrap();
    }

    fn svc(pool: SqlitePool) -> ErasureService {
        let keys = UserKeyService::new(pool.clone(), Some(CryptoService::new_for_tests()));
        ErasureService::with_user_keys(pool, keys)
    }

    #[tokio::test]
    async fn erases_everything_and_shreds_key() {
        let pool = test_pool().await;
        seed_user(&pool, "u1").await;
        // Give the user a DEK + encrypted session PII, mimicking production.
        let keys = UserKeyService::new(pool.clone(), Some(CryptoService::new_for_tests()));
        keys.encrypt_field("u1", "203.0.113.1").await.unwrap();

        seed_request(&pool, "r1", "u1", "2020-01-01T00:00:00+00:00").await;
        let view = svc(pool.clone()).run_erasure("r1").await.unwrap();
        assert_eq!(view.status, "completed");
        assert!(view.certificate.is_some());
        let cert = view.certificate.unwrap();
        assert!(cert.crypto_shredded);
        assert_eq!(cert.steps.len(), 4);

        // All user rows gone / anonymized.
        for t in ["sessions", "user_consents"] {
            let n: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT COUNT(*) FROM {t} WHERE user_id = 'u1'"
            )))
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(n, 0, "{t} still has rows");
        }
        let uname: String = sqlx::query_scalar("SELECT username FROM users WHERE id = 'u1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(uname.starts_with("erased_"));
        let audit: String =
            sqlx::query_scalar("SELECT user_id FROM admin_audit_log WHERE id = 'a1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(audit, "erased");
        // DEK destroyed.
        assert!(!keys.has_key("u1").await.unwrap());
        // Request marked completed.
        let status: String =
            sqlx::query_scalar("SELECT status FROM data_deletion_requests WHERE id = 'r1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "completed");
    }

    #[tokio::test]
    async fn not_due_requests_are_skipped() {
        let pool = test_pool().await;
        seed_user(&pool, "u2").await;
        seed_request(&pool, "r2", "u2", "2999-01-01T00:00:00+00:00").await;
        let ran = svc(pool.clone()).run_due().await.unwrap();
        assert_eq!(ran, 0);
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM erasure_jobs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn due_request_runs_via_scheduler_path() {
        let pool = test_pool().await;
        seed_user(&pool, "u3").await;
        seed_request(&pool, "r3", "u3", "2020-01-01T00:00:00+00:00").await;
        let ran = svc(pool.clone()).run_due().await.unwrap();
        assert_eq!(ran, 1);
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = 'u3'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn missing_request_errors() {
        let pool = test_pool().await;
        assert!(svc(pool).run_erasure("nope").await.is_err());
    }
}
