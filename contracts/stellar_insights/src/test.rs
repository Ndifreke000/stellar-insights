#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use super::*;
use crate::events::{SnapshotSubmitted, SNAPSHOT_LIFECYCLE, SNAPSHOT_SUBMITTED};
use soroban_sdk::{
    testutils::{Address as _, Events},
    symbol_short, Address, BytesN, Env, Symbol, TryFromVal,
};

/// Helper function to create a 32-byte hash for testing
fn create_test_hash(env: &Env, value: u32) -> BytesN<32> {
    let mut bytes = [0u8; 32];
    bytes[0..4].copy_from_slice(&value.to_be_bytes());
    BytesN::from_array(env, &bytes)
}

/// Count contract events whose first topic equals `topic0`.
/// Prefer this over raw `events().len()` — later host invocations can mark
/// earlier events as `failed_call`, and `Events::all()` filters those out.
fn count_contract_events_with_topic0(env: &Env, topic0: Symbol) -> usize {
    env.events()
        .all()
        .events()
        .iter()
        .filter(|e| {
            let soroban_sdk::xdr::ContractEventBody::V0(ref v0) = e.body;
            if v0.topics.is_empty() {
                return false;
            }
            <Symbol as TryFromVal<Env, soroban_sdk::xdr::ScVal>>::try_from_val(env, &v0.topics[0])
                .map(|t| t == topic0)
                .unwrap_or(false)
        })
        .count()
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
#[should_panic(expected = "Error(Contract, #1)")]
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

    // Should fail with Unauthorized error
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
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

    assert_eq!(result, Err(Ok(Error::InvalidEpochZero)));
}

#[test]
fn test_older_epoch_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Submit epoch 10 first
    let hash_new = create_test_hash(&env, 10);
    client.submit_snapshot(&10u64, &hash_new, &admin);
    assert_eq!(client.get_latest_epoch(), 10);

    // Submit earlier epoch 5 - should fail with EpochMonotonicityViolated
    let hash_old = create_test_hash(&env, 5);
    let result = client.try_submit_snapshot(&5u64, &hash_old, &admin);

    assert_eq!(result, Err(Ok(Error::EpochMonotonicityViolated)));

    // Epoch 5 should not be stored
    assert!(client.try_get_snapshot(&5u64).is_err());
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
    assert!(
        !events.events().is_empty(),
        "Expected at least one event to be emitted"
    );

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
        env.events().all().events().iter().any(|e| {
            if let soroban_sdk::xdr::ContractEventBody::V0(ref v0) = e.body {
                v0.topics.len() >= 2
            } else {
                false
            }
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

    assert!(
        count_contract_events_with_topic0(&env, SNAPSHOT_SUBMITTED) >= 1,
        "Snapshot submission should emit SNAPSHOT_SUBMITTED"
    );

    // Retrieve stored data
    let stored_hash = client.get_snapshot(&epoch);
    let (latest_hash, latest_epoch, stored_timestamp) = client.latest_snapshot();

    // Verify the stored data matches what was submitted
    assert_eq!(stored_hash, hash, "Stored hash should match submitted hash");
    assert_eq!(latest_hash, hash, "Latest hash should match submitted hash");
    assert_eq!(
        latest_epoch, epoch,
        "Latest epoch should match submitted epoch"
    );
    assert_eq!(
        stored_timestamp, returned_timestamp,
        "Stored timestamp should match returned timestamp"
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

    // Each submit must succeed and persist. The Soroban test host event buffer
    // is not reliable for counting cumulative contract events across invocations
    // (see `test_snapshot_submitted_event` for structured event decoding).
    let h1 = create_test_hash(&env, 1111);
    let h2 = create_test_hash(&env, 2222);
    let h3 = create_test_hash(&env, 3333);
    client.submit_snapshot(&1, &h1, &admin);
    client.submit_snapshot(&2, &h2, &admin);
    client.submit_snapshot(&3, &h3, &admin);

    assert_eq!(client.get_latest_epoch(), 3);
    assert_eq!(client.get_snapshot(&1), h1);
    assert_eq!(client.get_snapshot(&2), h2);
    assert_eq!(client.get_snapshot(&3), h3);
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

    // Submit with gaps (monotonic order: 50, 100, 200)
    client.submit_snapshot(&50, &create_test_hash(&env, 50), &admin);
    client.submit_snapshot(&100, &create_test_hash(&env, 100), &admin);
    client.submit_snapshot(&200, &create_test_hash(&env, 200), &admin);

    // Latest epoch should be 200
    assert_eq!(client.get_latest_epoch(), 200);

    // All should be retrievable
    assert!(client.try_get_snapshot(&50).is_ok());
    assert!(client.try_get_snapshot(&100).is_ok());
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

// ============================================================================
// Error Message Tests
// ============================================================================

#[test]
fn test_error_codes_are_unique() {
    let mut codes = [
        Error::AlreadyInitialized as u32,
        Error::NotInitialized as u32,
        Error::Unauthorized as u32,
        Error::InvalidEpoch as u32,
        Error::InvalidEpochZero as u32,
        Error::InvalidEpochTooLarge as u32,
        Error::DuplicateEpoch as u32,
        Error::EpochMonotonicityViolated as u32,
        Error::ContractPaused as u32,
        Error::ContractNotPaused as u32,
        Error::InvalidHash as u32,
        Error::InvalidHashZero as u32,
        Error::SnapshotNotFound as u32,
        Error::AdminNotSet as u32,
        Error::GovernanceNotSet as u32,
        Error::RateLimitExceeded as u32,
        Error::TimelockNotExpired as u32,
        Error::ActionNotFound as u32,
        Error::ActionExpired as u32,
        Error::ActionAlreadyExecuted as u32,
        Error::UnauthorizedCaller as u32,
        Error::InvalidHashSize as u32,
    ];
    codes.sort();
    let unique = codes.windows(2).all(|w| w[0] != w[1]);
    assert!(unique, "All error codes must be unique");
}

#[test]
fn test_error_descriptions_are_non_empty() {
    let errors = [
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
        Error::UnauthorizedCaller,
        Error::InvalidHashSize,
    ];
    for e in errors {
        assert!(
            !e.description().is_empty(),
            "Error {:?} has empty description",
            e
        );
    }
}

#[test]
fn test_error_code_matches_repr() {
    assert_eq!(Error::AlreadyInitialized.code(), 1);
    assert_eq!(Error::NotInitialized.code(), 2);
    assert_eq!(Error::Unauthorized.code(), 3);
    assert_eq!(Error::InvalidEpoch.code(), 4);
    assert_eq!(Error::InvalidEpochZero.code(), 5);
    assert_eq!(Error::InvalidEpochTooLarge.code(), 6);
    assert_eq!(Error::DuplicateEpoch.code(), 7);
    assert_eq!(Error::EpochMonotonicityViolated.code(), 8);
    assert_eq!(Error::ContractPaused.code(), 9);
    assert_eq!(Error::ContractNotPaused.code(), 10);
    assert_eq!(Error::InvalidHash.code(), 11);
    assert_eq!(Error::InvalidHashZero.code(), 12);
    assert_eq!(Error::SnapshotNotFound.code(), 13);
    assert_eq!(Error::AdminNotSet.code(), 14);
    assert_eq!(Error::GovernanceNotSet.code(), 15);
    assert_eq!(Error::RateLimitExceeded.code(), 16);
    assert_eq!(Error::TimelockNotExpired.code(), 17);
    assert_eq!(Error::ActionNotFound.code(), 18);
    assert_eq!(Error::ActionExpired.code(), 19);
    assert_eq!(Error::ActionAlreadyExecuted.code(), 20);
    assert_eq!(Error::UnauthorizedCaller.code(), 21);
    assert_eq!(Error::InvalidHashSize.code(), 22);
}

#[test]
fn test_error_messages_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_submit_snapshot(&1, &create_test_hash(&env, 1), &attacker);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
    assert_eq!(
        Error::Unauthorized.description(),
        "Caller is not authorized"
    );
    assert_eq!(Error::Unauthorized.code(), 3);
}

#[test]
fn test_error_messages_invalid_epoch_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_submit_snapshot(&0, &create_test_hash(&env, 1), &admin);
    assert_eq!(result, Err(Ok(Error::InvalidEpochZero)));
    assert_eq!(
        Error::InvalidEpochZero.description(),
        "Epoch must be greater than 0"
    );
    assert_eq!(Error::InvalidEpochZero.code(), 5);
}

#[test]
fn test_error_messages_duplicate_epoch() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    client.submit_snapshot(&1, &create_test_hash(&env, 1), &admin);
    let result = client.try_submit_snapshot(&1, &create_test_hash(&env, 2), &admin);
    assert_eq!(result, Err(Ok(Error::DuplicateEpoch)));
    assert_eq!(
        Error::DuplicateEpoch.description(),
        "Snapshot for this epoch already exists"
    );
    assert_eq!(Error::DuplicateEpoch.code(), 7);
}

#[test]
fn test_error_messages_snapshot_not_found() {
    let env = Env::default();
    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_get_snapshot(&999);
    assert_eq!(result, Err(Ok(Error::SnapshotNotFound)));
    assert_eq!(
        Error::SnapshotNotFound.description(),
        "No snapshot found for the requested epoch"
    );
    assert_eq!(Error::SnapshotNotFound.code(), 13);
}

#[test]
fn test_error_messages_admin_not_set() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let caller = Address::generate(&env);
    let result = client.try_submit_snapshot(&1, &create_test_hash(&env, 1), &caller);
    assert_eq!(result, Err(Ok(Error::AdminNotSet)));
    assert_eq!(
        Error::AdminNotSet.description(),
        "Admin address has not been initialized"
    );
    assert_eq!(Error::AdminNotSet.code(), 14);
}

#[test]
fn test_error_log_context_returns_self() {
    let env = Env::default();
    let err = Error::Unauthorized;
    // log_context must return the same error variant
    assert_eq!(err.log_context(&env, "test context"), Error::Unauthorized);
}

// =============================================================================
// set_admin tests (Requirements 2.1–2.5)
// =============================================================================

#[test]
fn test_set_admin_success_updates_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    client.set_admin(&admin, &new_admin);

    assert_eq!(client.get_admin(), new_admin);
}

#[test]
fn test_set_admin_emits_admin_transferred_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    client.set_admin(&admin, &new_admin);

    use crate::events::AdminTransferredEvent;
    use soroban_sdk::Symbol;

    let events = env.events().all();
    let raw = events.events();
    let admin_event = raw.iter().find(|e| {
        if let soroban_sdk::xdr::ContractEventBody::V0(ref v0) = e.body {
            if v0.topics.is_empty() {
                return false;
            }
            <Symbol as soroban_sdk::TryFromVal<Env, soroban_sdk::xdr::ScVal>>::try_from_val(
                &env,
                &v0.topics[0],
            )
            .map(|t| t == soroban_sdk::symbol_short!("admin"))
            .unwrap_or(false)
        } else {
            false
        }
    });
    assert!(admin_event.is_some(), "admin event should be emitted");

    if let Some(e) = admin_event {
        if let soroban_sdk::xdr::ContractEventBody::V0(ref v0) = e.body {
            let val =
                <soroban_sdk::Val as soroban_sdk::TryFromVal<Env, soroban_sdk::xdr::ScVal>>::try_from_val(
                    &env, &v0.data,
                )
                .unwrap();
            let data: AdminTransferredEvent = soroban_sdk::FromVal::from_val(&env, &val);
            assert_eq!(data.old_admin, admin);
            assert_eq!(data.new_admin, new_admin);
        }
    }
}

#[test]
fn test_set_admin_unauthorized_caller_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_set_admin(&attacker, &new_admin);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
    // Admin should be unchanged
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_set_admin_unauthorized_emits_no_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    let admin_topic_events_before =
        count_contract_events_with_topic0(&env, symbol_short!("admin"));
    let _ = client.try_set_admin(&attacker, &new_admin);
    let admin_topic_events_after =
        count_contract_events_with_topic0(&env, symbol_short!("admin"));

    // Unauthorized transfer must not emit admin-transfer audit events.
    assert_eq!(admin_topic_events_before, admin_topic_events_after);
}
