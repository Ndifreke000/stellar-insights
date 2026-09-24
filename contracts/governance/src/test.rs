#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use super::*;
use analytics::AnalyticsContractClient;
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger},
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
    // quorum_bps=2000 (20 %), voting_period=1000 seconds
    client.initialize(&admin, &2000, &1000);

    (env, client, admin)
}

#[test]
fn test_initialization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &3000, &500);

    let (config_admin, quorum, voting_period, proposal_count) = client.get_config();
    assert_eq!(config_admin, admin);
    assert_eq!(quorum, 3000);
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

    // total_supply=10: (2 * 10_000) / 10 = 2000 >= quorum_bps(2000) → Passed
    let status = client.finalize(&1, &10u64);
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

    // total_supply=10: (1 * 10_000) / 10 = 1000 < quorum_bps(2000) → Failed
    let status = client.finalize(&1, &10u64);
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

    // total_supply=10: quorum met (2000 >= 2000) but votes_for == votes_against → Failed
    let status = client.finalize(&1, &10u64);
    assert_eq!(status, ProposalStatus::Failed);
}

#[test]
fn test_mark_executed() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Test proposal");
    let target = Address::generate(&env);
    // Zero wasm hash: `mark_executed` skips `upgrade` (no uploaded Wasm required).
    let wasm_hash = BytesN::from_array(&env, &[0u8; 32]);
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
    client.finalize(&1, &10u64);

    // Admin marks as executed
    client.mark_executed(&admin, &1);

    let proposal = client.get_proposal(&1);
    assert_eq!(proposal.status, ProposalStatus::Executed);
}

#[test]
fn test_parameter_proposal_set_paused_execution() {
    let env = Env::default();
    env.mock_all_auths();

    let analytics_id = env.register_contract(None, analytics::AnalyticsContract);
    let governance_id = env.register_contract(None, GovernanceContract);

    let gov_client = GovernanceContractClient::new(&env, &governance_id);
    let analytics_client = AnalyticsContractClient::new(&env, &analytics_id);

    let admin = Address::generate(&env);
    analytics_client.initialize(&admin, &None);
    gov_client.initialize(&admin, &2000, &1000);

    analytics_client.set_governance(&admin, &governance_id);

    assert!(!analytics_client.is_paused());

    let title = String::from_str(&env, "Pause analytics for maintenance");
    let proposal_id = gov_client.create_parameter_proposal(
        &admin,
        &title,
        &analytics_id,
        &ParameterAction::SetPaused(true),
    );
    assert_eq!(proposal_id, 1);

    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    gov_client.vote(&voter1, &1, &VoteChoice::For);
    gov_client.vote(&voter2, &1, &VoteChoice::For);

    env.ledger().with_mut(|li| {
        li.timestamp = 2000;
    });
    let status = gov_client.finalize(&1, &10u64);
    assert_eq!(status, ProposalStatus::Passed);

    gov_client.mark_executed(&admin, &1);

    assert!(analytics_client.is_paused());
}

#[test]
fn test_create_parameter_proposal() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Set new admin");
    let target = Address::generate(&env);
    let new_admin = Address::generate(&env);

    let proposal_id = client.create_parameter_proposal(
        &admin,
        &title,
        &target,
        &ParameterAction::SetAdmin(new_admin.clone()),
    );
    assert_eq!(proposal_id, 1);

    let proposal = client.get_proposal(&1);
    assert_eq!(proposal.id, 1);
    assert_eq!(proposal.proposer, admin);
    assert_eq!(proposal.target_contract, target);
    assert_eq!(proposal.status, ProposalStatus::Active);

    let action = client.get_parameter_action(&1);
    assert!(action.is_some());
    match action.unwrap() {
        ParameterAction::SetAdmin(addr) => assert_eq!(addr, new_admin),
        _ => panic!("expected SetAdmin"),
    }
}

// ============================================================================
// Quorum basis-points precision tests (issue #1611)
// ============================================================================

#[test]
fn test_quorum_bps_precision_low_turnout_fails() {
    // With integer-division quorum (old: votes/supply*100) a single vote out of
    // 1 000 supply would compute 0 % and always pass a 0-% quorum threshold.
    // The bps fix: (1 * 10_000) / 1_000 = 10 bps, which correctly fails a
    // 100-bps (1 %) quorum.
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &100, &1000); // quorum_bps = 100 (1 %)

    let title = String::from_str(&env, "Low-turnout proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 42);
    client.create_proposal(&admin, &title, &target, &wasm_hash);

    // 1 vote out of total_supply=1000 → (1 * 10_000) / 1_000 = 10 bps < 100 bps → Failed
    let voter = Address::generate(&env);
    client.vote(&voter, &1, &VoteChoice::For);

    env.ledger().with_mut(|li| li.timestamp = 2000);
    let status = client.finalize(&1, &1000u64);
    assert_eq!(status, ProposalStatus::Failed, "1 vote out of 1000 supply must not meet 1% quorum");
}

#[test]
fn test_quorum_bps_precision_exact_boundary() {
    // 10 votes out of total_supply=100 → (10 * 10_000) / 100 = 1000 bps = quorum_bps → Passed
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &1000, &1000); // quorum_bps = 1000 (10 %)

    let title = String::from_str(&env, "Exact-quorum proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 43);
    client.create_proposal(&admin, &title, &target, &wasm_hash);

    for _ in 0..10 {
        let voter = Address::generate(&env);
        client.vote(&voter, &1, &VoteChoice::For);
    }

    env.ledger().with_mut(|li| li.timestamp = 2000);
    let status = client.finalize(&1, &100u64);
    assert_eq!(status, ProposalStatus::Passed, "exactly 10% turnout must meet 10% quorum");
}

#[test]
fn test_update_quorum_rejects_over_10000() {
    let (_, client, admin) = setup();
    let result = client.try_update_quorum(&admin, &10_001u64);
    assert_eq!(result, Err(Ok(Error::InvalidQuorum)));
}

#[test]
fn test_initialize_rejects_quorum_over_10000() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let result = client.try_initialize(&admin, &10_001u64, &1000u64);
    assert_eq!(result, Err(Ok(Error::InvalidQuorum)));
}

#[test]
fn test_finalize_rejects_zero_total_supply() {
    let (env, client, admin) = setup();
    let title = String::from_str(&env, "Zero supply proposal");
    let target = Address::generate(&env);
    let wasm_hash = create_test_hash(&env, 99);
    client.create_proposal(&admin, &title, &target, &wasm_hash);

    env.ledger().with_mut(|li| li.timestamp = 2000);
    let result = client.try_finalize(&1, &0u64);
    assert_eq!(result, Err(Ok(Error::InvalidTotalSupply)));
}

// ============================================================================
// Parameter Proposal Event Tests (Requirements 4.1, 4.2, 4.3)
// ============================================================================

#[test]
fn test_create_parameter_proposal_emits_prm_prop_topic() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Set new admin");
    let target = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.create_parameter_proposal(
        &admin,
        &title,
        &target,
        &ParameterAction::SetAdmin(new_admin.clone()),
    );

    use crate::events::{ParameterProposalCreatedEvent, PARAM_PROPOSAL};
    use soroban_sdk::Symbol;

    let events = env.events().all();
    let raw = events.events();
    let param_event = raw.iter().find(|e| {
        if let soroban_sdk::xdr::ContractEventBody::V0(ref v0) = e.body {
            if v0.topics.is_empty() {
                return false;
            }
            <Symbol as soroban_sdk::TryFromVal<Env, soroban_sdk::xdr::ScVal>>::try_from_val(
                &env,
                &v0.topics[0],
            )
            .map(|t| t == PARAM_PROPOSAL)
            .unwrap_or(false)
        } else {
            false
        }
    });

    assert!(param_event.is_some(), "PRM_PROP event should be emitted for parameter proposals");

    if let Some(e) = param_event {
        if let soroban_sdk::xdr::ContractEventBody::V0(ref v0) = e.body {
            let val =
                <soroban_sdk::Val as soroban_sdk::TryFromVal<Env, soroban_sdk::xdr::ScVal>>::try_from_val(
                    &env, &v0.data,
                )
                .unwrap();
            let data: ParameterProposalCreatedEvent = soroban_sdk::FromVal::from_val(&env, &val);
            assert_eq!(data.proposal_id, 1);
            assert_eq!(data.proposer, admin);
            assert_eq!(data.target_contract, target);
            assert_eq!(data.action_label, String::from_str(&env, "set_admin"));
        }
    }
}

#[test]
fn test_create_proposal_upgrade_still_emits_prop_crt_topic() {
    let (env, client, admin) = setup();

    let title = String::from_str(&env, "Upgrade contract");
    let target = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

    client.create_proposal(&admin, &title, &target, &wasm_hash);

    use crate::events::PROPOSAL_CREATED;
    use soroban_sdk::Symbol;

    let events = env.events().all();
    let raw = events.events();
    let upgrade_event = raw.iter().find(|e| {
        if let soroban_sdk::xdr::ContractEventBody::V0(ref v0) = e.body {
            if v0.topics.is_empty() {
                return false;
            }
            <Symbol as soroban_sdk::TryFromVal<Env, soroban_sdk::xdr::ScVal>>::try_from_val(
                &env,
                &v0.topics[0],
            )
            .map(|t| t == PROPOSAL_CREATED)
            .unwrap_or(false)
        } else {
            false
        }
    });

    assert!(upgrade_event.is_some(), "PROP_CRT event should still be emitted for upgrade proposals");
}
