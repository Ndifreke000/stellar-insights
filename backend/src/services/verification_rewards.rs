//! Verification Rewards Service
//!
//! This service handles the reward mechanism for users who verify snapshot hashes.
//! Users earn points for successfully verifying that snapshot hashes match backend data.

use crate::database::Database;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Base reward points for successful verification
const BASE_REWARD_POINTS: i32 = 10;

/// Bonus points for early verification (within first hour of snapshot creation)
const EARLY_VERIFICATION_BONUS: i32 = 5;

/// Penalty for failed verification attempts (to discourage spam)
const FAILED_VERIFICATION_PENALTY: i32 = 0;

/// Maximum verifications per user per day to prevent abuse
const MAX_VERIFICATIONS_PER_DAY: i32 = 50;

/// Request to verify a snapshot hash
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct VerifySnapshotRequest {
    pub snapshot_id: String,
    pub submitted_hash: String,
}

/// Response after verification attempt
#[derive(Debug, Serialize)]
pub struct VerificationResponse {
    pub verification_id: String,
    pub is_match: bool,
    pub reward_points: i32,
    pub total_points: i32,
    pub message: String,
}

/// Item in the manual review queue for flagged sybil or capped reward claims (#2823)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ReviewQueueItem {
    pub id: String,
    pub verification_id: String,
    pub user_id: String,
    pub snapshot_id: String,
    pub epoch: i64,
    pub cluster_id: Option<String>,
    pub reason: String,
    pub status: String,
    pub created_at: String,
    pub reviewed_at: Option<String>,
    pub reviewed_by: Option<String>,
}

/// Cluster statistics and information for sybil resistance (#2823)
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ClusterInfo {
    pub cluster_id: String,
    pub user_id: String,
    pub funding_account: Option<String>,
    pub daily_reward_cap: i32,
    pub current_day_rewards: i32,
    pub accounts_in_cluster: i64,
}

/// User reward statistics
#[derive(Debug, Serialize)]
pub struct UserRewardStats {
    pub user_id: String,
    pub username: String,
    pub total_points: i32,
    pub successful_verifications: i32,
    pub failed_verifications: i32,
    pub success_rate: f64,
    pub last_verification_at: Option<String>,
}

/// Leaderboard entry
#[derive(Debug, Serialize)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub username: String,
    pub total_points: i32,
    pub successful_verifications: i32,
    pub success_rate: f64,
}

/// Service for managing verification rewards
pub struct VerificationRewardsService {
    db: Arc<Database>,
}

impl VerificationRewardsService {
    /// Create a new verification rewards service
    #[must_use]
    pub const fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Verify a snapshot hash and award points if successful with anti-sybil and double-claim protection (#2823)
    pub async fn verify_and_reward(
        &self,
        user_id: &str,
        request: VerifySnapshotRequest,
    ) -> Result<VerificationResponse> {
        info!(
            "Processing verification request from user {} for snapshot {}",
            user_id, request.snapshot_id
        );

        // Check daily verification limit
        self.check_daily_limit(user_id).await?;

        // Fetch the snapshot from database
        let snapshot = self.fetch_snapshot(&request.snapshot_id).await?;

        // Double claim prevention: check if user already claimed for this snapshot epoch
        let already_verified: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM snapshot_verifications WHERE epoch = ? AND user_id = ?",
        )
        .bind(snapshot.epoch)
        .bind(user_id)
        .fetch_one(self.db.pool())
        .await
        .unwrap_or(0);

        if already_verified > 0 {
            return Err(anyhow!("Reward already claimed for this snapshot epoch"));
        }

        // Compare hashes
        let is_match = snapshot.hash == request.submitted_hash;

        // Calculate potential reward points
        let mut calculated_points = if is_match {
            let mut points = BASE_REWARD_POINTS;

            // Add early verification bonus
            if self.is_early_verification(&snapshot.created_at)? {
                points += EARLY_VERIFICATION_BONUS;
                debug!(
                    "Early verification bonus applied: +{}",
                    EARLY_VERIFICATION_BONUS
                );
            }

            points
        } else {
            FAILED_VERIFICATION_PENALTY
        };

        // Anti-sybil check: cluster clustering and cluster daily reward cap
        let cluster_id = self.get_or_create_cluster(user_id).await?;
        let cluster_cap = self.get_cluster_cap_status(&cluster_id).await?;

        let mut flagged_for_review = false;
        let mut review_reason = String::new();

        if is_match && calculated_points > 0 {
            if cluster_cap.current_day_rewards + calculated_points > cluster_cap.daily_reward_cap {
                flagged_for_review = true;
                review_reason = format!(
                    "Cluster reward cap exceeded ({}/{} daily points)",
                    cluster_cap.current_day_rewards, cluster_cap.daily_reward_cap
                );
                calculated_points = 0; // withheld pending review
            } else if self.is_suspicious_new_account(user_id).await? {
                flagged_for_review = true;
                review_reason = "New account velocity check triggered".to_string();
                calculated_points = 0; // withheld pending review
            }
        }

        let reward_points = calculated_points;
        let verification_id = Uuid::new_v4().to_string();

        // Transactional execution with unique constraint enforcement
        let mut tx = self.db.pool().begin().await.context("Failed to start transaction")?;

        let insert_res = sqlx::query(
            r"
            INSERT INTO snapshot_verifications
            (id, user_id, snapshot_id, epoch, submitted_hash, expected_hash, is_match, reward_points, verified_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&verification_id)
        .bind(user_id)
        .bind(&request.snapshot_id)
        .bind(snapshot.epoch)
        .bind(&request.submitted_hash)
        .bind(&snapshot.hash)
        .bind(is_match)
        .bind(reward_points)
        .bind(Utc::now().to_rfc3339())
        .execute(&mut *tx)
        .await;

        if let Err(e) = insert_res {
            let err_msg = e.to_string();
            if err_msg.contains("UNIQUE") || err_msg.contains("idx_snapshot_verifications_epoch_user") {
                return Err(anyhow!("Reward already claimed for this snapshot epoch"));
            }
            return Err(anyhow!(e));
        }

        // If flagged for review, record in verification_review_queue
        if flagged_for_review {
            let review_id = Uuid::new_v4().to_string();
            sqlx::query(
                r"
                INSERT INTO verification_review_queue (
                    id, verification_id, user_id, snapshot_id, epoch, cluster_id, reason, status, created_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', CURRENT_TIMESTAMP)
                ",
            )
            .bind(&review_id)
            .bind(&verification_id)
            .bind(user_id)
            .bind(&request.snapshot_id)
            .bind(snapshot.epoch)
            .bind(&cluster_id)
            .bind(&review_reason)
            .execute(&mut *tx)
            .await?;
        } else if reward_points > 0 {
            // Update cluster rewards
            sqlx::query(
                r"
                INSERT INTO cluster_reward_caps (cluster_id, daily_reward_cap, current_day_rewards, last_reset_date)
                VALUES (?, 100, ?, DATE('now'))
                ON CONFLICT(cluster_id) DO UPDATE SET
                    current_day_rewards = CASE 
                        WHEN last_reset_date = DATE('now') THEN current_day_rewards + excluded.current_day_rewards
                        ELSE excluded.current_day_rewards
                    END,
                    last_reset_date = DATE('now')
                ",
            )
            .bind(&cluster_id)
            .bind(reward_points)
            .execute(&mut *tx)
            .await?;
        }

        // Update user rewards
        sqlx::query(
            r"
            INSERT INTO user_rewards (user_id, total_points, successful_verifications, failed_verifications, last_verification_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                total_points = total_points + ?,
                successful_verifications = successful_verifications + ?,
                failed_verifications = failed_verifications + ?,
                last_verification_at = ?,
                updated_at = ?
            ",
        )
        .bind(user_id)
        .bind(reward_points)
        .bind(i32::from(is_match))
        .bind(i32::from(!is_match))
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(reward_points)
        .bind(i32::from(is_match))
        .bind(i32::from(!is_match))
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&mut *tx)
        .await
        .context("Failed to update user rewards")?;

        let total_points: i32 = sqlx::query_scalar(
            r"
            SELECT total_points
            FROM user_rewards
            WHERE user_id = ?
            ",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .context("Failed to fetch updated total")?;

        tx.commit().await.context("Failed to commit transaction")?;

        let message = if flagged_for_review {
            format!("Verification successful! Flagged for anti-sybil review: {review_reason}")
        } else if is_match {
            format!("Verification successful! You earned {reward_points} points.")
        } else {
            "Verification failed. Hash does not match.".to_string()
        };

        info!(
            "Verification complete: user={}, match={}, points={}, total={}, flagged={}",
            user_id, is_match, reward_points, total_points, flagged_for_review
        );

        Ok(VerificationResponse {
            verification_id,
            is_match,
            reward_points,
            total_points,
            message,
        })
    }

    /// Get user reward statistics
    pub async fn get_user_stats(&self, user_id: &str) -> Result<UserRewardStats> {
        let row = sqlx::query(
            r"
            SELECT
                u.id,
                u.username,
                COALESCE(ur.total_points, 0) as total_points,
                COALESCE(ur.successful_verifications, 0) as successful_verifications,
                COALESCE(ur.failed_verifications, 0) as failed_verifications,
                ur.last_verification_at
            FROM users u
            LEFT JOIN user_rewards ur ON u.id = ur.user_id
            WHERE u.id = ?
            ",
        )
        .bind(user_id)
        .fetch_one(self.db.pool())
        .await
        .context("Failed to fetch user stats")?;

        let successful: i32 = row.try_get("successful_verifications")?;
        let failed: i32 = row.try_get("failed_verifications")?;
        let total_attempts = successful + failed;
        let success_rate = if total_attempts > 0 {
            (f64::from(successful) / f64::from(total_attempts)) * 100.0
        } else {
            0.0
        };

        Ok(UserRewardStats {
            user_id: row.try_get::<String, _>("id")?,
            username: row.try_get::<String, _>("username")?,
            total_points: row.try_get::<i32, _>("total_points")?,
            successful_verifications: successful,
            failed_verifications: failed,
            success_rate,
            last_verification_at: row.try_get("last_verification_at").ok(),
        })
    }

    /// Get leaderboard of top verifiers
    pub async fn get_leaderboard(&self, limit: i32) -> Result<Vec<LeaderboardEntry>> {
        // Single query with `ROW_NUMBER()` so rank respects ties and matches DB ordering
        // (no per-row round-trips).
        let rows = sqlx::query(
            r"
            SELECT
                rank,
                username,
                total_points,
                successful_verifications,
                success_rate
            FROM (
                SELECT
                    ROW_NUMBER() OVER (
                        ORDER BY ur.total_points DESC,
                                 ur.successful_verifications DESC,
                                 u.username ASC
                    ) AS rank,
                    u.username AS username,
                    ur.total_points AS total_points,
                    ur.successful_verifications AS successful_verifications,
                    CAST(ur.successful_verifications AS REAL) /
                        NULLIF(ur.successful_verifications + ur.failed_verifications, 0) * 100
                        AS success_rate
                FROM users u
                INNER JOIN user_rewards ur ON u.id = ur.user_id
            ) ranked
            WHERE rank <= ?
            ORDER BY rank
            ",
        )
        .bind(limit)
        .fetch_all(self.db.pool())
        .await
        .context("Failed to fetch leaderboard")?;

        let leaderboard = rows
            .iter()
            .map(|row| -> Result<LeaderboardEntry> {
                Ok(LeaderboardEntry {
                    rank: row.try_get::<i64, _>("rank")? as i32,
                    username: row.try_get::<String, _>("username")?,
                    total_points: row.try_get::<i32, _>("total_points")?,
                    successful_verifications: row.try_get::<i32, _>("successful_verifications")?,
                    success_rate: row.try_get("success_rate").unwrap_or(0.0),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(leaderboard)
    }

    /// Get recent verifications for a user
    pub async fn get_user_verifications(
        &self,
        user_id: &str,
        limit: i32,
    ) -> Result<Vec<VerificationRecord>> {
        let rows = sqlx::query(
            r"
            SELECT
                id,
                snapshot_id,
                epoch,
                submitted_hash,
                expected_hash,
                is_match,
                reward_points,
                verified_at
            FROM snapshot_verifications
            WHERE user_id = ?
            ORDER BY verified_at DESC
            LIMIT ?
            ",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await
        .context("Failed to fetch user verifications")?;

        let verifications = rows
            .into_iter()
            .map(|row| {
                Ok(VerificationRecord {
                    id: row.try_get::<String, _>("id")?,
                    snapshot_id: row.try_get::<String, _>("snapshot_id")?,
                    epoch: row.try_get::<i64, _>("epoch")?,
                    submitted_hash: row.try_get::<String, _>("submitted_hash")?,
                    expected_hash: row.try_get::<String, _>("expected_hash")?,
                    is_match: row.try_get::<bool, _>("is_match")?,
                    reward_points: row.try_get::<i32, _>("reward_points")?,
                    verified_at: row.try_get::<String, _>("verified_at")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(verifications)
    }

    // Private helper methods

    async fn check_daily_limit(&self, user_id: &str) -> Result<()> {
        let count: i32 = sqlx::query_scalar(
            r"
            SELECT COUNT(*)
            FROM snapshot_verifications
            WHERE user_id = ?
            AND DATE(verified_at) = DATE('now')
            ",
        )
        .bind(user_id)
        .fetch_one(self.db.pool())
        .await
        .context("Failed to check daily limit")?;

        if count >= MAX_VERIFICATIONS_PER_DAY {
            return Err(anyhow!(
                "Daily verification limit reached ({count}/{MAX_VERIFICATIONS_PER_DAY})"
            ));
        }

        Ok(())
    }

    async fn fetch_snapshot(&self, snapshot_id: &str) -> Result<SnapshotRecord> {
        let row = sqlx::query(
            r"
            SELECT id, hash, epoch, created_at
            FROM snapshots
            WHERE id = ?
            ",
        )
        .bind(snapshot_id)
        .fetch_one(self.db.pool())
        .await
        .context("Snapshot not found")?;

        Ok(SnapshotRecord {
            id: row.try_get("id")?,
            hash: row.try_get("hash")?,
            epoch: row.try_get("epoch")?,
            created_at: row.try_get("created_at")?,
        })
    }

    fn is_early_verification(&self, snapshot_created_at: &str) -> Result<bool> {
        // Parse the timestamp and check if verification is within 1 hour
        let created = chrono::DateTime::parse_from_rfc3339(snapshot_created_at)
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(snapshot_created_at, "%Y-%m-%d %H:%M:%S")
                    .map(|dt| dt.and_utc().fixed_offset())
            })
            .context("Failed to parse snapshot timestamp")?;

        let now = Utc::now();
        let duration = now.signed_duration_since(created);

        Ok(duration.num_hours() < 1)
    }

    async fn record_verification(
        &self,
        verification_id: &str,
        user_id: &str,
        snapshot_id: &str,
        epoch: i64,
        submitted_hash: &str,
        expected_hash: &str,
        is_match: bool,
        reward_points: i32,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO snapshot_verifications
            (id, user_id, snapshot_id, epoch, submitted_hash, expected_hash, is_match, reward_points, verified_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(verification_id)
        .bind(user_id)
        .bind(snapshot_id)
        .bind(epoch)
        .bind(submitted_hash)
        .bind(expected_hash)
        .bind(is_match)
        .bind(reward_points)
        .bind(Utc::now().to_rfc3339())
        .execute(self.db.pool())
        .await
        .context("Failed to record verification")?;

        Ok(())
    }

    async fn update_user_rewards(
        &self,
        user_id: &str,
        is_match: bool,
        reward_points: i32,
    ) -> Result<i32> {
        // Use a single transaction to execute both operations atomically
        // This reduces round-trips while maintaining compatibility with older SQLite
        let pool = self.db.pool();
        let mut txn = pool.begin().await.context("Failed to begin transaction")?;

        // Insert or update user rewards
        sqlx::query(
            r"
            INSERT INTO user_rewards (user_id, total_points, successful_verifications, failed_verifications, last_verification_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                total_points = total_points + ?,
                successful_verifications = successful_verifications + ?,
                failed_verifications = failed_verifications + ?,
                last_verification_at = ?,
                updated_at = ?
            ",
        )
        .bind(user_id)
        .bind(reward_points)
        .bind(i32::from(is_match))
        .bind(i32::from(!is_match))
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(reward_points)
        .bind(i32::from(is_match))
        .bind(i32::from(!is_match))
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&mut *txn)
        .await
        .context("Failed to update user rewards")?;

        // Fetch updated total within the same transaction
        let total: i32 = sqlx::query_scalar(
            r"
            SELECT total_points
            FROM user_rewards
            WHERE user_id = ?
            ",
        )
        .bind(user_id)
        .fetch_one(&mut *txn)
        .await
        .context("Failed to fetch updated total")?;

        txn.commit().await.context("Failed to commit transaction")?;

        Ok(total)
    }

    // ========================================================================
    // Anti-Sybil & Review Queue Management (#2823)
    // ========================================================================

    /// Get or create cluster for a user. If the user has a funding account registered,
    /// it links to the cluster of that funding account.
    pub async fn get_or_create_cluster(&self, user_id: &str) -> Result<String> {
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT cluster_id FROM verifier_account_clusters WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(self.db.pool())
        .await
        .unwrap_or(None);

        if let Some(cluster_id) = existing {
            return Ok(cluster_id);
        }

        // New user cluster: default to user's own cluster ID
        let cluster_id = format!("cluster_{user_id}");
        sqlx::query(
            r"
            INSERT INTO verifier_account_clusters (user_id, cluster_id, created_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id) DO NOTHING
            ",
        )
        .bind(user_id)
        .bind(&cluster_id)
        .execute(self.db.pool())
        .await?;

        Ok(cluster_id)
    }

    /// Link a user account to a funding account for sybil clustering
    pub async fn set_account_funding_source(
        &self,
        user_id: &str,
        funding_account: &str,
    ) -> Result<String> {
        let existing_cluster: Option<String> = sqlx::query_scalar(
            "SELECT cluster_id FROM verifier_account_clusters WHERE funding_account = ? LIMIT 1",
        )
        .bind(funding_account)
        .fetch_optional(self.db.pool())
        .await
        .unwrap_or(None);

        let cluster_id = existing_cluster.unwrap_or_else(|| format!("cluster_fund_{funding_account}"));

        sqlx::query(
            r"
            INSERT INTO verifier_account_clusters (user_id, cluster_id, funding_account, created_at)
            VALUES (?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id) DO UPDATE SET
                cluster_id = excluded.cluster_id,
                funding_account = excluded.funding_account
            ",
        )
        .bind(user_id)
        .bind(&cluster_id)
        .bind(funding_account)
        .execute(self.db.pool())
        .await?;

        Ok(cluster_id)
    }

    /// Check current cluster rewards for today
    async fn get_cluster_cap_status(&self, cluster_id: &str) -> Result<ClusterCapStatus> {
        let row: Option<(i32, i32, String)> = sqlx::query_as(
            r"
            SELECT daily_reward_cap, current_day_rewards, last_reset_date
            FROM cluster_reward_caps
            WHERE cluster_id = ?
            ",
        )
        .bind(cluster_id)
        .fetch_optional(self.db.pool())
        .await?;

        match row {
            Some((cap, rewards, reset_date)) => {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let current_rewards = if reset_date == today { rewards } else { 0 };
                Ok(ClusterCapStatus {
                    daily_reward_cap: cap,
                    current_day_rewards: current_rewards,
                })
            }
            None => Ok(ClusterCapStatus {
                daily_reward_cap: 100,
                current_day_rewards: 0,
            }),
        }
    }

    /// Check if account is suspiciously new or lacks activity
    async fn is_suspicious_new_account(&self, user_id: &str) -> Result<bool> {
        let successful_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM snapshot_verifications WHERE user_id = ? AND is_match = 1",
        )
        .bind(user_id)
        .fetch_one(self.db.pool())
        .await
        .unwrap_or(0);

        let user_created_today: i64 = sqlx::query_scalar(
            r"
            SELECT COUNT(*) FROM users
            WHERE id = ? AND DATE(created_at) = DATE('now')
            ",
        )
        .bind(user_id)
        .fetch_one(self.db.pool())
        .await
        .unwrap_or(0);

        Ok(successful_count == 0 && user_created_today > 0)
    }

    /// Fetch items from the manual review queue
    pub async fn get_review_queue(&self, status: Option<&str>, limit: i32) -> Result<Vec<ReviewQueueItem>> {
        let q_status = status.unwrap_or("pending");
        let rows = sqlx::query_as::<_, ReviewQueueItem>(
            r"
            SELECT id, verification_id, user_id, snapshot_id, epoch, cluster_id, reason, status, created_at, reviewed_at, reviewed_by
            FROM verification_review_queue
            WHERE status = ?
            ORDER BY created_at DESC
            LIMIT ?
            ",
        )
        .bind(q_status)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows)
    }

    /// Approve a flagged verification claim and release reward points
    pub async fn approve_review(&self, review_id: &str, reviewer: &str) -> Result<()> {
        let row: Option<(String, String, i64)> = sqlx::query_as(
            "SELECT verification_id, user_id, epoch FROM verification_review_queue WHERE id = ? AND status = 'pending'",
        )
        .bind(review_id)
        .fetch_optional(self.db.pool())
        .await?;

        let (verification_id, user_id, _epoch) = row.ok_or_else(|| anyhow!("Pending review item not found"))?;

        let mut tx = self.db.pool().begin().await?;

        // Update review status
        sqlx::query(
            r"
            UPDATE verification_review_queue
            SET status = 'approved', reviewed_at = CURRENT_TIMESTAMP, reviewed_by = ?
            WHERE id = ?
            ",
        )
        .bind(reviewer)
        .bind(review_id)
        .execute(&mut *tx)
        .await?;

        // Update verification record reward_points to BASE_REWARD_POINTS
        sqlx::query(
            "UPDATE snapshot_verifications SET reward_points = ? WHERE id = ?",
        )
        .bind(BASE_REWARD_POINTS)
        .bind(&verification_id)
        .execute(&mut *tx)
        .await?;

        // Update user_rewards
        sqlx::query(
            r"
            UPDATE user_rewards
            SET total_points = total_points + ?, updated_at = CURRENT_TIMESTAMP
            WHERE user_id = ?
            ",
        )
        .bind(BASE_REWARD_POINTS)
        .bind(&user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Reject a flagged verification claim
    pub async fn reject_review(&self, review_id: &str, reviewer: &str) -> Result<()> {
        sqlx::query(
            r"
            UPDATE verification_review_queue
            SET status = 'rejected', reviewed_at = CURRENT_TIMESTAMP, reviewed_by = ?
            WHERE id = ? AND status = 'pending'
            ",
        )
        .bind(reviewer)
        .bind(review_id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Get cluster info and statistics for an account
    pub async fn get_cluster_info(&self, user_id: &str) -> Result<ClusterInfo> {
        let cluster_id = self.get_or_create_cluster(user_id).await?;
        let cap_status = self.get_cluster_cap_status(&cluster_id).await?;

        let funding: Option<String> = sqlx::query_scalar(
            "SELECT funding_account FROM verifier_account_clusters WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(self.db.pool())
        .await
        .unwrap_or(None);

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM verifier_account_clusters WHERE cluster_id = ?",
        )
        .bind(&cluster_id)
        .fetch_one(self.db.pool())
        .await
        .unwrap_or(1);

        Ok(ClusterInfo {
            cluster_id,
            user_id: user_id.to_string(),
            funding_account: funding,
            daily_reward_cap: cap_status.daily_reward_cap,
            current_day_rewards: cap_status.current_day_rewards,
            accounts_in_cluster: count,
        })
    }
}

struct ClusterCapStatus {
    daily_reward_cap: i32,
    current_day_rewards: i32,
}

#[derive(Debug)]
struct SnapshotRecord {
    #[allow(dead_code)]
    id: String,
    hash: String,
    epoch: i64,
    created_at: String,
}

#[derive(Debug, Serialize)]
pub struct VerificationRecord {
    pub id: String,
    pub snapshot_id: String,
    pub epoch: i64,
    pub submitted_hash: String,
    pub expected_hash: String,
    pub is_match: bool,
    pub reward_points: i32,
    pub verified_at: String,
}
