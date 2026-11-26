//! End-to-end backup/restore coverage for `BackupManager` (issue #1857).
//!
//! Exercises the full incident path against a real file on disk: take a backup,
//! destroy the live database, restore from the backup, and confirm the data
//! survived — rather than only asserting that the scheduler fires.

use payraider_backend::backup::{BackupConfig, BackupManager};
use std::path::Path;
use tempfile::tempdir;

const LIVE_VALUE: &str = "payraider-live-data";

/// Writes a real SQLite database holding one known row. Backups are taken
/// with `VACUUM INTO`, so the source must be a genuine database.
async fn seed_live_db(path: &Path) {
    let opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("open live db");
    sqlx::query("CREATE TABLE data (v TEXT NOT NULL)")
        .execute(&pool)
        .await
        .expect("create table");
    sqlx::query("INSERT INTO data (v) VALUES (?)")
        .bind(LIVE_VALUE)
        .execute(&pool)
        .await
        .expect("insert row");
    pool.close().await;
}

async fn read_value(path: &Path) -> String {
    let opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(path)
        .read_only(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("open db");
    let (v,): (String,) = sqlx::query_as("SELECT v FROM data")
        .fetch_one(&pool)
        .await
        .expect("read row");
    pool.close().await;
    v
}

fn config(db_path: &Path, backup_dir: &Path) -> BackupConfig {
    BackupConfig {
        enabled: true,
        db_path: db_path.to_string_lossy().into_owned(),
        backup_dir: backup_dir.to_string_lossy().into_owned(),
        keep_days: 30,
        schedule_hour_utc: 2,
        encryption_key: None,
    }
}

#[tokio::test]
async fn backup_then_restore_preserves_data() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("payraider.db");
    let backup_dir = dir.path().join("backups");
    seed_live_db(&db_path).await;

    let manager = BackupManager::new(config(&db_path, &backup_dir));

    // 1. Take a backup of the live database.
    let backup_path = manager
        .create_backup()
        .await
        .expect("backup should succeed");
    assert!(backup_path.exists(), "backup file should exist on disk");

    // 2. Simulate the incident: the live database is lost.
    tokio::fs::remove_file(&db_path)
        .await
        .expect("drop live db");
    assert!(!db_path.exists());

    // 3. Restore from the backup (the runbook's `cp` step).
    tokio::fs::copy(&backup_path, &db_path)
        .await
        .expect("restore should succeed");

    // 4. Verify data integrity after restore.
    assert_eq!(
        read_value(&db_path).await,
        LIVE_VALUE,
        "restored database should contain the pre-incident data"
    );
}

#[tokio::test]
async fn verify_backup_accepts_a_freshly_created_backup() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("payraider.db");
    let backup_dir = dir.path().join("backups");
    seed_live_db(&db_path).await;

    let manager = BackupManager::new(config(&db_path, &backup_dir));
    let backup_path = manager
        .create_backup()
        .await
        .expect("backup should succeed");

    let result = manager
        .verify_backup(&backup_path)
        .await
        .expect("verification should run");

    assert!(result.size_bytes > 0, "backup should not be empty");
    assert!(
        result.checksum_ok,
        "checksum should match: {:?}",
        result.error
    );
}

#[tokio::test]
async fn verify_backup_flags_a_corrupted_backup() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("payraider.db");
    let backup_dir = dir.path().join("backups");
    seed_live_db(&db_path).await;

    let manager = BackupManager::new(config(&db_path, &backup_dir));
    let backup_path = manager
        .create_backup()
        .await
        .expect("backup should succeed");

    // First verification writes the .sha256 sidecar; then corrupt the backup.
    manager
        .verify_backup(&backup_path)
        .await
        .expect("baseline verify");
    tokio::fs::write(&backup_path, b"corrupted")
        .await
        .expect("corrupt backup");

    let result = manager
        .verify_backup(&backup_path)
        .await
        .expect("verification should run");

    assert!(
        !result.checksum_ok,
        "a corrupted backup must fail checksum verification"
    );
}

#[tokio::test]
async fn backup_fails_loudly_when_live_database_is_missing() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("does_not_exist.db");
    let backup_dir = dir.path().join("backups");

    let manager = BackupManager::new(config(&db_path, &backup_dir));

    assert!(
        manager.create_backup().await.is_err(),
        "backing up a missing database should error, not silently no-op"
    );
}
