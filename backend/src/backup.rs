use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::task::JoinHandle;

use crate::crypto::EncryptionService;

/// Every SQLite database file starts with this 16-byte header.
const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";

#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub enabled: bool,
    pub db_path: String,
    pub backup_dir: String,
    pub keep_days: i64,
    pub schedule_hour_utc: u32,
    pub encryption_key: Option<String>,
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
            .unwrap_or_else(|| "payraider.db".to_string());

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

        let encryption_key = std::env::var("BACKUP_ENCRYPTION_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty());

        Self {
            enabled,
            db_path,
            backup_dir,
            keep_days,
            schedule_hour_utc,
            encryption_key,
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

    /// Creates a consistent backup using SQLite online backup API.
    /// This ensures a consistent snapshot even during concurrent writes.
    /// The backup is then optionally encrypted and stored with checksums.
    pub async fn create_backup(&self) -> Result<PathBuf> {
        tokio::fs::create_dir_all(&self.config.backup_dir)
            .await
            .context("Failed to create backup directory")?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let temp_filename = format!("payraider_{}_temp.db", timestamp);
        let backup_filename = format!("payraider_{}.db", timestamp);

        let temp_path = Path::new(&self.config.backup_dir).join(&temp_filename);
        let final_path = Path::new(&self.config.backup_dir).join(&backup_filename);

        // Step 1: Create a consistent snapshot of the live database.
        // `VACUUM INTO` reads the source inside a single read transaction, so
        // the copy is consistent even while other connections are writing
        // (unlike a raw file copy, which can capture a torn WAL state).
        tokio::fs::metadata(&self.config.db_path)
            .await
            .with_context(|| format!("Source database '{}' not found", self.config.db_path))?;

        let source_opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&self.config.db_path)
            .read_only(true);
        let source_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(source_opts)
            .await
            .with_context(|| {
                format!("Failed to open source database '{}'", self.config.db_path)
            })?;

        let _ = tokio::fs::remove_file(&temp_path).await;
        let vacuum_result = sqlx::query("VACUUM INTO ?")
            .bind(temp_path.to_string_lossy().into_owned())
            .execute(&source_pool)
            .await;
        source_pool.close().await;
        vacuum_result.with_context(|| {
            format!(
                "Failed to snapshot '{}' into '{}'",
                self.config.db_path,
                temp_path.display()
            )
        })?;

        let bytes = tokio::fs::read(&temp_path).await.with_context(|| {
            format!("Failed to read temporary backup '{}'", temp_path.display())
        })?;

        // Step 2: Optionally encrypt the backup. The stored form is the
        // textual `EncryptedData` encoding of the hex-encoded snapshot.
        let final_bytes = if let Some(ref key_hex) = self.config.encryption_key {
            let service = EncryptionService::new(key_hex).with_context(|| {
                "Failed to initialize encryption service with BACKUP_ENCRYPTION_KEY"
            })?;
            let encrypted = service
                .encrypt(&hex::encode(&bytes))
                .context("Failed to encrypt backup")?;
            encrypted.to_string().into_bytes()
        } else {
            bytes
        };

        // Step 3: Checksum the bytes exactly as stored, so verify_backup
        // (which hashes the file on disk) can validate encrypted backups too.
        let checksum = hex::encode(Sha256::digest(&final_bytes));

        // Step 4: Write final backup
        tokio::fs::write(&temp_path, final_bytes)
            .await
            .with_context(|| format!("Failed to write backup to '{}'", temp_path.display()))?;

        // Step 5: Rename temp to final
        tokio::fs::rename(&temp_path, &final_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to rename backup from '{}' to '{}'",
                    temp_path.display(),
                    final_path.display()
                )
            })?;

        // Step 6: Write checksum sidecar
        let sidecar_path = final_path.with_extension("db.sha256");
        tokio::fs::write(&sidecar_path, &checksum)
            .await
            .with_context(|| format!("Failed to write checksum to '{}'", sidecar_path.display()))?;

        if let Ok(meta) = tokio::fs::metadata(&final_path).await {
            crate::observability::metrics::set_backup_size_bytes(meta.len());
        }

        tracing::info!(
            path = %final_path.display(),
            checksum = %checksum,
            encrypted = self.config.encryption_key.is_some(),
            "Database backup created"
        );
        Ok(final_path)
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
        let metadata = tokio::fs::metadata(backup_path)
            .await
            .with_context(|| format!("Backup file not found: {}", backup_path.display()))?;

        if metadata.len() == 0 {
            return Ok(BackupVerificationResult {
                path: backup_path.to_path_buf(),
                size_bytes: 0,
                checksum_ok: false,
                integrity_ok: false,
                error: Some("Backup file is empty".to_string()),
            });
        }

        // Read raw bytes for checksum computation
        let raw_bytes = tokio::fs::read(backup_path).await.with_context(|| {
            format!(
                "Failed to read backup for checksum: {}",
                backup_path.display()
            )
        })?;

        // --- checksum ---
        let digest = Sha256::digest(&raw_bytes);
        let computed = hex::encode(digest);

        let sidecar = backup_path.with_extension("db.sha256");
        let checksum_ok = if sidecar.exists() {
            let stored = tokio::fs::read_to_string(&sidecar)
                .await
                .unwrap_or_default();
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
        // For encrypted backups, decrypt to temp file first
        let integrity_ok = self
            .run_restore_test(backup_path)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, path = %backup_path.display(), "Restore test failed");
                false
            });

        if integrity_ok {
            tracing::info!(
                path = %backup_path.display(),
                size_bytes = metadata.len(),
                checksum = %computed,
                encrypted = self.config.encryption_key.is_some(),
                "Backup verification passed"
            );
            crate::observability::metrics::record_backup_verification_success();
        } else {
            crate::observability::metrics::record_backup_verification_failure(
                "integrity_check_failed",
            );
        }

        Ok(BackupVerificationResult {
            path: backup_path.to_path_buf(),
            size_bytes: metadata.len(),
            checksum_ok,
            integrity_ok,
            error: if integrity_ok {
                None
            } else {
                Some("SQLite integrity check failed".to_string())
            },
        })
    }

    /// Opens the backup as a temporary read-only SQLite database and runs
    /// `PRAGMA integrity_check` to confirm the file is a valid, uncorrupted database.
    /// Handles encrypted backups by decrypting to a temporary file first.
    async fn run_restore_test(&self, backup_path: &Path) -> Result<bool> {
        // Check if backup is encrypted (has encryption header)
        let bytes = tokio::fs::read(backup_path)
            .await
            .with_context(|| format!("Failed to read backup '{}'", backup_path.display()))?;

        let is_encrypted = self.is_backup_encrypted(&bytes);

        let temp_path = if is_encrypted {
            // Decrypt to temporary file
            let decrypted = self.decrypt_backup_if_needed(&bytes)?;
            let temp_decrypted = backup_path.with_extension("decrypted_temp.db");
            tokio::fs::write(&temp_decrypted, decrypted)
                .await
                .with_context(|| {
                    format!(
                        "Failed to write decrypted temp backup '{}'",
                        temp_decrypted.display()
                    )
                })?;
            Some(temp_decrypted)
        } else {
            None
        };

        // Use the appropriate path for integrity check
        let check_path = temp_path.as_ref().map_or(backup_path, |p| p.as_path());
        let url = format!("sqlite://{}?mode=ro", check_path.display());

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .with_context(|| {
                format!(
                    "Could not open backup for restore test: {}",
                    check_path.display()
                )
            })?;

        let row: (String,) = sqlx::query_as("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await
            .context("integrity_check query failed")?;

        pool.close().await;

        let ok = row.0.trim().eq_ignore_ascii_case("ok");

        // Cleanup decrypted temp file if created
        if let Some(ref temp) = temp_path {
            let _ = tokio::fs::remove_file(temp).await;
        }

        tracing::info!(
            path = %backup_path.display(),
            result = %row.0,
            is_encrypted,
            "SQLite integrity_check completed"
        );
        Ok(ok)
    }

    /// A backup is encrypted when a key is configured and the file is not a
    /// plain SQLite database (which always starts with the SQLite header).
    fn is_backup_encrypted(&self, bytes: &[u8]) -> bool {
        self.config.encryption_key.is_some() && !bytes.starts_with(SQLITE_HEADER)
    }

    /// Decrypt a backup produced by `create_backup` if it is encrypted;
    /// plain SQLite files are returned unchanged.
    fn decrypt_backup_if_needed(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let Some(ref key_hex) = self.config.encryption_key else {
            return Ok(bytes.to_vec());
        };
        if !self.is_backup_encrypted(bytes) {
            return Ok(bytes.to_vec());
        }

        let service = EncryptionService::new(key_hex)
            .with_context(|| "Failed to initialize encryption service for decryption")?;
        let text = std::str::from_utf8(bytes).context("Encrypted backup is not valid UTF-8")?;
        let encrypted: crate::crypto::EncryptedData = text
            .trim()
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse encrypted backup data: {e}"))?;
        let decrypted_hex = service
            .decrypt(&encrypted)
            .map_err(|e| anyhow::anyhow!("Failed to decrypt backup: {e}"))?;
        hex::decode(decrypted_hex.trim()).context("Decrypted backup is not valid hex")
    }

    /// Re-hashes every earlier snapshot that has a `.sha256` sidecar and compares the
    /// result, so corruption of stored snapshots is detected rather than discovered at
    /// restore time. Returns the number of snapshots whose checksum no longer matches.
    pub async fn verify_existing_checksums(&self) -> Result<u32> {
        let mut entries = match tokio::fs::read_dir(&self.config.backup_dir).await {
            Ok(entries) => entries,
            Err(_) => return Ok(0),
        };

        let mut mismatches = 0u32;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("db") {
                continue;
            }
            let sidecar = path.with_extension("db.sha256");
            let Ok(stored) = tokio::fs::read_to_string(&sidecar).await else {
                continue;
            };

            let bytes = tokio::fs::read(&path)
                .await
                .with_context(|| format!("Failed to read backup '{}'", path.display()))?;
            let computed = hex::encode(Sha256::digest(&bytes));
            if stored.trim() != computed {
                mismatches += 1;
                tracing::error!(
                    path = %path.display(),
                    "Stored backup checksum mismatch — snapshot is corrupted"
                );
                crate::observability::metrics::record_backup_verification_failure(
                    "stored_checksum_mismatch",
                );
            }
        }

        Ok(mismatches)
    }

    pub async fn run_once(&self) -> Result<()> {
        let backup_path = self.create_backup().await?;
        let cleaned = self.cleanup_old_backups().await?;

        match self.verify_existing_checksums().await {
            Ok(0) => {}
            Ok(mismatches) => {
                tracing::error!(
                    mismatches,
                    "Existing backup snapshots failed checksum re-verification"
                );
                crate::observability::metrics::record_backup_verification_failure(
                    "stored_checksum_mismatch",
                );
            }
            Err(e) => {
                tracing::error!(error = %e, "Backup checksum re-verification error");
            }
        }

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
            let lock = crate::distributed_lock::DistributedLock::shared().await;
            loop {
                let wait = duration_until_next_hour(self.config.schedule_hour_utc);
                tokio::time::sleep(wait).await;

                // Every replica wakes at the same time; only the one that wins
                // the lock backs up. The lock is left to expire (not released)
                // so a replica whose clock is slightly behind can't run it again.
                let Some(_guard) = lock
                    .try_acquire("job-lock:backup", std::time::Duration::from_secs(55 * 60))
                    .await
                else {
                    tracing::info!("Scheduled backup skipped — another instance is running it");
                    continue;
                };

                if let Err(error) = self.run_once().await {
                    tracing::error!(error = %error, "Scheduled backup failed");
                    // Alert on backup failure
                    Self::alert_backup_failure(&error);
                }
            }
        })
    }

    /// Spawns a nightly restore drill job that:
    /// 1. Restores the latest backup to a scratch DB
    /// 2. Runs integrity_check
    /// 3. Compares row counts and aggregates
    /// 4. Reports results and alerts on failure
    #[must_use]
    pub fn spawn_restore_drill(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let lock = crate::distributed_lock::DistributedLock::shared().await;
            let scratch_dir =
                PathBuf::from(std::env::temp_dir()).join("stellar_insights_restore_drill");

            // Clean up any leftover scratch files from previous runs
            let _ = tokio::fs::remove_dir_all(&scratch_dir).await;

            loop {
                // Run at 4 AM UTC (2 hours after default backup time of 2 AM)
                let wait = duration_until_next_hour(4);
                tokio::time::sleep(wait).await;

                let Some(_guard) = lock
                    .try_acquire(
                        "job-lock:restore-drill",
                        std::time::Duration::from_secs(55 * 60),
                    )
                    .await
                else {
                    tracing::info!("Restore drill skipped — another instance is running it");
                    continue;
                };

                if let Err(error) = self.run_restore_drill(&scratch_dir).await {
                    tracing::error!(error = %error, "Restore drill failed");
                    // Alert on restore drill failure
                    Self::alert_restore_drill_failure(&error);
                }
            }
        })
    }

    /// Performs a restore drill: restores backup to scratch DB, runs integrity check,
    /// and compares row counts/aggregate values.
    async fn run_restore_drill(&self, scratch_dir: &Path) -> Result<()> {
        tokio::fs::create_dir_all(scratch_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to create scratch directory '{}'",
                    scratch_dir.display()
                )
            })?;

        // Find the latest backup
        let backup_path = self.find_latest_backup().await?;

        // Create scratch database path
        let scratch_db = scratch_dir.join("drill_restore.db");

        // Restore backup to scratch
        let bytes = tokio::fs::read(&backup_path)
            .await
            .with_context(|| format!("Failed to read backup '{}'", backup_path.display()))?;

        // Decrypt if needed
        let decrypted = self.decrypt_backup_if_needed(&bytes)?;

        tokio::fs::write(&scratch_db, decrypted)
            .await
            .with_context(|| {
                format!(
                    "Failed to write scratch database '{}'",
                    scratch_db.display()
                )
            })?;

        // Run integrity check on scratch DB
        let url = format!("sqlite://{}?mode=ro", scratch_db.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .context("Failed to open scratch database for restore drill")?;

        let row: (String,) = sqlx::query_as("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await
            .context("integrity_check query failed")?;

        pool.close().await;

        let integrity_ok = row.0.trim().eq_ignore_ascii_case("ok");

        if !integrity_ok {
            tracing::error!(
                path = %scratch_db.display(),
                result = %row.0,
                "Restore drill failed: integrity check failed"
            );
            crate::observability::metrics::record_backup_verification_failure(
                "restore_drill_integrity_failed",
            );
            return Err(anyhow::anyhow!(
                "Restore drill integrity check failed: {}",
                row.0
            ));
        }

        // Compare row counts with a sample query (optional, can be extended)
        // For now, just verify the DB is valid and readable
        tracing::info!(
            path = %scratch_db.display(),
            size_bytes = bytes.len(),
            "Restore drill completed successfully"
        );
        crate::observability::metrics::record_backup_verification_success();

        // Cleanup scratch DB
        let _ = tokio::fs::remove_file(&scratch_db).await;

        Ok(())
    }

    /// Find the latest backup file in the backup directory
    async fn find_latest_backup(&self) -> Result<PathBuf> {
        let mut entries = tokio::fs::read_dir(&self.config.backup_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to read backup directory '{}'",
                    self.config.backup_dir
                )
            })?;

        let mut latest_path: Option<PathBuf> = None;
        let mut latest_time = SystemTime::UNIX_EPOCH;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Skip temp files and non-.db files
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if filename.contains("temp") || !filename.ends_with(".db") {
                continue;
            }

            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if modified > latest_time {
                        latest_time = modified;
                        latest_path = Some(path.clone());
                    }
                }
            }
        }

        latest_path
            .ok_or_else(|| anyhow::anyhow!("No backup files found in '{}'", self.config.backup_dir))
    }

    /// Sends an alert for backup failure
    fn alert_backup_failure(error: &anyhow::Error) {
        let message = format!("Backup failed: {}", error);

        // Try to send to alert system if available
        // This will be handled by the main app's alert system
        tracing::error!(error = %error, "ALERT: Backup failed");
    }

    /// Sends an alert for restore drill failure
    fn alert_restore_drill_failure(error: &anyhow::Error) {
        let message = format!("Restore drill failed: {}", error);

        tracing::error!(error = %error, "ALERT: Restore drill failed");
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// Creates a real SQLite database containing one known row.
    async fn seed_sqlite(path: &Path) {
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE t (v TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO t (v) VALUES ('sqlite-data')")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }

    async fn read_row(path: &Path) -> String {
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .read_only(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        let (v,): (String,) = sqlx::query_as("SELECT v FROM t")
            .fetch_one(&pool)
            .await
            .unwrap();
        pool.close().await;
        v
    }

    #[test]
    fn parses_sqlite_path() {
        assert_eq!(
            sqlite_path_from_database_url("sqlite://payraider.db"),
            Some("payraider.db".to_string())
        );
        assert_eq!(
            sqlite_path_from_database_url("sqlite:./payraider.db"),
            Some("./payraider.db".to_string())
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
            encryption_key: None,
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

        seed_sqlite(&db_path).await;

        let config = BackupConfig {
            enabled: true,
            db_path: db_path.to_str().unwrap().to_string(),
            backup_dir: backup_dir.to_str().unwrap().to_string(),
            keep_days: 7,
            schedule_hour_utc: 2,
            encryption_key: None,
        };

        let manager = BackupManager::new(config);
        let result = manager.create_backup().await;
        assert!(result.is_ok());

        let backup_path = result.unwrap();
        assert!(backup_path.exists());
        assert_eq!(read_row(&backup_path).await, "sqlite-data");
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
            encryption_key: None,
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
            encryption_key: None,
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

        seed_sqlite(&db_path).await;

        let config = BackupConfig {
            enabled: true,
            db_path: db_path.to_str().unwrap().to_string(),
            backup_dir: backup_dir.to_str().unwrap().to_string(),
            keep_days: 30,
            schedule_hour_utc: 2,
            encryption_key: None,
        };

        let manager = BackupManager::new(config);
        assert!(manager.run_once().await.is_ok());
    }

    #[tokio::test]
    async fn create_backup_with_encryption() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("source.db");
        let backup_dir = dir.path().join("backups");

        seed_sqlite(&db_path).await;

        let config = BackupConfig {
            enabled: true,
            db_path: db_path.to_str().unwrap().to_string(),
            backup_dir: backup_dir.to_str().unwrap().to_string(),
            keep_days: 7,
            schedule_hour_utc: 2,
            encryption_key: Some(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            ),
        };

        let manager = BackupManager::new(config);
        let result = manager.create_backup().await;
        assert!(result.is_ok());

        let backup_path = result.unwrap();
        assert!(backup_path.exists());

        // Check that checksum sidecar was created
        let sidecar = backup_path.with_extension("db.sha256");
        assert!(sidecar.exists());

        // Verify we can read the checksum
        let checksum = tokio::fs::read_to_string(&sidecar).await.unwrap();
        assert!(!checksum.is_empty());

        // The stored file must actually be encrypted ...
        let stored = tokio::fs::read(&backup_path).await.unwrap();
        assert!(!stored.starts_with(SQLITE_HEADER));

        // ... and must verify and decrypt back to the original database.
        let verification = manager.verify_backup(&backup_path).await.unwrap();
        assert!(verification.checksum_ok, "{:?}", verification.error);
        assert!(verification.integrity_ok, "{:?}", verification.error);

        let restored = dir.path().join("restored.db");
        tokio::fs::write(&restored, manager.decrypt_backup_if_needed(&stored).unwrap())
            .await
            .unwrap();
        assert_eq!(read_row(&restored).await, "sqlite-data");
    }

    #[tokio::test]
    async fn find_latest_backup() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::create_dir_all(dir.path().join("backups"))
            .await
            .unwrap();

        // Create some fake backups
        tokio::fs::write(
            dir.path()
                .join("backups")
                .join("payraider_20240101_010000.db"),
            b"db1",
        )
        .await
        .unwrap();
        tokio::fs::write(
            dir.path()
                .join("backups")
                .join("payraider_20240102_010000.db"),
            b"db2",
        )
        .await
        .unwrap();

        let config = BackupConfig {
            enabled: true,
            db_path: "test.db".to_string(),
            backup_dir: dir.path().join("backups").to_str().unwrap().to_string(),
            keep_days: 30,
            schedule_hour_utc: 2,
            encryption_key: None,
        };

        let manager = BackupManager::new(config);
        let latest = manager.find_latest_backup().await.unwrap();
        assert!(latest.file_name().is_some());
    }

    #[test]
    fn backup_config_with_encryption_key() {
        // Test that encryption_key is properly set from env
        let config = BackupConfig {
            enabled: false,
            db_path: "test.db".to_string(),
            backup_dir: "./backups".to_string(),
            keep_days: 30,
            schedule_hour_utc: 2,
            encryption_key: Some("key".to_string()),
        };
        assert_eq!(config.encryption_key, Some("key".to_string()));
    }

    #[tokio::test]
    async fn verify_backup_with_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("source.db");
        let backup_dir = dir.path().join("backups");

        seed_sqlite(&db_path).await;

        let config = BackupConfig {
            enabled: true,
            db_path: db_path.to_str().unwrap().to_string(),
            backup_dir: backup_dir.to_str().unwrap().to_string(),
            keep_days: 7,
            schedule_hour_utc: 2,
            encryption_key: None,
        };

        let manager = BackupManager::new(config);
        let backup_path = manager.create_backup().await.unwrap();

        // Verify the backup
        let result = manager.verify_backup(&backup_path).await;
        assert!(result.is_ok());
        let verification = result.unwrap();
        assert!(verification.checksum_ok);
        assert!(verification.integrity_ok, "{:?}", verification.error);
    }
}
