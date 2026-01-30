use crate::snapshot::schema::{
    AnalyticsSnapshot, SnapshotAnchorMetrics, SnapshotCorridorMetrics, SCHEMA_VERSION,
};
use crate::database::Database;
use crate::models::{Anchor, SnapshotRecord};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::contract::{ContractService, SubmissionResult};

/// Result of snapshot generation and submission process
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotGenerationResult {
    pub snapshot_id: String,
    pub epoch: u64,
    pub hash: String,
    pub canonical_json: String,
    pub anchor_count: usize,
    pub corridor_count: usize,
    pub submission_result: Option<SubmissionResult>,
    pub verification_successful: bool,
    pub timestamp: DateTime<Utc>,
}

/// Service for creating cryptographically verifiable analytics snapshots
///
/// This service ensures that:
/// 1. Metrics are aggregated from all data sources
/// 2. Snapshots are serialized deterministically (same input = same output)
/// 3. SHA-256 hashes are computed and stored
/// 4. Hashes are submitted to smart contracts
/// 5. Submission success is verified
pub struct SnapshotService {
    db: Arc<Database>,
    contract_service: Option<Arc<ContractService>>,
}

impl SnapshotService {
    /// Create a new snapshot service
    pub fn new(db: Arc<Database>, contract_service: Option<Arc<ContractService>>) -> Self {
        Self {
            db,
            contract_service,
        }
    }

    /// Generate a complete analytics snapshot with hash generation and submission
    /// 
    /// This is the main entry point that fulfills all acceptance criteria:
    /// 1. Aggregate all metrics
    /// 2. Serialize to deterministic JSON
    /// 3. Compute SHA-256 hash
    /// 4. Store hash in database
    /// 5. Submit to smart contract
    /// 6. Verify submission success
    pub async fn generate_and_submit_snapshot(&self, epoch: u64) -> Result<SnapshotGenerationResult> {
        info!("Starting snapshot generation for epoch {}", epoch);

        // Step 1: Aggregate all metrics
        let snapshot = self.aggregate_all_metrics(epoch).await
            .context("Failed to aggregate metrics")?;

        info!("Aggregated {} anchor metrics and {} corridor metrics", 
              snapshot.anchor_metrics.len(), snapshot.corridor_metrics.len());

        // Step 2: Serialize to deterministic JSON
        let canonical_json = Self::serialize_deterministically(snapshot.clone())
            .context("Failed to serialize snapshot deterministically")?;

        // Step 3: Compute SHA-256 hash
        let hash = Self::compute_sha256_hash(&canonical_json);
        let hash_hex = hex::encode(&hash);

        info!("Generated snapshot hash: {}", hash_hex);

        // Step 4: Store hash in database
        let snapshot_id = self.store_snapshot_in_database(&snapshot, &hash_hex, &canonical_json).await
            .context("Failed to store snapshot in database")?;

        info!("Stored snapshot in database with ID: {}", snapshot_id);

        // Step 5: Submit to smart contract (if configured)
        let submission_result = if let Some(contract_service) = &self.contract_service {
            match contract_service.submit_snapshot(hash, epoch).await {
                Ok(result) => {
                    info!("Successfully submitted snapshot to contract: {:?}", result);
                    Some(result)
                }
                Err(e) => {
                    error!("Failed to submit snapshot to contract: {}", e);
                    return Err(e.context("Contract submission failed"));
                }
            }
        } else {
            warn!("Contract service not configured, skipping on-chain submission");
            None
        };

        // Step 6: Verify submission success (if submitted)
        let verification_result = if let Some(ref submission) = submission_result {
            self.verify_submission_success(&hash_hex, epoch, submission).await
                .context("Failed to verify submission success")?
        } else {
            false
        };

        Ok(SnapshotGenerationResult {
            snapshot_id,
            epoch,
            hash: hash_hex,
            canonical_json,
            anchor_count: snapshot.anchor_metrics.len(),
            corridor_count: snapshot.corridor_metrics.len(),
            submission_result,
            verification_successful: verification_result,
            timestamp: snapshot.timestamp,
        })
    }

    /// Aggregate all metrics from the database into a snapshot
    async fn aggregate_all_metrics(&self, epoch: u64) -> Result<AnalyticsSnapshot> {
        let timestamp = Utc::now();
        let mut snapshot = AnalyticsSnapshot::new(epoch, timestamp);

        // Aggregate anchor metrics
        let anchor_metrics = self.aggregate_anchor_metrics().await
            .context("Failed to aggregate anchor metrics")?;
        
        for metrics in anchor_metrics {
            snapshot.add_anchor_metrics(metrics);
        }

        // Aggregate corridor metrics
        let corridor_metrics = self.aggregate_corridor_metrics().await
            .context("Failed to aggregate corridor metrics")?;
        
        for metrics in corridor_metrics {
            snapshot.add_corridor_metrics(metrics);
        }

        Ok(snapshot)
    }

    /// Aggregate anchor metrics from database
    async fn aggregate_anchor_metrics(&self) -> Result<Vec<SnapshotAnchorMetrics>> {
        let query = r#"
            SELECT 
                id,
                name,
                stellar_account,
                total_transactions,
                successful_transactions,
                failed_transactions,
                total_volume_usd,
                avg_settlement_time_ms,
                reliability_score,
                status
            FROM anchors
            WHERE status != 'inactive'
            ORDER BY id
        "#;

        let rows = sqlx::query(query)
            .fetch_all(&self.db.pool)
            .await
            .context("Failed to fetch anchor data")?;

        let mut metrics = Vec::new();
        
        for row in rows {
            let total_transactions: i64 = row.get("total_transactions");
            let successful_transactions: i64 = row.get("successful_transactions");
            let failed_transactions: i64 = row.get("failed_transactions");
            
            let success_rate = if total_transactions > 0 {
                successful_transactions as f64 / total_transactions as f64
            } else {
                0.0
            };
            
            let failure_rate = if total_transactions > 0 {
                failed_transactions as f64 / total_transactions as f64
            } else {
                0.0
            };

            let anchor_metrics = SnapshotAnchorMetrics {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .context("Invalid anchor ID format")?,
                name: row.get("name"),
                stellar_account: row.get("stellar_account"),
                success_rate,
                failure_rate,
                reliability_score: row.get("reliability_score"),
                total_transactions,
                successful_transactions,
                failed_transactions,
                avg_settlement_time_ms: row.get("avg_settlement_time_ms"),
                volume_usd: row.get("total_volume_usd"),
                status: row.get("status"),
            };

            metrics.push(anchor_metrics);
        }

        debug!("Aggregated {} anchor metrics", metrics.len());
        Ok(metrics)
    }

    /// Aggregate corridor metrics from database
    async fn aggregate_corridor_metrics(&self) -> Result<Vec<SnapshotCorridorMetrics>> {
        let query = r#"
            SELECT 
                cm.id,
                cm.corridor_key,
                cm.asset_a_code,
                cm.asset_a_issuer,
                cm.asset_b_code,
                cm.asset_b_issuer,
                cm.total_transactions,
                cm.successful_transactions,
                cm.failed_transactions,
                cm.success_rate,
                cm.volume_usd,
                cm.avg_settlement_latency_ms,
                cm.liquidity_depth_usd
            FROM corridor_metrics cm
            WHERE cm.date >= datetime('now', '-1 day')
            GROUP BY cm.corridor_key
            HAVING cm.date = MAX(cm.date)
            ORDER BY cm.corridor_key
        "#;

        let rows = sqlx::query(query)
            .fetch_all(&self.db.pool)
            .await
            .context("Failed to fetch corridor metrics")?;

        let mut metrics = Vec::new();
        
        for row in rows {
            let corridor_metrics = SnapshotCorridorMetrics {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .context("Invalid corridor metrics ID format")?,
                corridor_key: row.get("corridor_key"),
                asset_a_code: row.get("asset_a_code"),
                asset_a_issuer: row.get("asset_a_issuer"),
                asset_b_code: row.get("asset_b_code"),
                asset_b_issuer: row.get("asset_b_issuer"),
                total_transactions: row.get("total_transactions"),
                successful_transactions: row.get("successful_transactions"),
                failed_transactions: row.get("failed_transactions"),
                success_rate: row.get("success_rate"),
                volume_usd: row.get("volume_usd"),
                avg_settlement_latency_ms: row.get("avg_settlement_latency_ms"),
                liquidity_depth_usd: row.get("liquidity_depth_usd"),
            };

            metrics.push(corridor_metrics);
        }

        debug!("Aggregated {} corridor metrics", metrics.len());
        Ok(metrics)
    }

    /// Store snapshot and hash in database
    async fn store_snapshot_in_database(
        &self,
        snapshot: &AnalyticsSnapshot,
        hash: &str,
        canonical_json: &str,
    ) -> Result<String> {
        let snapshot_id = Uuid::new_v4().to_string();
        
        let query = r#"
            INSERT INTO snapshots (
                id, entity_id, entity_type, data, hash, epoch, timestamp, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(&snapshot_id)
            .bind("system") // entity_id for system-wide snapshots
            .bind("analytics_snapshot") // entity_type
            .bind(canonical_json)
            .bind(hash)
            .bind(snapshot.epoch as i64)
            .bind(snapshot.timestamp)
            .bind(Utc::now())
            .execute(&self.db.pool)
            .await
            .context("Failed to insert snapshot record")?;

        Ok(snapshot_id)
    }

    /// Verify that the submission was successful by querying the contract
    async fn verify_submission_success(
        &self,
        hash: &str,
        epoch: u64,
        submission: &SubmissionResult,
    ) -> Result<bool> {
        if let Some(contract_service) = &self.contract_service {
            // Wait a moment for the transaction to be confirmed
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            match contract_service.verify_snapshot_exists(hash, epoch).await {
                Ok(exists) => {
                    if exists {
                        info!("Verification successful: snapshot exists on-chain for epoch {}", epoch);
                    } else {
                        warn!("Verification failed: snapshot not found on-chain for epoch {}", epoch);
                    }
                    Ok(exists)
                }
                Err(e) => {
                    error!("Verification error: {}", e);
                    Ok(false) // Don't fail the entire process for verification errors
                }
            }
        } else {
            Ok(false)
        }
    }

}

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
    pub fn serialize_deterministically(
        mut snapshot: AnalyticsSnapshot,
    ) -> Result<String, serde_json::Error> {
        // Normalize the snapshot (sort all arrays by ID)
        snapshot.normalize();

        // Build a BTreeMap to ensure key ordering
        let mut map = BTreeMap::new();

        // Add fields in a fixed order
        map.insert(
            "schema_version".to_string(),
            Value::Number(snapshot.schema_version.into()),
        );
        map.insert("epoch".to_string(), Value::Number(snapshot.epoch.into()));

        // Serialize timestamp as ISO 8601 string (deterministic format)
        map.insert(
            "timestamp".to_string(),
            Value::String(snapshot.timestamp.to_rfc3339()),
        );

        // Serialize anchor metrics array (already sorted by normalize())
        let anchor_metrics: Vec<Value> = snapshot
            .anchor_metrics
            .into_iter()
            .map(|m| Self::serialize_anchor_metrics(&m))
            .collect();
        map.insert("anchor_metrics".to_string(), Value::Array(anchor_metrics));

        // Serialize corridor metrics array (already sorted by normalize())
        let corridor_metrics: Vec<Value> = snapshot
            .corridor_metrics
            .into_iter()
            .map(|m| Self::serialize_corridor_metrics(&m))
            .collect();
        map.insert(
            "corridor_metrics".to_string(),
            Value::Array(corridor_metrics),
        );

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
        map.insert(
            "stellar_account".to_string(),
            Value::String(metrics.stellar_account.clone()),
        );
        map.insert(
            "success_rate".to_string(),
            Self::serialize_f64(metrics.success_rate),
        );
        map.insert(
            "failure_rate".to_string(),
            Self::serialize_f64(metrics.failure_rate),
        );
        map.insert(
            "reliability_score".to_string(),
            Self::serialize_f64(metrics.reliability_score),
        );
        map.insert(
            "total_transactions".to_string(),
            Value::Number(metrics.total_transactions.into()),
        );
        map.insert(
            "successful_transactions".to_string(),
            Value::Number(metrics.successful_transactions.into()),
        );
        map.insert(
            "failed_transactions".to_string(),
            Value::Number(metrics.failed_transactions.into()),
        );

        if let Some(ms) = metrics.avg_settlement_time_ms {
            map.insert(
                "avg_settlement_time_ms".to_string(),
                Value::Number(ms.into()),
            );
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
        map.insert(
            "corridor_key".to_string(),
            Value::String(metrics.corridor_key.clone()),
        );
        map.insert(
            "asset_a_code".to_string(),
            Value::String(metrics.asset_a_code.clone()),
        );
        map.insert(
            "asset_a_issuer".to_string(),
            Value::String(metrics.asset_a_issuer.clone()),
        );
        map.insert(
            "asset_b_code".to_string(),
            Value::String(metrics.asset_b_code.clone()),
        );
        map.insert(
            "asset_b_issuer".to_string(),
            Value::String(metrics.asset_b_issuer.clone()),
        );
        map.insert(
            "total_transactions".to_string(),
            Value::Number(metrics.total_transactions.into()),
        );
        map.insert(
            "successful_transactions".to_string(),
            Value::Number(metrics.successful_transactions.into()),
        );
        map.insert(
            "failed_transactions".to_string(),
            Value::Number(metrics.failed_transactions.into()),
        );
        map.insert(
            "success_rate".to_string(),
            Self::serialize_f64(metrics.success_rate),
        );
        map.insert(
            "volume_usd".to_string(),
            Self::serialize_f64(metrics.volume_usd),
        );

        if let Some(ms) = metrics.avg_settlement_latency_ms {
            map.insert(
                "avg_settlement_latency_ms".to_string(),
                Value::Number(ms.into()),
            );
        } else {
            map.insert("avg_settlement_latency_ms".to_string(), Value::Null);
        }

        map.insert(
            "liquidity_depth_usd".to_string(),
            Self::serialize_f64(metrics.liquidity_depth_usd),
        );

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
    pub fn version_and_hash(
        snapshot: AnalyticsSnapshot,
    ) -> Result<([u8; 32], String, u32), serde_json::Error> {
        // Ensure snapshot has correct schema version
        let hash = Self::hash_snapshot(snapshot)?;
        let hash_hex = hex::encode(hash);
        Ok((hash, hash_hex, SCHEMA_VERSION))
    }

    /// Create snapshot, hash it, and submit to on-chain contract
    /// 
    /// This method combines snapshot creation with automatic submission to the
    /// Soroban smart contract. It handles the complete workflow:
    /// 1. Generate snapshot hash
    /// 2. Submit to contract with retry logic
    /// 3. Return both hash and submission result
    /// 
    /// # Arguments
    /// * `snapshot` - The analytics snapshot to hash and submit
    /// * `contract_service` - Contract service for blockchain submission
    /// 
    /// # Returns
    /// Tuple of (hash_bytes, hash_hex, schema_version, submission_result)
    pub async fn version_hash_and_submit(
        snapshot: AnalyticsSnapshot,
        contract_service: &ContractService,
    ) -> Result<([u8; 32], String, u32, SubmissionResult), anyhow::Error> {
        use tracing::info;

        // Get epoch before consuming snapshot
        let epoch = snapshot.epoch;
        
        // Generate hash
        let (hash_bytes, hash_hex, version) = Self::version_and_hash(snapshot)
            .map_err(|e| anyhow::anyhow!("Failed to hash snapshot: {}", e))?;

        info!(
            "Generated snapshot hash for epoch {}: {}",
            epoch, hash_hex
        );

        // Submit to contract
        let submission = contract_service
            .submit_snapshot_hash(hash_bytes, epoch)
            .await?;

        info!(
            "Successfully submitted snapshot for epoch {} to contract",
            epoch
        );

        Ok((hash_bytes, hash_hex, version, submission))
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
        metrics.success_rate = 99.123_456_789_012_35;
        metrics.failure_rate = 0.876_543_210_987_655;
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
            assert_eq!(
                keys, sorted_keys,
                "Top-level JSON keys should be in sorted order"
            );
        }
    }
}
