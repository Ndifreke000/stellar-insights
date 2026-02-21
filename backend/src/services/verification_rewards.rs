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
#[derive(Debug, Deserialize)]
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
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Verify a snapshot hash and award points if successful
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

        // Compare hashes
        let is_match = snapshot.hash == request.submitted_hash;

        // Calculate reward points
        let reward_points = if is_match {
            let mut points = BASE_REWARD_POINTS;

            // Add early verification bonus
            if self.is_early_verification(&snapshot.created_at).await? {
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

        // Record verification attempt
        let verification_id = Uuid::new_v4().to_string();
        self.record_verification(
            &verification_id,
            user_id,
            &request.snapshot_id,
            snapshot.epoch,
            &request.submitted_hash,
            &snapshot.hash,
            is_match,
            reward_points,
        )
        .await?;

        // Update user rewards
        let total_points = self
            .update_user_rewards(user_id, is_match, reward_points)
            .await?;

        let message = if is_match {
            format!(
                "Verification successful! You earned {} points.",
                reward_points
            )
        } else {
            "Verification failed. Hash does not match.".to_string()
        };

        info!(
            "Verification complete: user={}, match={}, points={}, total={}",
            user_id, is_match, reward_points, total_points
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
            r#"
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
            "#,
        )
        .bind(user_id)
        .fetch_one(self.db.pool())
        .await
        .context("Failed to fetch user stats")?;

        let successful: i32 = row.try_get("successful_verifications")?;
        let failed: i32 = row.try_get("failed_verifications")?;
        let total_attempts = successful + failed;
        let success_rate = if total_attempts > 0 {
            (successful as f64 / total_attempts as f64) * 100.0
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
        let rows = sqlx::query(
            r#"
            SELECT 
                username,
                total_points,
                successful_verifications,
                CAST(successful_verifications AS REAL) / 
                    NULLIF(successful_verifications + failed_verifications, 0) * 100 AS success_rate
            FROM verification_leaderboard
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(self.db.pool())
        .await
        .context("Failed to fetch leaderboard")?;

        let mut leaderboard = Vec::new();
        for (rank, row) in rows.iter().enumerate() {
            leaderboard.push(LeaderboardEntry {
                rank: (rank + 1) as i32,
                username: row.try_get::<String, _>("username")?,
                total_points: row.try_get::<i32, _>("total_points")?,
                successful_verifications: row.try_get::<i32, _>("successful_verifications")?,
                success_rate: row.try_get("success_rate").unwrap_or(0.0),
            });
        }

        Ok(leaderboard)
    }

    /// Get recent verifications for a user
    pub async fn get_user_verifications(
        &self,
        user_id: &str,
        limit: i32,
    ) -> Result<Vec<VerificationRecord>> {
        let rows = sqlx::query(
            r#"
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
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await
        .context("Failed to fetch user verifications")?;

        let mut verifications = Vec::new();
        for row in rows {
            verifications.push(VerificationRecord {
                id: row.try_get::<String, _>("id")?,
                snapshot_id: row.try_get::<String, _>("snapshot_id")?,
                epoch: row.try_get::<i64, _>("epoch")?,
                submitted_hash: row.try_get::<String, _>("submitted_hash")?,
                expected_hash: row.try_get::<String, _>("expected_hash")?,
                is_match: row.try_get::<bool, _>("is_match")?,
                reward_points: row.try_get::<i32, _>("reward_points")?,
                verified_at: row.try_get::<String, _>("verified_at")?,
            });
        }

        Ok(verifications)
    }

    // Private helper methods

    async fn check_daily_limit(&self, user_id: &str) -> Result<()> {
        let count: i32 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM snapshot_verifications
            WHERE user_id = ?
            AND DATE(verified_at) = DATE('now')
            "#,
        )
        .bind(user_id)
        .fetch_one(self.db.pool())
        .await
        .context("Failed to check daily limit")?;

        if count >= MAX_VERIFICATIONS_PER_DAY {
            return Err(anyhow!(
                "Daily verification limit reached ({}/{})",
                count,
                MAX_VERIFICATIONS_PER_DAY
            ));
        }

        Ok(())
    }

    async fn fetch_snapshot(&self, snapshot_id: &str) -> Result<SnapshotRecord> {
        let row = sqlx::query(
            r#"
            SELECT id, hash, epoch, created_at
            FROM snapshots
            WHERE id = ?
            "#,
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

    async fn is_early_verification(&self, snapshot_created_at: &str) -> Result<bool> {
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
            r#"
            INSERT INTO snapshot_verifications 
            (id, user_id, snapshot_id, epoch, submitted_hash, expected_hash, is_match, reward_points, verified_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
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
        // Insert or update user rewards
        sqlx::query(
            r#"
            INSERT INTO user_rewards (user_id, total_points, successful_verifications, failed_verifications, last_verification_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                total_points = total_points + ?,
                successful_verifications = successful_verifications + ?,
                failed_verifications = failed_verifications + ?,
                last_verification_at = ?,
                updated_at = ?
            "#,
        )
        .bind(user_id)
        .bind(reward_points)
        .bind(if is_match { 1 } else { 0 })
        .bind(if is_match { 0 } else { 1 })
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(reward_points)
        .bind(if is_match { 1 } else { 0 })
        .bind(if is_match { 0 } else { 1 })
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(self.db.pool())
        .await
        .context("Failed to update user rewards")?;

        // Fetch updated total
        let total: i32 = sqlx::query_scalar(
            r#"
            SELECT total_points
            FROM user_rewards
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .fetch_one(self.db.pool())
        .await
        .context("Failed to fetch updated total")?;

        Ok(total)
    }
}

#[derive(Debug)]
struct SnapshotRecord {
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
