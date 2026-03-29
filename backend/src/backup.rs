use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};
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

    pub async fn run_once(&self) -> Result<()> {
        let backup_path = self.create_backup().await?;
        let cleaned = self.cleanup_old_backups().await?;

        tracing::info!(
            backup = %backup_path.display(),
            removed_old_backups = cleaned,
            "Backup run completed"
        );

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
        assert_eq!(sqlite_path_from_database_url("postgres://localhost/db"), None);
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
            db_path: dir.path().join("nonexistent.db").to_str().unwrap().to_string(),
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
