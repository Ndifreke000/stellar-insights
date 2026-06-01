#[cfg(test)]
mod tests {
    use chrono::Utc;
    use stellar_insights_backend::snapshot::{
        AnalyticsSnapshot, SnapshotAnchorMetrics, SnapshotCorridorMetrics, SnapshotGenerator,
        SCHEMA_VERSION,
    };
    use uuid::Uuid;

    fn create_anchor_metrics(id: Uuid, name: &str, success_rate: f64) -> SnapshotAnchorMetrics {
        SnapshotAnchorMetrics {
            id,
            name: name.to_string(),
            stellar_account: format!("G{}", name),
            success_rate,
            failure_rate: 100.0 - success_rate,
            reliability_score: success_rate / 100.0,
            total_transactions: 1000,
            successful_transactions: (success_rate * 10.0) as i64,
            failed_transactions: ((100.0 - success_rate) * 10.0) as i64,
            avg_settlement_time_ms: Some(500),
            volume_usd: Some(10000.0),
            status: if success_rate > 98.0 {
                "green"
            } else {
                "yellow"
            }
            .to_string(),
        }
    }

    fn create_corridor_metrics(id: Uuid, key: &str, success_rate: f64) -> SnapshotCorridorMetrics {
        SnapshotCorridorMetrics {
            id,
            corridor_key: key.to_string(),
            asset_a_code: "USDC".to_string(),
            asset_a_issuer: "issuer1".to_string(),
            asset_b_code: "EURC".to_string(),
            asset_b_issuer: "issuer2".to_string(),
            total_transactions: 500,
            successful_transactions: (success_rate * 5.0) as i64,
            failed_transactions: ((100.0 - success_rate) * 5.0) as i64,
            success_rate,
            volume_usd: 50000.0,
            avg_settlement_latency_ms: Some(250),
            liquidity_depth_usd: 100000.0,
        }
    }

    #[test]
    fn test_same_input_produces_same_hash() {
        let now = Utc::now();
        let anchor_id = Uuid::from_u128(1);
        let corridor_id = Uuid::from_u128(2);

        // Create first snapshot
        let mut snapshot1 = AnalyticsSnapshot::new(100, now);
        snapshot1.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.5));
        snapshot1.add_corridor_metrics(create_corridor_metrics(corridor_id, "USDC:EURC", 95.0));

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        // Create identical snapshot
        let mut snapshot2 = AnalyticsSnapshot::new(100, now);
        snapshot2.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.5));
        snapshot2.add_corridor_metrics(create_corridor_metrics(corridor_id, "USDC:EURC", 95.0));

        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        assert_eq!(hash1, hash2, "Same input should produce same hash");
    }

    #[test]
    fn test_order_changes_do_not_affect_hash() {
        let now = Utc::now();
        let anchor_id1 = Uuid::from_u128(1);
        let anchor_id2 = Uuid::from_u128(2);
        let corridor_id1 = Uuid::from_u128(3);
        let corridor_id2 = Uuid::from_u128(4);

        // Create snapshot with metrics in one order
        let mut snapshot1 = AnalyticsSnapshot::new(100, now);
        snapshot1.add_anchor_metrics(create_anchor_metrics(anchor_id1, "Anchor1", 99.0));
        snapshot1.add_anchor_metrics(create_anchor_metrics(anchor_id2, "Anchor2", 98.0));
        snapshot1.add_corridor_metrics(create_corridor_metrics(corridor_id1, "corridor1", 95.0));
        snapshot1.add_corridor_metrics(create_corridor_metrics(corridor_id2, "corridor2", 92.0));

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        // Create snapshot with metrics in reverse order
        let mut snapshot2 = AnalyticsSnapshot::new(100, now);
        snapshot2.add_corridor_metrics(create_corridor_metrics(corridor_id2, "corridor2", 92.0));
        snapshot2.add_corridor_metrics(create_corridor_metrics(corridor_id1, "corridor1", 95.0));
        snapshot2.add_anchor_metrics(create_anchor_metrics(anchor_id2, "Anchor2", 98.0));
        snapshot2.add_anchor_metrics(create_anchor_metrics(anchor_id1, "Anchor1", 99.0));

        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        assert_eq!(hash1, hash2, "Order changes should not affect hash");
    }

    #[test]
    fn test_different_metrics_produce_different_hash() {
        let now = Utc::now();
        let anchor_id = Uuid::from_u128(1);

        let mut snapshot1 = AnalyticsSnapshot::new(100, now);
        snapshot1.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.0));

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        let mut snapshot2 = AnalyticsSnapshot::new(100, now);
        snapshot2.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 98.0)); // Different success rate

        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        assert_ne!(
            hash1, hash2,
            "Different metrics should produce different hash"
        );
    }

    #[test]
    fn test_schema_version_affects_hash() {
        let now = Utc::now();
        let anchor_id = Uuid::from_u128(1);

        let mut snapshot1 = AnalyticsSnapshot::new(100, now);
        snapshot1.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.0));

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        // Manually change schema version for comparison
        let mut snapshot2 = AnalyticsSnapshot::new(100, now);
        snapshot2.schema_version = SCHEMA_VERSION + 1; // Different version
        snapshot2.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.0));

        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        assert_ne!(
            hash1, hash2,
            "Different schema versions should produce different hash"
        );
    }

    #[test]
    fn test_epoch_affects_hash() {
        let now = Utc::now();
        let anchor_id = Uuid::from_u128(1);

        let mut snapshot1 = AnalyticsSnapshot::new(100, now);
        snapshot1.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.0));

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        let mut snapshot2 = AnalyticsSnapshot::new(101, now); // Different epoch
        snapshot2.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.0));

        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        assert_ne!(
            hash1, hash2,
            "Different epochs should produce different hash"
        );
    }

    #[test]
    fn test_empty_snapshot_hash() {
        let now = Utc::now();

        let snapshot1 = AnalyticsSnapshot::new(1, now);
        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        let snapshot2 = AnalyticsSnapshot::new(1, now);
        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        assert_eq!(
            hash1, hash2,
            "Empty snapshots with same epoch should produce same hash"
        );
    }

    #[test]
    fn test_hash_format_is_32_bytes() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let hash = SnapshotGenerator::generate_hash(snapshot).unwrap();

        assert_eq!(hash.len(), 32, "Hash should be exactly 32 bytes (SHA-256)");
    }

    #[test]
    fn test_hash_hex_format() {
        let now = Utc::now();
        let anchor_id = Uuid::from_u128(1);

        let mut snapshot = AnalyticsSnapshot::new(100, now);
        snapshot.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.0));

        let hex = SnapshotGenerator::generate_hash_hex(snapshot).unwrap();

        assert_eq!(
            hex.len(),
            64,
            "Hex hash should be 64 characters (32 bytes * 2)"
        );
        assert!(
            hex.chars().all(|c: char| c.is_ascii_hexdigit()),
            "Hex should contain only valid hex characters"
        );
    }

    #[test]
    fn test_large_snapshot_with_many_metrics() {
        let now = Utc::now();

        // Create a large snapshot with many metrics
        let mut snapshot1 = AnalyticsSnapshot::new(1000, now);
        for i in 0..50 {
            let id = Uuid::from_u128(i as u128);
            snapshot1.add_anchor_metrics(create_anchor_metrics(
                id,
                &format!("Anchor{}", i),
                95.0 + (i as f64 * 0.1),
            ));
        }
        for i in 0..50 {
            let id = Uuid::from_u128((100 + i) as u128);
            snapshot1.add_corridor_metrics(create_corridor_metrics(
                id,
                &format!("corridor{}", i),
                90.0 + (i as f64 * 0.2),
            ));
        }

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();

        // Create the same snapshot with metrics added in reverse order
        let mut snapshot2 = AnalyticsSnapshot::new(1000, now);
        for i in (0..50).rev() {
            let id = Uuid::from_u128((100 + i) as u128);
            snapshot2.add_corridor_metrics(create_corridor_metrics(
                id,
                &format!("corridor{}", i),
                90.0 + (i as f64 * 0.2),
            ));
        }
        for i in (0..50).rev() {
            let id = Uuid::from_u128(i as u128);
            snapshot2.add_anchor_metrics(create_anchor_metrics(
                id,
                &format!("Anchor{}", i),
                95.0 + (i as f64 * 0.1),
            ));
        }

        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();

        assert_eq!(
            hash1, hash2,
            "Large snapshots should produce same hash regardless of insertion order"
        );
    }

    #[test]
    fn test_canonical_json_deterministic() {
        let now = Utc::now();
        let anchor_id = Uuid::from_u128(1);
        let corridor_id = Uuid::from_u128(2);

        let mut snapshot = AnalyticsSnapshot::new(100, now);
        snapshot.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.0));
        snapshot.add_corridor_metrics(create_corridor_metrics(corridor_id, "USDC:EURC", 95.0));

        let json1 = SnapshotGenerator::to_canonical_json(snapshot.clone()).unwrap();
        let json2 = SnapshotGenerator::to_canonical_json(snapshot).unwrap();

        assert_eq!(json1, json2, "Canonical JSON should be deterministic");
    }

    #[test]
    fn test_serialization_has_no_whitespace() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let json = SnapshotGenerator::to_canonical_json(snapshot).unwrap();

        // Should not have unnecessary whitespace
        assert!(!json.contains("  "), "No double spaces");
        assert!(!json.starts_with(" "), "No leading whitespace");
        assert!(!json.ends_with(" "), "No trailing whitespace");
    }

    #[test]
    fn test_snapshot_schema_version_constant() {
        assert_eq!(SCHEMA_VERSION, 1, "Schema version should be 1");
    }

    #[test]
    fn test_snapshot_normalization_sorts_anchors() {
        let now = Utc::now();
        let id1 = Uuid::from_u128(3);
        let id2 = Uuid::from_u128(1);
        let id3 = Uuid::from_u128(2);

        let mut snapshot = AnalyticsSnapshot::new(1, now);
        snapshot.add_anchor_metrics(create_anchor_metrics(id1, "Anchor1", 99.0));
        snapshot.add_anchor_metrics(create_anchor_metrics(id2, "Anchor2", 98.0));
        snapshot.add_anchor_metrics(create_anchor_metrics(id3, "Anchor3", 97.0));

        snapshot.normalize();

        // After normalization, anchors should be sorted by ID
        assert_eq!(snapshot.anchor_metrics[0].id, id2);
        assert_eq!(snapshot.anchor_metrics[1].id, id3);
        assert_eq!(snapshot.anchor_metrics[2].id, id1);
    }

    #[test]
    fn test_snapshot_normalization_sorts_corridors() {
        let now = Utc::now();
        let id1 = Uuid::from_u128(3);
        let id2 = Uuid::from_u128(1);
        let id3 = Uuid::from_u128(2);

        let mut snapshot = AnalyticsSnapshot::new(1, now);
        snapshot.add_corridor_metrics(create_corridor_metrics(id1, "corridor1", 95.0));
        snapshot.add_corridor_metrics(create_corridor_metrics(id2, "corridor2", 92.0));
        snapshot.add_corridor_metrics(create_corridor_metrics(id3, "corridor3", 93.0));

        snapshot.normalize();

        // After normalization, corridors should be sorted by ID
        assert_eq!(snapshot.corridor_metrics[0].id, id2);
        assert_eq!(snapshot.corridor_metrics[1].id, id3);
        assert_eq!(snapshot.corridor_metrics[2].id, id1);
    }

    #[test]
    fn test_multiple_snapshots_different_epochs() {
        let now = Utc::now();
        let anchor_id = Uuid::from_u128(1);

        let mut snapshot1 = AnalyticsSnapshot::new(1, now);
        snapshot1.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.0));

        let mut snapshot2 = AnalyticsSnapshot::new(2, now);
        snapshot2.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.0));

        let mut snapshot3 = AnalyticsSnapshot::new(3, now);
        snapshot3.add_anchor_metrics(create_anchor_metrics(anchor_id, "Anchor1", 99.0));

        let hash1 = SnapshotGenerator::generate_hash(snapshot1).unwrap();
        let hash2 = SnapshotGenerator::generate_hash(snapshot2).unwrap();
        let hash3 = SnapshotGenerator::generate_hash(snapshot3).unwrap();

        // All hashes should be different due to different epochs
        assert_ne!(hash1, hash2);
        assert_ne!(hash2, hash3);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_snapshot_accepts_optional_fields() {
        let now = Utc::now();
        let anchor_id = Uuid::from_u128(1);

        let mut snapshot = AnalyticsSnapshot::new(1, now);

        // Create anchor with None for optional fields
        let anchor = SnapshotAnchorMetrics {
            id: anchor_id,
            name: "Test".to_string(),
            stellar_account: "GTEST".to_string(),
            success_rate: 99.0,
            failure_rate: 1.0,
            reliability_score: 0.99,
            total_transactions: 1000,
            successful_transactions: 990,
            failed_transactions: 10,
            avg_settlement_time_ms: None,
            volume_usd: None,
            status: "green".to_string(),
        };

        snapshot.add_anchor_metrics(anchor);
        let hash = SnapshotGenerator::generate_hash(snapshot).unwrap();

        assert_eq!(hash.len(), 32, "Should handle optional fields correctly");
    }

    #[test]
    fn test_snapshot_with_zero_transactions() {
        let now = Utc::now();
        let anchor_id = Uuid::from_u128(1);

        let mut snapshot = AnalyticsSnapshot::new(1, now);

        let anchor = SnapshotAnchorMetrics {
            id: anchor_id,
            name: "Empty".to_string(),
            stellar_account: "GTEST".to_string(),
            success_rate: 0.0,
            failure_rate: 0.0,
            reliability_score: 0.0,
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            avg_settlement_time_ms: None,
            volume_usd: None,
            status: "red".to_string(),
        };

        snapshot.add_anchor_metrics(anchor);
        let hash = SnapshotGenerator::generate_hash(snapshot).unwrap();

        assert_eq!(hash.len(), 32, "Should handle zero transaction metrics");
    }
}
