#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, String,
};

/// Helper function to create a 32-byte hash for testing
fn create_test_hash(env: &Env, value: u32) -> BytesN<32> {
    let mut bytes = [0u8; 32];
    bytes[0..4].copy_from_slice(&value.to_be_bytes());
    BytesN::from_array(env, &bytes)
}

/// Helper to set up a standard test environment with initialized contract
fn setup() -> (Env, GovernanceContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    // quorum=2, voting_period=1000 seconds
    client.initialize(&admin, &2, &1000);

    (env, client, admin)
}

#[test]
fn test_initialization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &3, &500);

    let (config_admin, quorum, voting_period, proposal_count) = client.get_config();
    assert_eq!(config_admin, admin);
    assert_eq!(quorum, 3);
    assert_eq!(voting_period, 500);
    assert_eq!(proposal_count, 0);
}

#[test]
fn test_create_proposal() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Upgrade analytics contract");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 12345);

    let proposal_id = client.create_proposal(&admin, &title, &target, &wasm_hash);
    assert_eq!(proposal_id, 1);

    let proposal = client.get_proposal(&1);
    assert_eq!(proposal.id, 1);
    assert_eq!(proposal.proposer, admin);
    assert_eq!(proposal.title, title);
    assert_eq!(proposal.target_contract, target);
    assert_eq!(proposal.new_wasm_hash, wasm_hash);
    assert_eq!(proposal.status, ProposalStatus::Active);
}

#[test]
fn test_unauthorized_create_fails() {
    let (env, client, _admin) = setup();

    let unauthorized = Address::generate(&env);
    let title = String::from_str(&env, "Malicious proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 99999);

    let result = client.try_create_proposal(&unauthorized, &title, &target, &wasm_hash);
    assert_eq!(result, Err(Ok(Error::UnauthorizedCaller)));
}

#[test]
fn test_vote_success() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Test proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 11111);
    client.create_proposal(&admin, &title, &target, &wasm_hash);

    let voter = Address::generate(&env);
    client.vote(&voter, &1, &VoteChoice::For);

    assert!(client.has_voted(&1, &voter));

    let tally = client.get_tally(&1);
    assert_eq!(tally.votes_for, 1);
    assert_eq!(tally.votes_against, 0);
    assert_eq!(tally.votes_abstain, 0);
    assert_eq!(tally.total_voters, 1);
}

#[test]
fn test_double_vote_fails() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Test proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 11111);
    client.create_proposal(&admin, &title, &target, &wasm_hash);

    let voter = Address::generate(&env);
    client.vote(&voter, &1, &VoteChoice::For);

    let result = client.try_vote(&voter, &1, &VoteChoice::Against);
    assert_eq!(result, Err(Ok(Error::AlreadyVoted)));
}

#[test]
fn test_vote_after_deadline_fails() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Test proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 11111);
    client.create_proposal(&admin, &title, &target, &wasm_hash);

    // Advance time past voting period
    env.ledger().with_mut(|li| {
        li.timestamp = 2000;
    });

    let voter = Address::generate(&env);
    let result = client.try_vote(&voter, &1, &VoteChoice::For);
    assert_eq!(result, Err(Ok(Error::VotingNotActive)));
}

#[test]
fn test_finalize_passed() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Test proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 11111);
    client.create_proposal(&admin, &title, &target, &wasm_hash);

    // Two voters vote For (meets quorum of 2)
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    client.vote(&voter1, &1, &VoteChoice::For);
    client.vote(&voter2, &1, &VoteChoice::For);

    // Advance past voting period
    env.ledger().with_mut(|li| {
        li.timestamp = 2000;
    });

    let status = client.finalize(&1);
    assert_eq!(status, ProposalStatus::Passed);

    let proposal = client.get_proposal(&1);
    assert_eq!(proposal.status, ProposalStatus::Passed);
}

#[test]
fn test_finalize_failed_no_quorum() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Test proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 11111);
    client.create_proposal(&admin, &title, &target, &wasm_hash);

    // Only one voter (quorum is 2)
    let voter1 = Address::generate(&env);
    client.vote(&voter1, &1, &VoteChoice::For);

    // Advance past voting period
    env.ledger().with_mut(|li| {
        li.timestamp = 2000;
    });

    let status = client.finalize(&1);
    assert_eq!(status, ProposalStatus::Failed);
}

#[test]
fn test_finalize_failed_majority_against() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Test proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 11111);
    client.create_proposal(&admin, &title, &target, &wasm_hash);

    // Two voters: one For, one Against (quorum met but no majority)
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    client.vote(&voter1, &1, &VoteChoice::For);
    client.vote(&voter2, &1, &VoteChoice::Against);

    // Advance past voting period
    env.ledger().with_mut(|li| {
        li.timestamp = 2000;
    });

    let status = client.finalize(&1);
    assert_eq!(status, ProposalStatus::Failed);
}

#[test]
fn test_mark_executed() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Test proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 11111);
    client.create_proposal(&admin, &title, &target, &wasm_hash);

    // Get enough votes to pass
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    client.vote(&voter1, &1, &VoteChoice::For);
    client.vote(&voter2, &1, &VoteChoice::For);

    // Advance past voting period and finalize
    env.ledger().with_mut(|li| {
        li.timestamp = 2000;
    });
    client.finalize(&1);

    // Admin marks as executed
    client.mark_executed(&admin, &1);

    let proposal = client.get_proposal(&1);
    assert_eq!(proposal.status, ProposalStatus::Executed);
}
