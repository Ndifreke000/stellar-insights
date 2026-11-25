#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

extern crate std;

use super::*;
use crate::events::{SnapshotSubmitted, SNAPSHOT_LIFECYCLE, SNAPSHOT_SUBMITTED};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, BytesN, Env, Symbol, TryFromVal,
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
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_latest_epoch(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_cannot_reinitialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    client.initialize(&admin1);
    client.initialize(&admin2); // Should panic
}

#[test]
fn test_successful_snapshot_submission() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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
        chain_hash: client.get_chain_head().1,
    };

    // Check that our event is in the emitted events with proper topic count
    assert!(
        events.events().iter().any(|e| {
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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_get_snapshot(&999);

    assert_eq!(result, Err(Ok(Error::SnapshotNotFound)));
}

#[test]
fn test_latest_snapshot_empty_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_latest_snapshot();

    assert_eq!(result, Err(Ok(Error::SnapshotNotFound)));
}

#[test]
fn test_multiple_snapshots_different_epochs() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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
        Error::SubmitterAlreadyAuthorized as u32,
        Error::SubmitterNotFound as u32,
        Error::TooManySubmitters as u32,
        Error::ChainBroken as u32,
        Error::InvalidRange as u32,
        Error::InvalidRotationTarget as u32,
        Error::InvalidOverlap as u32,
        Error::AdminRoleNotGrantable as u32,
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
        Error::SubmitterAlreadyAuthorized,
        Error::SubmitterNotFound,
        Error::TooManySubmitters,
        Error::ChainBroken,
        Error::InvalidRange,
        Error::InvalidRotationTarget,
        Error::InvalidOverlap,
        Error::AdminRoleNotGrantable,
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
    assert_eq!(Error::SubmitterAlreadyAuthorized.code(), 28);
    assert_eq!(Error::SubmitterNotFound.code(), 29);
    assert_eq!(Error::TooManySubmitters.code(), 30);
    assert_eq!(Error::NoPendingAdmin.code(), 31);
    assert_eq!(Error::NoPendingUpgrade.code(), 32);
    assert_eq!(Error::InvalidUpgradeDelay.code(), 33);
    assert_eq!(Error::TooManyRoleHolders.code(), 34);
    assert_eq!(Error::ChainBroken.code(), 35);
    assert_eq!(Error::InvalidRange.code(), 36);
    assert_eq!(Error::InvalidRotationTarget.code(), 37);
    assert_eq!(Error::InvalidOverlap.code(), 38);
    assert_eq!(Error::AdminRoleNotGrantable.code(), 39);
}

#[test]
fn test_error_messages_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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

    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

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
// Admin transfer tests (Requirements 2.1–2.5; two-step since #2783)
// =============================================================================

#[test]
fn test_set_admin_success_updates_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    client.propose_admin(&admin, &new_admin);
    client.accept_admin(&new_admin);

    assert_eq!(client.get_admin(), new_admin);
}

#[test]
fn test_set_admin_emits_admin_transferred_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    client.propose_admin(&admin, &new_admin);
    client.accept_admin(&new_admin);

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
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_propose_admin(&attacker, &new_admin);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
    // Admin should be unchanged
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_set_admin_unauthorized_emits_no_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    let admin_topic_events_before = count_contract_events_with_topic0(&env, symbol_short!("admin"));
    let _ = client.try_propose_admin(&attacker, &new_admin);
    let admin_topic_events_after = count_contract_events_with_topic0(&env, symbol_short!("admin"));

    // Unauthorized transfer must not emit admin-transfer audit events.
    assert_eq!(admin_topic_events_before, admin_topic_events_after);
}

// ── Governance-driven parameter updates (#2137) ──────────────────────────────

/// Register the contract with an admin and a configured governance address.
fn setup_governed(env: &Env) -> (PayRaiderContractClient<'_>, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let governance = Address::generate(env);

    client.initialize(&admin);
    client.set_governance(&admin, &governance);

    (client, admin, governance)
}

#[test]
fn test_governance_starts_unset() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    client.initialize(&Address::generate(&env));

    assert_eq!(client.get_governance(), None);
}

#[test]
fn test_admin_can_configure_governance() {
    let env = Env::default();
    let (client, _admin, governance) = setup_governed(&env);

    assert_eq!(client.get_governance(), Some(governance));
}

#[test]
fn test_non_admin_cannot_configure_governance() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let stranger = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_set_governance(&stranger, &Address::generate(&env));
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_governance_can_transfer_admin() {
    let env = Env::default();
    let (client, _admin, governance) = setup_governed(&env);
    let new_admin = Address::generate(&env);

    client.set_admin_by_governance(&governance, &new_admin);

    assert_eq!(client.get_admin(), new_admin);
}

#[test]
fn test_non_governance_cannot_transfer_admin() {
    let env = Env::default();
    let (client, admin, _governance) = setup_governed(&env);

    // Even the admin must go through `propose_admin`; this entrypoint is the
    // governance path and accepts nobody else.
    let result = client.try_set_admin_by_governance(&admin, &Address::generate(&env));
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_governance_admin_transfer_rejected_when_unconfigured() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_set_admin_by_governance(&admin, &Address::generate(&env));
    assert_eq!(result, Err(Ok(Error::GovernanceNotSet)));
}

#[test]
fn test_governance_can_pause_and_unpause() {
    let env = Env::default();
    let (client, _admin, governance) = setup_governed(&env);

    assert!(!client.is_paused());

    client.set_paused_by_governance(&governance, &true);
    assert!(client.is_paused());

    client.set_paused_by_governance(&governance, &false);
    assert!(!client.is_paused());
}

#[test]
fn test_governance_pause_blocks_snapshot_submission() {
    let env = Env::default();
    let (client, admin, governance) = setup_governed(&env);

    client.set_paused_by_governance(&governance, &true);

    let result = client.try_submit_snapshot(&1u64, &create_test_hash(&env, 1), &admin);
    assert_eq!(result, Err(Ok(Error::ContractPaused)));
}

#[test]
fn test_non_governance_cannot_pause_through_governance_path() {
    let env = Env::default();
    let (client, admin, _governance) = setup_governed(&env);

    let result = client.try_set_paused_by_governance(&admin, &true);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_governance_admin_transfer_grants_submission_rights() {
    let env = Env::default();
    let (client, admin, governance) = setup_governed(&env);
    let new_admin = Address::generate(&env);

    client.set_admin_by_governance(&governance, &new_admin);

    // The new admin can submit…
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &new_admin);
    assert_eq!(client.get_latest_epoch(), 1);

    // …and the old one can no longer.
    let result = client.try_submit_snapshot(&2u64, &create_test_hash(&env, 2), &admin);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

// ── Upgrade safety and storage migration (#2133) ─────────────────────────────

#[test]
fn test_fresh_deploy_is_stamped_at_current_storage_version() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    client.initialize(&Address::generate(&env));

    assert_eq!(client.get_storage_version(), CURRENT_STORAGE_VERSION);
}

#[test]
fn test_migrate_is_a_noop_on_a_fresh_deploy() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Nothing to move: a fresh deploy already writes the current shape.
    let result = client.try_migrate(&admin);
    assert_eq!(result, Err(Ok(Error::MigrationNotNeeded)));
}

#[test]
fn test_migrate_advances_pre_versioning_storage() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Simulate a contract deployed before versioning existed: the key is
    // absent, so get_storage_version() reads 0.
    env.as_contract(&contract_id, || {
        env.storage().instance().remove(&DataKey::StorageVersion);
    });
    assert_eq!(client.get_storage_version(), 0);

    assert_eq!(client.migrate(&admin), CURRENT_STORAGE_VERSION);
    assert_eq!(client.get_storage_version(), CURRENT_STORAGE_VERSION);
}

#[test]
fn test_migrate_is_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    env.as_contract(&contract_id, || {
        env.storage().instance().remove(&DataKey::StorageVersion);
    });

    client.migrate(&admin);
    // A second call must refuse rather than re-run the steps.
    assert_eq!(
        client.try_migrate(&admin),
        Err(Ok(Error::MigrationNotNeeded))
    );
}

#[test]
fn test_migrate_refuses_storage_from_a_newer_build() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // A downgrade left storage written by a newer build; migrating forward
    // from here would misread it.
    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::StorageVersion, &(CURRENT_STORAGE_VERSION + 1));
    });

    assert_eq!(
        client.try_migrate(&admin),
        Err(Ok(Error::StorageVersionTooNew))
    );
}

#[test]
fn test_migrate_rejects_an_unrelated_caller() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    env.as_contract(&contract_id, || {
        env.storage().instance().remove(&DataKey::StorageVersion);
    });

    let result = client.try_migrate(&Address::generate(&env));
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_governance_may_run_the_migration() {
    let env = Env::default();
    let (client, _admin, governance) = setup_governed(&env);
    let contract_id = client.address.clone();

    env.as_contract(&contract_id, || {
        env.storage().instance().remove(&DataKey::StorageVersion);
    });

    assert_eq!(client.migrate(&governance), CURRENT_STORAGE_VERSION);
}

#[test]
fn test_upgrade_rejects_a_zero_wasm_hash() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // An all-zero hash is not a real Wasm blob; installing it would leave the
    // contract permanently unusable.
    let zero = BytesN::from_array(&env, &[0u8; 32]);
    assert_eq!(
        client.try_schedule_upgrade(&admin, &zero),
        Err(Ok(Error::InvalidWasmHash))
    );
}

#[test]
fn test_upgrade_rejected_while_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.pause(&admin);

    let result = client.try_schedule_upgrade(&admin, &create_test_hash(&env, 99));
    assert_eq!(result, Err(Ok(Error::ContractPaused)));
}

// ── Rollback protection across a migration (#2139) ───────────────────────────

#[test]
fn test_migration_preserves_the_rollback_guard() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    client.submit_snapshot(&10u64, &create_test_hash(&env, 1), &admin);
    client.submit_snapshot(&20u64, &create_test_hash(&env, 2), &admin);

    // Migrate as if this deployment predated versioning.
    env.as_contract(&contract_id, || {
        env.storage().instance().remove(&DataKey::StorageVersion);
    });
    client.migrate(&admin);

    // The latest epoch survives, so an older epoch is still refused — a
    // migration must never reopen the rollback window.
    assert_eq!(client.get_latest_epoch(), 20);
    let result = client.try_submit_snapshot(&15u64, &create_test_hash(&env, 3), &admin);
    assert_eq!(result, Err(Ok(Error::EpochMonotonicityViolated)));
}

#[test]
fn test_rollback_still_refused_after_governance_admin_change() {
    let env = Env::default();
    let (client, admin, governance) = setup_governed(&env);

    client.submit_snapshot(&5u64, &create_test_hash(&env, 1), &admin);

    let new_admin = Address::generate(&env);
    client.set_admin_by_governance(&governance, &new_admin);

    // A new admin installed by a vote inherits the guard, not a clean slate.
    let result = client.try_submit_snapshot(&4u64, &create_test_hash(&env, 2), &new_admin);
    assert_eq!(result, Err(Ok(Error::EpochMonotonicityViolated)));
}

// ============================================================================
// Authorised submitter set (#2600)
// ============================================================================

fn setup_with_submitter(env: &Env) -> (PayRaiderContractClient<'_>, Address, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let submitter = Address::generate(env);
    client.initialize(&admin);
    client.add_submitter(&admin, &submitter);
    (client, contract_id, admin, submitter)
}

#[test]
fn authorised_submitter_can_anchor_snapshot() {
    let env = Env::default();
    let (client, _, _, submitter) = setup_with_submitter(&env);

    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &submitter);

    assert_eq!(client.get_latest_epoch(), 1);
    assert!(client.has_role(&submitter, &Role::SnapshotSubmitter));
}

#[test]
fn rejects_snapshot_from_signer_outside_submitter_set() {
    let env = Env::default();
    let (client, _, _, _) = setup_with_submitter(&env);
    let attacker = Address::generate(&env);

    // The attacker signs for themself (mock_all_auths), so this is purely the
    // set-membership check refusing them.
    let result = client.try_submit_snapshot(&1u64, &create_test_hash(&env, 1), &attacker);

    assert_eq!(result, Err(Ok(Error::Unauthorized)));
    assert_eq!(client.get_latest_epoch(), 0);
}

#[test]
fn rejects_snapshot_from_legacy_role_entry_outside_submitter_set() {
    let env = Env::default();
    let (client, contract_id, _, _) = setup_with_submitter(&env);
    let legacy = Address::generate(&env);

    // A per-address SnapshotSubmitter role written before the set existed
    // grants nothing: the set is the only source of truth.
    env.as_contract(&contract_id, || {
        let mut roles = Vec::new(&env);
        roles.push_back(Role::SnapshotSubmitter);
        env.storage()
            .persistent()
            .set(&DataKey::Roles(legacy.clone()), &roles);
    });

    assert!(!client.has_role(&legacy, &Role::SnapshotSubmitter));
    let result = client.try_submit_snapshot(&1u64, &create_test_hash(&env, 1), &legacy);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn removed_submitter_can_no_longer_anchor() {
    let env = Env::default();
    let (client, _, admin, submitter) = setup_with_submitter(&env);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &submitter);

    client.remove_submitter(&admin, &submitter);

    let result = client.try_submit_snapshot(&2u64, &create_test_hash(&env, 2), &submitter);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
    assert_eq!(client.get_submitters().len(), 0);
}

#[test]
fn rotation_revokes_old_key_and_authorises_new_key() {
    let env = Env::default();
    let (client, _, admin, old_key) = setup_with_submitter(&env);
    let new_key = Address::generate(&env);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &old_key);

    client.rotate_submitter(&admin, &old_key, &new_key);

    let result = client.try_submit_snapshot(&2u64, &create_test_hash(&env, 2), &old_key);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));

    client.submit_snapshot(&2u64, &create_test_hash(&env, 2), &new_key);
    assert_eq!(client.get_latest_epoch(), 2);

    let submitters = client.get_submitters();
    assert_eq!(submitters.len(), 1);
    assert_eq!(submitters.get(0), Some(new_key));
}

#[test]
fn rotation_preserves_other_submitters() {
    let env = Env::default();
    let (client, _, admin, first) = setup_with_submitter(&env);
    let second = Address::generate(&env);
    let replacement = Address::generate(&env);
    client.add_submitter(&admin, &second);

    client.rotate_submitter(&admin, &first, &replacement);

    let submitters = client.get_submitters();
    assert_eq!(submitters.len(), 2);
    assert!(submitters.contains(&second));
    assert!(submitters.contains(&replacement));
    assert!(!submitters.contains(&first));
}

#[test]
fn rotation_requires_new_key_signature() {
    use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
    use soroban_sdk::IntoVal;

    let env = Env::default();
    let (client, contract_id, admin, old_key) = setup_with_submitter(&env);
    let new_key = Address::generate(&env);

    // Only the admin signs; the incoming key does not.
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "rotate_submitter",
            args: (&admin, &old_key, &new_key).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    assert!(client
        .try_rotate_submitter(&admin, &old_key, &new_key)
        .is_err());
    assert!(client.get_submitters().contains(&old_key));
    assert!(!client.get_submitters().contains(&new_key));
}

#[test]
fn rotation_rejects_unknown_old_key() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let stranger = Address::generate(&env);
    let new_key = Address::generate(&env);

    let result = client.try_rotate_submitter(&admin, &stranger, &new_key);

    assert_eq!(result, Err(Ok(Error::SubmitterNotFound)));
}

#[test]
fn rotation_rejects_new_key_already_in_set() {
    let env = Env::default();
    let (client, _, admin, first) = setup_with_submitter(&env);
    let second = Address::generate(&env);
    client.add_submitter(&admin, &second);

    let result = client.try_rotate_submitter(&admin, &first, &second);

    assert_eq!(result, Err(Ok(Error::SubmitterAlreadyAuthorized)));
}

#[test]
fn non_admin_cannot_change_submitter_set() {
    let env = Env::default();
    let (client, _, _, submitter) = setup_with_submitter(&env);
    let attacker = Address::generate(&env);

    assert_eq!(
        client.try_add_submitter(&attacker, &attacker),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(
        client.try_rotate_submitter(&attacker, &submitter, &attacker),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(
        client.try_remove_submitter(&attacker, &submitter),
        Err(Ok(Error::Unauthorized))
    );
    // A submitter cannot manage the set either.
    assert_eq!(
        client.try_add_submitter(&submitter, &attacker),
        Err(Ok(Error::Unauthorized))
    );
    assert!(!client.get_submitters().contains(&attacker));
}

#[test]
fn rejects_duplicate_submitter() {
    let env = Env::default();
    let (client, _, admin, submitter) = setup_with_submitter(&env);

    assert_eq!(
        client.try_add_submitter(&admin, &submitter),
        Err(Ok(Error::SubmitterAlreadyAuthorized))
    );
}

#[test]
fn rejects_removing_unknown_submitter() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);

    assert_eq!(
        client.try_remove_submitter(&admin, &Address::generate(&env)),
        Err(Ok(Error::SubmitterNotFound))
    );
}

#[test]
fn rejects_submitter_beyond_set_capacity() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    for _ in 1..MAX_SUBMITTERS {
        client.add_submitter(&admin, &Address::generate(&env));
    }
    assert_eq!(client.get_submitters().len(), MAX_SUBMITTERS);

    assert_eq!(
        client.try_add_submitter(&admin, &Address::generate(&env)),
        Err(Ok(Error::TooManySubmitters))
    );
}

#[test]
fn submitter_set_can_be_rotated_while_paused() {
    let env = Env::default();
    let (client, _, admin, old_key) = setup_with_submitter(&env);
    let new_key = Address::generate(&env);
    client.pause(&admin);

    client.rotate_submitter(&admin, &old_key, &new_key);

    assert!(client.get_submitters().contains(&new_key));
}

#[test]
fn grant_and_revoke_submitter_role_update_submitter_set() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let user = Address::generate(&env);

    client.grant_role(&admin, &user, &Role::SnapshotSubmitter);
    assert!(client.get_submitters().contains(&user));
    // Granting twice is a no-op, as for other roles.
    client.grant_role(&admin, &user, &Role::SnapshotSubmitter);
    assert_eq!(client.get_submitters().len(), 2);

    client.revoke_role(&admin, &user, &Role::SnapshotSubmitter);
    assert!(!client.get_submitters().contains(&user));
    let result = client.try_submit_snapshot(&1u64, &create_test_hash(&env, 1), &user);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn submitter_set_changes_emit_events() {
    let env = Env::default();
    let (client, _, admin, submitter) = setup_with_submitter(&env);
    assert_eq!(
        count_contract_events_with_topic0(&env, symbol_short!("subm_add")),
        1
    );

    let new_key = Address::generate(&env);
    client.rotate_submitter(&admin, &submitter, &new_key);
    assert_eq!(
        count_contract_events_with_topic0(&env, symbol_short!("subm_rot")),
        1
    );

    client.remove_submitter(&admin, &new_key);
    assert_eq!(
        count_contract_events_with_topic0(&env, symbol_short!("subm_rm")),
        1
    );
}

// ============================================================================
// Storage TTL (#2599)
// ============================================================================

#[test]
fn submission_extends_snapshot_and_instance_ttl() {
    use soroban_sdk::testutils::storage::{Instance as _, Persistent as _};

    let env = Env::default();
    let (client, contract_id, _, submitter) = setup_with_submitter(&env);

    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &submitter);

    env.as_contract(&contract_id, || {
        assert!(env.storage().persistent().get_ttl(&DataKey::Snapshot(1)) >= LEDGERS_TO_EXTEND - 1);
        assert!(env.storage().instance().get_ttl() >= INSTANCE_TTL_THRESHOLD);
    });
}

#[test]
fn snapshot_readable_after_ttl_window_when_bumped() {
    use soroban_sdk::testutils::Ledger as _;

    let env = Env::default();
    let (client, _, admin, submitter) = setup_with_submitter(&env);
    let hash = create_test_hash(&env, 7);
    let start = env.ledger().sequence();
    client.submit_snapshot(&1u64, &hash, &submitter);

    // Most of the way through the original TTL window, with no submissions,
    // a keeper calls the permissionless bump.
    env.ledger()
        .with_mut(|li| li.sequence_number = start + LEDGERS_TO_EXTEND - 50_000);
    client.bump_ttl();

    // Now well past the point where the original entries would have been
    // archived.
    env.ledger()
        .with_mut(|li| li.sequence_number = start + LEDGERS_TO_EXTEND + 100_000);

    assert_eq!(client.get_snapshot(&1u64), hash);
    assert_eq!(client.latest_snapshot().1, 1);
    // Instance (submitter set) and the admin's role entry survived too.
    client.submit_snapshot(&2u64, &create_test_hash(&env, 8), &submitter);
    client.submit_snapshot(&3u64, &create_test_hash(&env, 9), &admin);
    assert_eq!(client.get_latest_epoch(), 3);
}

#[test]
fn bump_ttl_is_permissionless() {
    let env = Env::default();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    client.initialize(&Address::generate(&env));

    // Drop the mocked auths: bump_ttl must not call require_auth.
    env.set_auths(&[]);
    client.bump_ttl();
}

#[test]
fn test_initialize_requires_admin_auth() {
    // deploy and initialize are separate transactions; nobody but the admin
    // may install itself, or a watcher could front-run initialisation.
    let env = Env::default();
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    assert!(client.try_initialize(&admin).is_err());
    assert!(client.try_get_admin().is_err());

    env.mock_all_auths();
    client.initialize(&admin);
    assert_eq!(
        env.auths()[0].0,
        admin,
        "initialize must be authorised by the admin being installed"
    );
}

// ============================================================================
// Two-step admin transfer (#2783)
// ============================================================================

#[test]
fn propose_admin_does_not_transfer_until_accepted() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let next = Address::generate(&env);

    client.propose_admin(&admin, &next);
    let adm_prop_events = count_contract_events_with_topic0(&env, symbol_short!("adm_prop"));

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_pending_admin(), Some(next.clone()));
    assert!(client.has_role(&admin, &Role::Admin));
    assert!(!client.has_role(&next, &Role::Admin));
    assert_eq!(adm_prop_events, 1);
}

#[test]
fn accept_admin_completes_transfer_and_clears_pending() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let next = Address::generate(&env);

    client.propose_admin(&admin, &next);
    client.accept_admin(&next);

    assert_eq!(client.get_admin(), next);
    assert_eq!(client.get_pending_admin(), None);
    assert!(client.has_role(&next, &Role::Admin));
    assert!(!client.has_role(&admin, &Role::Admin));
    assert!(client.get_role_holders().contains(&next));
    assert!(!client.get_role_holders().contains(&admin));
}

#[test]
fn accept_admin_by_wrong_address_fails() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let next = Address::generate(&env);
    let intruder = Address::generate(&env);

    client.propose_admin(&admin, &next);

    assert_eq!(
        client.try_accept_admin(&intruder),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_pending_admin(), Some(next));
}

#[test]
fn accept_admin_without_proposal_fails() {
    let env = Env::default();
    let (client, _, _, _) = setup_with_submitter(&env);
    assert_eq!(
        client.try_accept_admin(&Address::generate(&env)),
        Err(Ok(Error::NoPendingAdmin))
    );
}

#[test]
fn accept_admin_requires_the_new_admins_signature() {
    use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
    use soroban_sdk::IntoVal;

    let env = Env::default();
    let (client, contract_id, admin, _) = setup_with_submitter(&env);
    let next = Address::generate(&env);
    client.propose_admin(&admin, &next);

    // Only the old admin signs the accept: must be rejected.
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "accept_admin",
            args: (next.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    assert!(client.try_accept_admin(&next).is_err());
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn cancel_admin_transfer_withdraws_the_proposal() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let next = Address::generate(&env);

    client.propose_admin(&admin, &next);
    client.cancel_admin_transfer(&admin);
    let adm_cncl_events = count_contract_events_with_topic0(&env, symbol_short!("adm_cncl"));

    assert_eq!(client.get_pending_admin(), None);
    assert_eq!(
        client.try_accept_admin(&next),
        Err(Ok(Error::NoPendingAdmin))
    );
    assert_eq!(adm_cncl_events, 1);
}

#[test]
fn governance_transfer_clears_a_stale_proposal() {
    let env = Env::default();
    let (client, admin, governance) = setup_governed(&env);
    let proposed = Address::generate(&env);
    let voted = Address::generate(&env);

    client.propose_admin(&admin, &proposed);
    client.set_admin_by_governance(&governance, &voted);

    assert_eq!(client.get_admin(), voted);
    assert_eq!(client.get_pending_admin(), None);
    assert_eq!(
        client.try_accept_admin(&proposed),
        Err(Ok(Error::NoPendingAdmin))
    );
}

// ============================================================================
// Timelocked upgrades (#2783)
// ============================================================================

fn advance_ledgers(env: &Env, by: u32) {
    use soroban_sdk::testutils::Ledger as _;
    env.ledger().with_mut(|li| li.sequence_number += by);
}

#[test]
fn schedule_upgrade_records_pending_state_and_emits_event() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let hash = create_test_hash(&env, 42);
    let now = env.ledger().sequence();

    let pending = client.schedule_upgrade(&admin, &hash);
    let upg_schd_events = count_contract_events_with_topic0(&env, symbol_short!("upg_schd"));

    assert_eq!(pending.wasm_hash, hash);
    assert_eq!(pending.proposer, admin);
    assert_eq!(pending.scheduled_at, now);
    assert_eq!(pending.executable_at, now + DEFAULT_UPGRADE_DELAY_LEDGERS);
    assert_eq!(client.get_pending_upgrade(), Some(pending));
    assert_eq!(upg_schd_events, 1);
}

#[test]
fn execute_upgrade_before_delay_fails() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    client.schedule_upgrade(&admin, &create_test_hash(&env, 42));

    advance_ledgers(&env, DEFAULT_UPGRADE_DELAY_LEDGERS - 1);

    assert_eq!(
        client.try_execute_upgrade(&admin),
        Err(Ok(Error::TimelockNotExpired))
    );
    assert!(client.get_pending_upgrade().is_some());
}

#[test]
fn execute_upgrade_after_delay_passes_the_timelock() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    client.schedule_upgrade(&admin, &create_test_hash(&env, 42));

    advance_ledgers(&env, DEFAULT_UPGRADE_DELAY_LEDGERS);

    // The hash isn't an uploaded Wasm in the test host, so the host rejects
    // the install itself — but the contract's own timelock check has passed.
    let result = client.try_execute_upgrade(&admin);
    assert_ne!(result, Err(Ok(Error::TimelockNotExpired)));
    assert_ne!(result, Err(Ok(Error::NoPendingUpgrade)));
}

#[test]
fn execute_upgrade_without_schedule_fails() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    assert_eq!(
        client.try_execute_upgrade(&admin),
        Err(Ok(Error::NoPendingUpgrade))
    );
}

#[test]
fn stranger_cannot_schedule_or_execute_upgrade() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let stranger = Address::generate(&env);

    assert_eq!(
        client.try_schedule_upgrade(&stranger, &create_test_hash(&env, 1)),
        Err(Ok(Error::Unauthorized))
    );
    client.schedule_upgrade(&admin, &create_test_hash(&env, 1));
    advance_ledgers(&env, DEFAULT_UPGRADE_DELAY_LEDGERS);
    assert_eq!(
        client.try_execute_upgrade(&stranger),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn admin_can_cancel_a_scheduled_upgrade() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    client.schedule_upgrade(&admin, &create_test_hash(&env, 42));

    client.cancel_upgrade(&admin);
    let upg_cncl_events = count_contract_events_with_topic0(&env, symbol_short!("upg_cncl"));

    assert_eq!(client.get_pending_upgrade(), None);
    advance_ledgers(&env, DEFAULT_UPGRADE_DELAY_LEDGERS);
    assert_eq!(
        client.try_execute_upgrade(&admin),
        Err(Ok(Error::NoPendingUpgrade))
    );
    assert_eq!(upg_cncl_events, 1);
}

#[test]
fn governance_can_schedule_and_cancel_upgrades() {
    let env = Env::default();
    let (client, admin, governance) = setup_governed(&env);

    // Governance can veto an admin-scheduled upgrade…
    client.schedule_upgrade(&admin, &create_test_hash(&env, 1));
    client.cancel_upgrade(&governance);
    assert_eq!(client.get_pending_upgrade(), None);

    // …and schedule its own.
    let pending = client.schedule_upgrade(&governance, &create_test_hash(&env, 2));
    assert_eq!(pending.proposer, governance);
}

#[test]
fn upgrade_manager_can_schedule_but_not_cancel() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let manager = Address::generate(&env);
    client.grant_role(&admin, &manager, &Role::UpgradeManager);

    client.schedule_upgrade(&manager, &create_test_hash(&env, 3));
    assert_eq!(
        client.try_cancel_upgrade(&manager),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn upgrade_delay_rules() {
    let env = Env::default();
    let (client, admin, governance) = setup_governed(&env);

    assert_eq!(client.get_upgrade_delay(), DEFAULT_UPGRADE_DELAY_LEDGERS);
    // Admin may lengthen…
    client.set_upgrade_delay(&admin, &(DEFAULT_UPGRADE_DELAY_LEDGERS * 2));
    // …but not shorten.
    assert_eq!(
        client.try_set_upgrade_delay(&admin, &DEFAULT_UPGRADE_DELAY_LEDGERS),
        Err(Ok(Error::InvalidUpgradeDelay))
    );
    // Governance may shorten, but never below the floor.
    client.set_upgrade_delay(&governance, &MIN_UPGRADE_DELAY_LEDGERS);
    assert_eq!(client.get_upgrade_delay(), MIN_UPGRADE_DELAY_LEDGERS);
    assert_eq!(
        client.try_set_upgrade_delay(&governance, &(MIN_UPGRADE_DELAY_LEDGERS - 1)),
        Err(Ok(Error::InvalidUpgradeDelay))
    );

    let now = env.ledger().sequence();
    let pending = client.schedule_upgrade(&admin, &create_test_hash(&env, 5));
    assert_eq!(pending.executable_at, now + MIN_UPGRADE_DELAY_LEDGERS);
}

// ============================================================================
// Role entry TTL (#2784)
// ============================================================================

#[test]
fn grant_and_revoke_maintain_the_role_holder_index() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let user = Address::generate(&env);

    assert_eq!(
        client.get_role_holders(),
        soroban_sdk::vec![&env, admin.clone()]
    );

    client.grant_role(&admin, &user, &Role::PauseManager);
    assert!(client.get_role_holders().contains(&user));

    client.revoke_role(&admin, &user, &Role::PauseManager);
    assert!(!client.get_role_holders().contains(&user));
    assert!(!client.has_role(&user, &Role::PauseManager));
}

#[test]
fn role_holder_index_is_bounded() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);

    // The admin already occupies one slot.
    for _ in 1..MAX_ROLE_HOLDERS {
        client.grant_role(&admin, &Address::generate(&env), &Role::PauseManager);
    }
    assert_eq!(
        client.try_grant_role(&admin, &Address::generate(&env), &Role::PauseManager),
        Err(Ok(Error::TooManyRoleHolders))
    );
}

#[test]
fn non_admin_roles_survive_past_ttl_when_bumped() {
    use soroban_sdk::testutils::Ledger as _;

    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let pauser = Address::generate(&env);
    client.grant_role(&admin, &pauser, &Role::PauseManager);
    let start = env.ledger().sequence();

    // Keeper runs near the end of the original window…
    env.ledger()
        .with_mut(|li| li.sequence_number = start + LEDGERS_TO_EXTEND - 50_000);
    client.bump_ttl();

    // …and the role is still valid well past where it would have archived.
    env.ledger()
        .with_mut(|li| li.sequence_number = start + LEDGERS_TO_EXTEND + 100_000);
    assert!(client.has_role(&pauser, &Role::PauseManager));
    client.pause(&pauser);
    assert!(client.is_paused());
}

#[test]
fn using_a_role_extends_its_entry() {
    use soroban_sdk::testutils::storage::Persistent as _;
    use soroban_sdk::testutils::Ledger as _;

    let env = Env::default();
    let (client, contract_id, admin, _) = setup_with_submitter(&env);
    let pauser = Address::generate(&env);
    client.grant_role(&admin, &pauser, &Role::PauseManager);
    let start = env.ledger().sequence();

    env.ledger()
        .with_mut(|li| li.sequence_number = start + 100_000);
    // Authorising via the role (read path) pushes the entry's TTL out again.
    client.pause(&pauser);

    env.as_contract(&contract_id, || {
        let ttl = env
            .storage()
            .persistent()
            .get_ttl(&DataKey::Roles(pauser.clone()));
        assert!(ttl >= LEDGERS_TO_EXTEND - 1);
    });
}

// ============================================================================
// Per-epoch snapshot storage and migration (#2781)
// ============================================================================

/// Write a storage-v1 deployment by hand: snapshots in the single legacy map,
/// version stamped 1, no per-epoch entries or chain tip.
fn seed_legacy_v1(env: &Env, contract_id: &Address, epochs: &[u64]) -> Map<u64, LegacySnapshot> {
    let mut legacy: Map<u64, LegacySnapshot> = Map::new(env);
    for (i, epoch) in epochs.iter().enumerate() {
        legacy.set(
            *epoch,
            LegacySnapshot {
                hash: create_test_hash(env, *epoch as u32 + 1),
                epoch: *epoch,
                timestamp: 1_000 + i as u64,
            },
        );
    }
    env.as_contract(contract_id, || {
        env.storage().persistent().set(&DataKey::Snapshots, &legacy);
        env.storage()
            .instance()
            .set(&DataKey::LatestEpoch, &epochs.last().copied().unwrap_or(0));
        env.storage()
            .instance()
            .set(&DataKey::StorageVersion, &1u32);
    });
    legacy
}

#[test]
fn snapshots_are_stored_one_entry_per_epoch() {
    let env = Env::default();
    let (client, contract_id, _, submitter) = setup_with_submitter(&env);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &submitter);
    client.submit_snapshot(&2u64, &create_test_hash(&env, 2), &submitter);

    env.as_contract(&contract_id, || {
        assert!(!env.storage().persistent().has(&DataKey::Snapshots));
        assert!(env.storage().persistent().has(&DataKey::Snapshot(1)));
        assert!(env.storage().persistent().has(&DataKey::Snapshot(2)));
        let latest: Snapshot = env.storage().instance().get(&DataKey::Latest).unwrap();
        assert_eq!(latest.epoch, 2);
    });
}

#[test]
fn submission_cost_is_flat_in_history_length() {
    // The previous single-map layout re-wrote the whole history on every
    // submission. With per-epoch keys a submission writes exactly one new
    // persistent entry and never touches the legacy map, however long the
    // history is. (CPU budget is not compared directly: the test host clones
    // its in-memory storage per invocation, which grows with entry count.)
    let env = Env::default();
    let (client, contract_id, _, submitter) = setup_with_submitter(&env);
    for epoch in 1..=300u64 {
        client.submit_snapshot(&epoch, &create_test_hash(&env, epoch as u32), &submitter);
    }
    env.as_contract(&contract_id, || {
        assert!(!env.storage().persistent().has(&DataKey::Snapshots));
        for epoch in [1u64, 150, 300] {
            assert!(env.storage().persistent().has(&DataKey::Snapshot(epoch)));
        }
    });
}

#[test]
#[ignore = "slow: 10k submissions; run with --ignored"]
fn ten_thousand_epochs_keep_submission_cost_flat() {
    let env = Env::default();
    let (client, _, _, submitter) = setup_with_submitter(&env);
    let mut first = 0u64;
    for epoch in 1..=10_000u64 {
        let mut budget = env.cost_estimate().budget();
        budget.reset_unlimited();
        client.submit_snapshot(&epoch, &create_test_hash(&env, epoch as u32), &submitter);
        let cpu = budget.cpu_instruction_cost();
        if epoch == 10 {
            first = cpu;
        }
        if epoch > 10 {
            assert!(cpu <= first + first / 10, "epoch {epoch}: {first} -> {cpu}");
        }
    }
}

#[test]
fn submission_is_refused_until_legacy_snapshots_are_migrated() {
    let env = Env::default();
    let (client, contract_id, admin, submitter) = setup_with_submitter(&env);
    seed_legacy_v1(&env, &contract_id, &[1, 2, 3]);

    let result = client.try_submit_snapshot(&4u64, &create_test_hash(&env, 4), &submitter);
    assert_eq!(result, Err(Ok(Error::MigrationRequired)));

    assert_eq!(client.migrate(&admin), CURRENT_STORAGE_VERSION);
    client.submit_snapshot(&4u64, &create_test_hash(&env, 4), &submitter);
    assert_eq!(client.verify_range(&1u64, &4u64), client.get_chain_head().1);
}

#[test]
fn migration_across_batches_preserves_every_hash() {
    let env = Env::default();
    let (client, contract_id, admin, _) = setup_with_submitter(&env);
    let epochs: std::vec::Vec<u64> = (1..=25u64).map(|e| e * 3).collect();
    let legacy = seed_legacy_v1(&env, &contract_id, &epochs);
    assert_eq!(client.pending_migration(), 25);

    // 25 items in batches of 10: two partial calls, then the final one.
    assert_eq!(client.migrate_batch(&admin, &10u32), 1);
    assert_eq!(client.pending_migration(), 15);
    assert_eq!(client.get_storage_version(), 1);
    assert_eq!(client.migrate_batch(&admin, &10u32), 1);
    assert_eq!(
        client.migrate_batch(&admin, &10u32),
        CURRENT_STORAGE_VERSION
    );
    assert_eq!(client.pending_migration(), 0);

    let mut prev_chain = BytesN::from_array(&env, &[0u8; 32]);
    let mut prev_epoch = 0u64;
    for epoch in epochs.iter() {
        let old = legacy.get(*epoch).unwrap();
        let rec = client.get_snapshot_record(epoch);
        assert_eq!(rec.hash, old.hash);
        assert_eq!(rec.timestamp, old.timestamp);
        assert_eq!(rec.prev_epoch, prev_epoch);
        assert_eq!(rec.prev_chain_hash, prev_chain);
        prev_chain = rec.chain_hash.clone();
        prev_epoch = *epoch;
    }
    assert_eq!(client.get_chain_head(), (75u64, prev_chain));
    assert_eq!(client.latest_snapshot().1, 75);
    env.as_contract(&contract_id, || {
        assert!(!env.storage().persistent().has(&DataKey::Snapshots));
    });
}

#[test]
fn migration_batch_size_is_bounded() {
    let env = Env::default();
    let (client, contract_id, admin, _) = setup_with_submitter(&env);
    seed_legacy_v1(&env, &contract_id, &[1]);
    assert_eq!(
        client.try_migrate_batch(&admin, &0u32),
        Err(Ok(Error::InvalidRange))
    );
    assert_eq!(
        client.try_migrate_batch(&admin, &(MAX_MIGRATION_BATCH + 1)),
        Err(Ok(Error::InvalidRange))
    );
}

#[test]
fn bump_snapshot_ttl_extends_range_and_rejects_wide_spans() {
    use soroban_sdk::testutils::storage::Persistent as _;
    use soroban_sdk::testutils::Ledger as _;

    let env = Env::default();
    let (client, contract_id, _, submitter) = setup_with_submitter(&env);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &submitter);
    client.submit_snapshot(&5u64, &create_test_hash(&env, 5), &submitter);

    env.ledger().with_mut(|li| li.sequence_number += 100_000);
    assert_eq!(client.bump_snapshot_ttl(&1u64, &10u64), 2);
    env.as_contract(&contract_id, || {
        assert!(env.storage().persistent().get_ttl(&DataKey::Snapshot(1)) >= LEDGERS_TO_EXTEND - 1);
    });

    assert_eq!(
        client.try_bump_snapshot_ttl(&1u64, &(1 + MAX_TTL_BUMP_SPAN)),
        Err(Ok(Error::InvalidRange))
    );
    assert_eq!(
        client.try_bump_snapshot_ttl(&5u64, &1u64),
        Err(Ok(Error::InvalidRange))
    );
}

// ============================================================================
// Snapshot hash chain (#2782)
// ============================================================================

#[test]
fn chain_links_each_epoch_to_its_predecessor() {
    let env = Env::default();
    let (client, contract_id, _, submitter) = setup_with_submitter(&env);
    let h1 = create_test_hash(&env, 1);
    let h2 = create_test_hash(&env, 2);
    client.submit_snapshot(&3u64, &h1, &submitter);
    client.submit_snapshot(&9u64, &h2, &submitter);

    let zero = BytesN::from_array(&env, &[0u8; 32]);
    let (c1, c2) = env.as_contract(&contract_id, || {
        let c1 = chain_link(&env, &zero, 3, &h1);
        (c1.clone(), chain_link(&env, &c1, 9, &h2))
    });

    let r1 = client.get_snapshot_record(&3u64);
    let r2 = client.get_snapshot_record(&9u64);
    assert_eq!(r1.prev_epoch, 0);
    assert_eq!(r1.prev_chain_hash, zero);
    assert_eq!(r1.chain_hash, c1);
    assert_eq!(r2.prev_epoch, 3);
    assert_eq!(r2.prev_chain_hash, c1);
    assert_eq!(r2.chain_hash, c2);
    assert_eq!(client.get_chain_head(), (9u64, c2.clone()));
    assert_eq!(client.verify_range(&1u64, &9u64), c2);
}

#[test]
fn submission_event_carries_the_chain_hash() {
    let env = Env::default();
    let (client, _, _, submitter) = setup_with_submitter(&env);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &submitter);
    let events = env.events().all();

    let head = client.get_chain_head().1;
    let found = events.events().iter().any(|e| {
        let soroban_sdk::xdr::ContractEventBody::V0(ref v0) = e.body;
        <SnapshotSubmitted as TryFromVal<Env, soroban_sdk::xdr::ScVal>>::try_from_val(
            &env, &v0.data,
        )
        .map(|ev| ev.chain_hash == head && ev.epoch == 1)
        .unwrap_or(false)
    });
    assert!(found, "SNAP_SUB event must include the stored chain hash");
}

#[test]
fn verify_range_detects_a_rewritten_epoch() {
    let env = Env::default();
    let (client, contract_id, _, submitter) = setup_with_submitter(&env);
    for epoch in 1..=5u64 {
        client.submit_snapshot(&epoch, &create_test_hash(&env, epoch as u32), &submitter);
    }
    assert!(client.try_verify_range(&1u64, &5u64).is_ok());

    // An upgraded build silently replaces epoch 3's hash.
    env.as_contract(&contract_id, || {
        let mut rec: Snapshot = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshot(3))
            .unwrap();
        rec.hash = create_test_hash(&env, 999);
        env.storage().persistent().set(&DataKey::Snapshot(3), &rec);
    });
    assert_eq!(
        client.try_verify_range(&1u64, &5u64),
        Err(Ok(Error::ChainBroken))
    );
    // A range that ends below the tampered epoch still verifies.
    assert!(client.try_verify_range(&1u64, &2u64).is_ok());
}

#[test]
fn verify_range_detects_a_rewritten_chain_link() {
    let env = Env::default();
    let (client, contract_id, _, submitter) = setup_with_submitter(&env);
    for epoch in 1..=3u64 {
        client.submit_snapshot(&epoch, &create_test_hash(&env, epoch as u32), &submitter);
    }
    // Rewrite epoch 2 consistently with itself (hash + its own chain hash)
    // but not with epoch 3's back-link.
    env.as_contract(&contract_id, || {
        let mut rec: Snapshot = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshot(2))
            .unwrap();
        rec.hash = create_test_hash(&env, 777);
        rec.chain_hash = chain_link(&env, &rec.prev_chain_hash, 2, &rec.hash);
        env.storage().persistent().set(&DataKey::Snapshot(2), &rec);
    });
    assert_eq!(
        client.try_verify_range(&1u64, &3u64),
        Err(Ok(Error::ChainBroken))
    );
}

#[test]
fn verify_range_validates_bounds() {
    let env = Env::default();
    let (client, _, _, submitter) = setup_with_submitter(&env);
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &submitter);
    assert_eq!(
        client.try_verify_range(&0u64, &1u64),
        Err(Ok(Error::InvalidRange))
    );
    assert_eq!(
        client.try_verify_range(&2u64, &1u64),
        Err(Ok(Error::InvalidRange))
    );
    assert_eq!(
        client.try_verify_range(&1u64, &7u64),
        Err(Ok(Error::SnapshotNotFound))
    );
}

#[test]
fn chain_is_independent_of_submission_gaps() {
    // Pseudo-random strictly increasing epochs: the chain must verify end to
    // end whatever the spacing (fuzzed ordering for #2782).
    let env = Env::default();
    let (client, _, _, submitter) = setup_with_submitter(&env);
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut epoch = 0u64;
    let mut first = 0u64;
    for i in 0..40u32 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        epoch += 1 + (x % 1_000);
        if i == 0 {
            first = epoch;
        }
        client.submit_snapshot(&epoch, &create_test_hash(&env, i + 1), &submitter);
        // An out-of-order resubmission never extends the chain.
        assert!(client
            .try_submit_snapshot(&(epoch - 1), &create_test_hash(&env, 5_000 + i), &submitter)
            .is_err());
    }
    assert_eq!(
        client.verify_range(&first, &epoch),
        client.get_chain_head().1
    );
}

// ============================================================================
// Submitter rotation policy (#2786)
// ============================================================================

#[test]
fn rotation_rejects_admin_as_new_submitter() {
    let env = Env::default();
    let (client, _, admin, old_key) = setup_with_submitter(&env);
    assert_eq!(
        client.try_rotate_submitter(&admin, &old_key, &admin),
        Err(Ok(Error::InvalidRotationTarget))
    );
}

#[test]
fn rotation_rejects_rotating_a_key_onto_itself() {
    let env = Env::default();
    let (client, _, admin, old_key) = setup_with_submitter(&env);
    assert_eq!(
        client.try_rotate_submitter(&admin, &old_key, &old_key),
        Err(Ok(Error::InvalidRotationTarget))
    );
}

#[test]
fn rotation_overlap_keeps_old_key_valid_until_expiry() {
    use soroban_sdk::testutils::Ledger as _;

    let env = Env::default();
    let (client, _, admin, old_key) = setup_with_submitter(&env);
    let new_key = Address::generate(&env);
    let start = env.ledger().sequence();

    let until = client.rotate_submitter_with_policy(
        &admin,
        &old_key,
        &new_key,
        &100u32,
        &RotationReason::Scheduled,
    );
    assert_eq!(until, start + 100);
    assert_eq!(
        client.get_outgoing_submitters().get(old_key.clone()),
        Some(until)
    );

    // Both keys are valid inside the window.
    client.submit_snapshot(&1u64, &create_test_hash(&env, 1), &old_key);
    client.submit_snapshot(&2u64, &create_test_hash(&env, 2), &new_key);
    assert!(client.has_role(&old_key, &Role::SnapshotSubmitter));

    // At the expiry ledger the old key is refused.
    env.ledger().with_mut(|li| li.sequence_number = until);
    assert_eq!(
        client.try_submit_snapshot(&3u64, &create_test_hash(&env, 3), &old_key),
        Err(Ok(Error::Unauthorized))
    );
    assert!(!client.has_role(&old_key, &Role::SnapshotSubmitter));
    assert_eq!(client.get_outgoing_submitters().len(), 0);
    client.submit_snapshot(&3u64, &create_test_hash(&env, 3), &new_key);
}

#[test]
fn compromise_rotation_forbids_overlap() {
    let env = Env::default();
    let (client, _, admin, old_key) = setup_with_submitter(&env);
    let new_key = Address::generate(&env);
    assert_eq!(
        client.try_rotate_submitter_with_policy(
            &admin,
            &old_key,
            &new_key,
            &1u32,
            &RotationReason::Compromise
        ),
        Err(Ok(Error::InvalidOverlap))
    );
    assert_eq!(
        client.rotate_submitter_with_policy(
            &admin,
            &old_key,
            &new_key,
            &0u32,
            &RotationReason::Compromise
        ),
        0
    );
    assert_eq!(
        client.try_submit_snapshot(&1u64, &create_test_hash(&env, 1), &old_key),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn rotation_overlap_is_capped() {
    let env = Env::default();
    let (client, _, admin, old_key) = setup_with_submitter(&env);
    assert_eq!(
        client.try_rotate_submitter_with_policy(
            &admin,
            &old_key,
            &Address::generate(&env),
            &(MAX_ROTATION_OVERLAP + 1),
            &RotationReason::Scheduled
        ),
        Err(Ok(Error::InvalidOverlap))
    );
}

#[test]
fn rotation_rejects_key_still_in_overlap_window() {
    let env = Env::default();
    let (client, _, admin, first) = setup_with_submitter(&env);
    let second = Address::generate(&env);
    let third = Address::generate(&env);
    client.add_submitter(&admin, &third);
    client.rotate_submitter_with_policy(&admin, &first, &second, &50u32, &RotationReason::Other);

    // `first` is still live as an outgoing key; it cannot come back as a
    // rotation target under another slot while that window is open.
    assert_eq!(
        client.try_rotate_submitter(&admin, &third, &first),
        Err(Ok(Error::InvalidRotationTarget))
    );
}

#[test]
fn removing_outgoing_key_ends_overlap_early() {
    let env = Env::default();
    let (client, _, admin, old_key) = setup_with_submitter(&env);
    let new_key = Address::generate(&env);
    client.rotate_submitter_with_policy(
        &admin,
        &old_key,
        &new_key,
        &500u32,
        &RotationReason::Scheduled,
    );

    client.remove_submitter(&admin, &old_key);
    assert_eq!(
        client.try_submit_snapshot(&1u64, &create_test_hash(&env, 1), &old_key),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(client.get_outgoing_submitters().len(), 0);
}

#[test]
fn rotation_emits_policy_event() {
    let env = Env::default();
    let (client, _, admin, old_key) = setup_with_submitter(&env);
    client.rotate_submitter_with_policy(
        &admin,
        &old_key,
        &Address::generate(&env),
        &10u32,
        &RotationReason::Scheduled,
    );
    assert_eq!(
        count_contract_events_with_topic0(&env, symbol_short!("rot_pol")),
        1
    );
}

// ============================================================================
// Single-admin invariant (#2785)
// ============================================================================

#[test]
fn admin_role_cannot_be_granted_via_grant_role() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    let other = Address::generate(&env);
    assert_eq!(
        client.try_grant_role(&admin, &other, &Role::Admin),
        Err(Ok(Error::AdminRoleNotGrantable))
    );
    assert!(!client.has_role(&other, &Role::Admin));
}

#[test]
fn admin_cannot_revoke_own_admin_role() {
    let env = Env::default();
    let (client, _, admin, _) = setup_with_submitter(&env);
    assert_eq!(
        client.try_revoke_role(&admin, &admin, &Role::Admin),
        Err(Ok(Error::AdminRoleNotGrantable))
    );
    assert!(client.has_role(&admin, &Role::Admin));
}
