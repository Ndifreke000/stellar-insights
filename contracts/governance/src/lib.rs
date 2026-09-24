#![no_std]
extern crate std;

mod errors;
mod events;

use analytics::AnalyticsContractClient;
use errors::Error;
use events::{
    emit_governance_initialized, emit_proposal_created, emit_proposal_executed,
    emit_proposal_finalized, emit_vote_cast,
};
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Map, String};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

/// Parameter update action for governed contracts (e.g. analytics).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParameterAction {
    SetAdmin(Address),
    SetPaused(bool),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub title: String,
    pub target_contract: Address,
    /// For upgrade proposals; zero hash means this is a parameter-update proposal.
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
    Version,
    /// Legacy: all proposals in one map. Read-only fallback; new writes use `Proposal(id)`.
    Proposals,
    /// Legacy: all votes for a proposal in one map. Read-only fallback; new writes use `Vote`.
    Votes(u64),
    VoteTally(u64),
    /// Parameter-update action for a proposal (when present, proposal is parameter type).
    ParameterAction(u64),
    /// One entry per proposal so reads/writes cost O(1) instead of O(proposals).
    Proposal(u64),
    /// One entry per (proposal, voter) so voting costs O(1) instead of O(voters).
    Vote(u64, Address),
}

const EMPTY_TALLY: VoteTally = VoteTally {
    votes_for: 0,
    votes_against: 0,
    votes_abstain: 0,
    total_voters: 0,
};

fn load_proposal(env: &Env, proposal_id: u64) -> Result<Proposal, Error> {
    let persistent = env.storage().persistent();
    if let Some(p) = persistent.get(&DataKey::Proposal(proposal_id)) {
        return Ok(p);
    }
    persistent
        .get::<_, Map<u64, Proposal>>(&DataKey::Proposals)
        .and_then(|m| m.get(proposal_id))
        .ok_or(Error::ProposalNotFound)
}

fn save_proposal(env: &Env, proposal: &Proposal) {
    env.storage()
        .persistent()
        .set(&DataKey::Proposal(proposal.id), proposal);
}

fn voted(env: &Env, proposal_id: u64, voter: &Address) -> bool {
    let persistent = env.storage().persistent();
    persistent.has(&DataKey::Vote(proposal_id, voter.clone()))
        || persistent
            .get::<_, Map<Address, VoteChoice>>(&DataKey::Votes(proposal_id))
            .is_some_and(|m| m.contains_key(voter.clone()))
}

/// Allocate the next proposal id and read the voting period (instance storage, single access each).
fn next_proposal(env: &Env) -> (u64, u64) {
    let instance = env.storage().instance();
    let count: u64 = instance.get(&DataKey::ProposalCount).unwrap_or(0) + 1;
    let voting_period: u64 = instance.get(&DataKey::VotingPeriod).unwrap_or(0);
    instance.set(&DataKey::ProposalCount, &count);
    (count, voting_period)
}

// ============================================================================
// Contract
// ============================================================================

/// Extended contract metadata for public disclosure
#[contracttype]
#[derive(Clone, Debug)]
pub struct PublicMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub repository: String,
    pub license: String,
}

/// Contract info combining metadata with runtime state
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractInfo {
    pub metadata: PublicMetadata,
    pub initialized: bool,
    pub admin: Option<Address>,
    pub total_proposals: u64,
}

#[contract]
pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    /// Initialize the governance contract with an admin, quorum, and voting period.
    pub fn initialize(
        env: Env,
        admin: Address,
        quorum: u64,
        voting_period: u64,
    ) -> Result<(), errors::Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(errors::Error::AlreadyInitialized);
        }

        let instance = env.storage().instance();
        instance.set(&DataKey::Admin, &admin);
        instance.set(&DataKey::ProposalCount, &0u64);
        instance.set(&DataKey::Quorum, &quorum);
        instance.set(&DataKey::VotingPeriod, &voting_period);
        instance.set(&DataKey::Version, &String::from_str(&env, VERSION));

        emit_governance_initialized(&env, admin, quorum, voting_period);

        Ok(())
    }

    pub fn get_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or_else(|| String::from_str(&env, VERSION))
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

        let (count, voting_period) = next_proposal(&env);
        let now = env.ledger().timestamp();
        let voting_ends_at = now + voting_period;

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
        save_proposal(&env, &proposal);

        env.storage()
            .persistent()
            .set(&DataKey::VoteTally(count), &EMPTY_TALLY);

        emit_proposal_created(&env, count, caller, target_contract, voting_ends_at);

        Ok(count)
    }

    /// Create a parameter-update proposal (e.g. set admin or paused on analytics). Only the admin can create.
    pub fn create_parameter_proposal(
        env: Env,
        caller: Address,
        title: String,
        target_contract: Address,
        action: ParameterAction,
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

        let (count, voting_period) = next_proposal(&env);
        let now = env.ledger().timestamp();
        let voting_ends_at = now + voting_period;

        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let proposal = Proposal {
            id: count,
            proposer: caller.clone(),
            title,
            target_contract: target_contract.clone(),
            new_wasm_hash: zero_hash,
            status: ProposalStatus::Active,
            created_at: now,
            voting_ends_at,
        };

        save_proposal(&env, &proposal);

        let persistent = env.storage().persistent();
        persistent.set(&DataKey::ParameterAction(count), &action);
        persistent.set(&DataKey::VoteTally(count), &EMPTY_TALLY);

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

        let proposal = load_proposal(&env, proposal_id)?;

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
        if voted(&env, proposal_id, &voter) {
            return Err(Error::AlreadyVoted);
        }

        // Record the vote (per-voter key: O(1) regardless of voter count)
        let persistent = env.storage().persistent();
        persistent.set(&DataKey::Vote(proposal_id, voter.clone()), &choice);

        // Update tally
        let mut tally: VoteTally = persistent
            .get(&DataKey::VoteTally(proposal_id))
            .unwrap_or(EMPTY_TALLY);

        match choice {
            VoteChoice::For => tally.votes_for += 1,
            VoteChoice::Against => tally.votes_against += 1,
            VoteChoice::Abstain => tally.votes_abstain += 1,
        }
        tally.total_voters += 1;

        persistent.set(&DataKey::VoteTally(proposal_id), &tally);

        let choice_val = choice as u32;
        emit_vote_cast(&env, proposal_id, voter, choice_val);

        Ok(())
    }

    /// Finalize a proposal after the voting period has ended.
    /// Anyone can call this function once the deadline passes.
    pub fn finalize(env: Env, proposal_id: u64) -> Result<ProposalStatus, Error> {
        let mut proposal = load_proposal(&env, proposal_id)?;

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
            .unwrap_or(EMPTY_TALLY);

        let quorum: u64 = env.storage().instance().get(&DataKey::Quorum).unwrap_or(0);

        // Determine outcome: passes if quorum met AND more for than against
        let new_status = if tally.total_voters >= quorum && tally.votes_for > tally.votes_against {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Failed
        };

        proposal.status = new_status;
        save_proposal(&env, &proposal);

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

    /// Mark a passed proposal as executed and apply it. Only the admin can call this.
    /// For parameter-update proposals, invokes the target contract (e.g. analytics) to apply the change.
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

        let mut proposal = load_proposal(&env, proposal_id)?;

        if proposal.status != ProposalStatus::Passed {
            return Err(Error::ProposalNotPassed);
        }

        if let Some(action) = env
            .storage()
            .persistent()
            .get(&DataKey::ParameterAction(proposal_id))
        {
            let governance = env.current_contract_address();
            let client = AnalyticsContractClient::new(&env, &proposal.target_contract);
            match action {
                ParameterAction::SetAdmin(addr) => {
                    let _ = client.set_admin_by_governance(&governance, &addr);
                }
                ParameterAction::SetPaused(p) => {
                    let _ = client.set_paused_by_governance(&governance, &p);
                }
            }
        }
        // Upgrade proposals: execution is off-chain (deploy new WASM); we only mark executed here.

        proposal.status = ProposalStatus::Executed;
        save_proposal(&env, &proposal);
        let target_contract = proposal.target_contract;

        emit_proposal_executed(&env, proposal_id, caller, target_contract);

        Ok(())
    }

    // ========================================================================
    // Query Functions
    // ========================================================================

    /// Get a proposal by ID.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<Proposal, Error> {
        load_proposal(&env, proposal_id)
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
        voted(&env, proposal_id, &voter)
    }

    /// Get the parameter action for a proposal (if it is a parameter-update proposal).
    pub fn get_parameter_action(env: Env, proposal_id: u64) -> Option<ParameterAction> {
        env.storage()
            .persistent()
            .get(&DataKey::ParameterAction(proposal_id))
    }

    /// Get contract configuration (admin, quorum, voting_period, proposal_count).
    pub fn get_config(env: Env) -> Result<(Address, u64, u64, u64), Error> {
        let instance = env.storage().instance();
        let admin: Address = instance.get(&DataKey::Admin).ok_or(Error::AdminNotSet)?;
        let quorum: u64 = instance.get(&DataKey::Quorum).unwrap_or(0);
        let voting_period: u64 = instance.get(&DataKey::VotingPeriod).unwrap_or(0);
        let proposal_count: u64 = instance.get(&DataKey::ProposalCount).unwrap_or(0);

        Ok((admin, quorum, voting_period, proposal_count))
    }

    pub fn getversion(env: Env) -> String {
        String::from_str(&env, VERSION)
    }

    // =========================================================================
    // Contract Metadata
    // =========================================================================

    /// Get public contract metadata
    pub fn get_metadata(env: Env) -> PublicMetadata {
        PublicMetadata {
            name: String::from_str(&env, "Stellar Insights Governance"),
            version: String::from_str(&env, VERSION),
            author: String::from_str(&env, "Stellar Insights Team"),
            description: String::from_str(
                &env,
                "Decentralized governance and voting contract for Stellar Insights",
            ),
            repository: String::from_str(&env, "https://github.com/stellar-insights/contracts"),
            license: String::from_str(&env, "MIT"),
        }
    }

    /// Get comprehensive contract information
    pub fn get_contract_info(env: Env) -> ContractInfo {
        ContractInfo {
            metadata: Self::get_metadata(env.clone()),
            initialized: env.storage().instance().has(&DataKey::Admin),
            admin: env.storage().instance().get(&DataKey::Admin),
            total_proposals: env
                .storage()
                .instance()
                .get(&DataKey::ProposalCount)
                .unwrap_or(0),
        }
    }
}

mod test;
