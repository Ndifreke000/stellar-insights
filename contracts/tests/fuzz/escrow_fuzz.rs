//! Fuzz-style property tests for the escrow contract.
//!
//! Covers: token amounts (zero, negative, large), escrow ID counter
//! monotonicity, non-existent IDs, and invalid state transitions.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use escrow::{EscrowServiceContract, EscrowServiceContractClient, EscrowState};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup(env: &Env) -> (EscrowServiceContractClient, Address) {
    let id = env.register_contract(None, EscrowServiceContract);
    let client = EscrowServiceContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin)
}

fn make_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract_v2(admin.clone()).address()
}

// ── 1. Amount ≤ 0 must always be rejected ─────────────────────────────────────

#[test]
fn fuzz_invalid_amounts_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token = make_token(&env, &admin);
    let deadline = 9999u64;

    for amount in [0i128, -1, -100, i128::MIN] {
        let result = client.try_create_escrow(&depositor, &beneficiary, &token, &amount, &deadline);
        assert!(
            matches!(result, Ok(Err(_))),
            "amount={amount} must be rejected (InvalidAmount)"
        );
    }
}

// ── 2. Escrow ID counter increments monotonically ─────────────────────────────

#[test]
fn fuzz_escrow_id_monotonically_increases() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let token = make_token(&env, &admin);
    let deadline = 9999u64;

    for expected_id in 1u64..=15 {
        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let id = client.create_escrow(&depositor, &beneficiary, &token, &100i128, &deadline);
        assert_eq!(id, expected_id, "escrow IDs must be sequential starting from 1");
        assert_eq!(
            client.get_escrow_count(),
            expected_id,
            "escrow count must match created ID"
        );
    }
}

// ── 3. Non-existent escrow ID must return an error ────────────────────────────

#[test]
fn fuzz_nonexistent_escrow_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    for id in [0u64, 1, 999, u64::MAX / 2, u64::MAX] {
        let result = client.try_get_escrow(&id);
        assert!(
            matches!(result, Ok(Err(_))),
            "escrow id={id} does not exist — must return EscrowNotFound"
        );
    }
}

// ── 4. Funding an already-funded escrow must be rejected ──────────────────────

#[test]
fn fuzz_double_fund_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token = make_token(&env, &admin);

    // Mint tokens to depositor so the transfer doesn't fail for wrong reasons
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&depositor, &1_000_000i128);

    let escrow_id = client.create_escrow(&depositor, &beneficiary, &token, &100i128, &9999u64);
    client.fund_escrow(&depositor, &escrow_id);

    let double_fund = client.try_fund_escrow(&depositor, &escrow_id);
    assert!(
        matches!(double_fund, Ok(Err(_))),
        "funding an already-funded escrow must be rejected"
    );
}

// ── 5. Escrow state after creation is Created ─────────────────────────────────

#[test]
fn fuzz_initial_escrow_state_is_created() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let token = make_token(&env, &admin);

    for _ in 0..5 {
        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let id = client.create_escrow(&depositor, &beneficiary, &token, &50i128, &9999u64);
        let escrow = client.get_escrow(&id);
        assert_eq!(
            escrow.state,
            EscrowState::Created,
            "newly created escrow must be in Created state"
        );
        assert_eq!(escrow.amount, 50i128);
    }
}

// ── 6. Release before funding must be rejected ────────────────────────────────

#[test]
fn fuzz_release_before_fund_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token = make_token(&env, &admin);

    let id = client.create_escrow(&depositor, &beneficiary, &token, &100i128, &9999u64);

    // Attempt release without funding
    let result = client.try_release_funds(&admin, &id);
    assert!(
        matches!(result, Ok(Err(_))),
        "releasing funds from an unfunded escrow must be rejected"
    );
}

// ── 7. Paused contract rejects new escrows ────────────────────────────────────

#[test]
fn fuzz_paused_contract_rejects_create_escrow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let token = make_token(&env, &admin);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    client.pause(&admin);
    assert!(client.is_paused());

    let result = client.try_create_escrow(&depositor, &beneficiary, &token, &100i128, &9999u64);
    assert!(
        matches!(result, Ok(Err(_))),
        "paused contract must reject create_escrow"
    );
}

// ── 8. Large but valid amounts create escrow correctly ────────────────────────

#[test]
fn fuzz_large_valid_amounts_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let token = make_token(&env, &admin);

    for amount in [1i128, 1_000, 1_000_000, i128::MAX / 2] {
        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let id = client.create_escrow(&depositor, &beneficiary, &token, &amount, &9999u64);
        let escrow = client.get_escrow(&id);
        assert_eq!(escrow.amount, amount, "stored amount must match input");
    }
}
