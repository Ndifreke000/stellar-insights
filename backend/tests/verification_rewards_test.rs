use anyhow::Result;
use sqlx::SqlitePool;
use std::sync::Arc;
use stellar_insights_backend::database::Database;
use stellar_insights_backend::services::verification_rewards::{
    VerificationRewardsService, VerifySnapshotRequest,
};
use uuid::Uuid;

async fn setup_test_db() -> Result<SqlitePool> {
    let pool = SqlitePool::connect(":memory:").await?;

    sqlx::query(
        r#"
        CREATE TABLE users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE snapshots (
            id TEXT PRIMARY KEY,
            entity_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            data TEXT NOT NULL,
            hash TEXT,
            epoch INTEGER,
            timestamp TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE snapshot_verifications (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            snapshot_id TEXT NOT NULL,
            epoch INTEGER NOT NULL,
            submitted_hash TEXT NOT NULL,
            expected_hash TEXT NOT NULL,
            is_match BOOLEAN NOT NULL,
            reward_points INTEGER DEFAULT 0,
            verified_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE user_rewards (
            user_id TEXT PRIMARY KEY,
            total_points INTEGER DEFAULT 0,
            successful_verifications INTEGER DEFAULT 0,
            failed_verifications INTEGER DEFAULT 0,
            last_verification_at DATETIME,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&pool)
    .await?;

    for index_sql in [
        "CREATE INDEX idx_verifications_user ON snapshot_verifications(user_id)",
        "CREATE INDEX idx_verifications_snapshot ON snapshot_verifications(snapshot_id)",
        "CREATE INDEX idx_verifications_epoch ON snapshot_verifications(epoch DESC)",
        "CREATE INDEX idx_verifications_verified_at ON snapshot_verifications(verified_at DESC)",
        "CREATE INDEX idx_user_rewards_points ON user_rewards(total_points DESC)",
    ] {
        sqlx::query(index_sql).execute(&pool).await?;
    }

    sqlx::query(
        r#"
        CREATE VIEW verification_leaderboard AS
        SELECT
            u.id,
            u.username,
            ur.total_points,
            ur.successful_verifications,
            ur.failed_verifications,
            CAST(ur.successful_verifications AS REAL) /
                NULLIF(ur.successful_verifications + ur.failed_verifications, 0) * 100 AS success_rate,
            ur.last_verification_at
        FROM users u
        INNER JOIN user_rewards ur ON u.id = ur.user_id
        ORDER BY ur.total_points DESC
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

async fn create_test_user(pool: &SqlitePool, user_id: &str) -> Result<()> {
    sqlx::query("INSERT INTO users (id, username) VALUES (?, ?)")
        .bind(user_id)
        .bind(format!("user_{}", user_id))
        .execute(pool)
        .await?;
    Ok(())
}

async fn create_test_snapshot(
    pool: &SqlitePool,
    snapshot_id: &str,
    hash: &str,
    epoch: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO snapshots (id, entity_id, entity_type, data, hash, epoch, timestamp) 
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(snapshot_id)
    .bind("test-entity")
    .bind("test")
    .bind("{}")
    .bind(hash)
    .bind(epoch)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

#[tokio::test]
async fn test_successful_verification() -> Result<()> {
    let pool = setup_test_db().await?;
    let db = Arc::new(Database::new(pool.clone()));
    let service = VerificationRewardsService::new(db);

    let user_id = "test-user-1";
    let snapshot_id = Uuid::new_v4().to_string();
    let hash = "abc123def456";

    create_test_user(&pool, user_id).await?;
    create_test_snapshot(&pool, &snapshot_id, hash, 1).await?;

    let request = VerifySnapshotRequest {
        snapshot_id: snapshot_id.clone(),
        submitted_hash: hash.to_string(),
    };

    let response = service.verify_and_reward(user_id, request).await?;

    assert!(response.is_match);
    assert!(response.reward_points >= 10); // Base reward
    assert_eq!(response.total_points, response.reward_points);

    Ok(())
}

#[tokio::test]
async fn test_failed_verification() -> Result<()> {
    let pool = setup_test_db().await?;
    let db = Arc::new(Database::new(pool.clone()));
    let service = VerificationRewardsService::new(db);

    let user_id = "test-user-2";
    let snapshot_id = Uuid::new_v4().to_string();
    let correct_hash = "abc123def456";
    let wrong_hash = "wrong_hash";

    create_test_user(&pool, user_id).await?;
    create_test_snapshot(&pool, &snapshot_id, correct_hash, 1).await?;

    let request = VerifySnapshotRequest {
        snapshot_id: snapshot_id.clone(),
        submitted_hash: wrong_hash.to_string(),
    };

    let response = service.verify_and_reward(user_id, request).await?;

    assert!(!response.is_match);
    assert_eq!(response.reward_points, 0);
    assert_eq!(response.total_points, 0);

    Ok(())
}

#[tokio::test]
async fn test_user_stats() -> Result<()> {
    let pool = setup_test_db().await?;
    let db = Arc::new(Database::new(pool.clone()));
    let service = VerificationRewardsService::new(db);

    let user_id = "test-user-3";
    let snapshot_id = Uuid::new_v4().to_string();
    let hash = "test_hash_123";

    create_test_user(&pool, user_id).await?;
    create_test_snapshot(&pool, &snapshot_id, hash, 1).await?;

    // Perform successful verification
    let request = VerifySnapshotRequest {
        snapshot_id: snapshot_id.clone(),
        submitted_hash: hash.to_string(),
    };
    service.verify_and_reward(user_id, request).await?;

    // Check stats
    let stats = service.get_user_stats(user_id).await?;

    assert_eq!(stats.user_id, user_id);
    assert_eq!(stats.successful_verifications, 1);
    assert_eq!(stats.failed_verifications, 0);
    assert!(stats.total_points >= 10);
    assert_eq!(stats.success_rate, 100.0);

    Ok(())
}

#[tokio::test]
async fn test_leaderboard() -> Result<()> {
    let pool = setup_test_db().await?;
    let db = Arc::new(Database::new(pool.clone()));
    let service = VerificationRewardsService::new(db);

    // Create multiple users with verifications
    for i in 1..=3 {
        let user_id = format!("test-user-{}", i);
        let snapshot_id = Uuid::new_v4().to_string();
        let hash = format!("hash_{}", i);

        create_test_user(&pool, &user_id).await?;
        create_test_snapshot(&pool, &snapshot_id, &hash, i as i64).await?;

        let request = VerifySnapshotRequest {
            snapshot_id: snapshot_id.clone(),
            submitted_hash: hash.clone(),
        };
        service.verify_and_reward(&user_id, request).await?;
    }

    let leaderboard = service.get_leaderboard(10).await?;

    assert_eq!(leaderboard.len(), 3);
    assert!(leaderboard[0].total_points >= 10);

    Ok(())
}

#[tokio::test]
async fn test_daily_limit() -> Result<()> {
    let pool = setup_test_db().await?;
    let db = Arc::new(Database::new(pool.clone()));
    let service = VerificationRewardsService::new(db);

    let user_id = "test-user-limit";
    create_test_user(&pool, user_id).await?;

    // Try to exceed daily limit (50 verifications)
    for i in 0..51 {
        let snapshot_id = Uuid::new_v4().to_string();
        let hash = format!("hash_{}", i);
        create_test_snapshot(&pool, &snapshot_id, &hash, i as i64).await?;

        let request = VerifySnapshotRequest {
            snapshot_id: snapshot_id.clone(),
            submitted_hash: hash.clone(),
        };

        let result = service.verify_and_reward(user_id, request).await;

        if i < 50 {
            assert!(result.is_ok(), "Verification {} should succeed", i);
        } else {
            assert!(
                result.is_err(),
                "Verification {} should fail (daily limit)",
                i
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_verification_history() -> Result<()> {
    let pool = setup_test_db().await?;
    let db = Arc::new(Database::new(pool.clone()));
    let service = VerificationRewardsService::new(db);

    let user_id = "test-user-history";
    create_test_user(&pool, user_id).await?;

    // Create multiple verifications
    for i in 0..5 {
        let snapshot_id = Uuid::new_v4().to_string();
        let hash = format!("hash_{}", i);
        create_test_snapshot(&pool, &snapshot_id, &hash, i as i64).await?;

        let request = VerifySnapshotRequest {
            snapshot_id: snapshot_id.clone(),
            submitted_hash: hash.clone(),
        };
        service.verify_and_reward(user_id, request).await?;
    }

    let history = service.get_user_verifications(user_id, 10).await?;

    assert_eq!(history.len(), 5);
    // History should be in reverse chronological order
    assert!(history[0].epoch >= history[4].epoch);

    Ok(())
}
