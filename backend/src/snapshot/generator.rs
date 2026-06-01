use crate::snapshot::schema::AnalyticsSnapshot;
use sha2::{Digest, Sha256};

/// Generator for deterministic analytics snapshots
pub struct SnapshotGenerator;

impl SnapshotGenerator {
    /// Generate a canonical JSON representation of the snapshot
    ///
    /// This ensures deterministic serialization:
    /// 1. All arrays are sorted by object identifiers
    /// 2. JSON is serialized in canonical form (no extra whitespace, sorted keys)
    /// 3. Result is suitable for hashing
    pub fn to_canonical_json(mut snapshot: AnalyticsSnapshot) -> Result<String, serde_json::Error> {
        // Normalize the snapshot (sort all arrays)
        snapshot.normalize();

        // Convert to a JSON value to ensure key ordering
        let value = serde_json::to_value(&snapshot)?;

        // Serialize with no extra whitespace and keys sorted
        // This produces canonical JSON suitable for hashing
        serde_json::to_string(&value)
    }

    /// Generate SHA-256 hash of the snapshot
    ///
    /// This hash represents the snapshot and can be submitted to the Soroban contract
    /// The same snapshot content will always produce the same hash, regardless of
    /// the original ordering of metrics in memory.
    pub fn generate_hash(snapshot: AnalyticsSnapshot) -> Result<[u8; 32], serde_json::Error> {
        let canonical_json = Self::to_canonical_json(snapshot)?;
        let mut hasher = Sha256::new();
        hasher.update(canonical_json.as_bytes());
        let result = hasher.finalize();

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result[..]);
        Ok(hash)
    }

    /// Generate hex-encoded hash string suitable for display/storage
    pub fn generate_hash_hex(snapshot: AnalyticsSnapshot) -> Result<String, serde_json::Error> {
        let hash = Self::generate_hash(snapshot)?;
        Ok(hex::encode(hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::schema::{SnapshotAnchorMetrics, SnapshotCorridorMetrics};
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_anchor_metrics(id: Uuid, name: &str) -> SnapshotAnchorMetrics {
        SnapshotAnchorMetrics {
            id,
            name: name.to_string(),
            stellar_account: format!("G{}", name),
            success_rate: 99.5,
            failure_rate: 0.5,
            reliability_score: 0.995,
            total_transactions: 1000,
            successful_transactions: 995,
            failed_transactions: 5,
            avg_settlement_time_ms: Some(500),
            volume_usd: Some(10000.0),
            status: "green".to_string(),
        }
    }

    fn create_test_corridor_metrics(id: Uuid, key: &str) -> SnapshotCorridorMetrics {
        SnapshotCorridorMetrics {
            id,
            corridor_key: key.to_string(),
            asset_a_code: "USDC".to_string(),
            asset_a_issuer: "issuer1".to_string(),
            asset_b_code: "EURC".to_string(),
            asset_b_issuer: "issuer2".to_string(),
            total_transactions: 500,
            successful_transactions: 475,
            failed_transactions: 25,
            success_rate: 95.0,
            volume_usd: 50000.0,
            avg_settlement_latency_ms: Some(250),
            liquidity_depth_usd: 100000.0,
        }
    }

    #[test]
    fn test_to_canonical_json() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let json = SnapshotGenerator::to_canonical_json(snapshot).unwrap();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should contain expected fields
        assert!(parsed.get("schema_version").is_some());
        assert!(parsed.get("epoch").is_some());
        assert!(parsed.get("timestamp").is_some());
        assert!(parsed.get("anchor_metrics").is_some());
        assert!(parsed.get("corridor_metrics").is_some());
    }

    #[test]
    fn test_deterministic_hashing() {
        let now = Utc::now();
        let id1 = Uuid::from_u128(1);
        let id2 = Uuid::from_u128(2);

        let mut snapshot1 = AnalyticsSnapshot::new(1, now);
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(id1, "Anchor1"));
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(id2, "Anchor2"));

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        // Create same snapshot with metrics added in different order
        let mut snapshot2 = AnalyticsSnapshot::new(1, now);
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(id2, "Anchor2"));
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(id1, "Anchor1"));

        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        // Same content should produce same hash regardless of insertion order
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_content_different_hash() {
        let now = Utc::now();
        let id1 = Uuid::from_u128(1);
        let id2 = Uuid::from_u128(2);

        let mut snapshot1 = AnalyticsSnapshot::new(1, now);
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(id1, "Anchor1"));

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        let mut snapshot2 = AnalyticsSnapshot::new(1, now);
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(id2, "Anchor2"));

        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        // Different content should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_changes_with_epoch() {
        let now = Utc::now();
        let id = Uuid::from_u128(1);

        let mut snapshot1 = AnalyticsSnapshot::new(1, now);
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(id, "Anchor1"));

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        let mut snapshot2 = AnalyticsSnapshot::new(2, now);
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(id, "Anchor1"));

        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        // Different epoch should produce different hash
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_generate_hash_hex() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let hex = SnapshotGenerator::generate_hash_hex(snapshot).unwrap();

        // Should be 64 characters (32 bytes × 2 hex chars)
        assert_eq!(hex.len(), 64);

        // Should only contain valid hex characters
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_reproducibility_with_multiple_metrics() {
        let now = Utc::now();
        let anchor_id1 = Uuid::from_u128(1);
        let anchor_id2 = Uuid::from_u128(2);
        let corridor_id1 = Uuid::from_u128(3);
        let corridor_id2 = Uuid::from_u128(4);

        // Create snapshot in one order
        let mut snapshot1 = AnalyticsSnapshot::new(100, now);
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(anchor_id1, "Anchor1"));
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(anchor_id2, "Anchor2"));
        snapshot1.add_corridor_metrics(create_test_corridor_metrics(corridor_id1, "corridor1"));
        snapshot1.add_corridor_metrics(create_test_corridor_metrics(corridor_id2, "corridor2"));

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        // Create snapshot in reverse order
        let mut snapshot2 = AnalyticsSnapshot::new(100, now);
        snapshot2.add_corridor_metrics(create_test_corridor_metrics(corridor_id2, "corridor2"));
        snapshot2.add_corridor_metrics(create_test_corridor_metrics(corridor_id1, "corridor1"));
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(anchor_id2, "Anchor2"));
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(anchor_id1, "Anchor1"));

        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        // Should produce identical hashes
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_canonical_json_no_extra_whitespace() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let json = SnapshotGenerator::to_canonical_json(snapshot).unwrap();

        // Should not contain unnecessary whitespace
        assert!(!json.contains("  ")); // No double spaces
        assert!(!json.starts_with(" "));
        assert!(!json.ends_with(" "));
    }

    #[test]
    fn test_hash_as_bytes() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let hash = SnapshotGenerator::generate_hash(snapshot).unwrap();

        // Should be exactly 32 bytes
        assert_eq!(hash.len(), 32);
    }
}
