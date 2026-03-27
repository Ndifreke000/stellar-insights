#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, BytesN, Env, Vec,
};

fn create_test_hash(env: &Env, value: u8) -> BytesN<32> {
    BytesN::from_array(env, &[value; 32])
}

// ============================================================================
// Initialization Tests
// ============================================================================

#[test]
fn test_functions_require_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let result = client.try_get_snapshot(&1u64);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}


#[test]
fn test_get_latest_snapshot_requires_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let result = client.try_get_latest_snapshot();
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}


#[test]
fn test_get_snapshot_history_requires_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let result = client.try_get_snapshot_history();
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}


#[test]
fn test_get_latest_epoch_requires_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let result = client.try_get_latest_epoch();
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}


#[test]
fn test_get_all_epochs_requires_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let result = client.try_get_all_epochs();
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}


#[test]
fn test_is_snapshot_expired_requires_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let result = client.try_is_snapshot_expired(&1u64);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}


#[test]
fn test_batch_get_snapshots_requires_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let mut epochs = Vec::new(&env);
    epochs.push_back(1u64);
    let result = client.try_batch_get_snapshots(&epochs);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
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
fn test_initialize_cannot_reinitialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    let result = client.try_initialize(&admin);
    assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
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
    assert_eq!(
        version,
        soroban_sdk::String::from_str(&env, env!("CARGO_PKG_VERSION"))
    );
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

    let snapshot = client.get_snapshot(&epoch).unwrap(); // Snapshot

    assert_eq!(snapshot.epoch, epoch);
    assert_eq!(snapshot.hash, hash);
    assert_eq!(snapshot.timestamp, timestamp);
   assert_eq!(client.get_latest_epoch(), epoch);
 
   let latest = client.get_latest_snapshot().unwrap(); // Optional<Snapshot>
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

    for i in 1u64..=3 {
        let hash = create_test_hash(&env, i as u8);
        client.submit_snapshot(&i, &hash, &admin);
    }

    for i in 1u64..=3 {
       assert_eq!(client.get_snapshot(&i).unwrap().hash, create_test_hash(&env, i as u8));
     }
   assert_eq!(client.get_latest_epoch(), 3);
   assert_eq!(client.get_snapshot_history().len(), 3);
   assert_eq!(client.get_all_epochs().len(), 3);
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
    let ts1 = client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
    let snap1_before = client.get_snapshot(&1u64).unwrap();

    env.ledger().set_timestamp(200);
    let ts2 = client.submit_snapshot(&2u64, &create_test_hash(&env, 2), &admin);
    let snap2_before = client.get_snapshot(&2u64).unwrap();

    env.ledger().set_timestamp(300);
    client.submit_snapshot(&5u64, &create_test_hash(&env, 5), &admin);

    assert_eq!(client.get_snapshot(&1u64).unwrap(), snap1_before);
     assert_eq!(client.get_snapshot(&2u64).unwrap(), snap2_before);
     assert_eq!(client.get_snapshot(&1u64).unwrap().timestamp, ts1);
     assert_eq!(client.get_snapshot(&2u64).unwrap().timestamp, ts2);
   assert_eq!(client.get_latest_epoch(), 5u64);
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
fn test_invalid_epoch_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    let result = client.try_submit_snapshot(&0u64, &create_test_hash(&env, 1), &admin);
    assert_eq!(result, Err(Ok(Error::InvalidEpochZero)));
}

#[test]
fn test_duplicate_epoch_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
    let result = client.try_submit_snapshot(&1u64, &create_test_hash(&env, 2), &admin);
    assert_eq!(result, Err(Ok(Error::DuplicateEpoch)));
}

#[test]
fn test_older_epoch_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    client.submit_snapshot(&10u64, &create_test_hash(&env, 10), &admin);
    let result = client.try_submit_snapshot(&5u64, &create_test_hash(&env, 5), &admin);
    assert_eq!(result, Err(Ok(Error::EpochMonotonicityViolated)));
}

#[test]
fn test_bounded_storage_growth_simulation() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    for epoch in 1u64..=20 {
        let hash = create_test_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash, &admin);
    }

    for epoch in 1u64..=20 {
       client.get_snapshot(&epoch);
     }
   assert_eq!(client.get_latest_epoch(), 20);
   assert_eq!(client.get_snapshot_history().len(), 20);
   assert_eq!(client.get_all_epochs().len(), 20);
 }


// ============================================================================
// Access Control Tests
// ============================================================================

#[test]
fn test_unauthorized_submission_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);

    client.initialize(&admin);
    let result = client.try_submit_snapshot(&1u64, &create_test_hash(&env, 1), &unauthorized_user);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
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

    let timestamp = client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
     assert_eq!(timestamp, 1000);
   assert_eq!(client.get_latest_epoch(), 1u64);
 }


#[test]
fn test_get_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    // Before initialization, should error
    let result = client.try_get_admin();
    assert_eq!(result, Err(Ok(Error::NotInitialized)));

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
     client.set_admin(&admin, &new_admin);
   assert_eq!(client.get_admin(), Some(new_admin.clone()));
 
     client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &new_admin);
   assert_eq!(client.get_latest_epoch(), 1u64);
 }


#[test]
fn test_set_admin_by_unauthorized_user_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin);
    let result = client.try_set_admin(&unauthorized_user, &new_admin);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_snapshot_immutability() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
    let result = client.try_submit_snapshot(&1u64, &create_test_hash(&env, 2), &admin);
    assert_eq!(result, Err(Ok(Error::DuplicateEpoch)));
}

#[test]
fn test_old_admin_cannot_submit_after_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin);
    client.set_admin(&admin, &new_admin);

    let result = client.try_submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
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

    let epochs = Vec::from_array(&env, [1u64, 2u64, 99u64]); // non-existent
 
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

    let mut snapshots = Vec::new(&env);
    for i in 1u64..=10 {
        snapshots.push_back((i, create_test_hash(&env, i as u8)));
    }

    let timestamps = client.batch_submit_snapshots(&admin, &snapshots);
     assert_eq!(timestamps.len(), 10);
   assert_eq!(client.get_latest_epoch(), 10);
 
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
fn test_batch_submit_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    client.initialize(&admin);

    let input = vec![&env, (1u64, create_test_hash(&env, 1))];
    let result = client.try_batch_submit(&input, &attacker);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_batch_submit_non_monotonic_epochs() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let input = vec![
        &env,
        (5u64, create_test_hash(&env, 5)),
        (3u64, create_test_hash(&env, 3)),
    ];
    let result = client.try_batch_submit(&input, &admin);
    assert_eq!(result, Err(Ok(Error::EpochMonotonicityViolated)));
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

    env.ledger().set_timestamp(1000);
    client.submit_snapshot_with_ttl(&1u64, &create_test_hash(&env, 1), &admin, &Some(500u64));

    let snapshot = client.get_snapshot(&1u64).unwrap();
    assert_eq!(snapshot.expires_at, Some(1500u64));

    env.ledger().set_timestamp(1499);
    assert!(!client.is_snapshot_expired(&1u64));

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
    client.submit_snapshot_with_ttl(&1u64, &create_test_hash(&env, 1), &admin, &None);

    let snapshot = client.get_snapshot(&1u64).unwrap();
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
    env.ledger().set_timestamp(1000);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);

    let snapshot = client.get_snapshot(&1u64).unwrap();
    assert_eq!(snapshot.expires_at, None);

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
    env.ledger().set_timestamp(1000);

    for epoch in 1u64..=3 {
        client.submit_snapshot_with_ttl(
            &epoch,
            &create_test_hash(&env, epoch as u8),
            &admin,
            &Some(100u64),
        );
    }
    client.submit_snapshot_with_ttl(&4u64, &create_test_hash(&env, 4), &admin, &Some(10_000u64));

    env.ledger().set_timestamp(1200);

    let cleaned = client.cleanup_expired_snapshots(&admin, &2u32);
    assert_eq!(cleaned, 2);
    assert!(client.get_snapshot(&1u64).is_none());
    assert!(client.get_snapshot(&2u64).is_none());
    assert!(client.get_snapshot(&3u64).is_some());
    assert!(client.get_snapshot(&4u64).is_some());

    let cleaned2 = client.cleanup_expired_snapshots(&admin, &10u32);
    assert_eq!(cleaned2, 1);
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
        client.submit_snapshot_with_ttl(
            &epoch,
            &create_test_hash(&env, epoch as u8),
            &admin,
            &Some(100u64),
        );
    }

    env.ledger().set_timestamp(200);
    let cleaned = client.cleanup_expired_snapshots(&admin, &3u32);
    assert_eq!(cleaned, 3);
    assert_eq!(client.get_snapshot_history().len(), 2);
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
    client.submit_snapshot_with_ttl(&1u64, &create_test_hash(&env, 1), &admin, &Some(10_000u64));

    env.ledger().set_timestamp(100);
    let cleaned = client.cleanup_expired_snapshots(&admin, &10u32);
    assert_eq!(cleaned, 0);
    assert!(client.get_snapshot(&1u64).is_some());
}

#[test]
fn test_cleanup_unauthorized_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.initialize(&admin);
    let result = client.try_cleanup_expired_snapshots(&attacker, &10u32);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
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
    assert_eq!(action.executable_at, 1000 + 172_800);
    assert!(!action.executed);
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
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
    env.ledger().set_timestamp(1000 + 172_799);

    let result = client.try_execute_timelock_action(&admin, &action_id);
    assert_eq!(result, Err(Ok(Error::TimelockNotExpired)));
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
    env.ledger().set_timestamp(1000 + 172_800);
    client.execute_timelock_action(&admin, &action_id);

    assert_eq!(client.get_admin(), Some(new_admin));
    assert!(client.get_timelock_action(&action_id).unwrap().executed);
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
    client.cancel_timelock_action(&admin, &action_id);

    assert!(client.get_timelock_action(&action_id).is_none());
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn test_timelock_already_executed() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin);
    env.ledger().set_timestamp(1000);

    let action_id = client.propose_admin_change(&admin, &new_admin);
    env.ledger().set_timestamp(1000 + 172_800);
    client.execute_timelock_action(&admin, &action_id);

    // Second execution must fail with ActionAlreadyExecuted
    let result = client.try_execute_timelock_action(&admin, &action_id);
    assert_eq!(result, Err(Ok(Error::ActionAlreadyExecuted)));
}

#[test]
fn test_timelock_action_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    let result = client.try_execute_timelock_action(&admin, &999u64);
    assert_eq!(result, Err(Ok(Error::ActionNotFound)));
}

// ============================================================================
// Pause / Unpause Tests
// ============================================================================

#[test]
fn test_pause_and_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    assert!(!client.is_paused());

    client.pause(&admin, &soroban_sdk::String::from_str(&env, "maintenance"));
    assert!(client.is_paused());

    client.unpause(&admin, &soroban_sdk::String::from_str(&env, "done"));
    assert!(!client.is_paused());
}

#[test]
fn test_submit_while_paused_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    client.pause(&admin, &soroban_sdk::String::from_str(&env, "test"));

    let result = client.try_submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
    assert_eq!(result, Err(Ok(Error::ContractPaused)));
}

#[test]
fn test_pause_unauthorized_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.initialize(&admin);
    let result = client.try_pause(&attacker, &soroban_sdk::String::from_str(&env, "hack"));
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

// ============================================================================
// Error Messages Tests (verifies the spec requirement)
// ============================================================================

#[test]
fn test_error_descriptions_are_non_empty() {
    // Every variant must have a non-empty description — catches missing match arms.
    let variants = [
        Error::AlreadyInitialized,
        Error::NotInitialized,
        Error::Unauthorized,
        Error::InvalidEpoch,
        Error::InvalidEpochZero,
        Error::InvalidEpochTooLarge,
        Error::DuplicateEpoch,
        Error::EpochMonotonicityViolated,
        Error::ContractPaused,
        Error::ContractNotPaused,
        Error::InvalidHash,
        Error::InvalidHashZero,
        Error::SnapshotNotFound,
        Error::AdminNotSet,
        Error::GovernanceNotSet,
        Error::RateLimitExceeded,
        Error::TimelockNotExpired,
        Error::ActionNotFound,
        Error::ActionExpired,
        Error::ActionAlreadyExecuted,
        Error::MultiSigNotInitialized,
        Error::InvalidThreshold,
        Error::SignerNotAdmin,
    ];

    for variant in variants {
        assert!(
            !variant.description().is_empty(),
            "description missing for {:?}",
            variant
        );
        assert!(variant.code() > 0, "code must be > 0 for {:?}", variant);
    }
}

#[test]
fn test_error_codes_are_unique() {
    let variants = [
        Error::AlreadyInitialized,
        Error::NotInitialized,
        Error::Unauthorized,
        Error::InvalidEpoch,
        Error::InvalidEpochZero,
        Error::InvalidEpochTooLarge,
        Error::DuplicateEpoch,
        Error::EpochMonotonicityViolated,
        Error::ContractPaused,
        Error::ContractNotPaused,
        Error::InvalidHash,
        Error::InvalidHashZero,
        Error::SnapshotNotFound,
        Error::AdminNotSet,
        Error::GovernanceNotSet,
        Error::RateLimitExceeded,
        Error::TimelockNotExpired,
        Error::ActionNotFound,
        Error::ActionExpired,
        Error::ActionAlreadyExecuted,
        Error::MultiSigNotInitialized,
        Error::InvalidThreshold,
        Error::SignerNotAdmin,
    ];

    // Use a plain std Vec — no soroban runtime needed for this pure logic check.
    let mut seen: std::vec::Vec<u32> = std::vec::Vec::new();
    for variant in variants {
        let code = variant.code();
        assert!(
            !seen.contains(&code),
            "duplicate error code {} for {:?}",
            code,
            variant
        );
        seen.push(code);
    }
}

// ============================================================================
// Rate Limiting Tests
// ============================================================================

#[test]
fn test_rate_limiting_within_window() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    env.ledger().set_timestamp(1000);

    for epoch in 1u64..=5 {
        client.submit_snapshot(&epoch, &create_test_hash(&env, epoch as u8), &admin);
    }
    assert_eq!(client.get_latest_epoch(), 5);
}

#[test]
fn test_rate_limit_exceeded() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    env.ledger().set_timestamp(1000);

    for epoch in 1u64..=100 {
        client.submit_snapshot(&epoch, &create_test_hash(&env, (epoch % 255) as u8), &admin);
    }

    let result = client.try_submit_snapshot(&101u64, &create_test_hash(&env, 101), &admin);
    assert_eq!(result, Err(Ok(Error::RateLimitExceeded)));
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

    for epoch in 1u64..=100 {
        client.submit_snapshot(&epoch, &create_test_hash(&env, (epoch % 255) as u8), &admin);
    }

    // Advance past the 1-hour window
    env.ledger().set_timestamp(1000 + 3_601);
    let ts = client.submit_snapshot(&101u64, &create_test_hash(&env, 101), &admin);
    assert_eq!(ts, 1000 + 3_601);
    assert_eq!(client.get_latest_epoch(), 101);
}

// ============================================================================
// Prune Snapshots Tests
// ============================================================================

#[test]
fn test_prune_old_snapshots() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    for i in 1u64..=100 {
        client.submit_snapshot(&i, &create_test_hash(&env, (i % 255) as u8), &admin);
    }

    assert_eq!(client.get_latest_epoch(), 100);
    let removed = client.prune_old_snapshots(&admin, &10u32);
    assert_eq!(removed, 90);

    assert!(client.get_snapshot(&1u64).is_none());
    assert!(client.get_snapshot(&90u64).is_none());
    assert!(client.get_snapshot(&91u64).is_some());
    assert!(client.get_snapshot(&100u64).is_some());
}

// ============================================================================
// Pagination Tests
// ============================================================================

#[test]
fn test_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    for epoch in 1u64..=5 {
        client.submit_snapshot(&epoch, &create_test_hash(&env, epoch as u8), &admin);
    }

    let page1 = client.get_snapshots_paginated(&3u32, &None);
    assert_eq!(page1.snapshots.len(), 3);
    assert_eq!(page1.total_count, 5);
    assert!(page1.has_more);
    assert_eq!(page1.next_cursor, Some(4u64));
    assert_eq!(page1.snapshots.get(0).unwrap().epoch, 1u64);
    assert_eq!(page1.snapshots.get(2).unwrap().epoch, 3u64);

    let page2 = client.get_snapshots_paginated(&3u32, &page1.next_cursor);
    assert_eq!(page2.snapshots.len(), 2);
    assert!(!page2.has_more);
    assert_eq!(page2.next_cursor, None);
    assert_eq!(page2.snapshots.get(0).unwrap().epoch, 4u64);
    assert_eq!(page2.snapshots.get(1).unwrap().epoch, 5u64);
}

#[test]
fn test_pagination_cursor_non_sequential() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    for epoch in [1u64, 3u64, 5u64, 7u64, 9u64] {
        client.submit_snapshot(&epoch, &create_test_hash(&env, epoch as u8), &admin);
    }

    let page1 = client.get_snapshots_paginated(&2u32, &None);
    assert_eq!(page1.snapshots.get(0).unwrap().epoch, 1u64);
    assert_eq!(page1.snapshots.get(1).unwrap().epoch, 3u64);
    assert_eq!(page1.next_cursor, Some(4u64));

    let page2 = client.get_snapshots_paginated(&2u32, &page1.next_cursor);
    assert_eq!(page2.snapshots.get(0).unwrap().epoch, 5u64);
    assert_eq!(page2.snapshots.get(1).unwrap().epoch, 7u64);

    let page3 = client.get_snapshots_paginated(&2u32, &page2.next_cursor);
    assert_eq!(page3.snapshots.len(), 1);
    assert_eq!(page3.snapshots.get(0).unwrap().epoch, 9u64);
    assert!(!page3.has_more);
}

#[test]
fn test_pagination_empty_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let page = client.get_snapshots_paginated(&10u32, &None);
    assert_eq!(page.snapshots.len(), 0);
    assert_eq!(page.total_count, 0);
    assert!(!page.has_more);
    assert_eq!(page.next_cursor, None);
}

// ============================================================================
// Per-epoch storage key Tests
// ============================================================================

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

    assert!(client.get_snapshot(&99u64).is_none());
}

#[test]
fn test_submit_snapshot_with_ttl_stores_metadata() {
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

