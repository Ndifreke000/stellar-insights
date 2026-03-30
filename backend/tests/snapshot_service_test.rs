//! Integration test for the snapshot hash generation service

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use stellar_insights_backend::services::snapshot::SnapshotService;
    use stellar_insights_backend::snapshot::schema::AnalyticsSnapshot;

    #[test]
    fn test_deterministic_serialization_unit() {
        let now = Utc::now();
        let snapshot1 = AnalyticsSnapshot::new(1, now);
        let snapshot2 = AnalyticsSnapshot::new(1, now);

        let json1 = SnapshotService::serialize_deterministically(snapshot1).unwrap();
        let json2 = SnapshotService::serialize_deterministically(snapshot2).unwrap();

        assert_eq!(json1, json2);
    }

    #[test]
    fn test_hash_computation_unit() {
        let now = Utc::now();
        let snapshot = AnalyticsSnapshot::new(1, now);

        let hash_bytes = SnapshotService::hash_snapshot(snapshot.clone()).unwrap();
        let hash_hex = SnapshotService::hash_snapshot_hex(snapshot).unwrap();

        assert_eq!(hash_bytes.len(), 32);
        assert_eq!(hash_hex.len(), 64);
        assert!(hash_hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hash_hex, hex::encode(hash_bytes));
    }

    #[test]
    fn test_same_content_same_hash() {
        let now = Utc::now();
        let snapshot1 = AnalyticsSnapshot::new(42, now);
        let snapshot2 = AnalyticsSnapshot::new(42, now);

        let hash1 = SnapshotService::hash_snapshot_hex(snapshot1).unwrap();
        let hash2 = SnapshotService::hash_snapshot_hex(snapshot2).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_epoch_different_hash() {
        let now = Utc::now();
        let snapshot1 = AnalyticsSnapshot::new(1, now);
        let snapshot2 = AnalyticsSnapshot::new(2, now);

        let hash1 = SnapshotService::hash_snapshot_hex(snapshot1).unwrap();
        let hash2 = SnapshotService::hash_snapshot_hex(snapshot2).unwrap();

        assert_ne!(hash1, hash2);
    }
}
