use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Snapshot schema version for backward compatibility
pub const SCHEMA_VERSION: u32 = 1;

/// Individual anchor metrics within a snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotAnchorMetrics {
    pub id: Uuid,
    pub name: String,
    pub stellar_account: String,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub reliability_score: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub volume_usd: Option<f64>,
    pub status: String,
}

/// Individual corridor metrics within a snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotCorridorMetrics {
    pub id: Uuid,
    pub corridor_key: String,
    pub asset_a_code: String,
    pub asset_a_issuer: String,
    pub asset_b_code: String,
    pub asset_b_issuer: String,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub success_rate: f64,
    pub volume_usd: f64,
    pub avg_settlement_latency_ms: Option<i32>,
    pub liquidity_depth_usd: f64,
}

/// Complete snapshot containing all metrics at a specific epoch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSnapshot {
    /// Schema version for compatibility checking
    pub schema_version: u32,
    /// Epoch number for this snapshot
    pub epoch: u64,
    /// Timestamp when snapshot was created
    pub timestamp: DateTime<Utc>,
    /// All anchor metrics at this epoch
    pub anchor_metrics: Vec<SnapshotAnchorMetrics>,
    /// All corridor metrics at this epoch
    pub corridor_metrics: Vec<SnapshotCorridorMetrics>,
}

impl AnalyticsSnapshot {
    /// Create a new snapshot with given epoch and timestamp
    pub fn new(epoch: u64, timestamp: DateTime<Utc>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            epoch,
            timestamp,
            anchor_metrics: Vec::new(),
            corridor_metrics: Vec::new(),
        }
    }

    /// Add anchor metrics to the snapshot
    pub fn add_anchor_metrics(&mut self, metrics: SnapshotAnchorMetrics) {
        self.anchor_metrics.push(metrics);
    }

    /// Add corridor metrics to the snapshot
    pub fn add_corridor_metrics(&mut self, metrics: SnapshotCorridorMetrics) {
        self.corridor_metrics.push(metrics);
    }

    /// Sort all arrays deterministically for consistent serialization
    pub fn normalize(&mut self) {
        // Sort anchor metrics by id for deterministic ordering
        self.anchor_metrics
            .sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));

        // Sort corridor metrics by id for deterministic ordering
        self.corridor_metrics
            .sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(42, now);

        assert_eq!(snapshot.schema_version, SCHEMA_VERSION);
        assert_eq!(snapshot.epoch, 42);
        assert_eq!(snapshot.timestamp, now);
        assert_eq!(snapshot.anchor_metrics.len(), 0);
        assert_eq!(snapshot.corridor_metrics.len(), 0);
    }

    #[test]
    fn test_add_metrics() {
        let mut snapshot = AnalyticsSnapshot::new(1, Utc::now());
        let anchor = SnapshotAnchorMetrics {
            id: Uuid::new_v4(),
            name: "Test Anchor".to_string(),
            stellar_account: "GTEST".to_string(),
            success_rate: 99.5,
            failure_rate: 0.5,
            reliability_score: 0.995,
            total_transactions: 1000,
            successful_transactions: 995,
            failed_transactions: 5,
            avg_settlement_time_ms: Some(500),
            volume_usd: Some(10000.0),
            status: "green".to_string(),
        };

        snapshot.add_anchor_metrics(anchor.clone());
        assert_eq!(snapshot.anchor_metrics.len(), 1);
        assert_eq!(snapshot.anchor_metrics[0].id, anchor.id);
    }

    #[test]
    fn test_normalize_sorts_deterministically() {
        let mut snapshot = AnalyticsSnapshot::new(1, Utc::now());

        // Create metrics with specific UUIDs to control ordering
        let id1 = Uuid::from_u128(2);
        let id2 = Uuid::from_u128(1);
        let id3 = Uuid::from_u128(3);

        let anchor1 = SnapshotAnchorMetrics {
            id: id1,
            name: "Anchor1".to_string(),
            stellar_account: "GTEST1".to_string(),
            success_rate: 99.0,
            failure_rate: 1.0,
            reliability_score: 0.99,
            total_transactions: 1000,
            successful_transactions: 990,
            failed_transactions: 10,
            avg_settlement_time_ms: Some(500),
            volume_usd: Some(10000.0),
            status: "green".to_string(),
        };

        let anchor2 = SnapshotAnchorMetrics {
            id: id2,
            name: "Anchor2".to_string(),
            stellar_account: "GTEST2".to_string(),
            success_rate: 98.0,
            failure_rate: 2.0,
            reliability_score: 0.98,
            total_transactions: 2000,
            successful_transactions: 1960,
            failed_transactions: 40,
            avg_settlement_time_ms: Some(600),
            volume_usd: Some(20000.0),
            status: "yellow".to_string(),
        };

        let anchor3 = SnapshotAnchorMetrics {
            id: id3,
            name: "Anchor3".to_string(),
            stellar_account: "GTEST3".to_string(),
            success_rate: 97.0,
            failure_rate: 3.0,
            reliability_score: 0.97,
            total_transactions: 3000,
            successful_transactions: 2910,
            failed_transactions: 90,
            avg_settlement_time_ms: Some(700),
            volume_usd: Some(30000.0),
            status: "yellow".to_string(),
        };

        // Add in non-sorted order
        snapshot.add_anchor_metrics(anchor1.clone());
        snapshot.add_anchor_metrics(anchor3.clone());
        snapshot.add_anchor_metrics(anchor2.clone());

        // After normalization, should be sorted by ID
        snapshot.normalize();
        assert_eq!(snapshot.anchor_metrics[0].id, id2);
        assert_eq!(snapshot.anchor_metrics[1].id, id1);
        assert_eq!(snapshot.anchor_metrics[2].id, id3);
    }
}
