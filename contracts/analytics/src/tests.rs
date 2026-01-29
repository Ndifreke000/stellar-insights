use super::*;
use soroban_sdk::{testutils::Ledger, BytesN, Env};

fn create_test_hash(env: &Env, value: u8) -> BytesN<32> {
    BytesN::from_array(env, &[value; 32])
}

#[test]
fn test_initialization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();

    assert_eq!(client.get_latest_epoch(), 0);
    assert_eq!(client.get_snapshot_history().len(), 0);
    assert_eq!(client.get_latest_snapshot(), None);
}

#[test]
fn test_initialize_idempotent() {
    let env = Env::default();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();
    client.initialize();

    assert_eq!(client.get_latest_epoch(), 0);
    assert_eq!(client.get_snapshot_history().len(), 0);
}

#[test]
fn test_submit_single_snapshot() {
    let env = Env::default();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();

    env.ledger().set_timestamp(1234);

    let epoch = 1u64;
    let hash = create_test_hash(&env, 1);

    let timestamp = client.submit_snapshot(&epoch, &hash);

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
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();

    let epoch1 = 1u64;
    let hash1 = create_test_hash(&env, 1);
    client.submit_snapshot(&epoch1, &hash1);

    let epoch2 = 2u64;
    let hash2 = create_test_hash(&env, 2);
    client.submit_snapshot(&epoch2, &hash2);

    let epoch3 = 3u64;
    let hash3 = create_test_hash(&env, 3);
    client.submit_snapshot(&epoch3, &hash3);

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
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();

    let epochs = [1u64, 5u64, 10u64];
    for (i, &epoch) in epochs.iter().enumerate() {
        let hash = create_test_hash(&env, (i + 1) as u8);
        client.submit_snapshot(&epoch, &hash);
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
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();

    env.ledger().set_timestamp(100);
    let epoch1 = 1u64;
    let hash1 = create_test_hash(&env, 1);
    let timestamp1 = client.submit_snapshot(&epoch1, &hash1);

    env.ledger().set_timestamp(200);
    let epoch2 = 2u64;
    let hash2 = create_test_hash(&env, 2);
    let timestamp2 = client.submit_snapshot(&epoch2, &hash2);

    let snapshot1_before = client.get_snapshot(&epoch1).unwrap();
    let snapshot2_before = client.get_snapshot(&epoch2).unwrap();

    env.ledger().set_timestamp(300);
    let epoch3 = 5u64;
    let hash3 = create_test_hash(&env, 5);
    client.submit_snapshot(&epoch3, &hash3);

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
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();

    assert_eq!(client.get_snapshot(&999), None);
}

#[test]
#[should_panic(expected = "Invalid epoch: must be greater than 0")]
fn test_invalid_epoch_zero() {
    let env = Env::default();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();

    let hash = create_test_hash(&env, 1);
    client.submit_snapshot(&0, &hash);
}

#[test]
#[should_panic(expected = "already exists")]
fn test_duplicate_epoch_fails() {
    let env = Env::default();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();

    let epoch = 1u64;
    let hash1 = create_test_hash(&env, 1);
    let hash2 = create_test_hash(&env, 2);

    client.submit_snapshot(&epoch, &hash1);
    client.submit_snapshot(&epoch, &hash2);
}

#[test]
#[should_panic(expected = "Epoch monotonicity violated")]
fn test_older_epoch_rejected() {
    let env = Env::default();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();

    let epoch_new = 10u64;
    let hash_new = create_test_hash(&env, 10);
    client.submit_snapshot(&epoch_new, &hash_new);
    assert_eq!(client.get_latest_epoch(), epoch_new);

    let epoch_old = 5u64;
    let hash_old = create_test_hash(&env, 5);
    client.submit_snapshot(&epoch_old, &hash_old);
}

#[test]
fn test_bounded_storage_growth_simulation() {
    let env = Env::default();
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(&env, &contract_id);

    client.initialize();

    let num_epochs = 100u64;
    for epoch in 1..=num_epochs {
        let hash = create_test_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash);
    }

    for epoch in 1..=num_epochs {
        assert!(client.get_snapshot(&epoch).is_some());
    }

    assert_eq!(client.get_latest_epoch(), num_epochs);
    assert_eq!(client.get_snapshot_history().len(), num_epochs as u32);
    assert_eq!(client.get_all_epochs().len(), num_epochs as u32);
}
