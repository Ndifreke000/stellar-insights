//! Fuzz-style property tests for the governance contract.
//!
//! Complements the per-crate fuzz_tests.rs (quorum math, double-vote, voting
//! period) with cases that are best exercised from an external perspective:
//! parameter proposals with extreme values, quorum/period admin updates,
//! and cross-proposal state isolation.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use governance::{GovernanceContract, GovernanceContractClient, ProposalStatus, VoteChoice};
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, BytesN, Env, String};

fn setup(env: &Env, quorum: u64, voting_period: u64) -> (GovernanceContractClient, Address) {
    let id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin, &quorum, &voting_period);
    (client, admin)
}

fn hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn title(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

// ── 1. Non-admin cannot create a proposal ─────────────────────────────────────

#[test]
fn fuzz_non_admin_proposal_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env, 5000, 1000);

    for _ in 0..8 {
        let caller = Address::generate(&env);
        let target = Address::generate(&env);
        let result = client.try_create_proposal(&caller, &title(&env, "bad"), &target, &hash(&env, 1));
        assert!(
            matches!(result, Ok(Err(_))),
            "non-admin must not be able to create proposals"
        );
    }
}

// ── 2. Proposal IDs increase monotonically across multiple admins ─────────────

#[test]
fn fuzz_proposal_ids_monotonic() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1, 1000);

    for expected in 1u64..=15 {
        let target = Address::generate(&env);
        let pid = client.create_proposal(&admin, &title(&env, "P"), &target, &hash(&env, expected as u8));
        assert_eq!(pid, expected, "proposal ID must equal sequential count");
    }
}

// ── 3. Proposals are independent — voting on one does not affect another ──────

#[test]
fn fuzz_proposals_are_independent() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1, 1000);
    let target = Address::generate(&env);

    let pid_a = client.create_proposal(&admin, &title(&env, "A"), &target, &hash(&env, 1));
    let pid_b = client.create_proposal(&admin, &title(&env, "B"), &target, &hash(&env, 2));

    // Vote on A only
    for _ in 0..5 {
        let voter = Address::generate(&env);
        client.vote(&voter, &pid_a, &VoteChoice::For);
    }

    let tally_b = client.get_tally(&pid_b);
    assert_eq!(tally_b.total_voters, 0, "votes on proposal A must not affect proposal B");
}

// ── 4. Vote after deadline is rejected ────────────────────────────────────────

#[test]
fn fuzz_vote_after_deadline_rejected() {
    for voting_period in [10u64, 100, 1000] {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup(&env, 1, voting_period);
        let target = Address::generate(&env);
        let pid = client.create_proposal(&admin, &title(&env, "P"), &target, &hash(&env, 1));

        env.ledger().with_mut(|l| l.timestamp = voting_period + 1);
        let voter = Address::generate(&env);
        let result = client.try_vote(&voter, &pid, &VoteChoice::For);
        assert!(
            matches!(result, Ok(Err(_))),
            "vote at ts={} (period={voting_period}) must be rejected",
            voting_period + 1
        );
    }
}

// ── 5. Finalized proposal cannot be voted on ─────────────────────────────────

#[test]
fn fuzz_vote_on_finalized_proposal_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env, 1, 100);
    let target = Address::generate(&env);
    let pid = client.create_proposal(&admin, &title(&env, "P"), &target, &hash(&env, 1));

    let voter = Address::generate(&env);
    client.vote(&voter, &pid, &VoteChoice::For);

    // Advance past voting period and finalize
    env.ledger().with_mut(|l| l.timestamp = 200);
    client.finalize(&pid, &100u64);

    // Attempt another vote after finalization
    let late_voter = Address::generate(&env);
    let result = client.try_vote(&late_voter, &pid, &VoteChoice::For);
    assert!(
        matches!(result, Ok(Err(_))),
        "voting on a finalized proposal must be rejected"
    );
}

// ── 6. Quorum=10000 (100%) requires all voters to pass ───────────────────────

#[test]
fn fuzz_full_quorum_requires_full_participation() {
    let env = Env::default();
    env.mock_all_auths();
    // quorum_bps = 10_000 means 100% of supply must vote
    let (client, admin) = setup(&env, 10_000, 1000);
    let target = Address::generate(&env);
    let pid = client.create_proposal(&admin, &title(&env, "P"), &target, &hash(&env, 1));

    // 5 votes_for out of total_supply=10 → 50% participation → fails quorum
    for _ in 0..5 {
        let voter = Address::generate(&env);
        client.vote(&voter, &pid, &VoteChoice::For);
    }
    env.ledger().with_mut(|l| l.timestamp = 2000);
    let status = client.finalize(&pid, &10u64);
    assert_eq!(
        status,
        ProposalStatus::Failed,
        "5/10 votes does not meet 100% quorum"
    );
}

// ── 7. Non-admin cannot update quorum ────────────────────────────────────────

#[test]
fn fuzz_non_admin_quorum_update_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env, 5000, 1000);

    for _ in 0..5 {
        let attacker = Address::generate(&env);
        let result = client.try_update_quorum(&attacker, &1u64);
        assert!(
            matches!(result, Ok(Err(_))),
            "non-admin must not be able to update quorum"
        );
    }
}

// ── 8. Non-admin cannot update voting period ──────────────────────────────────

#[test]
fn fuzz_non_admin_voting_period_update_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env, 5000, 1000);

    for _ in 0..5 {
        let attacker = Address::generate(&env);
        let result = client.try_update_voting_period(&attacker, &9999u64);
        assert!(
            matches!(result, Ok(Err(_))),
            "non-admin must not be able to update voting period"
        );
    }
}

// ── 9. get_proposal on non-existent ID must return an error ──────────────────

#[test]
fn fuzz_nonexistent_proposal_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env, 1, 1000);

    for id in [0u64, 1, 99, u64::MAX / 2, u64::MAX] {
        let result = client.try_get_proposal(&id);
        assert!(
            matches!(result, Ok(Err(_))),
            "proposal id={id} does not exist — must return ProposalNotFound"
        );
    }
}

// ── 10. Abstain votes count toward quorum but not toward passing ──────────────

#[test]
fn fuzz_abstain_counts_toward_quorum_not_outcome() {
    let env = Env::default();
    env.mock_all_auths();
    // quorum_bps = 5000 (50%), total_supply = 10 → need ≥5 votes to meet quorum
    let (client, admin) = setup(&env, 5000, 1000);
    let target = Address::generate(&env);
    let pid = client.create_proposal(&admin, &title(&env, "P"), &target, &hash(&env, 1));

    // 5 abstain votes — meets quorum but nobody voted For
    for _ in 0..5 {
        let voter = Address::generate(&env);
        client.vote(&voter, &pid, &VoteChoice::Abstain);
    }
    env.ledger().with_mut(|l| l.timestamp = 2000);
    let status = client.finalize(&pid, &10u64);
    // votes_for (0) must NOT exceed votes_against (0) for a pass,
    // and 0 For vs 0 Against is not a win → Failed
    assert_eq!(
        status,
        ProposalStatus::Failed,
        "all-abstain must not result in Passed"
    );
}
