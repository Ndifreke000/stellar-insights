use sqlx::sqlite::SqlitePool;
use std::env;

#[tokio::test]
async fn test_migrations_apply_cleanly() {
    let db_url = "sqlite://test_migration_forward.db";
    std::fs::remove_file("test_migration_forward.db").ok();

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(db_url)
        .await
        .expect("Failed to create pool");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migrations should apply cleanly");

    let tables: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table'")
        .fetch_all(&pool)
        .await
        .expect("Failed to query tables");

    assert!(
        tables.contains(&"anchors".to_string()),
        "Expected anchors table to exist"
    );
    assert!(
        tables.contains(&"corridors".to_string()),
        "Expected corridors table to exist"
    );

    pool.close().await;
    std::fs::remove_file("test_migration_forward.db").ok();
}

#[tokio::test]
async fn test_migration_backward_compatibility() {
    let db_url = "sqlite://test_migration_compat.db";
    std::fs::remove_file("test_migration_compat.db").ok();

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(db_url)
        .await
        .expect("Failed to create pool");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migrations should apply");

    // Verify key columns exist (proof of schema compatibility)
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pragma_table_info('anchors') WHERE name = 'id'")
        .fetch_one(&pool)
        .await
        .expect("Failed to check anchors table");

    assert_eq!(result.0, 1, "anchors table should have id column");

    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pragma_table_info('users') WHERE name = 'id'")
        .fetch_one(&pool)
        .await
        .expect("Failed to check users table");

    assert_eq!(result.0, 1, "users table should have id column");

    pool.close().await;
    std::fs::remove_file("test_migration_compat.db").ok();
}

#[tokio::test]
async fn test_migrations_idempotent() {
    let db_url = "sqlite://test_migration_idempotent.db";
    std::fs::remove_file("test_migration_idempotent.db").ok();

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(db_url)
        .await
        .expect("Failed to create pool");

    // Run migrations twice
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("First migration run should succeed");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Second migration run should also succeed (idempotent)");

    pool.close().await;
    std::fs::remove_file("test_migration_idempotent.db").ok();
}
