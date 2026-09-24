#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

use crate::{EscrowServiceContract, EscrowServiceContractClient, EscrowState};

fn advance_time(env: &Env, seconds: u64) {
    env.ledger().with_mut(|li| {
        li.timestamp += seconds;
    });
}

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    assert_eq!(client.get_admin(), admin);
    assert!(!client.is_paused());
    assert_eq!(client.get_escrow_count(), 0);
}

#[test]
#[should_panic]
fn test_double_initialize_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);

    client.initialize(&admin);
    client.initialize(&admin);
}

#[test]
fn test_create_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();

    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let deadline = env.ledger().timestamp() + 3600;
    let escrow_id = client.create_escrow(&depositor, &beneficiary, &token_addr, &1_000_000, &deadline);
    assert_eq!(escrow_id, 1);
    assert_eq!(client.get_escrow_count(), 1);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.depositor, depositor);
    assert_eq!(escrow.beneficiary, beneficiary);
    assert_eq!(escrow.amount, 1_000_000);
    assert_eq!(escrow.state, EscrowState::Created);
}

#[test]
fn test_fund_and_release() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);
    asset_client.mint(&depositor, &10_000_000);

    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let deadline = env.ledger().timestamp() + 3600;
    let amount: i128 = 1_000_000;

    let escrow_id = client.create_escrow(&depositor, &beneficiary, &token_addr, &amount, &deadline);
    client.fund_escrow(&depositor, &escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.state, EscrowState::Funded);

    client.release_funds(&depositor, &escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.state, EscrowState::Released);

    let token_client = token::Client::new(&env, &token_addr);
    assert_eq!(token_client.balance(&beneficiary), amount);
}

#[test]
fn test_refund_after_deadline() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);
    asset_client.mint(&depositor, &10_000_000);

    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let initial_balance = token::Client::new(&env, &token_addr).balance(&depositor);
    let deadline = env.ledger().timestamp() + 3600;
    let amount: i128 = 1_000_000;

    let escrow_id = client.create_escrow(&depositor, &beneficiary, &token_addr, &amount, &deadline);
    client.fund_escrow(&depositor, &escrow_id);

    advance_time(&env, 7200);
    client.refund(&depositor, &escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.state, EscrowState::Refunded);

    let token_client = token::Client::new(&env, &token_addr);
    assert_eq!(token_client.balance(&depositor), initial_balance);
}

#[test]
#[should_panic]
fn test_refund_before_deadline_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);
    asset_client.mint(&depositor, &10_000_000);

    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let deadline = env.ledger().timestamp() + 3600;
    let escrow_id = client.create_escrow(&depositor, &beneficiary, &token_addr, &1_000_000, &deadline);
    client.fund_escrow(&depositor, &escrow_id);

    // Deadline not passed — should panic
    client.refund(&depositor, &escrow_id);
}

#[test]
fn test_dispute_and_resolve_to_beneficiary() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);
    asset_client.mint(&depositor, &10_000_000);

    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let amount: i128 = 1_000_000;
    let deadline = env.ledger().timestamp() + 3600;

    let escrow_id = client.create_escrow(&depositor, &beneficiary, &token_addr, &amount, &deadline);
    client.fund_escrow(&depositor, &escrow_id);
    client.raise_dispute(&depositor, &escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.state, EscrowState::Disputed);

    client.resolve_dispute(&admin, &escrow_id, &true);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.state, EscrowState::Released);

    let token_client = token::Client::new(&env, &token_addr);
    assert_eq!(token_client.balance(&beneficiary), amount);
}

#[test]
fn test_dispute_resolve_to_depositor() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);
    asset_client.mint(&depositor, &10_000_000);

    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let initial_balance = token::Client::new(&env, &token_addr).balance(&depositor);
    let amount: i128 = 1_000_000;
    let deadline = env.ledger().timestamp() + 3600;

    let escrow_id = client.create_escrow(&depositor, &beneficiary, &token_addr, &amount, &deadline);
    client.fund_escrow(&depositor, &escrow_id);
    client.raise_dispute(&beneficiary, &escrow_id);
    client.resolve_dispute(&admin, &escrow_id, &false);

    let token_client = token::Client::new(&env, &token_addr);
    assert_eq!(token_client.balance(&depositor), initial_balance);
}

#[test]
fn test_cancel_unfunded_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();

    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let deadline = env.ledger().timestamp() + 3600;
    let escrow_id = client.create_escrow(&depositor, &beneficiary, &token_addr, &1_000_000, &deadline);
    client.cancel_escrow(&depositor, &escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.state, EscrowState::Cancelled);
}

#[test]
fn test_pause_prevents_create() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();

    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.pause(&admin);
    assert!(client.is_paused());

    let deadline = env.ledger().timestamp() + 3600;
    let result = client.try_create_escrow(&depositor, &beneficiary, &token_addr, &1_000_000, &deadline);
    assert!(result.is_err());

    client.unpause(&admin);
    assert!(!client.is_paused());
}

#[test]
#[should_panic]
fn test_invalid_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();

    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let deadline = env.ledger().timestamp() + 3600;
    client.create_escrow(&depositor, &beneficiary, &token_addr, &0, &deadline);
}

#[test]
#[should_panic]
fn test_non_depositor_cannot_release() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);
    asset_client.mint(&depositor, &10_000_000);

    let contract_id = env.register(EscrowServiceContract, ());
    let client = EscrowServiceContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let deadline = env.ledger().timestamp() + 3600;
    let escrow_id = client.create_escrow(&depositor, &beneficiary, &token_addr, &1_000_000, &deadline);
    client.fund_escrow(&depositor, &escrow_id);

    // beneficiary tries to release — should panic
    client.release_funds(&beneficiary, &escrow_id);
}
