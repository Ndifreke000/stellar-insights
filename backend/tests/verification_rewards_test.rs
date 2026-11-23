use anyhow::Result;
use payraider_backend::database::Database;
use payraider_backend::services::verification_rewards::{
    VerificationRewardsService, VerifySnapshotRequest,
};
use sqlx::SqlitePool;
use std::sync::Arc;
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
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_snapshot_verifications_epoch_user ON snapshot_verifications(epoch, user_id)",
    ] {
        sqlx::query(index_sql).execute(&pool).await?;
    }

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS verifier_account_clusters (
            user_id TEXT PRIMARY KEY,
            cluster_id TEXT NOT NULL,
            funding_account TEXT,
            risk_score REAL NOT NULL DEFAULT 0.0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS cluster_reward_caps (
            cluster_id TEXT PRIMARY KEY,
            daily_reward_cap INTEGER NOT NULL DEFAULT 100,
            current_day_rewards INTEGER NOT NULL DEFAULT 0,
            last_reset_date TEXT NOT NULL DEFAULT (DATE('now'))
        );
        CREATE TABLE IF NOT EXISTS verification_review_queue (
            id TEXT PRIMARY KEY,
            verification_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            snapshot_id TEXT NOT NULL,
            epoch INTEGER NOT NULL,
            cluster_id TEXT,
            reason TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            reviewed_at DATETIME,
            reviewed_by TEXT
        );
        "#,
    )
    .execute(&pool)
    .await?;

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

#[tokio::test]
async fn test_double_claim_prevention() -> Result<()> {
    let pool = setup_test_db().await?;
    let db = Arc::new(Database::new(pool.clone()));
    let service = VerificationRewardsService::new(db);

    let user_id = "test-user-double-claim";
    create_test_user(&pool, user_id).await?;

    let snapshot_id = "snapshot-double-claim";
    let hash = "valid_hash_12345";
    create_test_snapshot(&pool, snapshot_id, hash, 100).await?;

    let req1 = VerifySnapshotRequest {
        snapshot_id: snapshot_id.to_string(),
        submitted_hash: hash.to_string(),
    };
    let res1 = service.verify_and_reward(user_id, req1).await?;
    assert!(res1.is_match);
    assert_eq!(res1.reward_points, 15); // base 10 + early 5

    // Second claim attempt for the same epoch should be rejected
    let req2 = VerifySnapshotRequest {
        snapshot_id: snapshot_id.to_string(),
        submitted_hash: hash.to_string(),
    };
    let res2 = service.verify_and_reward(user_id, req2).await;
    assert!(res2.is_err(), "Second claim for same epoch must fail");
    let err_str = res2.unwrap_err().to_string();
    assert!(err_str.contains("already claimed"));

    Ok(())
}

#[tokio::test]
async fn test_sybil_cluster_cap_and_review_queue() -> Result<()> {
    let pool = setup_test_db().await?;
    let db = Arc::new(Database::new(pool.clone()));
    let service = VerificationRewardsService::new(db);

    let user1 = "sybil-user-1";
    let user2 = "sybil-user-2";
    let funding_parent = "parent-funder-account";
    create_test_user(&pool, user1).await?;
    create_test_user(&pool, user2).await?;

    // Link both users to the same funding account cluster
    let c1 = service
        .set_account_funding_source(user1, funding_parent)
        .await?;
    let c2 = service
        .set_account_funding_source(user2, funding_parent)
        .await?;
    assert_eq!(c1, c2);

    let cluster_info = service.get_cluster_info(user1).await?;
    assert_eq!(cluster_info.accounts_in_cluster, 2);
    assert_eq!(cluster_info.daily_reward_cap, 100);

    // Manually fill cluster cap to near limit (95 points)
    sqlx::query(
        "INSERT INTO cluster_reward_caps (cluster_id, daily_reward_cap, current_day_rewards, last_reset_date) VALUES (?, 100, 95, DATE('now'))",
    )
    .bind(&c1)
    .execute(&pool)
    .await?;

    // Verification attempt by user2: 15 points will exceed cap of 100 => routed to review queue
    let snapshot_id = "snapshot-sybil-cap";
    let hash = "hash_sybil_test";
    create_test_snapshot(&pool, snapshot_id, hash, 999).await?;

    let req = VerifySnapshotRequest {
        snapshot_id: snapshot_id.to_string(),
        submitted_hash: hash.to_string(),
    };
    let res = service.verify_and_reward(user2, req).await?;
    assert!(res.is_match);
    assert_eq!(res.reward_points, 0); // Withheld
    assert!(res.message.contains("Flagged for anti-sybil review"));

    // Check review queue
    let queue = service.get_review_queue(Some("pending"), 10).await?;
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].user_id, user2);
    assert!(queue[0].reason.contains("Cluster reward cap exceeded"));

    // Admin approves review
    service
        .approve_review(&queue[0].id, "admin-reviewer")
        .await?;
    let remaining_pending = service.get_review_queue(Some("pending"), 10).await?;
    assert_eq!(remaining_pending.len(), 0);

    let stats = service.get_user_stats(user2).await?;
    assert_eq!(stats.total_points, 10); // Base points granted upon review approval

    Ok(())
}
