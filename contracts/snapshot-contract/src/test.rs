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
