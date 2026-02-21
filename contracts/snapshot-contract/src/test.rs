#[test]
fn test_initialize_multiple_admins_and_permissions() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admins = vec![admin1.clone(), admin2.clone()];
    client.initialize(&admins);

    let stored_admins = client.get_admins();
    assert_eq!(stored_admins.len(), 2);
    assert!(stored_admins.contains(&admin1));
    assert!(stored_admins.contains(&admin2));

    // Add a new admin
    let admin3 = Address::generate(&env);
    client.add_admin(&admin1, &admin3);
    let stored_admins = client.get_admins();
    assert_eq!(stored_admins.len(), 3);
    assert!(stored_admins.contains(&admin3));

    // Remove an admin
    client.remove_admin(&admin2, &admin3);
    let stored_admins = client.get_admins();
    assert_eq!(stored_admins.len(), 2);
    assert!(!stored_admins.contains(&admin3));

    // Cannot remove last admin
    client.remove_admin(&admin1, &admin2);
    let stored_admins = client.get_admins();
    assert_eq!(stored_admins.len(), 1);
    assert!(stored_admins.contains(&admin1));
    // Removing last admin should panic
    let result = std::panic::catch_unwind(|| {
        client.remove_admin(&admin1, &admin1);
    });
    assert!(result.is_err());
}
#![cfg(test)]

use super::*;
use soroban_sdk::{
    bytes,
    testutils::{Address as _, Events},
    symbol_short, Env,
};

#[test]
fn test_submit_and_retrieve() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash = bytes!(&env, 0x1234567890abcdef1234567890abcdef);
    let epoch = 42u64;

    let timestamp = client.submit_snapshot(&hash, &epoch);

    let retrieved_hash = client.get_snapshot(&epoch);
    assert_eq!(retrieved_hash, hash);
}

#[test]
fn test_snapshot_submitted_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash = bytes!(&env, 0xabcdef1234567890abcdef1234567890);
    let epoch = 100u64;

    client.submit_snapshot(&hash, &epoch);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let ev = events.get(0).unwrap();
    assert_eq!(ev.0, contract_id); // contract address

    let topics = ev.1;
    assert_eq!(topics.len(), 1);
    assert_eq!(topics.get_unchecked(0), symbol_short!("SNAP_SUB"));
}

#[test]
#[should_panic(expected = "No snapshot found for epoch")]
fn test_get_nonexistent_snapshot_panics() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    client.get_snapshot(&999);
}

#[test]
fn test_multiple_snapshots() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash1 = bytes!(&env, 0x1111111111111111);
    let epoch1 = 1u64;
    client.submit_snapshot(&hash1, &epoch1);

    let hash2 = bytes!(&env, 0x2222222222222222);
    let epoch2 = 2u64;
    client.submit_snapshot(&hash2, &epoch2);

    assert_eq!(client.get_snapshot(&epoch1), hash1);
    assert_eq!(client.get_snapshot(&epoch2), hash2);
}

#[test]
fn test_latest_snapshot() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    client.submit_snapshot(&bytes!(&env, 0x1111), &1);
    env.ledger().set_timestamp(1000);

    client.submit_snapshot(&bytes!(&env, 0x2222), &3);
    env.ledger().set_timestamp(2000);

    client.submit_snapshot(&bytes!(&env, 0x3333), &7);
    env.ledger().set_timestamp(3000);

    let (h, e, t) = client.latest_snapshot();
    assert_eq!(e, 7);
    assert_eq!(t, 3000);
    assert_eq!(h, bytes!(&env, 0x3333));
}

#[test]
#[should_panic(expected = "No snapshots exist")]
fn test_latest_snapshot_empty_panics() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    client.latest_snapshot();
}

#[test]
fn test_verify_found() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash = bytes!(&env, 0xabcdef1234567890abcdef);
    client.submit_snapshot(&hash.clone(), &100);

    assert!(client.verify_snapshot(&hash));
}

#[test]
fn test_verify_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    client.submit_snapshot(&bytes!(&env, 0x111122223333), &5);

    assert!(!client.verify_snapshot(&bytes!(&env, 0x999999999999)));
}

#[test]
#[should_panic(expected = "Invalid hash size")]
fn test_invalid_hash_size() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    // Hash too short (not 32 bytes)
    let short_hash = bytes!(&env, 0x1234);
    client.submit_snapshot(&short_hash, &1);
}

#[test]
#[should_panic(expected = "Invalid epoch")]
fn test_invalid_epoch_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash = bytes!(&env, 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef);
    client.submit_snapshot(&hash, &0);
}

#[test]
#[should_panic(expected = "already exists")]
fn test_duplicate_epoch_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SnapshotContract);
    let client = SnapshotContractClient::new(&env, &contract_id);

    let hash1 = bytes!(&env, 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef);
    let hash2 = bytes!(&env, 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890);

    client.submit_snapshot(&hash1, &1);
    // Attempt to overwrite with different hash should panic
    client.submit_snapshot(&hash2, &1);
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
