use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env,
};

fn create_test_hash(env: &Env, value: u8) -> BytesN<32> {
    BytesN::from_array(env, &[value; 32])
}

#[test]
fn test_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert_eq!(client.get_latest_epoch(), 0);
    assert_eq!(client.get_snapshot_history().len(), 0);
    assert_eq!(client.get_latest_snapshot(), None);
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_initialize_cannot_reinitialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    // Second initialization should fail
    client.initialize(&admin);
}

#[test]
fn test_submit_single_snapshot() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    env.ledger().set_timestamp(1234);

    let epoch = 1u64;
    let hash = create_test_hash(&env, 1);

    let timestamp = client.submit_snapshot(&epoch, &hash, &admin);

    assert_eq!(timestamp, 1234);

    let snapshot = client.get_snapshot(&epoch).unwrap();
    assert_eq!(snapshot.epoch, epoch);
    assert_eq!(snapshot.hash, hash);
    assert_eq!(snapshot.timestamp, timestamp);

    assert_eq!(client.get_latest_epoch(), epoch);

    let latest = client.get_latest_snapshot().unwrap();
    assert_eq!(latest.epoch, epoch);
    assert_eq!(latest.hash, hash);
    assert_eq!(latest.timestamp, timestamp);
}

#[test]
fn test_multiple_snapshots_strictly_increasing_epochs() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let epoch1 = 1u64;
    let hash1 = create_test_hash(&env, 1);
    client.submit_snapshot(&epoch1, &hash1, &admin);

    let epoch2 = 2u64;
    let hash2 = create_test_hash(&env, 2);
    client.submit_snapshot(&epoch2, &hash2, &admin);

    let epoch3 = 3u64;
    let hash3 = create_test_hash(&env, 3);
    client.submit_snapshot(&epoch3, &hash3, &admin);

    assert_eq!(client.get_snapshot(&epoch1).unwrap().hash, hash1);
    assert_eq!(client.get_snapshot(&epoch2).unwrap().hash, hash2);
    assert_eq!(client.get_snapshot(&epoch3).unwrap().hash, hash3);

    assert_eq!(client.get_latest_epoch(), epoch3);

    let latest = client.get_latest_snapshot().unwrap();
    assert_eq!(latest.epoch, epoch3);
    assert_eq!(latest.hash, hash3);

    let history = client.get_snapshot_history();
    assert_eq!(history.len(), 3);

    let all_epochs = client.get_all_epochs();
    assert_eq!(all_epochs.len(), 3);
    assert!(all_epochs.contains(epoch1));
    assert!(all_epochs.contains(epoch2));
    assert!(all_epochs.contains(epoch3));
}

#[test]
fn test_non_sequential_epochs_monotonic_order() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let epochs = [1u64, 5u64, 10u64];
    for (i, &epoch) in epochs.iter().enumerate() {
        let hash = create_test_hash(&env, (i + 1) as u8);
        client.submit_snapshot(&epoch, &hash, &admin);
    }

    for (i, &epoch) in epochs.iter().enumerate() {
        let snapshot = client.get_snapshot(&epoch).unwrap();
        assert_eq!(snapshot.epoch, epoch);
        assert_eq!(snapshot.hash, create_test_hash(&env, (i + 1) as u8));
    }

    assert_eq!(client.get_latest_epoch(), 10u64);
    assert_eq!(client.get_snapshot_history().len(), 3);
}

#[test]
fn test_historical_data_integrity_after_new_submissions() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    env.ledger().set_timestamp(100);
    let epoch1 = 1u64;
    let hash1 = create_test_hash(&env, 1);
    let timestamp1 = client.submit_snapshot(&epoch1, &hash1, &admin);

    env.ledger().set_timestamp(200);
    let epoch2 = 2u64;
    let hash2 = create_test_hash(&env, 2);
    let timestamp2 = client.submit_snapshot(&epoch2, &hash2, &admin);

    let snapshot1_before = client.get_snapshot(&epoch1).unwrap();
    let snapshot2_before = client.get_snapshot(&epoch2).unwrap();

    env.ledger().set_timestamp(300);
    let epoch3 = 5u64;
    let hash3 = create_test_hash(&env, 5);
    client.submit_snapshot(&epoch3, &hash3, &admin);

    let snapshot1_after = client.get_snapshot(&epoch1).unwrap();
    let snapshot2_after = client.get_snapshot(&epoch2).unwrap();

    assert_eq!(snapshot1_after, snapshot1_before);
    assert_eq!(snapshot2_after, snapshot2_before);
    assert_eq!(snapshot1_after.timestamp, timestamp1);
    assert_eq!(snapshot2_after.timestamp, timestamp2);

    assert_eq!(client.get_latest_epoch(), epoch3);
}

#[test]
fn test_get_nonexistent_snapshot() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert_eq!(client.get_snapshot(&999), None);
}

#[test]
#[should_panic(expected = "Invalid epoch: must be greater than 0")]
fn test_invalid_epoch_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let hash = create_test_hash(&env, 1);
    client.submit_snapshot(&0, &hash, &admin);
}

#[test]
#[should_panic(expected = "already exists")]
fn test_duplicate_epoch_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let epoch = 1u64;
    let hash1 = create_test_hash(&env, 1);
    let hash2 = create_test_hash(&env, 2);

    client.submit_snapshot(&epoch, &hash1, &admin);
    client.submit_snapshot(&epoch, &hash2, &admin);
}

#[test]
#[should_panic(expected = "Epoch monotonicity violated")]
fn test_older_epoch_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let epoch_new = 10u64;
    let hash_new = create_test_hash(&env, 10);
    client.submit_snapshot(&epoch_new, &hash_new, &admin);
    assert_eq!(client.get_latest_epoch(), epoch_new);

    let epoch_old = 5u64;
    let hash_old = create_test_hash(&env, 5);
    client.submit_snapshot(&epoch_old, &hash_old, &admin);
}

#[test]
fn test_bounded_storage_growth_simulation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    // Use a smaller set to stay within budget limits
    let num_epochs = 20u64;
    for epoch in 1..=num_epochs {
        let hash = create_test_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash, &admin);
    }

    for epoch in 1..=num_epochs {
        assert!(client.get_snapshot(&epoch).is_some());
    }

    assert_eq!(client.get_latest_epoch(), num_epochs);
    assert_eq!(client.get_snapshot_history().len(), num_epochs as u32);
    assert_eq!(client.get_all_epochs().len(), num_epochs as u32);
}

// ============================================================================
// Access Control Tests - Tests for Issue #41
// ============================================================================

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_unauthorized_submission_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);

    client.initialize(&admin);

    let epoch = 1u64;
    let hash = create_test_hash(&env, 1);

    // Attempt to submit snapshot with unauthorized address should fail
    client.submit_snapshot(&epoch, &hash, &unauthorized_user);
}

#[test]
fn test_authorized_submission_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    client.initialize(&admin);

    env.ledger().set_timestamp(1000);

    let epoch = 1u64;
    let hash = create_test_hash(&env, 1);

    // Authorized admin should be able to submit
    let timestamp = client.submit_snapshot(&epoch, &hash, &admin);

    assert_eq!(timestamp, 1000);
    assert_eq!(client.get_latest_epoch(), epoch);

    let snapshot = client.get_snapshot(&epoch).unwrap();
    assert_eq!(snapshot.hash, hash);
}

#[test]
fn test_get_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    // Before initialization, admin should be None
    assert_eq!(client.get_admin(), None);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // After initialization, admin should match
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn test_set_admin_by_authorized_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin);
    assert_eq!(client.get_admin(), Some(admin.clone()));

    // Current admin transfers rights to new admin
    client.set_admin(&admin, &new_admin);
    assert_eq!(client.get_admin(), Some(new_admin.clone()));

    // New admin can now submit snapshots
    let epoch = 1u64;
    let hash = create_test_hash(&env, 1);
    client.submit_snapshot(&epoch, &hash, &new_admin);

    assert_eq!(client.get_latest_epoch(), epoch);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_set_admin_by_unauthorized_user_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin);

    // Unauthorized user attempts to change admin should fail
    client.set_admin(&unauthorized_user, &new_admin);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_old_admin_cannot_submit_after_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin);

    // Transfer admin rights
    client.set_admin(&admin, &new_admin);

    // Old admin should no longer be able to submit
    let epoch = 1u64;
    let hash = create_test_hash(&env, 1);
    client.submit_snapshot(&epoch, &hash, &admin);
}

// ============================================================================
// Snapshot Expiry Tests
// ============================================================================

#[test]
fn test_snapshot_expiry() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    // Submit at t=1000 with 500s TTL -> expires at t=1500
    env.ledger().set_timestamp(1000);
    let hash = create_test_hash(&env, 1);
    client.submit_snapshot_with_ttl(&1u64, &hash, &admin, &Some(500u64));

    let snapshot = client.get_snapshot(&1u64).unwrap();
    assert_eq!(snapshot.expires_at, Some(1500u64));

    // Before expiry: not expired
    env.ledger().set_timestamp(1499);
    assert!(!client.is_snapshot_expired(&1u64));

    // After expiry: expired
    env.ledger().set_timestamp(1501);
    assert!(client.is_snapshot_expired(&1u64));
}

#[test]
fn test_snapshot_default_ttl() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    env.ledger().set_timestamp(0);
    let hash = create_test_hash(&env, 1);
    client.submit_snapshot_with_ttl(&1u64, &hash, &admin, &None);

    let snapshot = client.get_snapshot(&1u64).unwrap();
    // Default TTL is 90 days = 7_776_000 seconds
    assert_eq!(snapshot.expires_at, Some(7_776_000u64));
}

#[test]
fn test_snapshot_no_expiry_by_default() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    // submit_snapshot (no TTL) should have expires_at = None
    env.ledger().set_timestamp(1000);
    let hash = create_test_hash(&env, 1);
    client.submit_snapshot(&1u64, &hash, &admin);

    let snapshot = client.get_snapshot(&1u64).unwrap();
    assert_eq!(snapshot.expires_at, None);

    // Should never be considered expired
    env.ledger().set_timestamp(u64::MAX / 2);
    assert!(!client.is_snapshot_expired(&1u64));
}

#[test]
fn test_cleanup_expired_snapshots() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    // Submit 3 snapshots with short TTL (100s) and 1 with long TTL (10000s)
    env.ledger().set_timestamp(1000);
    for epoch in 1u64..=3 {
        let hash = create_test_hash(&env, epoch as u8);
        client.submit_snapshot_with_ttl(&epoch, &hash, &admin, &Some(100u64));
    }
    let hash4 = create_test_hash(&env, 4);
    client.submit_snapshot_with_ttl(&4u64, &hash4, &admin, &Some(10_000u64));

    // Advance past the short TTL expiry
    env.ledger().set_timestamp(1200);

    // Clean up max 2 at a time
    let cleaned = client.cleanup_expired_snapshots(&admin, &2u32);
    assert_eq!(cleaned, 2);

    // Epochs 1 and 2 removed, 3 and 4 still present
    assert!(client.get_snapshot(&1u64).is_none());
    assert!(client.get_snapshot(&2u64).is_none());
    assert!(client.get_snapshot(&3u64).is_some());
    assert!(client.get_snapshot(&4u64).is_some());

    // Clean remaining expired
    let cleaned2 = client.cleanup_expired_snapshots(&admin, &10u32);
    assert_eq!(cleaned2, 1); // epoch 3 expired, epoch 4 not yet

    assert!(client.get_snapshot(&3u64).is_none());
    assert!(client.get_snapshot(&4u64).is_some());
}

#[test]
fn test_cleanup_respects_max_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    env.ledger().set_timestamp(0);
    for epoch in 1u64..=5 {
        let hash = create_test_hash(&env, epoch as u8);
        client.submit_snapshot_with_ttl(&epoch, &hash, &admin, &Some(100u64));
    }

    // All 5 expired
    env.ledger().set_timestamp(200);

    // Only clean 3
    let cleaned = client.cleanup_expired_snapshots(&admin, &3u32);
    assert_eq!(cleaned, 3);

    // 2 still remain
    let history = client.get_snapshot_history();
    assert_eq!(history.len(), 2);
}

#[test]
fn test_cleanup_no_expired_snapshots() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    env.ledger().set_timestamp(0);
    let hash = create_test_hash(&env, 1);
    client.submit_snapshot_with_ttl(&1u64, &hash, &admin, &Some(10_000u64));

    // Not yet expired
    env.ledger().set_timestamp(100);
    let cleaned = client.cleanup_expired_snapshots(&admin, &10u32);
    assert_eq!(cleaned, 0);
    assert!(client.get_snapshot(&1u64).is_some());
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_cleanup_unauthorized_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.initialize(&admin);

    client.cleanup_expired_snapshots(&attacker, &10u32);
}

#[test]
fn test_submit_snapshot_with_ttl_stores_submitter_and_ledger() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    env.ledger().set_timestamp(5000);
    let hash = create_test_hash(&env, 42);
    client.submit_snapshot_with_ttl(&1u64, &hash, &admin, &Some(1000u64));

    let snapshot = client.get_snapshot(&1u64).unwrap();
    assert_eq!(snapshot.submitter, admin);
    assert_eq!(snapshot.timestamp, 5000);
    assert_eq!(snapshot.expires_at, Some(6000u64));
    assert_eq!(snapshot.hash, hash);
}
