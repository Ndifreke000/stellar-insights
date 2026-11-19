#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

use crate::{OfferState, TokenSwapContract, TokenSwapContractClient};

fn setup(env: &Env) -> (TokenSwapContractClient, Address, Address) {
    let admin = Address::generate(env);
    let contract_id = env.register(TokenSwapContract, ());
    let client = TokenSwapContractClient::new(env, &contract_id);
    client.initialize(&admin);
    (client, contract_id, admin)
}

fn advance_time(env: &Env, seconds: u64) {
    env.ledger().with_mut(|li| {
        li.timestamp += seconds;
    });
}

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin) = setup(&env);

    assert_eq!(client.get_admin(), admin);
    assert!(!client.is_paused());
    assert_eq!(client.get_offer_count(), 0);
}

#[test]
#[should_panic]
fn test_double_initialize_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin) = setup(&env);
    client.initialize(&admin);
}

#[test]
fn test_create_offer() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin) = setup(&env);
    let maker = Address::generate(&env);
    let token_a_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_b_admin = Address::generate(&env);
    let token_b_addr = env.register_stellar_asset_contract_v2(token_b_admin.clone()).address();

    let asset_a = token::StellarAssetClient::new(&env, &token_a_addr);
    asset_a.mint(&maker, &10_000_000);

    let offer_amount: i128 = 2_000_000;
    let want_amount: i128 = 3_000_000;
    let offer_id = client.create_offer(
        &maker,
        &token_a_addr,
        &offer_amount,
        &token_b_addr,
        &want_amount,
        &0u64, // no expiry
    );

    assert_eq!(offer_id, 1);
    assert_eq!(client.get_offer_count(), 1);

    let offer = client.get_offer(&offer_id);
    assert_eq!(offer.maker, maker);
    assert_eq!(offer.offer_amount, offer_amount);
    assert_eq!(offer.want_amount, want_amount);
    assert_eq!(offer.state, OfferState::Open);

    // maker's tokens are now held by the contract
    let token_a_client = token::Client::new(&env, &token_a_addr);
    assert_eq!(token_a_client.balance(&maker), 8_000_000);
}

#[test]
fn test_accept_offer_swaps_tokens() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, admin) = setup(&env);
    let maker = Address::generate(&env);
    let taker = Address::generate(&env);

    let token_a_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_b_admin = Address::generate(&env);
    let token_b_addr = env.register_stellar_asset_contract_v2(token_b_admin.clone()).address();

    let asset_a = token::StellarAssetClient::new(&env, &token_a_addr);
    let asset_b = token::StellarAssetClient::new(&env, &token_b_addr);

    let offer_amount: i128 = 2_000_000;
    let want_amount: i128 = 3_000_000;

    asset_a.mint(&maker, &offer_amount);
    asset_b.mint(&taker, &want_amount);

    let offer_id = client.create_offer(
        &maker,
        &token_a_addr,
        &offer_amount,
        &token_b_addr,
        &want_amount,
        &0u64,
    );

    let _ = contract_id; // contract_id holds offer_amount of token_a at this point
    client.accept_offer(&taker, &offer_id, &0i128);

    let offer = client.get_offer(&offer_id);
    assert_eq!(offer.state, OfferState::Filled);

    // Taker received offer_amount of token_a
    let token_a_client = token::Client::new(&env, &token_a_addr);
    assert_eq!(token_a_client.balance(&taker), offer_amount);

    // Maker received want_amount of token_b
    let token_b_client = token::Client::new(&env, &token_b_addr);
    assert_eq!(token_b_client.balance(&maker), want_amount);
}

#[test]
fn test_cancel_offer_returns_tokens() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin) = setup(&env);
    let maker = Address::generate(&env);
    let token_b_admin = Address::generate(&env);

    let token_a_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_b_addr = env.register_stellar_asset_contract_v2(token_b_admin.clone()).address();

    let asset_a = token::StellarAssetClient::new(&env, &token_a_addr);
    let offer_amount: i128 = 2_000_000;
    asset_a.mint(&maker, &offer_amount);

    let offer_id = client.create_offer(
        &maker,
        &token_a_addr,
        &offer_amount,
        &token_b_addr,
        &1_000_000i128,
        &0u64,
    );

    // Before cancelling, maker has 0 token_a (deposited into contract)
    let token_a_client = token::Client::new(&env, &token_a_addr);
    assert_eq!(token_a_client.balance(&maker), 0);

    client.cancel_offer(&maker, &offer_id);

    assert_eq!(client.get_offer(&offer_id).state, OfferState::Cancelled);
    // Tokens returned to maker
    assert_eq!(token_a_client.balance(&maker), offer_amount);
}

#[test]
fn test_admin_can_cancel_offer() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin) = setup(&env);
    let maker = Address::generate(&env);
    let token_b_admin = Address::generate(&env);

    let token_a_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_b_addr = env.register_stellar_asset_contract_v2(token_b_admin.clone()).address();

    let asset_a = token::StellarAssetClient::new(&env, &token_a_addr);
    asset_a.mint(&maker, &2_000_000);

    let offer_id = client.create_offer(
        &maker,
        &token_a_addr,
        &2_000_000i128,
        &token_b_addr,
        &1_000_000i128,
        &0u64,
    );

    client.cancel_offer(&admin, &offer_id);
    assert_eq!(client.get_offer(&offer_id).state, OfferState::Cancelled);
}

#[test]
#[should_panic]
fn test_accept_filled_offer_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin) = setup(&env);
    let maker = Address::generate(&env);
    let taker1 = Address::generate(&env);
    let taker2 = Address::generate(&env);
    let token_b_admin = Address::generate(&env);

    let token_a_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_b_addr = env.register_stellar_asset_contract_v2(token_b_admin.clone()).address();

    let asset_a = token::StellarAssetClient::new(&env, &token_a_addr);
    let asset_b = token::StellarAssetClient::new(&env, &token_b_addr);

    asset_a.mint(&maker, &2_000_000);
    asset_b.mint(&taker1, &3_000_000);
    asset_b.mint(&taker2, &3_000_000);

    let offer_id = client.create_offer(
        &maker,
        &token_a_addr,
        &2_000_000i128,
        &token_b_addr,
        &3_000_000i128,
        &0u64,
    );

    client.accept_offer(&taker1, &offer_id, &0i128);
    client.accept_offer(&taker2, &offer_id, &0i128); // already filled — should panic
}

#[test]
fn test_offer_expires_and_can_be_cancelled() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin) = setup(&env);
    let maker = Address::generate(&env);
    let outsider = Address::generate(&env);
    let token_b_admin = Address::generate(&env);

    let token_a_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_b_addr = env.register_stellar_asset_contract_v2(token_b_admin.clone()).address();

    let asset_a = token::StellarAssetClient::new(&env, &token_a_addr);
    asset_a.mint(&maker, &2_000_000);

    let expiry = env.ledger().timestamp() + 100;
    let offer_id = client.create_offer(
        &maker,
        &token_a_addr,
        &2_000_000i128,
        &token_b_addr,
        &1_000_000i128,
        &expiry,
    );

    // Advance past expiry — now anyone can cancel
    advance_time(&env, 200);
    client.cancel_offer(&outsider, &offer_id);

    assert_eq!(client.get_offer(&offer_id).state, OfferState::Cancelled);
    // Tokens returned to maker even though outsider triggered the cancel
    let token_a_client = token::Client::new(&env, &token_a_addr);
    assert_eq!(token_a_client.balance(&maker), 2_000_000);
}

#[test]
#[should_panic]
fn test_accept_expired_offer_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin) = setup(&env);
    let maker = Address::generate(&env);
    let taker = Address::generate(&env);
    let token_b_admin = Address::generate(&env);

    let token_a_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_b_addr = env.register_stellar_asset_contract_v2(token_b_admin.clone()).address();

    let asset_a = token::StellarAssetClient::new(&env, &token_a_addr);
    let asset_b = token::StellarAssetClient::new(&env, &token_b_addr);

    asset_a.mint(&maker, &2_000_000);
    asset_b.mint(&taker, &1_000_000);

    let expiry = env.ledger().timestamp() + 100;
    let offer_id = client.create_offer(
        &maker,
        &token_a_addr,
        &2_000_000i128,
        &token_b_addr,
        &1_000_000i128,
        &expiry,
    );

    advance_time(&env, 200); // past expiry
    client.accept_offer(&taker, &offer_id, &0i128); // should panic
}

#[test]
#[should_panic]
fn test_same_token_swap_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin) = setup(&env);
    let maker = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset = token::StellarAssetClient::new(&env, &token_addr);
    asset.mint(&maker, &2_000_000);

    // Offering and wanting same token — should panic
    client.create_offer(
        &maker,
        &token_addr,
        &2_000_000i128,
        &token_addr, // same token
        &1_000_000i128,
        &0u64,
    );
}

#[test]
fn test_pause_prevents_offer_creation() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin) = setup(&env);
    let maker = Address::generate(&env);
    let token_b_admin = Address::generate(&env);

    let token_a_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_b_addr = env.register_stellar_asset_contract_v2(token_b_admin.clone()).address();

    client.pause(&admin);
    assert!(client.is_paused());

    let result = client.try_create_offer(
        &maker,
        &token_a_addr,
        &1_000_000i128,
        &token_b_addr,
        &1_000_000i128,
        &0u64,
    );
    assert!(result.is_err());

    client.unpause(&admin);
    assert!(!client.is_paused());
}
