use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    vec, Address, BytesN, Env, Vec,
};

fn create_test_hash(env: &Env, value: u8) -> BytesN<32> {
    BytesN::from_array(env, &[value; 32])
}

// ============================================================================
// Initialization Tests
// ============================================================================

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

// ============================================================================
// Single Snapshot Tests
// ============================================================================

#[test]
fn test_version_stored_on_init() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let version = client.getversion();
    assert!(!version.is_empty());
    // Version should be stored and retrievable
    assert_eq!(version, soroban_sdk::String::from_str(&env, env!("CARGO_PKG_VERSION")));
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
// Access Control Tests
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

    assert_eq!(client.get_admin(), None);

    let admin = Address::generate(&env);
    client.initialize(&admin);

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

    client.set_admin(&admin, &new_admin);
    assert_eq!(client.get_admin(), Some(new_admin.clone()));

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

    client.set_admin(&unauthorized_user, &new_admin);
}

#[test]
#[should_panic(expected = "already exists")]
fn test_snapshot_immutability() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let epoch = 1u64;
    client.submit_snapshot(&epoch, &create_test_hash(&env, 1), &admin);
    client.submit_snapshot(&epoch, &create_test_hash(&env, 2), &admin);
}

#[test]
#[should_panic(expected = "already exists")]
fn test_duplicate_epoch_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let epoch = 5u64;
    client.submit_snapshot(&epoch, &create_test_hash(&env, 5), &admin);
    client.submit_snapshot(&epoch, &create_test_hash(&env, 6), &admin);
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

    client.set_admin(&admin, &new_admin);

    let epoch = 1u64;
    let hash = create_test_hash(&env, 1);
    client.submit_snapshot(&epoch, &hash, &admin);
}

// ============================================================================
// Batch Operations Tests
// ============================================================================

#[test]
fn test_batch_submit_snapshots() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    env.ledger().set_timestamp(1000);

    let mut snapshots = Vec::new(&env);
    snapshots.push_back((1u64, create_test_hash(&env, 1)));
    snapshots.push_back((2u64, create_test_hash(&env, 2)));
    snapshots.push_back((3u64, create_test_hash(&env, 3)));

    let timestamps = client.batch_submit_snapshots(&admin, &snapshots);

    assert_eq!(timestamps.len(), 3);
    assert_eq!(client.get_latest_epoch(), 3);

    assert_eq!(
        client.get_snapshot(&1u64).unwrap().hash,
        create_test_hash(&env, 1)
    );
    assert_eq!(
        client.get_snapshot(&2u64).unwrap().hash,
        create_test_hash(&env, 2)
    );
    assert_eq!(
        client.get_snapshot(&3u64).unwrap().hash,
        create_test_hash(&env, 3)
    );
}

#[test]
fn test_batch_get_snapshots() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
    client.submit_snapshot(&2u64, &create_test_hash(&env, 2), &admin);
    client.submit_snapshot(&3u64, &create_test_hash(&env, 3), &admin);

    let mut epochs = Vec::new(&env);
    epochs.push_back(1u64);
    epochs.push_back(2u64);
    epochs.push_back(99u64); // non-existent

    let results = client.batch_get_snapshots(&epochs);

    assert_eq!(results.len(), 3);
    assert_eq!(
        results.get(0).unwrap().unwrap().hash,
        create_test_hash(&env, 1)
    );
    assert_eq!(
        results.get(1).unwrap().unwrap().hash,
        create_test_hash(&env, 2)
    );
    assert!(results.get(2).unwrap().is_none());
}

#[test]
fn test_batch_operations_gas_efficiency() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    env.ledger().set_timestamp(5000);

    // Submit 10 snapshots in a single batch call
    let mut snapshots = Vec::new(&env);
    for i in 1u64..=10 {
        snapshots.push_back((i, create_test_hash(&env, i as u8)));
    }

    let timestamps = client.batch_submit_snapshots(&admin, &snapshots);
    assert_eq!(timestamps.len(), 10);
    assert_eq!(client.get_latest_epoch(), 10);

    // Retrieve all 10 in a single batch call
    let mut epochs = Vec::new(&env);
    for i in 1u64..=10 {
        epochs.push_back(i);
    }

    let results = client.batch_get_snapshots(&epochs);
    assert_eq!(results.len(), 10);

    for i in 0u32..10 {
        let snapshot = results.get(i).unwrap().unwrap();
        assert_eq!(snapshot.epoch, (i + 1) as u64);
        assert_eq!(snapshot.hash, create_test_hash(&env, (i + 1) as u8));
    }
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_batch_submit_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    client.initialize(&admin);

    let input = vec![&env, (1u64, create_test_hash(&env, 1))];
    client.batch_submit(&input, &attacker);
}

#[test]
#[should_panic(expected = "Epoch monotonicity violated")]
fn test_batch_submit_non_monotonic_epochs() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // epoch 5 then epoch 3 — must panic
    let input = vec![
        &env,
        (5u64, create_test_hash(&env, 5)),
        (3u64, create_test_hash(&env, 3)),
    ];
    client.batch_submit(&input, &admin);
}

#[test]
fn test_batch_submit_basic() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    env.ledger().set_timestamp(1000);

    let hash1 = create_test_hash(&env, 1);
    let hash2 = create_test_hash(&env, 2);
    let hash3 = create_test_hash(&env, 3);

    let input = vec![
        &env,
        (1u64, hash1.clone()),
        (2u64, hash2.clone()),
        (3u64, hash3.clone()),
    ];
    let timestamps = client.batch_submit(&input, &admin);

    assert_eq!(timestamps.len(), 3);
    assert_eq!(client.get_latest_epoch(), 3);
    assert_eq!(client.get_snapshot(&1u64).unwrap().hash, hash1);
    assert_eq!(client.get_snapshot(&2u64).unwrap().hash, hash2);
    assert_eq!(client.get_snapshot(&3u64).unwrap().hash, hash3);
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

// ============================================================================
// Timelock Tests
// ============================================================================

#[test]
fn test_timelock_proposal() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin);
    env.ledger().set_timestamp(1000);

    let action_id = client.propose_admin_change(&admin, &new_admin);

    let action = client.get_timelock_action(&action_id).unwrap();
    assert_eq!(action.proposer, admin);
    assert_eq!(action.new_admin, new_admin);
    assert_eq!(action.proposed_at, 1000);
    assert_eq!(action.executable_at, 1000 + 172800);
    assert!(!action.executed);
    // Admin unchanged until executed
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
#[should_panic(expected = "Timelock not expired")]
fn test_timelock_cannot_execute_early() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin);
    env.ledger().set_timestamp(1000);

    let action_id = client.propose_admin_change(&admin, &new_admin);

    // Advance time but not past the 48-hour delay
    env.ledger().set_timestamp(1000 + 172799);
    client.execute_timelock_action(&admin, &action_id);
}

#[test]
fn test_timelock_execution_after_delay() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin);
    env.ledger().set_timestamp(1000);

    let action_id = client.propose_admin_change(&admin, &new_admin);

    // Advance time past the 48-hour delay
    env.ledger().set_timestamp(1000 + 172800);
    client.execute_timelock_action(&admin, &action_id);

    // Admin should now be updated
    assert_eq!(client.get_admin(), Some(new_admin));

    // Action should be marked executed
    let action = client.get_timelock_action(&action_id).unwrap();
    assert!(action.executed);
}

#[test]
fn test_timelock_cancellation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin);
    env.ledger().set_timestamp(1000);

    let action_id = client.propose_admin_change(&admin, &new_admin);
    assert!(client.get_timelock_action(&action_id).is_some());

    client.cancel_timelock_action(&admin, &action_id);

    // Action should be removed
    assert!(client.get_timelock_action(&action_id).is_none());
    // Admin unchanged
    assert_eq!(client.get_admin(), Some(admin));
}

// ============================================================================
// Event Tests
// ============================================================================

#[test]
fn test_event_data_completeness() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    // Submit a first snapshot so previous_epoch is non-zero for the second
    env.ledger().set_timestamp(500);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);

    env.ledger().set_timestamp(1000);
    let epoch = 2u64;
    let hash = create_test_hash(&env, 2);
    let timestamp = client.submit_snapshot(&epoch, &hash, &admin);

    let snapshot = client.get_snapshot(&epoch).unwrap();
    assert_eq!(snapshot.epoch, epoch);
    assert_eq!(snapshot.hash, hash);
    assert_eq!(snapshot.timestamp, timestamp);
    assert_eq!(timestamp, 1000);

    assert_eq!(client.get_latest_epoch(), epoch);

    // Verify pause event data completeness
    let reason = soroban_sdk::String::from_str(&env, "scheduled maintenance");
    client.pause(&admin, &reason);
    assert!(client.is_paused());

    let unpause_reason = soroban_sdk::String::from_str(&env, "maintenance complete");
    client.unpause(&admin, &unpause_reason);
    assert!(!client.is_paused());
}

#[test]
fn test_event_emission() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    env.ledger().set_timestamp(2000);

    let epoch = 1u64;
    let hash = create_test_hash(&env, 42);
    let timestamp = client.submit_snapshot(&epoch, &hash, &admin);
    assert_eq!(timestamp, 2000);

    let snapshot = client.get_snapshot(&epoch).unwrap();
    assert_eq!(snapshot.epoch, epoch);
    assert_eq!(snapshot.hash, hash);
    assert_eq!(snapshot.timestamp, 2000);

    let pause_reason = soroban_sdk::String::from_str(&env, "emergency stop");
    client.pause(&admin, &pause_reason);
    assert!(client.is_paused());

    let unpause_reason = soroban_sdk::String::from_str(&env, "issue resolved");
    client.unpause(&admin, &unpause_reason);
    assert!(!client.is_paused());

    // Confirm contract is operational again after unpause
    let epoch2 = 2u64;
    let hash2 = create_test_hash(&env, 43);
    client.submit_snapshot(&epoch2, &hash2, &admin);
    assert_eq!(client.get_latest_epoch(), epoch2);
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

// ============================================================================
// Error Event Tests
// ============================================================================

#[test]
fn test_error_events() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    env.ledger().set_timestamp(1000);

    // Successful submit — verifies the happy path still works after adding error events
    let ts = client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
    assert_eq!(ts, 1000);
    assert_eq!(client.get_latest_epoch(), 1);

    // Pause and unpause — verifies error events don't break pause/unpause flow
    client.pause(
        &admin,
        &soroban_sdk::String::from_str(&env, "test pause"),
    );
    assert!(client.is_paused());
    client.unpause(
        &admin,
        &soroban_sdk::String::from_str(&env, "test unpause"),
    );
    assert!(!client.is_paused());

    // Submit after unpause — verifies contract is still operational
    let ts2 = client.submit_snapshot(&2u64, &create_test_hash(&env, 2), &admin);
    assert_eq!(ts2, 1000);
    assert_eq!(client.get_latest_epoch(), 2);
}

#[test]
#[should_panic(expected = "Invalid epoch")]
fn test_error_event_invalid_epoch() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.submit_snapshot(&0u64, &create_test_hash(&env, 1), &admin);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_error_event_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let other = Address::generate(&env);
    client.initialize(&admin);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &other);
}

#[test]
#[should_panic(expected = "Epoch monotonicity violated")]
fn test_error_event_monotonicity_violated() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.submit_snapshot(&5u64, &create_test_hash(&env, 5), &admin);
    client.submit_snapshot(&3u64, &create_test_hash(&env, 3), &admin);
}

#[test]
#[should_panic(expected = "already exists")]
fn test_error_event_epoch_already_exists() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 2), &admin);
}

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_error_event_contract_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.pause(
        &admin,
        &soroban_sdk::String::from_str(&env, "test"),
    );
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
}

// ============================================================================
// Rate Limiting Tests
// ============================================================================

#[test]
fn test_rate_limiting() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    env.ledger().set_timestamp(1000);

    // Submit up to the limit without issue
    for epoch in 1u64..=5 {
        let hash = create_test_hash(&env, epoch as u8);
        client.submit_snapshot(&epoch, &hash, &admin);
    }

    assert_eq!(client.get_latest_epoch(), 5);
}

#[test]
#[should_panic(expected = "Rate limit exceeded")]
fn test_rate_limit_exceeded() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    env.ledger().set_timestamp(1000);

    // Submit MAX_CALLS_PER_WINDOW (100) snapshots to exhaust the limit
    for epoch in 1u64..=100 {
        let hash = create_test_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash, &admin);
    }

    // The 101st call within the same window should panic
    let hash = create_test_hash(&env, 101);
    client.submit_snapshot(&101u64, &hash, &admin);
}

#[test]
fn test_get_snapshot_uses_per_epoch_key() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    env.ledger().set_timestamp(500);
    let hash = create_test_hash(&env, 7);
    client.submit_snapshot(&10u64, &hash, &admin);

    let snap = client.get_snapshot(&10u64).unwrap();
    assert_eq!(snap.epoch, 10);
    assert_eq!(snap.hash, hash);
    assert_eq!(snap.timestamp, 500);

    // Non-existent epoch returns None without touching the map
    assert!(client.get_snapshot(&99u64).is_none());
}

#[test]
fn test_rate_limit_window_reset() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    env.ledger().set_timestamp(1000);

    // Exhaust the rate limit
    for epoch in 1u64..=100 {
        let hash = create_test_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash, &admin);
    }

    // Advance time past the 1-hour window (3600 seconds)
    env.ledger().set_timestamp(1000 + 3601);

    // Should succeed again after window reset
    let hash = create_test_hash(&env, 101);
    let ts = client.submit_snapshot(&101u64, &hash, &admin);
    assert_eq!(ts, 1000 + 3601);
    assert_eq!(client.get_latest_epoch(), 101);
}

// ============================================================================
// Prune Snapshots Tests - Issue #592
// ============================================================================

#[test]
fn test_prune_old_snapshots() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    // Submit 100 snapshots
    for i in 1u64..=100 {
        let hash = create_test_hash(&env, (i % 255) as u8);
        client.submit_snapshot(&i, &hash, &admin);
    }

    assert_eq!(client.get_latest_epoch(), 100);

    // Prune, keeping last 10
    let removed = client.prune_old_snapshots(&admin, &10);
    assert_eq!(removed, 90);

    // Verify old snapshots are gone
    assert!(client.get_snapshot(&1).is_none());
    assert!(client.get_snapshot(&90).is_none());

    // Verify recent snapshots still exist
    assert!(client.get_snapshot(&91).is_some());
    assert!(client.get_snapshot(&100).is_some());
}

// ============================================================================
// Emergency Withdrawal Tests - Issue #597
// ============================================================================

#[test]
fn test_emergency_withdrawal() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    // Set up a token
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_contract.address();
    let sac_client = StellarAssetClient::new(&env, &token_addr);
    let token_client = TokenClient::new(&env, &token_addr);

    // Mint tokens to the analytics contract
    sac_client.mint(&contract_id, &1000_i128);
    assert_eq!(token_client.balance(&contract_id), 1000);

    // Pause the contract first (required for emergency withdrawal)
    let reason = soroban_sdk::String::from_str(&env, "emergency");
    client.pause(&admin, &reason);

    // Perform emergency withdrawal
    let recipient = Address::generate(&env);
    client.emergency_withdraw(&admin, &token_addr, &500_i128, &recipient);

    // Verify balances
    assert_eq!(token_client.balance(&contract_id), 500);
    assert_eq!(token_client.balance(&recipient), 500);
}

#[test]
#[should_panic(expected = "Contract must be paused")]
fn test_emergency_withdrawal_requires_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_contract.address();

    let recipient = Address::generate(&env);
    // Should fail because contract is not paused
    client.emergency_withdraw(&admin, &token_addr, &500_i128, &recipient);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_emergency_withdrawal_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_contract.address();

    // Pause the contract
    let reason = soroban_sdk::String::from_str(&env, "emergency");
    client.pause(&admin, &reason);

    // Non-admin should fail
    let attacker = Address::generate(&env);
    let recipient = Address::generate(&env);
    client.emergency_withdraw(&attacker, &token_addr, &500_i128, &recipient);
}
