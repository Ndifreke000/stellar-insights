#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, String,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn setup_contract(env: &Env, quorum: u64, voting_period: u64) -> (GovernanceContractClient, Address) {
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin, &quorum, &voting_period);
    (client, admin)
}

fn make_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

// ── 1. Quorum boundary: votes_for must strictly exceed votes_against ──────────

#[test]
fn fuzz_quorum_boundary() {
    // Test quorum values from 1 to 10 and vote distributions around the boundary
    for quorum in 1u64..=5 {
        for votes_for in 0u64..=(quorum + 2) {
            for votes_against in 0u64..=(quorum + 2) {
                let env = Env::default();
                env.mock_all_auths();
                let (client, admin) = setup_contract(&env, quorum, 1000);

                let target = Address::generate(&env);
                let hash = make_hash(&env, 1);
                let pid = client.create_proposal(&admin, &String::from_str(&env, "P"), &target, &hash);

                for _ in 0..votes_for {
                    let voter = Address::generate(&env);
                    client.vote(&voter, &pid, &VoteChoice::For);
                }
                for _ in 0..votes_against {
                    let voter = Address::generate(&env);
                    client.vote(&voter, &pid, &VoteChoice::Against);
                }

                env.ledger().with_mut(|li| li.timestamp = 2000);
                let status = client.finalize(&pid);

                let total = votes_for + votes_against;
                let expected = if total >= quorum && votes_for > votes_against {
                    ProposalStatus::Passed
                } else {
                    ProposalStatus::Failed
                };
                assert_eq!(
                    status, expected,
                    "quorum={quorum} for={votes_for} against={votes_against}"
                );
            }
        }
    }
}

// ── 2. Double-vote invariant: second vote must always be rejected ─────────────

#[test]
fn fuzz_double_vote_always_rejected() {
    for choice_a in [VoteChoice::For, VoteChoice::Against, VoteChoice::Abstain] {
        for choice_b in [VoteChoice::For, VoteChoice::Against, VoteChoice::Abstain] {
            let env = Env::default();
            env.mock_all_auths();
            let (client, admin) = setup_contract(&env, 1, 1000);

            let target = Address::generate(&env);
            let pid = client.create_proposal(
                &admin,
                &String::from_str(&env, "P"),
                &target,
                &make_hash(&env, 1),
            );

            let voter = Address::generate(&env);
            client.vote(&voter, &pid, &choice_a);

            let result = client.try_vote(&voter, &pid, &choice_b);
            assert_eq!(
                result,
                Err(Ok(Error::AlreadyVoted)),
                "second vote ({choice_a:?} then {choice_b:?}) should be rejected"
            );
        }
    }
}

// ── 3. Voting-period boundary: vote at deadline must be rejected ──────────────

#[test]
fn fuzz_voting_period_boundary() {
    for voting_period in [1u64, 100, 1000, u64::MAX / 2] {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup_contract(&env, 1, voting_period);

        let target = Address::generate(&env);
        let pid = client.create_proposal(
            &admin,
            &String::from_str(&env, "P"),
            &target,
            &make_hash(&env, 2),
        );

        // Vote exactly at the deadline (voting_ends_at = 0 + voting_period)
        env.ledger().with_mut(|li| li.timestamp = voting_period);
        let voter = Address::generate(&env);
        let result = client.try_vote(&voter, &pid, &VoteChoice::For);
        assert_eq!(
            result,
            Err(Ok(Error::VotingNotActive)),
            "vote at deadline (ts={voting_period}) should be rejected"
        );

        // Vote one second before deadline must succeed
        let env2 = Env::default();
        env2.mock_all_auths();
        let (client2, admin2) = setup_contract(&env2, 1, voting_period);
        let target2 = Address::generate(&env2);
        let pid2 = client2.create_proposal(
            &admin2,
            &String::from_str(&env2, "P"),
            &target2,
            &make_hash(&env2, 3),
        );
        if voting_period > 0 {
            env2.ledger().with_mut(|li| li.timestamp = voting_period - 1);
            let voter2 = Address::generate(&env2);
            assert!(
                client2.try_vote(&voter2, &pid2, &VoteChoice::For).is_ok(),
                "vote before deadline should succeed (period={voting_period})"
            );
        }
    }
}

// ── 4. Unauthorized caller must always be rejected ───────────────────────────

#[test]
fn fuzz_unauthorized_always_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env, 1, 1000);

    for _ in 0..10 {
        let non_admin = Address::generate(&env);
        let target = Address::generate(&env);
        let result = client.try_create_proposal(
            &non_admin,
            &String::from_str(&env, "Malicious"),
            &target,
            &make_hash(&env, 99),
        );
        assert_eq!(result, Err(Ok(Error::UnauthorizedCaller)));
    }
}

// ── 5. Proposal count monotonically increases ────────────────────────────────

#[test]
fn fuzz_proposal_count_monotonic() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env, 1, 1000);

    for i in 1u64..=20 {
        let target = Address::generate(&env);
        let pid = client.create_proposal(
            &admin,
            &String::from_str(&env, "P"),
            &target,
            &make_hash(&env, i as u8),
        );
        assert_eq!(pid, i, "proposal id must equal sequential count");

        let (_, _, _, count) = client.get_config();
        assert_eq!(count, i);
    }
}

// ── 6. Finalize before voting period ends must fail ──────────────────────────

#[test]
fn fuzz_premature_finalize_rejected() {
    for voting_period in [100u64, 1000, 86_400] {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup_contract(&env, 1, voting_period);

        let target = Address::generate(&env);
        let pid = client.create_proposal(
            &admin,
            &String::from_str(&env, "P"),
            &target,
            &make_hash(&env, 5),
        );

        // Advance to just before deadline
        env.ledger().with_mut(|li| li.timestamp = voting_period - 1);
        let result = client.try_finalize(&pid);
        assert_eq!(
            result,
            Err(Ok(Error::VotingPeriodNotEnded)),
            "finalize before deadline (period={voting_period}) must fail"
        );
    }
}

// ── 7. Tally invariant: total_voters == votes_for + votes_against + votes_abstain

#[test]
fn fuzz_tally_invariant() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env, 1, 1000);

    let target = Address::generate(&env);
    let pid = client.create_proposal(
        &admin,
        &String::from_str(&env, "P"),
        &target,
        &make_hash(&env, 7),
    );

    let choices = [VoteChoice::For, VoteChoice::Against, VoteChoice::Abstain,
                   VoteChoice::For, VoteChoice::For, VoteChoice::Against];

    for choice in choices {
        let voter = Address::generate(&env);
        client.vote(&voter, &pid, &choice);
    }

    let tally = client.get_tally(&pid);
    assert_eq!(
        tally.total_voters,
        tally.votes_for + tally.votes_against + tally.votes_abstain,
        "tally invariant violated"
    );
    assert_eq!(tally.total_voters, 6);
    assert_eq!(tally.votes_for, 3);
    assert_eq!(tally.votes_against, 2);
    assert_eq!(tally.votes_abstain, 1);
}

// ── 8. has_voted reflects actual vote state ───────────────────────────────────

#[test]
fn fuzz_has_voted_consistency() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env, 1, 1000);

    let target = Address::generate(&env);
    let pid = client.create_proposal(
        &admin,
        &String::from_str(&env, "P"),
        &target,
        &make_hash(&env, 8),
    );

    for _ in 0..10 {
        let voter = Address::generate(&env);
        assert!(!client.has_voted(&pid, &voter), "should not have voted yet");
        client.vote(&voter, &pid, &VoteChoice::For);
        assert!(client.has_voted(&pid, &voter), "should have voted after casting");
    }
}

// ── 9. Proposal not found for arbitrary IDs ───────────────────────────────────

#[test]
fn fuzz_nonexistent_proposal_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env, 1, 1000);

    for id in [0u64, 1, 100, u64::MAX / 2, u64::MAX] {
        assert_eq!(
            client.try_get_proposal(&id),
            Err(Ok(Error::ProposalNotFound)),
            "non-existent proposal id={id} must return ProposalNotFound"
        );
    }
}

// ── 10. Quorum=0 edge case: any single vote passes ────────────────────────────

#[test]
fn fuzz_zero_quorum_edge_case() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env, 0, 1000);

    let target = Address::generate(&env);
    let pid = client.create_proposal(
        &admin,
        &String::from_str(&env, "P"),
        &target,
        &make_hash(&env, 10),
    );

    let voter = Address::generate(&env);
    client.vote(&voter, &pid, &VoteChoice::For);

    env.ledger().with_mut(|li| li.timestamp = 2000);
    let status = client.finalize(&pid);
    // quorum=0 means total_voters(1) >= 0 AND votes_for(1) > votes_against(0) → Passed
    assert_eq!(status, ProposalStatus::Passed);
}
