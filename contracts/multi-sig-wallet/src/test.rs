#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec,
};

use crate::{MultiSigWalletContract, MultiSigWalletContractClient, TxState};

fn setup_wallet<'a>(
    env: &'a Env,
    owners: &[Address],
    threshold: u32,
) -> (MultiSigWalletContractClient<'a>, Address) {
    let contract_id = env.register(MultiSigWalletContract, ());
    let client = MultiSigWalletContractClient::new(env, &contract_id);
    let mut owner_vec = Vec::new(env);
    for o in owners {
        owner_vec.push_back(o.clone());
    }
    client.initialize(&owner_vec, &threshold);
    (client, contract_id)
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

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let owner3 = Address::generate(&env);

    let (client, _) = setup_wallet(&env, &[owner1.clone(), owner2.clone(), owner3.clone()], 2);

    assert_eq!(client.get_threshold(), 2);
    assert_eq!(client.get_tx_count(), 0);
    let owners = client.get_owners();
    assert_eq!(owners.len(), 3);
}

#[test]
#[should_panic]
fn test_double_initialize_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let (client, _) = setup_wallet(&env, &[owner1.clone()], 1);

    let mut owners = Vec::new(&env);
    owners.push_back(owner1);
    client.initialize(&owners, &1);
}

#[test]
fn test_threshold_exceeds_owners_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let contract_id = env.register(MultiSigWalletContract, ());
    let client = MultiSigWalletContractClient::new(&env, &contract_id);
    let mut owners = Vec::new(&env);
    owners.push_back(owner1);
    let result = client.try_initialize(&owners, &2); // threshold 2 > 1 owner
    assert_eq!(result, Err(Ok(crate::errors::Error::ThresholdExceedsSigCount)));
}

#[test]
#[should_panic]
fn test_duplicate_owner_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let contract_id = env.register(MultiSigWalletContract, ());
    let client = MultiSigWalletContractClient::new(&env, &contract_id);
    let mut owners = Vec::new(&env);
    owners.push_back(owner1.clone());
    owners.push_back(owner1); // duplicate
    client.initialize(&owners, &1);
}

#[test]
fn test_propose_tx_auto_confirms_for_proposer() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();

    let (client, _) = setup_wallet(&env, &[owner1.clone(), owner2.clone()], 2);

    let tx_id = client.propose_tx(&owner1, &recipient, &token_addr, &1_000_000i128);
    assert_eq!(tx_id, 1);
    assert_eq!(client.get_tx_count(), 1);

    let tx = client.get_tx(&tx_id);
    assert_eq!(tx.confirmations, 1);
    assert_eq!(tx.state, TxState::Pending);

    let confirmers = client.get_confirmers(&tx_id);
    assert_eq!(confirmers.len(), 1);
    assert_eq!(confirmers.get(0).unwrap(), owner1);
}

#[test]
fn test_confirm_and_execute() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);

    let (client, contract_id) = setup_wallet(&env, &[owner1.clone(), owner2.clone()], 2);

    // Fund the wallet contract
    asset_client.mint(&contract_id, &5_000_000);

    let amount: i128 = 1_000_000;
    let tx_id = client.propose_tx(&owner1, &recipient, &token_addr, &amount);

    // owner2 confirms — now threshold (2) is met
    client.confirm_tx(&owner2, &tx_id);

    let tx = client.get_tx(&tx_id);
    assert_eq!(tx.confirmations, 2);

    client.execute_tx(&owner1, &tx_id);

    let tx = client.get_tx(&tx_id);
    assert_eq!(tx.state, TxState::Executed);

    let token_client = token::Client::new(&env, &token_addr);
    assert_eq!(token_client.balance(&recipient), amount);
}

#[test]
#[should_panic]
fn test_execute_below_threshold_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let owner3 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);

    let (client, contract_id) =
        setup_wallet(&env, &[owner1.clone(), owner2.clone(), owner3.clone()], 3);
    asset_client.mint(&contract_id, &5_000_000);

    let tx_id = client.propose_tx(&owner1, &recipient, &token_addr, &1_000_000i128);
    client.confirm_tx(&owner2, &tx_id); // only 2/3 — should panic

    client.execute_tx(&owner1, &tx_id);
}

#[test]
fn test_revoke_confirmation() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let (client, _) = setup_wallet(&env, &[owner1.clone(), owner2.clone()], 2);

    let tx_id = client.propose_tx(&owner1, &recipient, &token_addr, &1_000_000i128);
    client.confirm_tx(&owner2, &tx_id);
    assert_eq!(client.get_tx(&tx_id).confirmations, 2);

    client.revoke_confirmation(&owner2, &tx_id);
    assert_eq!(client.get_tx(&tx_id).confirmations, 1);
}

#[test]
#[should_panic]
fn test_double_confirm_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let (client, _) = setup_wallet(&env, &[owner1.clone(), owner2.clone()], 2);

    let tx_id = client.propose_tx(&owner1, &recipient, &token_addr, &1_000_000i128);
    client.confirm_tx(&owner2, &tx_id);
    client.confirm_tx(&owner2, &tx_id); // double confirm — should panic
}

#[test]
fn test_cancel_tx() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let (client, _) = setup_wallet(&env, &[owner1.clone(), owner2.clone()], 2);

    let tx_id = client.propose_tx(&owner1, &recipient, &token_addr, &1_000_000i128);
    client.cancel_tx(&owner1, &tx_id);

    assert_eq!(client.get_tx(&tx_id).state, TxState::Cancelled);
}

#[test]
#[should_panic]
fn test_non_proposer_cannot_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let (client, _) = setup_wallet(&env, &[owner1.clone(), owner2.clone()], 2);

    let tx_id = client.propose_tx(&owner1, &recipient, &token_addr, &1_000_000i128);
    client.cancel_tx(&owner2, &tx_id); // non-proposer — should panic
}

#[test]
#[should_panic]
fn test_non_owner_cannot_propose() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let outsider = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let (client, _) = setup_wallet(&env, &[owner1.clone()], 1);

    client.propose_tx(&outsider, &recipient, &token_addr, &1_000_000i128); // should panic
}

#[test]
fn test_advance_time_does_not_affect_multisig() {
    let env = Env::default();
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(&env, &token_addr);

    let (client, contract_id) = setup_wallet(&env, &[owner1.clone(), owner2.clone()], 2);
    asset_client.mint(&contract_id, &5_000_000);

    let tx_id = client.propose_tx(&owner1, &recipient, &token_addr, &1_000_000i128);
    advance_time(&env, 86400 * 30); // 30 days later — still works
    client.confirm_tx(&owner2, &tx_id);
    client.execute_tx(&owner1, &tx_id);

    assert_eq!(client.get_tx(&tx_id).state, TxState::Executed);
}
