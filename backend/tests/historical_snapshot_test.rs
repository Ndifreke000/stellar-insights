use backend::database::Database;
use sqlx::SqlitePool;

#[tokio::test]
async fn test_snapshot_storage_with_hash_and_epoch() {
    // Create in-memory database
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    
    // Run migrations
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS snapshots (
            id TEXT PRIMARY KEY,
            entity_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            data TEXT NOT NULL,
            hash TEXT,
            epoch INTEGER,
            timestamp TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX idx_snapshots_epoch ON snapshots(epoch DESC);
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let db = Database::new(pool);

    // Test 1: Create snapshot with hash and epoch
    let snapshot_data = serde_json::json!({
        "anchor_metrics": [],
        "corridor_metrics": []
    });

    let snapshot1 = db
        .create_snapshot(
            "analytics",
            "global",
            snapshot_data.clone(),
            Some("abc123def456".to_string()),
            Some(100),
        )
        .await
        .unwrap();

    assert_eq!(snapshot1.hash, Some("abc123def456".to_string()));
    assert_eq!(snapshot1.epoch, Some(100));

    // Test 2: Retrieve snapshot by epoch
    let retrieved = db.get_snapshot_by_epoch(100).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.hash, Some("abc123def456".to_string()));
    assert_eq!(retrieved.epoch, Some(100));
    assert_eq!(retrieved.entity_id, "analytics");

    // Test 3: Create multiple snapshots at different epochs
    db.create_snapshot(
        "analytics",
        "global",
        snapshot_data.clone(),
        Some("hash200".to_string()),
        Some(200),
    )
    .await
    .unwrap();

    db.create_snapshot(
        "analytics",
        "global",
        snapshot_data.clone(),
        Some("hash150".to_string()),
        Some(150),
    )
    .await
    .unwrap();

    // Test 4: List snapshots (should be ordered by epoch DESC)
    let snapshots = db.list_snapshots(10, 0).await.unwrap();
    assert_eq!(snapshots.len(), 3);
    assert_eq!(snapshots[0].epoch, Some(200)); // Most recent
    assert_eq!(snapshots[1].epoch, Some(150));
    assert_eq!(snapshots[2].epoch, Some(100)); // Oldest

    // Test 5: Verify past snapshots are retrievable
    let epoch_150 = db.get_snapshot_by_epoch(150).await.unwrap().unwrap();
    assert_eq!(epoch_150.hash, Some("hash150".to_string()));

    let epoch_200 = db.get_snapshot_by_epoch(200).await.unwrap().unwrap();
    assert_eq!(epoch_200.hash, Some("hash200".to_string()));
}

#[tokio::test]
async fn test_snapshot_without_hash_and_epoch() {
    // Create in-memory database
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS snapshots (
            id TEXT PRIMARY KEY,
            entity_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            data TEXT NOT NULL,
            hash TEXT,
            epoch INTEGER,
            timestamp TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let db = Database::new(pool);

    // Create snapshot without hash and epoch (backward compatibility)
    let snapshot_data = serde_json::json!({"test": "data"});
    
    let snapshot = db
        .create_snapshot("test", "type", snapshot_data, None, None)
        .await
        .unwrap();

    assert_eq!(snapshot.hash, None);
    assert_eq!(snapshot.epoch, None);
    assert_eq!(snapshot.entity_id, "test");
}
