use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub enabled: bool,
    pub db_path: String,
    pub backup_dir: String,
    pub keep_days: i64,
    pub schedule_hour_utc: u32,
}

impl BackupConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let enabled = std::env::var("BACKUP_ENABLED")
            .ok()
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
        let db_path = std::env::var("BACKUP_DB_PATH")
            .ok()
            .or_else(|| sqlite_path_from_database_url(&database_url))
            .unwrap_or_else(|| "stellar_insights.db".to_string());

        let backup_dir = std::env::var("BACKUP_DIR").unwrap_or_else(|_| "./backups".to_string());
        let keep_days = std::env::var("BACKUP_RETENTION_DAYS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(30);
        let schedule_hour_utc = std::env::var("BACKUP_SCHEDULE_HOUR_UTC")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(2)
            .min(23);

        Self {
            enabled,
            db_path,
            backup_dir,
            keep_days,
            schedule_hour_utc,
        }
    }
}

fn sqlite_path_from_database_url(url: &str) -> Option<String> {
    let stripped = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))?;

    if stripped == ":memory:" || stripped == "memory:" {
        return None;
    }

    Some(stripped.to_string())
}

#[derive(Debug, Clone)]
pub struct BackupVerificationResult {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub checksum_ok: bool,
    pub integrity_ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackupManager {
    config: BackupConfig,
}

impl BackupManager {
    #[must_use]
    pub const fn new(config: BackupConfig) -> Self {
        Self { config }
    }

    pub async fn create_backup(&self) -> Result<PathBuf> {
        tokio::fs::create_dir_all(&self.config.backup_dir)
            .await
            .context("Failed to create backup directory")?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("stellar_insights_{}.db", timestamp);
        let destination = Path::new(&self.config.backup_dir).join(filename);

        tokio::fs::copy(&self.config.db_path, &destination)
            .await
            .with_context(|| {
                format!(
                    "Failed to copy database '{}' to backup '{}'",
                    self.config.db_path,
                    destination.display()
                )
            })?;

        if let Ok(meta) = tokio::fs::metadata(&destination).await {
            crate::observability::metrics::set_backup_size_bytes(meta.len());
        }

        tracing::info!(path = %destination.display(), "Database backup created");
        Ok(destination)
    }

    pub async fn cleanup_old_backups(&self) -> Result<u32> {
        let cutoff = Utc::now() - Duration::days(self.config.keep_days);
        let mut removed = 0u32;

        let mut entries = match tokio::fs::read_dir(&self.config.backup_dir).await {
            Ok(entries) => entries,
            Err(_) => return Ok(0),
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;

            if !metadata.is_file() {
                continue;
            }

            let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let modified_utc: DateTime<Utc> = modified.into();
            if modified_utc < cutoff {
                tokio::fs::remove_file(&path).await.with_context(|| {
                    format!("Failed removing expired backup '{}'", path.display())
                })?;
                removed += 1;
                tracing::info!(path = %path.display(), "Old backup removed");
            }
        }

        Ok(removed)
    }

    /// Verifies a backup file by:
    /// 1. Checking the file exists and is non-empty
    /// 2. Computing SHA-256 checksum and comparing to a sidecar `.sha256` file (if present)
    /// 3. Opening the backup as a read-only SQLite connection and running `PRAGMA integrity_check`
    pub async fn verify_backup(&self, backup_path: &Path) -> Result<BackupVerificationResult> {
        // --- existence & size ---
        let metadata = tokio::fs::metadata(backup_path).await.with_context(|| {
            format!("Backup file not found: {}", backup_path.display())
        })?;

        if metadata.len() == 0 {
            return Ok(BackupVerificationResult {
                path: backup_path.to_path_buf(),
                size_bytes: 0,
                checksum_ok: false,
                integrity_ok: false,
                error: Some("Backup file is empty".to_string()),
            });
        }

        // --- checksum ---
        let bytes = tokio::fs::read(backup_path).await.with_context(|| {
            format!("Failed to read backup for checksum: {}", backup_path.display())
        })?;
        let digest = Sha256::digest(&bytes);
        let computed = hex::encode(digest);

        let sidecar = backup_path.with_extension("db.sha256");
        let checksum_ok = if sidecar.exists() {
            let stored = tokio::fs::read_to_string(&sidecar).await.unwrap_or_default();
            stored.trim() == computed
        } else {
            // Write sidecar for future verifications
            let _ = tokio::fs::write(&sidecar, &computed).await;
            true // first time — treat as ok
        };

        if !checksum_ok {
            tracing::warn!(
                path = %backup_path.display(),
                "Backup checksum mismatch — file may be corrupted"
            );
            crate::observability::metrics::record_backup_verification_failure("checksum_mismatch");
            return Ok(BackupVerificationResult {
                path: backup_path.to_path_buf(),
                size_bytes: metadata.len(),
                checksum_ok: false,
                integrity_ok: false,
                error: Some("Checksum mismatch".to_string()),
            });
        }

        // --- SQLite integrity check via restore test ---
        let integrity_ok = self.run_restore_test(backup_path).await.unwrap_or_else(|e| {
            tracing::warn!(error = %e, path = %backup_path.display(), "Restore test failed");
            false
        });

        if integrity_ok {
            tracing::info!(
                path = %backup_path.display(),
                size_bytes = metadata.len(),
                checksum = %computed,
                "Backup verification passed"
            );
            crate::observability::metrics::record_backup_verification_success();
        } else {
            crate::observability::metrics::record_backup_verification_failure("integrity_check_failed");
        }

        Ok(BackupVerificationResult {
            path: backup_path.to_path_buf(),
            size_bytes: metadata.len(),
            checksum_ok,
            integrity_ok,
            error: if integrity_ok { None } else { Some("SQLite integrity check failed".to_string()) },
        })
    }

    /// Opens the backup as a temporary read-only SQLite database and runs
    /// `PRAGMA integrity_check` to confirm the file is a valid, uncorrupted database.
    async fn run_restore_test(&self, backup_path: &Path) -> Result<bool> {
        let url = format!("sqlite://{}?mode=ro", backup_path.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .with_context(|| format!("Could not open backup for restore test: {}", backup_path.display()))?;

        let row: (String,) = sqlx::query_as("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await
            .context("integrity_check query failed")?;

        pool.close().await;

        let ok = row.0.trim().eq_ignore_ascii_case("ok");
        tracing::info!(
            path = %backup_path.display(),
            result = %row.0,
            "SQLite integrity_check completed"
        );
        Ok(ok)
    }

    pub async fn run_once(&self) -> Result<()> {
        let backup_path = self.create_backup().await?;
        let cleaned = self.cleanup_old_backups().await?;

        // Verify the backup we just created
        match self.verify_backup(&backup_path).await {
            Ok(result) if result.integrity_ok && result.checksum_ok => {
                tracing::info!(
                    backup = %backup_path.display(),
                    size_bytes = result.size_bytes,
                    removed_old_backups = cleaned,
                    "Backup run completed and verified"
                );
            }
            Ok(result) => {
                tracing::error!(
                    backup = %backup_path.display(),
                    checksum_ok = result.checksum_ok,
                    integrity_ok = result.integrity_ok,
                    error = ?result.error,
                    "Backup created but verification FAILED"
                );
            }
            Err(e) => {
                tracing::error!(error = %e, backup = %backup_path.display(), "Backup verification error");
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn spawn_scheduler(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let wait = duration_until_next_hour(self.config.schedule_hour_utc);
                tokio::time::sleep(wait).await;

                if let Err(error) = self.run_once().await {
                    tracing::error!(error = %error, "Scheduled backup failed");
                }
            }
        })
    }
}

fn duration_until_next_hour(hour_utc: u32) -> std::time::Duration {
    let now = Utc::now();

    let today_target = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), hour_utc, 0, 0)
        .single();

    let next_target = match today_target {
        Some(target) if now < target => target,
        _ => {
            let tomorrow = now + Duration::days(1);
            Utc.with_ymd_and_hms(
                tomorrow.year(),
                tomorrow.month(),
                tomorrow.day(),
                hour_utc,
                0,
                0,
            )
            .single()
            .unwrap_or(now + Duration::hours(24))
        }
    };

    (next_target - now)
        .to_std()
        .unwrap_or(std::time::Duration::from_secs(60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sqlite_path() {
        assert_eq!(
            sqlite_path_from_database_url("sqlite://stellar_insights.db"),
            Some("stellar_insights.db".to_string())
        );
        assert_eq!(
            sqlite_path_from_database_url("sqlite:./stellar_insights.db"),
            Some("./stellar_insights.db".to_string())
        );
        assert_eq!(sqlite_path_from_database_url("sqlite::memory:"), None);
    }

    #[test]
    fn parses_sqlite_memory_variants() {
        assert_eq!(sqlite_path_from_database_url("sqlite://memory:"), None);
        assert_eq!(sqlite_path_from_database_url("sqlite:memory:"), None);
    }

    #[test]
    fn parses_unknown_url_scheme_returns_none() {
        assert_eq!(
            sqlite_path_from_database_url("postgres://localhost/db"),
            None
        );
        assert_eq!(sqlite_path_from_database_url(""), None);
    }

    #[test]
    fn backup_config_defaults() {
        let config = BackupConfig {
            enabled: false,
            db_path: "test.db".to_string(),
            backup_dir: "./backups".to_string(),
            keep_days: 30,
            schedule_hour_utc: 2,
        };
        assert!(!config.enabled);
        assert_eq!(config.keep_days, 30);
        assert_eq!(config.schedule_hour_utc, 2);
    }

    #[test]
    fn duration_until_next_hour_is_positive() {
        // Whatever the current time, the wait should always be > 0 and <= 24h.
        let wait = duration_until_next_hour(3);
        assert!(wait.as_secs() > 0);
        assert!(wait.as_secs() <= 86_400);
    }

    #[test]
    fn duration_until_next_hour_all_hours() {
        // Smoke-test every valid hour value.
        for h in 0u32..24 {
            let wait = duration_until_next_hour(h);
            assert!(wait.as_secs() <= 86_400, "hour {h} produced wait > 24h");
        }
    }

    #[tokio::test]
    async fn create_backup_copies_file() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("source.db");
        let backup_dir = dir.path().join("backups");

        tokio::fs::write(&db_path, b"sqlite-data").await.unwrap();

        let config = BackupConfig {
            enabled: true,
            db_path: db_path.to_str().unwrap().to_string(),
            backup_dir: backup_dir.to_str().unwrap().to_string(),
            keep_days: 7,
            schedule_hour_utc: 2,
        };

        let manager = BackupManager::new(config);
        let result = manager.create_backup().await;
        assert!(result.is_ok());

        let backup_path = result.unwrap();
        assert!(backup_path.exists());
        let contents = tokio::fs::read(&backup_path).await.unwrap();
        assert_eq!(contents, b"sqlite-data");
    }

    #[tokio::test]
    async fn create_backup_fails_when_source_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config = BackupConfig {
            enabled: true,
            db_path: dir
                .path()
                .join("nonexistent.db")
                .to_str()
                .unwrap()
                .to_string(),
            backup_dir: dir.path().join("backups").to_str().unwrap().to_string(),
            keep_days: 7,
            schedule_hour_utc: 2,
        };

        let manager = BackupManager::new(config);
        assert!(manager.create_backup().await.is_err());
    }

    #[tokio::test]
    async fn cleanup_old_backups_returns_zero_when_dir_missing() {
        let config = BackupConfig {
            enabled: true,
            db_path: "test.db".to_string(),
            backup_dir: "/tmp/nonexistent_backup_dir_xyz".to_string(),
            keep_days: 30,
            schedule_hour_utc: 2,
        };

        let manager = BackupManager::new(config);
        let removed = manager.cleanup_old_backups().await.unwrap();
        assert_eq!(removed, 0);
    }

    #[tokio::test]
    async fn run_once_creates_backup_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("source.db");
        let backup_dir = dir.path().join("backups");

        tokio::fs::write(&db_path, b"data").await.unwrap();

        let config = BackupConfig {
            enabled: true,
            db_path: db_path.to_str().unwrap().to_string(),
            backup_dir: backup_dir.to_str().unwrap().to_string(),
            keep_days: 30,
            schedule_hour_utc: 2,
        };

        let manager = BackupManager::new(config);
        assert!(manager.run_once().await.is_ok());
    }
}
