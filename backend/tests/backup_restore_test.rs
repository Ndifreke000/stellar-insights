//! End-to-end backup/restore coverage for `BackupManager` (issue #1857).
//!
//! Exercises the full incident path against a real file on disk: take a backup,
//! destroy the live database, restore from the backup, and confirm the data
//! survived — rather than only asserting that the scheduler fires.

use std::path::Path;
use stellar_insights_backend::backup::{BackupConfig, BackupManager};
use tempfile::tempdir;

const LIVE_CONTENTS: &[u8] = b"SQLite format 3\0stellar-insights-live-data";

fn config(db_path: &Path, backup_dir: &Path) -> BackupConfig {
    BackupConfig {
        enabled: true,
        db_path: db_path.to_string_lossy().into_owned(),
        backup_dir: backup_dir.to_string_lossy().into_owned(),
        keep_days: 30,
        schedule_hour_utc: 2,
    }
}

#[tokio::test]
async fn backup_then_restore_preserves_data() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("stellar_insights.db");
    let backup_dir = dir.path().join("backups");
    tokio::fs::write(&db_path, LIVE_CONTENTS)
        .await
        .expect("seed live db");

    let manager = BackupManager::new(config(&db_path, &backup_dir));

    // 1. Take a backup of the live database.
    let backup_path = manager.create_backup().await.expect("backup should succeed");
    assert!(backup_path.exists(), "backup file should exist on disk");

    // 2. Simulate the incident: the live database is lost.
    tokio::fs::remove_file(&db_path).await.expect("drop live db");
    assert!(!db_path.exists());

    // 3. Restore from the backup (the runbook's `cp` step).
    tokio::fs::copy(&backup_path, &db_path)
        .await
        .expect("restore should succeed");

    // 4. Verify data integrity after restore.
    let restored = tokio::fs::read(&db_path).await.expect("read restored db");
    assert_eq!(
        restored, LIVE_CONTENTS,
        "restored database should byte-for-byte match the pre-incident state"
    );
}

#[tokio::test]
async fn verify_backup_accepts_a_freshly_created_backup() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("stellar_insights.db");
    let backup_dir = dir.path().join("backups");
    tokio::fs::write(&db_path, LIVE_CONTENTS)
        .await
        .expect("seed live db");

    let manager = BackupManager::new(config(&db_path, &backup_dir));
    let backup_path = manager.create_backup().await.expect("backup should succeed");

    let result = manager
        .verify_backup(&backup_path)
        .await
        .expect("verification should run");

    assert!(result.size_bytes > 0, "backup should not be empty");
    assert!(result.checksum_ok, "checksum should match: {:?}", result.error);
}

#[tokio::test]
async fn verify_backup_flags_a_corrupted_backup() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("stellar_insights.db");
    let backup_dir = dir.path().join("backups");
    tokio::fs::write(&db_path, LIVE_CONTENTS)
        .await
        .expect("seed live db");

    let manager = BackupManager::new(config(&db_path, &backup_dir));
    let backup_path = manager.create_backup().await.expect("backup should succeed");

    // First verification writes the .sha256 sidecar; then corrupt the backup.
    manager.verify_backup(&backup_path).await.expect("baseline verify");
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
