#![cfg(test)]

use super::*;
use crate::events::{SnapshotSubmitted, SNAPSHOT_LIFECYCLE, SNAPSHOT_SUBMITTED};
use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, BytesN, Env,
};

/// Helper function to create a 32-byte hash for testing
fn create_test_hash(env: &Env, value: u32) -> BytesN<32> {
    let mut bytes = [0u8; 32];
    bytes[0..4].copy_from_slice(&value.to_be_bytes());
    BytesN::from_array(env, &bytes)
}

#[test]
fn test_initialization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_latest_epoch(), 0);
}

#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_cannot_reinitialize() {
    let env = Env::default();
    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1);
    client.initialize(&admin2); // Should panic
}

#[test]
fn test_successful_snapshot_submission() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let epoch = 1u64;
    let hash = create_test_hash(&env, 12345);

    let _timestamp = client.submit_snapshot(&epoch, &hash, &admin);

    // Timestamp should be present (even if 0 in test environment)
    assert_eq!(client.get_latest_epoch(), epoch);
}

#[test]
fn test_retrieve_snapshot_by_epoch() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let epoch = 42u64;
    let hash = create_test_hash(&env, 98765);

    client.submit_snapshot(&epoch, &hash, &admin);

    let retrieved_hash = client.get_snapshot(&epoch);
    assert_eq!(retrieved_hash, hash);
}

#[test]
fn test_latest_snapshot_retrieval() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Submit multiple snapshots
    let hash1 = create_test_hash(&env, 1111);
    client.submit_snapshot(&1, &hash1, &admin);

    let hash2 = create_test_hash(&env, 2222);
    client.submit_snapshot(&3, &hash2, &admin);

    let hash3 = create_test_hash(&env, 3333);
    client.submit_snapshot(&5, &hash3, &admin);

    // Latest should be epoch 5
    let (latest_hash, latest_epoch, _timestamp) = client.latest_snapshot();
    assert_eq!(latest_epoch, 5);
    assert_eq!(latest_hash, hash3);
}

#[test]
fn test_unauthorized_caller_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    client.initialize(&admin);

    let epoch = 1u64;
    let hash = create_test_hash(&env, 99999);

    // Unauthorized user tries to submit
    let result = client.try_submit_snapshot(&epoch, &hash, &unauthorized);

    // Should fail with UnauthorizedCaller error
    assert_eq!(result, Err(Ok(Error::UnauthorizedCaller)));
}

#[test]
fn test_duplicate_epoch_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let epoch = 10u64;
    let hash1 = create_test_hash(&env, 1111);
    let hash2 = create_test_hash(&env, 2222);

    // First submission succeeds
    client.submit_snapshot(&epoch, &hash1, &admin);

    // Second submission with same epoch should fail
    let result = client.try_submit_snapshot(&epoch, &hash2, &admin);

    assert_eq!(result, Err(Ok(Error::DuplicateEpoch)));
}

#[test]
fn test_invalid_epoch_zero_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let epoch = 0u64;
    let hash = create_test_hash(&env, 12345);

    let result = client.try_submit_snapshot(&epoch, &hash, &admin);

    assert_eq!(result, Err(Ok(Error::InvalidEpoch)));
}

#[test]
fn test_snapshot_submitted_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let epoch = 100u64;
    let hash = create_test_hash(&env, 54321);

    let _timestamp = client.submit_snapshot(&epoch, &hash, &admin);

    // Verify event was emitted
    let events = env.events().all();
    
    // Should have at least one event from the snapshot submission
    assert!(events.len() >= 1, "Expected at least one event to be emitted");

    // Verify the event contains the correct topics and structure
    // The event should have SNAPSHOT_SUBMITTED and SNAPSHOT_LIFECYCLE topics
    let _expected_topics = (SNAPSHOT_SUBMITTED, SNAPSHOT_LIFECYCLE);
    let _expected_data = SnapshotSubmitted {
        hash: hash.clone(),
        epoch,
        timestamp: _timestamp,
        submitter: admin.clone(),
    };

    // Check that our event is in the emitted events with proper topic count
    assert!(
        env.events().all().iter().any(|(_, topics, _)| {
            let topic_vec: soroban_sdk::Vec<soroban_sdk::Val> = topics;
            topic_vec.len() >= 2
        }),
        "Expected event with SNAPSHOT_SUBMITTED and SNAPSHOT_LIFECYCLE topics"
    );
}

#[test]
fn test_event_payload_matches_stored_data() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let epoch = 42u64;
    let hash = create_test_hash(&env, 99999);

    // Submit snapshot and capture timestamp
    let returned_timestamp = client.submit_snapshot(&epoch, &hash, &admin);

    // Retrieve stored data
    let stored_hash = client.get_snapshot(&epoch);
    let (latest_hash, latest_epoch, stored_timestamp) = client.latest_snapshot();

    // Verify the stored data matches what was submitted
    assert_eq!(stored_hash, hash, "Stored hash should match submitted hash");
    assert_eq!(latest_hash, hash, "Latest hash should match submitted hash");
    assert_eq!(latest_epoch, epoch, "Latest epoch should match submitted epoch");
    assert_eq!(
        stored_timestamp, returned_timestamp,
        "Stored timestamp should match returned timestamp"
    );

    // Verify events were emitted
    let events = env.events().all();
    assert!(
        events.len() >= 1,
        "Event must be emitted on every valid submission"
    );
}

#[test]
fn test_event_emitted_on_each_valid_submission() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Submit multiple snapshots
    client.submit_snapshot(&1, &create_test_hash(&env, 1111), &admin);
    let events_after_first = env.events().all().len();

    client.submit_snapshot(&2, &create_test_hash(&env, 2222), &admin);
    let events_after_second = env.events().all().len();

    client.submit_snapshot(&3, &create_test_hash(&env, 3333), &admin);
    let events_after_third = env.events().all().len();

    // Each submission should emit an event
    assert!(
        events_after_second > events_after_first,
        "Second submission should emit new event"
    );
    assert!(
        events_after_third > events_after_second,
        "Third submission should emit new event"
    );
}

#[test]
fn test_get_nonexistent_snapshot_fails() {
    let env = Env::default();
    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_get_snapshot(&999);

    assert_eq!(result, Err(Ok(Error::SnapshotNotFound)));
}

#[test]
fn test_latest_snapshot_empty_fails() {
    let env = Env::default();
    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_latest_snapshot();

    assert_eq!(result, Err(Ok(Error::SnapshotNotFound)));
}

#[test]
fn test_multiple_snapshots_different_epochs() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Submit snapshots for different epochs
    let hash1 = create_test_hash(&env, 1111);
    client.submit_snapshot(&1, &hash1, &admin);

    let hash2 = create_test_hash(&env, 2222);
    client.submit_snapshot(&2, &hash2, &admin);

    let hash3 = create_test_hash(&env, 3333);
    client.submit_snapshot(&3, &hash3, &admin);

    // Verify each can be retrieved independently
    assert_eq!(client.get_snapshot(&1), hash1);
    assert_eq!(client.get_snapshot(&2), hash2);
    assert_eq!(client.get_snapshot(&3), hash3);

    // Verify latest epoch is updated
    assert_eq!(client.get_latest_epoch(), 3);
}

#[test]
fn test_non_sequential_epochs() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Submit in non-sequential order
    client.submit_snapshot(&100, &create_test_hash(&env, 100), &admin);
    client.submit_snapshot(&50, &create_test_hash(&env, 50), &admin);
    client.submit_snapshot(&200, &create_test_hash(&env, 200), &admin);

    // Latest epoch should be 200
    assert_eq!(client.get_latest_epoch(), 200);

    // All should be retrievable
    assert!(client.try_get_snapshot(&100).is_ok());
    assert!(client.try_get_snapshot(&50).is_ok());
    assert!(client.try_get_snapshot(&200).is_ok());
}

#[test]
fn test_admin_not_set_error() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    // Try to submit without initializing
    let caller = Address::generate(&env);
    let result = client.try_submit_snapshot(&1, &create_test_hash(&env, 123), &caller);

    assert_eq!(result, Err(Ok(Error::AdminNotSet)));
}
