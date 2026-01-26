use crate::snapshot::schema::{AnalyticsSnapshot, SnapshotAnchorMetrics, SnapshotCorridorMetrics, SCHEMA_VERSION};
use sha2::{Digest, Sha256};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Service for creating cryptographically verifiable analytics snapshots
/// 
/// This service ensures that:
/// 1. Metrics are serialized deterministically (same input = same output)
/// 2. Snapshots are hashed using SHA-256
/// 3. Snapshot versions are tracked via schema_version
pub struct SnapshotService;

impl SnapshotService {
    /// Serialize metrics deterministically to JSON
    /// 
    /// This method ensures that:
    /// - All arrays are sorted by object identifiers
    /// - All object keys are in a consistent order
    /// - Floating point numbers are serialized consistently
    /// - No extra whitespace or formatting variations
    /// 
    /// # Arguments
    /// * `snapshot` - The analytics snapshot to serialize
    /// 
    /// # Returns
    /// A canonical JSON string representation suitable for hashing
    pub fn serialize_deterministically(mut snapshot: AnalyticsSnapshot) -> Result<String, serde_json::Error> {
        // Normalize the snapshot (sort all arrays by ID)
        snapshot.normalize();

        // Build a BTreeMap to ensure key ordering
        let mut map = BTreeMap::new();
        
        // Add fields in a fixed order
        map.insert("schema_version".to_string(), Value::Number(snapshot.schema_version.into()));
        map.insert("epoch".to_string(), Value::Number(snapshot.epoch.into()));
        
        // Serialize timestamp as ISO 8601 string (deterministic format)
        map.insert("timestamp".to_string(), Value::String(snapshot.timestamp.to_rfc3339()));
        
        // Serialize anchor metrics array (already sorted by normalize())
        let anchor_metrics: Vec<Value> = snapshot.anchor_metrics
            .into_iter()
            .map(|m| Self::serialize_anchor_metrics(&m))
            .collect();
        map.insert("anchor_metrics".to_string(), Value::Array(anchor_metrics));
        
        // Serialize corridor metrics array (already sorted by normalize())
        let corridor_metrics: Vec<Value> = snapshot.corridor_metrics
            .into_iter()
            .map(|m| Self::serialize_corridor_metrics(&m))
            .collect();
        map.insert("corridor_metrics".to_string(), Value::Array(corridor_metrics));
        
        // Convert to JSON string with no extra whitespace
        // Note: serde_json::Map uses IndexMap internally which preserves insertion order.
        // Since we iterate over BTreeMap (sorted), insertion order is sorted, ensuring determinism.
        let mut json_map = Map::new();
        for (k, v) in map {
            json_map.insert(k, v);
        }
        serde_json::to_string(&Value::Object(json_map))
    }

    /// Serialize anchor metrics to a deterministic JSON value
    fn serialize_anchor_metrics(metrics: &SnapshotAnchorMetrics) -> Value {
        let mut map = BTreeMap::new();
        
        map.insert("id".to_string(), Value::String(metrics.id.to_string()));
        map.insert("name".to_string(), Value::String(metrics.name.clone()));
        map.insert("stellar_account".to_string(), Value::String(metrics.stellar_account.clone()));
        map.insert("success_rate".to_string(), Self::serialize_f64(metrics.success_rate));
        map.insert("failure_rate".to_string(), Self::serialize_f64(metrics.failure_rate));
        map.insert("reliability_score".to_string(), Self::serialize_f64(metrics.reliability_score));
        map.insert("total_transactions".to_string(), Value::Number(metrics.total_transactions.into()));
        map.insert("successful_transactions".to_string(), Value::Number(metrics.successful_transactions.into()));
        map.insert("failed_transactions".to_string(), Value::Number(metrics.failed_transactions.into()));
        
        if let Some(ms) = metrics.avg_settlement_time_ms {
            map.insert("avg_settlement_time_ms".to_string(), Value::Number(ms.into()));
        } else {
            map.insert("avg_settlement_time_ms".to_string(), Value::Null);
        }
        
        if let Some(volume) = metrics.volume_usd {
            map.insert("volume_usd".to_string(), Self::serialize_f64(volume));
        } else {
            map.insert("volume_usd".to_string(), Value::Null);
        }
        
        map.insert("status".to_string(), Value::String(metrics.status.clone()));
        
        // serde_json::Map preserves insertion order (uses IndexMap internally)
        // Since BTreeMap iteration is sorted, insertion order is sorted
        let mut json_map = Map::new();
        for (k, v) in map {
            json_map.insert(k, v);
        }
        Value::Object(json_map)
    }

    /// Serialize corridor metrics to a deterministic JSON value
    fn serialize_corridor_metrics(metrics: &SnapshotCorridorMetrics) -> Value {
        let mut map = BTreeMap::new();
        
        map.insert("id".to_string(), Value::String(metrics.id.to_string()));
        map.insert("corridor_key".to_string(), Value::String(metrics.corridor_key.clone()));
        map.insert("asset_a_code".to_string(), Value::String(metrics.asset_a_code.clone()));
        map.insert("asset_a_issuer".to_string(), Value::String(metrics.asset_a_issuer.clone()));
        map.insert("asset_b_code".to_string(), Value::String(metrics.asset_b_code.clone()));
        map.insert("asset_b_issuer".to_string(), Value::String(metrics.asset_b_issuer.clone()));
        map.insert("total_transactions".to_string(), Value::Number(metrics.total_transactions.into()));
        map.insert("successful_transactions".to_string(), Value::Number(metrics.successful_transactions.into()));
        map.insert("failed_transactions".to_string(), Value::Number(metrics.failed_transactions.into()));
        map.insert("success_rate".to_string(), Self::serialize_f64(metrics.success_rate));
        map.insert("volume_usd".to_string(), Self::serialize_f64(metrics.volume_usd));
        
        if let Some(ms) = metrics.avg_settlement_latency_ms {
            map.insert("avg_settlement_latency_ms".to_string(), Value::Number(ms.into()));
        } else {
            map.insert("avg_settlement_latency_ms".to_string(), Value::Null);
        }
        
        map.insert("liquidity_depth_usd".to_string(), Self::serialize_f64(metrics.liquidity_depth_usd));
        
        // serde_json::Map preserves insertion order (uses IndexMap internally)
        // Since BTreeMap iteration is sorted, insertion order is sorted
        let mut json_map = Map::new();
        for (k, v) in map {
            json_map.insert(k, v);
        }
        Value::Object(json_map)
    }

    /// Serialize f64 to a deterministic JSON number representation
    /// 
    /// This ensures that floating point numbers are always serialized
    /// in the same way. serde_json handles this deterministically,
    /// but we ensure special cases are handled consistently.
    fn serialize_f64(value: f64) -> Value {
        if value.is_finite() {
            // serde_json::Number::from_f64 uses ryu algorithm which is deterministic
            serde_json::Number::from_f64(value)
                .map(Value::Number)
                .unwrap_or_else(|| {
                    // Fallback for edge cases (shouldn't happen for normal values)
                    Value::String(value.to_string())
                })
        } else if value.is_nan() {
            Value::String("NaN".to_string())
        } else if value.is_infinite() {
            if value.is_sign_positive() {
                Value::String("Infinity".to_string())
            } else {
                Value::String("-Infinity".to_string())
            }
        } else {
            Value::Null
        }
    }

    /// Generate SHA-256 hash of the snapshot
    /// 
    /// This method creates a cryptographically verifiable hash of the snapshot.
    /// The same snapshot content will always produce the same hash, regardless
    /// of the original ordering of metrics in memory.
    /// 
    /// # Arguments
    /// * `snapshot` - The analytics snapshot to hash
    /// 
    /// # Returns
    /// A 32-byte SHA-256 hash as a byte array
    pub fn hash_snapshot(snapshot: AnalyticsSnapshot) -> Result<[u8; 32], serde_json::Error> {
        let canonical_json = Self::serialize_deterministically(snapshot)?;
        let mut hasher = Sha256::new();
        hasher.update(canonical_json.as_bytes());
        let result = hasher.finalize();
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result[..]);
        Ok(hash)
    }

    /// Generate hex-encoded hash string suitable for display/storage
    /// 
    /// # Arguments
    /// * `snapshot` - The analytics snapshot to hash
    /// 
    /// # Returns
    /// A 64-character hexadecimal string representation of the hash
    pub fn hash_snapshot_hex(snapshot: AnalyticsSnapshot) -> Result<String, serde_json::Error> {
        let hash = Self::hash_snapshot(snapshot)?;
        Ok(hex::encode(hash))
    }

    /// Create a versioned snapshot with hash
    /// 
    /// This method creates a snapshot with the current schema version and
    /// generates its cryptographic hash for verification.
    /// 
    /// # Arguments
    /// * `snapshot` - The analytics snapshot to version and hash
    /// 
    /// # Returns
    /// A tuple containing (hash_bytes, hash_hex, schema_version)
    pub fn version_and_hash(snapshot: AnalyticsSnapshot) -> Result<([u8; 32], String, u32), serde_json::Error> {
        // Ensure snapshot has correct schema version
        let hash = Self::hash_snapshot(snapshot)?;
        let hash_hex = hex::encode(hash);
        Ok((hash, hash_hex, SCHEMA_VERSION))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use crate::snapshot::schema::{SnapshotAnchorMetrics, SnapshotCorridorMetrics};

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
    fn test_deterministic_serialization() {
        let now = Utc::now();
        let id1 = Uuid::from_u128(1);
        let id2 = Uuid::from_u128(2);

        let mut snapshot1 = AnalyticsSnapshot::new(1, now);
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(id1, "Anchor1"));
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(id2, "Anchor2"));

        let json1 = SnapshotService::serialize_deterministically(snapshot1).unwrap();

        // Create same snapshot with metrics added in different order
        let mut snapshot2 = AnalyticsSnapshot::new(1, now);
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(id2, "Anchor2"));
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(id1, "Anchor1"));

        let json2 = SnapshotService::serialize_deterministically(snapshot2).unwrap();

        // Same content should produce same JSON regardless of insertion order
        assert_eq!(json1, json2);
    }

    #[test]
    fn test_same_input_same_hash() {
        let now = Utc::now();
        let id1 = Uuid::from_u128(1);
        let id2 = Uuid::from_u128(2);

        let mut snapshot1 = AnalyticsSnapshot::new(1, now);
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(id1, "Anchor1"));
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(id2, "Anchor2"));

        let hash1 = SnapshotService::hash_snapshot(snapshot1).unwrap();

        // Create same snapshot with metrics added in different order
        let mut snapshot2 = AnalyticsSnapshot::new(1, now);
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(id2, "Anchor2"));
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(id1, "Anchor1"));

        let hash2 = SnapshotService::hash_snapshot(snapshot2).unwrap();

        // Same input should always yield same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_content_different_hash() {
        let now = Utc::now();
        let id1 = Uuid::from_u128(1);
        let id2 = Uuid::from_u128(2);

        let mut snapshot1 = AnalyticsSnapshot::new(1, now);
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(id1, "Anchor1"));

        let hash1 = SnapshotService::hash_snapshot(snapshot1).unwrap();

        let mut snapshot2 = AnalyticsSnapshot::new(1, now);
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(id2, "Anchor2"));

        let hash2 = SnapshotService::hash_snapshot(snapshot2).unwrap();

        // Different content should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_changes_with_epoch() {
        let now = Utc::now();
        let id = Uuid::from_u128(1);

        let mut snapshot1 = AnalyticsSnapshot::new(1, now);
        snapshot1.add_anchor_metrics(create_test_anchor_metrics(id, "Anchor1"));

        let hash1 = SnapshotService::hash_snapshot(snapshot1).unwrap();

        let mut snapshot2 = AnalyticsSnapshot::new(2, now);
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(id, "Anchor1"));

        let hash2 = SnapshotService::hash_snapshot(snapshot2).unwrap();

        // Different epoch should produce different hash
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_hex_format() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let hex = SnapshotService::hash_snapshot_hex(snapshot).unwrap();
        
        // Should be 64 characters (32 bytes × 2 hex chars)
        assert_eq!(hex.len(), 64);
        
        // Should only contain valid hex characters
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_hash_is_32_bytes() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let hash = SnapshotService::hash_snapshot(snapshot).unwrap();
        
        // Should be exactly 32 bytes
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_version_and_hash() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let (hash_bytes, hash_hex, version) = SnapshotService::version_and_hash(snapshot).unwrap();
        
        assert_eq!(hash_bytes.len(), 32);
        assert_eq!(hash_hex.len(), 64);
        assert_eq!(version, SCHEMA_VERSION);
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

        let hash1 = SnapshotService::hash_snapshot(snapshot1).unwrap();

        // Create snapshot in reverse order
        let mut snapshot2 = AnalyticsSnapshot::new(100, now);
        snapshot2.add_corridor_metrics(create_test_corridor_metrics(corridor_id2, "corridor2"));
        snapshot2.add_corridor_metrics(create_test_corridor_metrics(corridor_id1, "corridor1"));
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(anchor_id2, "Anchor2"));
        snapshot2.add_anchor_metrics(create_test_anchor_metrics(anchor_id1, "Anchor1"));

        let hash2 = SnapshotService::hash_snapshot(snapshot2).unwrap();

        // Should produce identical hashes
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_deterministic_json_no_extra_whitespace() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let json = SnapshotService::serialize_deterministically(snapshot).unwrap();
        
        // Should not contain unnecessary whitespace
        assert!(!json.contains("  ")); // No double spaces
        assert!(!json.starts_with(" "));
        assert!(!json.ends_with(" "));
    }

    #[test]
    fn test_floating_point_determinism() {
        let now = Utc::now();
        let id = Uuid::from_u128(1);

        // Create snapshot with specific floating point values
        let mut snapshot = AnalyticsSnapshot::new(1, now);
        let mut metrics = create_test_anchor_metrics(id, "Anchor1");
        metrics.success_rate = 99.123456789012345;
        metrics.failure_rate = 0.876543210987655;
        snapshot.add_anchor_metrics(metrics);

        let json1 = SnapshotService::serialize_deterministically(snapshot.clone()).unwrap();
        let json2 = SnapshotService::serialize_deterministically(snapshot).unwrap();

        // Same floating point values should serialize identically
        assert_eq!(json1, json2);
    }

    #[test]
    fn test_json_key_ordering() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let json = SnapshotService::serialize_deterministically(snapshot).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        // Verify top-level keys are in sorted order
        if let serde_json::Value::Object(map) = parsed {
            let keys: Vec<&String> = map.keys().collect();
            let mut sorted_keys = keys.clone();
            sorted_keys.sort();
            assert_eq!(keys, sorted_keys, "Top-level JSON keys should be in sorted order");
        }
    }
}

