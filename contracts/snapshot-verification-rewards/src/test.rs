#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

fn make_hash(env: &Env, seed: u8) -> BytesN<32> {
    let mut bytes = [seed; 32];
    bytes[0] = seed.wrapping_add(1);
    BytesN::from_array(env, &bytes)
}

fn setup() -> (Env, SnapshotVerificationRewardsContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, SnapshotVerificationRewardsContract);
    let client = SnapshotVerificationRewardsContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &0u64);
    (env, client, admin)
}

#[test]
fn test_initialize() {
    let (env, client, admin) = setup();
    assert_eq!(client.get_admin(), admin);
    assert!(!client.is_paused());
    assert_eq!(client.get_reward_per_verification(), 100u64);
    let _ = env;
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_double_initialize() {
    let (env, client, admin) = setup();
    client.initialize(&admin, &0u64);
    let _ = env;
}

#[test]
fn test_register_and_verify_matching_hash() {
    let (env, client, admin) = setup();
    let epoch: u64 = 1;
    let hash = make_hash(&env, 42);

    client.register_snapshot(&admin, &epoch, &hash);

    let verifier = Address::generate(&env);
    let points = client.verify_snapshot(&verifier, &epoch, &hash);
    assert_eq!(points, 100u64);
    assert_eq!(client.get_reward_points(&verifier), 100u64);
}

#[test]
fn test_verify_wrong_hash_earns_zero_points() {
    let (env, client, admin) = setup();
    let epoch: u64 = 1;
    let correct_hash = make_hash(&env, 42);
    let wrong_hash = make_hash(&env, 99);

    client.register_snapshot(&admin, &epoch, &correct_hash);

    let verifier = Address::generate(&env);
    let points = client.verify_snapshot(&verifier, &epoch, &wrong_hash);
    assert_eq!(points, 0u64);
    assert_eq!(client.get_reward_points(&verifier), 0u64);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_double_verify_rejected() {
    let (env, client, admin) = setup();
    let epoch: u64 = 1;
    let hash = make_hash(&env, 1);

    client.register_snapshot(&admin, &epoch, &hash);

    let verifier = Address::generate(&env);
    client.verify_snapshot(&verifier, &epoch, &hash);
    client.verify_snapshot(&verifier, &epoch, &hash); // should panic
}

#[test]
fn test_claim_reward() {
    let (env, client, admin) = setup();
    let epoch: u64 = 1;
    let hash = make_hash(&env, 5);
    client.register_snapshot(&admin, &epoch, &hash);

    let verifier = Address::generate(&env);
    client.verify_snapshot(&verifier, &epoch, &hash);
    assert_eq!(client.get_reward_points(&verifier), 100u64);

    let claimed = client.claim_reward(&verifier);
    assert_eq!(claimed, 100u64);
    assert_eq!(client.get_reward_points(&verifier), 0u64);
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_claim_with_no_rewards() {
    let (env, client, _admin) = setup();
    let verifier = Address::generate(&env);
    client.claim_reward(&verifier);
}

#[test]
fn test_deactivate_epoch() {
    let (env, client, admin) = setup();
    let epoch: u64 = 1;
    let hash = make_hash(&env, 7);
    client.register_snapshot(&admin, &epoch, &hash);
    client.deactivate_epoch(&admin, &epoch);

    let record = client.get_registered_snapshot(&epoch);
    assert!(!record.active);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_verify_inactive_epoch() {
    let (env, client, admin) = setup();
    let epoch: u64 = 1;
    let hash = make_hash(&env, 7);
    client.register_snapshot(&admin, &epoch, &hash);
    client.deactivate_epoch(&admin, &epoch);

    let verifier = Address::generate(&env);
    client.verify_snapshot(&verifier, &epoch, &hash);
}

#[test]
fn test_has_verified() {
    let (env, client, admin) = setup();
    let epoch: u64 = 1;
    let hash = make_hash(&env, 3);
    client.register_snapshot(&admin, &epoch, &hash);

    let verifier = Address::generate(&env);
    assert!(!client.has_verified(&verifier, &epoch));
    client.verify_snapshot(&verifier, &epoch, &hash);
    assert!(client.has_verified(&verifier, &epoch));
}

#[test]
fn test_pause_prevents_verify() {
    let (env, client, admin) = setup();
    let epoch: u64 = 1;
    let hash = make_hash(&env, 2);
    client.register_snapshot(&admin, &epoch, &hash);

    client.pause(&admin);
    assert!(client.is_paused());

    let verifier = Address::generate(&env);
    // Should fail with ContractPaused
    let result = client.try_verify_snapshot(&verifier, &epoch, &hash);
    assert!(result.is_err());

    client.unpause(&admin);
    assert!(!client.is_paused());
    let points = client.verify_snapshot(&verifier, &epoch, &hash);
    assert_eq!(points, 100u64);
}

#[test]
fn test_set_reward_per_verification() {
    let (env, client, admin) = setup();
    client.set_reward_per_verification(&admin, &250u64);
    assert_eq!(client.get_reward_per_verification(), 250u64);

    let epoch: u64 = 1;
    let hash = make_hash(&env, 9);
    client.register_snapshot(&admin, &epoch, &hash);

    let verifier = Address::generate(&env);
    let points = client.verify_snapshot(&verifier, &epoch, &hash);
    assert_eq!(points, 250u64);
}

#[test]
fn test_multiple_verifiers_same_epoch() {
    let (env, client, admin) = setup();
    let epoch: u64 = 1;
    let hash = make_hash(&env, 11);
    client.register_snapshot(&admin, &epoch, &hash);

    let v1 = Address::generate(&env);
    let v2 = Address::generate(&env);
    let p1 = client.verify_snapshot(&v1, &epoch, &hash);
    let p2 = client.verify_snapshot(&v2, &epoch, &hash);
    assert_eq!(p1, 100u64);
    assert_eq!(p2, 100u64);
}
