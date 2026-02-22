#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{emit_proposal_created, emit_proposal_finalized, emit_vote_cast};
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Map, String};

// ============================================================================
// Data Types
// ============================================================================

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ProposalStatus {
    Active = 0,
    Passed = 1,
    Failed = 2,
    Executed = 3,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VoteChoice {
    For = 0,
    Against = 1,
    Abstain = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub title: String,
    pub target_contract: Address,
    pub new_wasm_hash: BytesN<32>,
    pub status: ProposalStatus,
    pub created_at: u64,
    pub voting_ends_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteTally {
    pub votes_for: u64,
    pub votes_against: u64,
    pub votes_abstain: u64,
    pub total_voters: u64,
}

// ============================================================================
// Storage Keys
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    ProposalCount,
    Quorum,
    VotingPeriod,
    Proposals,
    Votes(u64),
    VoteTally(u64),
}

// ============================================================================
// Contract
// ============================================================================

#[contract]
pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    /// Initialize the governance contract with an admin, quorum, and voting period.
    pub fn initialize(env: Env, admin: Address, quorum: u64, voting_period: u64) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCount, &0u64);
        env.storage().instance().set(&DataKey::Quorum, &quorum);
        env.storage()
            .instance()
            .set(&DataKey::VotingPeriod, &voting_period);
    }

    /// Create a new governance proposal. Only the admin can create proposals.
    pub fn create_proposal(
        env: Env,
        caller: Address,
        title: String,
        target_contract: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<u64, Error> {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if caller != admin {
            return Err(Error::UnauthorizedCaller);
        }

        if title.len() == 0 {
            return Err(Error::InvalidTitle);
        }

        let voting_period: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VotingPeriod)
            .unwrap_or(0);

        let now = env.ledger().timestamp();
        let voting_ends_at = now + voting_period;

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);
        count += 1;

        let proposal = Proposal {
            id: count,
            proposer: caller.clone(),
            title,
            target_contract: target_contract.clone(),
            new_wasm_hash,
            status: ProposalStatus::Active,
            created_at: now,
            voting_ends_at,
        };

        // Store proposal in the proposals map
        let mut proposals: Map<u64, Proposal> = env
            .storage()
            .persistent()
            .get(&DataKey::Proposals)
            .unwrap_or_else(|| Map::new(&env));
        proposals.set(count, proposal);
        env.storage()
            .persistent()
            .set(&DataKey::Proposals, &proposals);

        // Initialize vote tally
        let tally = VoteTally {
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            total_voters: 0,
        };
        env.storage()
            .persistent()
            .set(&DataKey::VoteTally(count), &tally);

        // Initialize votes map for this proposal
        let votes: Map<Address, VoteChoice> = Map::new(&env);
        env.storage()
            .persistent()
            .set(&DataKey::Votes(count), &votes);

        // Update proposal count
        env.storage()
            .instance()
            .set(&DataKey::ProposalCount, &count);

        emit_proposal_created(&env, count, caller, target_contract, voting_ends_at);

        Ok(count)
    }

    /// Cast a vote on an active proposal. Each address can only vote once.
    pub fn vote(
        env: Env,
        voter: Address,
        proposal_id: u64,
        choice: VoteChoice,
    ) -> Result<(), Error> {
        voter.require_auth();

        // Get the proposal
        let proposals: Map<u64, Proposal> = env
            .storage()
            .persistent()
            .get(&DataKey::Proposals)
            .unwrap_or_else(|| Map::new(&env));

        let proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;

        // Check proposal is still active
        if proposal.status != ProposalStatus::Active {
            return Err(Error::VotingNotActive);
        }

        // Check voting period has not ended
        let now = env.ledger().timestamp();
        if now >= proposal.voting_ends_at {
            return Err(Error::VotingNotActive);
        }

        // Check voter has not already voted
        let mut votes: Map<Address, VoteChoice> = env
            .storage()
            .persistent()
            .get(&DataKey::Votes(proposal_id))
            .unwrap_or_else(|| Map::new(&env));

        if votes.contains_key(voter.clone()) {
            return Err(Error::AlreadyVoted);
        }

        // Record the vote
        votes.set(voter.clone(), choice.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Votes(proposal_id), &votes);

        // Update tally
        let mut tally: VoteTally = env
            .storage()
            .persistent()
            .get(&DataKey::VoteTally(proposal_id))
            .unwrap_or(VoteTally {
                votes_for: 0,
                votes_against: 0,
                votes_abstain: 0,
                total_voters: 0,
            });

        match choice {
            VoteChoice::For => tally.votes_for += 1,
            VoteChoice::Against => tally.votes_against += 1,
            VoteChoice::Abstain => tally.votes_abstain += 1,
        }
        tally.total_voters += 1;

        env.storage()
            .persistent()
            .set(&DataKey::VoteTally(proposal_id), &tally);

        let choice_val = choice as u32;
        emit_vote_cast(&env, proposal_id, voter, choice_val);

        Ok(())
    }

    /// Finalize a proposal after the voting period has ended.
    /// Anyone can call this function once the deadline passes.
    pub fn finalize(env: Env, proposal_id: u64) -> Result<ProposalStatus, Error> {
        let mut proposals: Map<u64, Proposal> = env
            .storage()
            .persistent()
            .get(&DataKey::Proposals)
            .unwrap_or_else(|| Map::new(&env));

        let mut proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;

        // Must still be active
        if proposal.status != ProposalStatus::Active {
            return Err(Error::AlreadyFinalized);
        }

        // Voting period must have ended
        let now = env.ledger().timestamp();
        if now < proposal.voting_ends_at {
            return Err(Error::VotingPeriodNotEnded);
        }

        let tally: VoteTally = env
            .storage()
            .persistent()
            .get(&DataKey::VoteTally(proposal_id))
            .unwrap_or(VoteTally {
                votes_for: 0,
                votes_against: 0,
                votes_abstain: 0,
                total_voters: 0,
            });

        let quorum: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Quorum)
            .unwrap_or(0);

        // Determine outcome: passes if quorum met AND more for than against
        let new_status = if tally.total_voters >= quorum && tally.votes_for > tally.votes_against {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Failed
        };

        proposal.status = new_status.clone();
        proposals.set(proposal_id, proposal);
        env.storage()
            .persistent()
            .set(&DataKey::Proposals, &proposals);

        let status_val = new_status.clone() as u32;
        emit_proposal_finalized(
            &env,
            proposal_id,
            status_val,
            tally.votes_for,
            tally.votes_against,
            tally.total_voters,
        );

        Ok(new_status)
    }

    /// Mark a passed proposal as executed. Only the admin can call this.
    pub fn mark_executed(env: Env, caller: Address, proposal_id: u64) -> Result<(), Error> {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if caller != admin {
            return Err(Error::UnauthorizedCaller);
        }

        let mut proposals: Map<u64, Proposal> = env
            .storage()
            .persistent()
            .get(&DataKey::Proposals)
            .unwrap_or_else(|| Map::new(&env));

        let mut proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Passed {
            return Err(Error::ProposalNotPassed);
        }

        proposal.status = ProposalStatus::Executed;
        proposals.set(proposal_id, proposal);
        env.storage()
            .persistent()
            .set(&DataKey::Proposals, &proposals);

        Ok(())
    }

    // ========================================================================
    // Query Functions
    // ========================================================================

    /// Get a proposal by ID.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<Proposal, Error> {
        let proposals: Map<u64, Proposal> = env
            .storage()
            .persistent()
            .get(&DataKey::Proposals)
            .unwrap_or_else(|| Map::new(&env));

        proposals.get(proposal_id).ok_or(Error::ProposalNotFound)
    }

    /// Get the vote tally for a proposal.
    pub fn get_tally(env: Env, proposal_id: u64) -> Result<VoteTally, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::VoteTally(proposal_id))
            .ok_or(Error::ProposalNotFound)
    }

    /// Check if an address has voted on a proposal.
    pub fn has_voted(env: Env, proposal_id: u64, voter: Address) -> bool {
        let votes: Map<Address, VoteChoice> = env
            .storage()
            .persistent()
            .get(&DataKey::Votes(proposal_id))
            .unwrap_or_else(|| Map::new(&env));

        votes.contains_key(voter)
    }

    /// Get contract configuration (admin, quorum, voting_period, proposal_count).
    pub fn get_config(env: Env) -> Result<(Address, u64, u64, u64), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        let quorum: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Quorum)
            .unwrap_or(0);

        let voting_period: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VotingPeriod)
            .unwrap_or(0);

        let proposal_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);

        Ok((admin, quorum, voting_period, proposal_count))
    }
}

mod test;
