#![cfg(test)]

use super::*;
use soroban_sdk::{bytes, testutils::Events, Env};

#[test]
fn test_submit_snapshot() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash = bytes!(&env, 0x1234567890abcdef);
    let epoch = 42u64;

    // Submit snapshot
    let timestamp = client.submit_snapshot(&hash, &epoch);

    // Verify snapshot was stored
    let stored_snapshot = client.get_snapshot(&epoch).unwrap();
    assert_eq!(stored_snapshot.hash, hash);
    assert_eq!(stored_snapshot.epoch, epoch);
    assert_eq!(stored_snapshot.timestamp, timestamp);
}

#[test]
fn test_snapshot_submitted_event() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash = bytes!(&env, 0xabcdef1234567890);
    let epoch = 100u64;

    // Submit snapshot
    client.submit_snapshot(&hash, &epoch);

    // Check event was emitted
    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let (event_contract_id, _, _) = events.get(0).unwrap();
    assert_eq!(event_contract_id, contract_id);
}

#[test]
fn test_get_nonexistent_snapshot() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let epoch = 999u64;
    let snapshot = client.get_snapshot(&epoch);
    assert!(snapshot.is_none());
}

#[test]
fn test_multiple_snapshots() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    // Submit first snapshot
    let hash1 = bytes!(&env, 0x1111111111111111);
    let epoch1 = 1u64;
    let timestamp1 = client.submit_snapshot(&hash1, &epoch1);

    // Submit second snapshot
    let hash2 = bytes!(&env, 0x2222222222222222);
    let epoch2 = 2u64;
    let timestamp2 = client.submit_snapshot(&hash2, &epoch2);

    // Verify both snapshots
    let snapshot1 = client.get_snapshot(&epoch1).unwrap();
    assert_eq!(snapshot1.hash, hash1);
    assert_eq!(snapshot1.epoch, epoch1);
    assert_eq!(snapshot1.timestamp, timestamp1);

    let snapshot2 = client.get_snapshot(&epoch2).unwrap();
    assert_eq!(snapshot2.hash, hash2);
    assert_eq!(snapshot2.epoch, epoch2);
    assert_eq!(snapshot2.timestamp, timestamp2);

    // Timestamps may be the same in test environment, which is acceptable
}

#[test]
fn test_verify_snapshot_returns_true_for_valid_hash() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash = bytes!(&env, 0x1234567890abcdef);
    let epoch = 1u64;

    // Submit snapshot
    client.submit_snapshot(&hash, &epoch);

    // Verify should return true for the stored hash
    assert!(client.verify_snapshot(&hash));
}

#[test]
fn test_verify_snapshot_returns_false_for_invalid_hash() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash = bytes!(&env, 0x1234567890abcdef);
    let epoch = 1u64;

    // Submit snapshot
    client.submit_snapshot(&hash, &epoch);

    // Verify should return false for a different hash
    let invalid_hash = bytes!(&env, 0xdeadbeefdeadbeef);
    assert!(!client.verify_snapshot(&invalid_hash));
}

#[test]
fn test_verify_snapshot_returns_false_when_no_snapshots() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    // Verify should return false when no snapshots exist
    let hash = bytes!(&env, 0x1234567890abcdef);
    assert!(!client.verify_snapshot(&hash));
}

#[test]
fn test_verify_snapshot_finds_historical_snapshots() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    // Submit multiple snapshots
    let hash1 = bytes!(&env, 0x1111111111111111);
    let hash2 = bytes!(&env, 0x2222222222222222);
    let hash3 = bytes!(&env, 0x3333333333333333);

    client.submit_snapshot(&hash1, &1u64);
    client.submit_snapshot(&hash2, &2u64);
    client.submit_snapshot(&hash3, &3u64);

    // All historical hashes should be verifiable
    assert!(client.verify_snapshot(&hash1));
    assert!(client.verify_snapshot(&hash2));
    assert!(client.verify_snapshot(&hash3));

    // Invalid hash should still return false
    let invalid_hash = bytes!(&env, 0xdeadbeefdeadbeef);
    assert!(!client.verify_snapshot(&invalid_hash));
}

#[test]
fn test_verify_snapshot_at_epoch() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash1 = bytes!(&env, 0x1111111111111111);
    let hash2 = bytes!(&env, 0x2222222222222222);

    client.submit_snapshot(&hash1, &1u64);
    client.submit_snapshot(&hash2, &2u64);

    // Hash1 should only verify at epoch 1
    assert!(client.verify_snapshot_at_epoch(&hash1, &1u64));
    assert!(!client.verify_snapshot_at_epoch(&hash1, &2u64));

    // Hash2 should only verify at epoch 2
    assert!(!client.verify_snapshot_at_epoch(&hash2, &1u64));
    assert!(client.verify_snapshot_at_epoch(&hash2, &2u64));

    // Non-existent epoch should return false
    assert!(!client.verify_snapshot_at_epoch(&hash1, &999u64));
}

#[test]
fn test_verify_latest_snapshot() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash1 = bytes!(&env, 0x1111111111111111);
    let hash2 = bytes!(&env, 0x2222222222222222);

    // Submit first snapshot
    client.submit_snapshot(&hash1, &1u64);
    assert!(client.verify_latest_snapshot(&hash1));
    assert!(!client.verify_latest_snapshot(&hash2));

    // Submit second snapshot (newer epoch)
    client.submit_snapshot(&hash2, &2u64);
    assert!(!client.verify_latest_snapshot(&hash1));
    assert!(client.verify_latest_snapshot(&hash2));
}

#[test]
fn test_verify_latest_snapshot_when_no_snapshots() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash = bytes!(&env, 0x1234567890abcdef);
    assert!(!client.verify_latest_snapshot(&hash));
}

#[test]
fn test_get_latest_snapshot() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    // No snapshots yet
    assert!(client.get_latest_snapshot().is_none());

    let hash1 = bytes!(&env, 0x1111111111111111);
    let hash2 = bytes!(&env, 0x2222222222222222);

    // Submit first snapshot
    client.submit_snapshot(&hash1, &1u64);
    let latest = client.get_latest_snapshot().unwrap();
    assert_eq!(latest.hash, hash1);
    assert_eq!(latest.epoch, 1u64);

    // Submit second snapshot with higher epoch
    client.submit_snapshot(&hash2, &5u64);
    let latest = client.get_latest_snapshot().unwrap();
    assert_eq!(latest.hash, hash2);
    assert_eq!(latest.epoch, 5u64);
}

#[test]
fn test_latest_epoch_not_updated_for_older_epoch() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash1 = bytes!(&env, 0x1111111111111111);
    let hash2 = bytes!(&env, 0x2222222222222222);

    // Submit snapshot at epoch 10
    client.submit_snapshot(&hash1, &10u64);
    let latest = client.get_latest_snapshot().unwrap();
    assert_eq!(latest.epoch, 10u64);

    // Submit snapshot at earlier epoch (should not update latest)
    client.submit_snapshot(&hash2, &5u64);
    let latest = client.get_latest_snapshot().unwrap();
    assert_eq!(latest.epoch, 10u64);
    assert_eq!(latest.hash, hash1);
}

#[test]
fn test_no_false_positives_similar_hashes() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    // Submit a snapshot
    let hash = bytes!(&env, 0x1234567890abcdef);
    client.submit_snapshot(&hash, &1u64);

    // Test with similar but different hashes (off by one bit patterns)
    let similar_hash1 = bytes!(&env, 0x1234567890abcdee);
    let similar_hash2 = bytes!(&env, 0x1234567890abcded);
    let similar_hash3 = bytes!(&env, 0x0234567890abcdef);

    // None of these similar hashes should verify
    assert!(!client.verify_snapshot(&similar_hash1));
    assert!(!client.verify_snapshot(&similar_hash2));
    assert!(!client.verify_snapshot(&similar_hash3));

    // Only the exact hash should verify
    assert!(client.verify_snapshot(&hash));
}
