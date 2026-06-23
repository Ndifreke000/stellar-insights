#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{
    emit_admin_changed, emit_initialized, emit_proposal_created, emit_proposal_finalized,
    emit_vote_cast, emit_voter_registered,
};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;
const DATA_TTL_THRESHOLD: u32 = 100_000;
const DATA_TTL_EXTEND: u32 = 518_400;

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

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
    pub description: String,
    pub status: ProposalStatus,
    pub created_at: u64,
    pub voting_ends_at: u64,
    /// Sum of voting weights cast For
    pub votes_for: u64,
    /// Sum of voting weights cast Against
    pub votes_against: u64,
    /// Sum of voting weights cast Abstain
    pub votes_abstain: u64,
    /// Total number of unique voters (not weight-adjusted)
    pub voter_count: u64,
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
    Version,
    /// Per-proposal storage
    Proposal(u64),
    /// Per-voter weight (registered voters)
    VoterWeight(Address),
    /// Whether a voter has voted on a specific proposal
    VoteCast(u64, Address),
    /// Ordered list of all registered voter addresses (for snapshotting)
    VoterList,
    /// Snapshot of a voter's weight taken at proposal creation time
    ProposalVoterSnapshot(u64, Address),
}

// ============================================================================
// Contract
// ============================================================================

#[contract]
pub struct GovernanceVotingContract;

#[contractimpl]
impl GovernanceVotingContract {
    /// Initialize the governance voting contract.
    ///
    /// * `admin` — address that can register voters and change parameters.
    /// * `quorum` — minimum total weight-adjusted votes needed for a proposal to pass.
    /// * `voting_period` — voting window in seconds.
    pub fn initialize(
        env: Env,
        admin: Address,
        quorum: u64,
        voting_period: u64,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::ProposalCount, &0u64);
        env.storage().instance().set(&DataKey::Quorum, &quorum);
        env.storage()
            .instance()
            .set(&DataKey::VotingPeriod, &voting_period);
        env.storage()
            .instance()
            .set(&DataKey::Version, &String::from_str(&env, VERSION));
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);

        emit_initialized(&env, admin, quorum, voting_period);
        Ok(())
    }

    pub fn get_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or_else(|| String::from_str(&env, VERSION))
    }

    // ========================================================================
    // Voter Management
    // ========================================================================

    /// Register a voter with a given voting weight.
    ///
    /// Admin-only. `weight` represents how many votes this address carries
    /// (e.g. token balance snapshot). Must be > 0.
    pub fn register_voter(
        env: Env,
        caller: Address,
        voter: Address,
        weight: u64,
    ) -> Result<(), Error> {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if caller != admin {
            return Err(Error::Unauthorized);
        }

        if weight == 0 {
            return Err(Error::InvalidVotingWeight);
        }

        env.storage()
            .persistent()
            .set(&DataKey::VoterWeight(voter.clone()), &weight);
        env.storage().persistent().extend_ttl(
            &DataKey::VoterWeight(voter.clone()),
            DATA_TTL_THRESHOLD,
            DATA_TTL_EXTEND,
        );

        // Maintain the global voter list so create_proposal can snapshot weights.
        let mut voter_list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::VoterList)
            .unwrap_or_else(|| Vec::new(&env));
        if !voter_list.contains(&voter) {
            voter_list.push_back(voter.clone());
            env.storage()
                .instance()
                .set(&DataKey::VoterList, &voter_list);
        }

        bump_instance(&env);

        emit_voter_registered(&env, voter, weight);
        Ok(())
    }

    /// Remove a voter's registration. Admin-only.
    pub fn deregister_voter(env: Env, caller: Address, voter: Address) -> Result<(), Error> {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if caller != admin {
            return Err(Error::Unauthorized);
        }

        env.storage()
            .persistent()
            .remove(&DataKey::VoterWeight(voter.clone()));

        // Remove from voter list.
        let voter_list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::VoterList)
            .unwrap_or_else(|| Vec::new(&env));
        let mut updated: Vec<Address> = Vec::new(&env);
        for addr in voter_list.iter() {
            if addr != voter {
                updated.push_back(addr);
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::VoterList, &updated);

        bump_instance(&env);
        Ok(())
    }

    /// Get the voting weight of a registered voter (returns 0 if not registered).
    pub fn get_voter_weight(env: Env, voter: Address) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::VoterWeight(voter))
            .unwrap_or(0)
    }

    // ========================================================================
    // Proposal Lifecycle
    // ========================================================================

    /// Create a new voting proposal.
    ///
    /// Any registered voter (or the admin) may create a proposal.
    pub fn create_proposal(
        env: Env,
        caller: Address,
        title: String,
        description: String,
    ) -> Result<u64, Error> {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        // Must be admin or a registered voter
        let is_admin = caller == admin;
        let is_voter = env
            .storage()
            .persistent()
            .get::<DataKey, u64>(&DataKey::VoterWeight(caller.clone()))
            .is_some();

        if !is_admin && !is_voter {
            return Err(Error::VoterNotRegistered);
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
        count = count.checked_add(1).ok_or(Error::ProposalIdOverflow)?;

        let proposal = Proposal {
            id: count,
            proposer: caller.clone(),
            title: title.clone(),
            description,
            status: ProposalStatus::Active,
            created_at: now,
            voting_ends_at,
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            voter_count: 0,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Proposal(count), &proposal);
        env.storage().persistent().extend_ttl(
            &DataKey::Proposal(count),
            DATA_TTL_THRESHOLD,
            DATA_TTL_EXTEND,
        );

        // Snapshot every registered voter's weight at proposal creation so that
        // transfers or admin re-registrations after this point cannot influence
        // the vote tally (prevents the double-vote window).
        let voter_list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::VoterList)
            .unwrap_or_else(|| Vec::new(&env));
        for voter_addr in voter_list.iter() {
            if let Some(w) = env
                .storage()
                .persistent()
                .get::<DataKey, u64>(&DataKey::VoterWeight(voter_addr.clone()))
            {
                let snapshot_key = DataKey::ProposalVoterSnapshot(count, voter_addr.clone());
                env.storage().persistent().set(&snapshot_key, &w);
                env.storage().persistent().extend_ttl(
                    &snapshot_key,
                    DATA_TTL_THRESHOLD,
                    DATA_TTL_EXTEND,
                );
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::ProposalCount, &count);
        bump_instance(&env);

        emit_proposal_created(&env, count, caller, title, voting_ends_at);

        Ok(count)
    }

    /// Cast a weighted vote on an active proposal.
    ///
    /// Each registered address may vote exactly once per proposal.
    /// The voter's registered weight is applied to the chosen option.
    pub fn vote(
        env: Env,
        voter: Address,
        proposal_id: u64,
        choice: VoteChoice,
    ) -> Result<(), Error> {
        voter.require_auth();

        // Use the weight that was snapshotted at proposal creation time.
        // This closes the double-vote window where a voter could cast a vote and
        // then transfer/re-register tokens to a second address before the tally.
        let weight: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ProposalVoterSnapshot(proposal_id, voter.clone()))
            .ok_or(Error::VoterNotRegistered)?;

        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Active {
            return Err(Error::VotingNotActive);
        }

        let now = env.ledger().timestamp();
        if now >= proposal.voting_ends_at {
            return Err(Error::VotingNotActive);
        }

        let vote_key = DataKey::VoteCast(proposal_id, voter.clone());
        if env.storage().persistent().has(&vote_key) {
            return Err(Error::AlreadyVoted);
        }

        env.storage().persistent().set(&vote_key, &choice);
        env.storage().persistent().extend_ttl(
            &vote_key,
            DATA_TTL_THRESHOLD,
            DATA_TTL_EXTEND,
        );

        match choice {
            VoteChoice::For => {
                proposal.votes_for = proposal.votes_for.saturating_add(weight);
            }
            VoteChoice::Against => {
                proposal.votes_against = proposal.votes_against.saturating_add(weight);
            }
            VoteChoice::Abstain => {
                proposal.votes_abstain = proposal.votes_abstain.saturating_add(weight);
            }
        }
        proposal.voter_count = proposal.voter_count.saturating_add(1);

        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        env.storage().persistent().extend_ttl(
            &DataKey::Proposal(proposal_id),
            DATA_TTL_THRESHOLD,
            DATA_TTL_EXTEND,
        );

        let choice_val = choice as u32;
        emit_vote_cast(&env, proposal_id, voter, choice_val, weight);

        Ok(())
    }

    /// Finalize a proposal after the voting period has ended.
    ///
    /// Anyone may call this once the deadline passes.
    /// A proposal passes when `votes_for > votes_against` AND
    /// `votes_for >= quorum`.
    pub fn finalize_proposal(env: Env, proposal_id: u64) -> Result<ProposalStatus, Error> {
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Active {
            return Err(Error::AlreadyFinalized);
        }

        let now = env.ledger().timestamp();
        if now < proposal.voting_ends_at {
            return Err(Error::VotingPeriodNotEnded);
        }

        let quorum: u64 = env.storage().instance().get(&DataKey::Quorum).unwrap_or(0);

        let new_status =
            if proposal.votes_for >= quorum && proposal.votes_for > proposal.votes_against {
                ProposalStatus::Passed
            } else {
                ProposalStatus::Failed
            };

        proposal.status = new_status;
        let votes_for = proposal.votes_for;
        let votes_against = proposal.votes_against;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        env.storage().persistent().extend_ttl(
            &DataKey::Proposal(proposal_id),
            DATA_TTL_THRESHOLD,
            DATA_TTL_EXTEND,
        );

        let status_val = new_status as u32;
        emit_proposal_finalized(&env, proposal_id, status_val, votes_for, votes_against);

        Ok(new_status)
    }

    // ========================================================================
    // Admin Functions
    // ========================================================================

    pub fn update_quorum(env: Env, caller: Address, new_quorum: u64) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Quorum, &new_quorum);
        bump_instance(&env);
        Ok(())
    }

    pub fn update_voting_period(env: Env, caller: Address, new_period: u64) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::VotingPeriod, &new_period);
        bump_instance(&env);
        Ok(())
    }

    pub fn set_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
        caller.require_auth();
        let old_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        if caller != old_admin {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        bump_instance(&env);
        emit_admin_changed(&env, old_admin, new_admin);
        Ok(())
    }

    // ========================================================================
    // Query Functions
    // ========================================================================

    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<Proposal, Error> {
        let proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;
        env.storage().persistent().extend_ttl(
            &DataKey::Proposal(proposal_id),
            DATA_TTL_THRESHOLD,
            DATA_TTL_EXTEND,
        );
        Ok(proposal)
    }

    pub fn has_voted(env: Env, proposal_id: u64, voter: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::VoteCast(proposal_id, voter))
    }

    pub fn get_config(env: Env) -> Result<(Address, u64, u64, u64), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        let quorum: u64 = env.storage().instance().get(&DataKey::Quorum).unwrap_or(0);
        let voting_period: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VotingPeriod)
            .unwrap_or(0);
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);
        Ok((admin, quorum, voting_period, count))
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)
    }
}

mod test;
