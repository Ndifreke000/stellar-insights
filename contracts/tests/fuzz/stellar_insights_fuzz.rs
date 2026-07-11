//! Fuzz-style property tests for the stellar_insights (analytics snapshot) contract.
//!
//! Covers: epoch IDs (zero, monotonicity, duplicates), hash inputs (all-zero),
//! and caller authorization.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};
use stellar_insights::{StellarInsightsContract, StellarInsightsContractClient};

fn setup(env: &Env) -> (StellarInsightsContractClient, Address) {
    let id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin)
}

fn hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn nonzero_hash(env: &Env, seed: u8) -> BytesN<32> {
    let s = if seed == 0 { 1 } else { seed };
    BytesN::from_array(env, &[s; 32])
}

// ── 1. Epoch zero must always be rejected ─────────────────────────────────────

#[test]
fn fuzz_epoch_zero_always_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let result = client.try_submit_snapshot(&0u64, &nonzero_hash(&env, 1), &admin);
    assert!(
        matches!(result, Err(Ok(_))),
        "epoch=0 must be rejected by the contract"
    );
}

// ── 2. Epochs must strictly increase (monotonicity) ──────────────────────────

#[test]
fn fuzz_epoch_monotonicity_enforced() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    // Submit epoch 5 successfully
    client.submit_snapshot(&5u64, &nonzero_hash(&env, 1), &admin);
    assert_eq!(client.get_latest_epoch(), 5);

    // Same epoch must be rejected (duplicate)
    let dup = client.try_submit_snapshot(&5u64, &nonzero_hash(&env, 2), &admin);
    assert!(matches!(dup, Err(Ok(_))), "duplicate epoch=5 must be rejected");

    // Earlier epoch must also be rejected
    for earlier in [1u64, 3, 4] {
        let result = client.try_submit_snapshot(&earlier, &nonzero_hash(&env, 3), &admin);
        assert!(
            matches!(result, Err(Ok(_))),
            "epoch={earlier} < latest(5) must be rejected"
        );
    }

    // Next valid epoch must succeed
    client.submit_snapshot(&6u64, &nonzero_hash(&env, 4), &admin);
    assert_eq!(client.get_latest_epoch(), 6);
}

// ── 3. Sequential epoch submissions increase the counter monotonically ────────

#[test]
fn fuzz_sequential_epochs_monotonic() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    for epoch in 1u64..=20 {
        client.submit_snapshot(&epoch, &nonzero_hash(&env, epoch as u8), &admin);
        assert_eq!(
            client.get_latest_epoch(),
            epoch,
            "latest_epoch must equal the last submitted epoch"
        );
    }
}

// ── 4. All-zero hash must be rejected ────────────────────────────────────────

#[test]
fn fuzz_zero_hash_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let zero_hash = hash(&env, 0);
    let result = client.try_submit_snapshot(&1u64, &zero_hash, &admin);
    assert!(
        matches!(result, Err(Ok(_))),
        "all-zero hash must be rejected"
    );
}

// ── 5. Non-admin caller must be rejected ─────────────────────────────────────

#[test]
fn fuzz_non_admin_always_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    for _ in 0..8 {
        let non_admin = Address::generate(&env);
        let result = client.try_submit_snapshot(&1u64, &nonzero_hash(&env, 1), &non_admin);
        assert!(
            matches!(result, Err(Ok(_))),
            "non-admin caller must always be rejected"
        );
    }
}

// ── 6. get_snapshot on missing epoch must return an error ─────────────────────

#[test]
fn fuzz_missing_epoch_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    for epoch in [0u64, 1, 100, u64::MAX / 2, u64::MAX] {
        let result = client.try_get_snapshot(&epoch);
        assert!(
            matches!(result, Err(Ok(_))),
            "epoch={epoch} not submitted — get_snapshot must return an error"
        );
    }
}

// ── 7. Paused contract rejects submissions ────────────────────────────────────

#[test]
fn fuzz_paused_contract_rejects_submissions() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    client.pause(&admin);
    assert!(client.is_paused());

    let result = client.try_submit_snapshot(&1u64, &nonzero_hash(&env, 1), &admin);
    assert!(
        matches!(result, Err(Ok(_))),
        "paused contract must reject submit_snapshot"
    );

    client.unpause(&admin);
    assert!(!client.is_paused());

    // After unpause the same submission must succeed
    client.submit_snapshot(&1u64, &nonzero_hash(&env, 1), &admin);
}

// ── 8. latest_snapshot reflects the last submission ──────────────────────────

#[test]
fn fuzz_latest_snapshot_tracks_last_submission() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    for epoch in 1u64..=10 {
        let h = nonzero_hash(&env, epoch as u8);
        client.submit_snapshot(&epoch, &h, &admin);
        let (stored_hash, stored_epoch, _ts) = client.latest_snapshot();
        assert_eq!(stored_epoch, epoch, "latest_snapshot must report epoch={epoch}");
        assert_eq!(stored_hash, h, "latest_snapshot must return the correct hash");
    }
}
