#![no_std]
extern crate std;

mod errors;
mod events;

use analytics::AnalyticsContractClient;
use errors::Error;
use events::{
    emit_governance_initialized, emit_governance_param_changed, emit_governance_admin_changed,
    emit_parameter_proposal_created, emit_proposal_created,
    emit_proposal_executed, emit_proposal_finalized, emit_vote_cast,
};
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, String};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ~30 days at 5 s/ledger
const LEDGERS_TO_EXTEND: u32 = 518_400;
const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;
/// ~7 days — TTL for vote records after a proposal is finalized.
const VOTE_CLEANUP_TTL: u32 = 120_960;

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
    /// Per-proposal storage — replaces the unbounded Proposals map.
    Proposal(u64),
    /// Per-voter storage — replaces the unbounded Votes(id) map.
    Vote(u64, Address),
    VoteTally(u64),
    /// Parameter-update action for a proposal (when present, proposal is parameter type).
    ParameterAction(u64),
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

        // Store proposal under its own key — avoids re-serializing the entire map on every write.
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(count), &proposal);
        env.storage().persistent().extend_ttl(
            &DataKey::Proposal(count),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

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
        env.storage().persistent().extend_ttl(
            &DataKey::VoteTally(count),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

        // Update proposal count
        env.storage()
            .instance()
            .set(&DataKey::ProposalCount, &count);
        bump_instance(&env);

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

        env.storage()
            .persistent()
            .set(&DataKey::Proposal(count), &proposal);
        env.storage().persistent().extend_ttl(
            &DataKey::Proposal(count),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

        env.storage()
            .persistent()
            .set(&DataKey::ParameterAction(count), &action);
        env.storage().persistent().extend_ttl(
            &DataKey::ParameterAction(count),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

        let tally = VoteTally {
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            total_voters: 0,
        };
        env.storage()
            .persistent()
            .set(&DataKey::VoteTally(count), &tally);
        env.storage().persistent().extend_ttl(
            &DataKey::VoteTally(count),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

        env.storage()
            .instance()
            .set(&DataKey::ProposalCount, &count);
        bump_instance(&env);

        let action_label = match &action {
            ParameterAction::SetAdmin(_) => String::from_str(&env, "set_admin"),
            ParameterAction::SetPaused(_) => String::from_str(&env, "set_paused"),
        };
        emit_parameter_proposal_created(&env, count, caller, target_contract, voting_ends_at, action_label);

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
        let proposal: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;

        // Check proposal is still active
        if proposal.status != ProposalStatus::Active {
            return Err(Error::VotingNotActive);
        }

        // Check voting period has not ended
        let now = env.ledger().timestamp();
        if now >= proposal.voting_ends_at {
            return Err(Error::VotingNotActive);
        }

        // Check voter has not already voted — O(1) key lookup, no map load.
        let vote_key = DataKey::Vote(proposal_id, voter.clone());
        if env.storage().persistent().has(&vote_key) {
            return Err(Error::AlreadyVoted);
        }

        // Record the vote as a single persistent entry.
        env.storage().persistent().set(&vote_key, &choice);
        env.storage().persistent().extend_ttl(
            &vote_key,
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

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
        env.storage().persistent().extend_ttl(
            &DataKey::VoteTally(proposal_id),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

        let choice_val = choice as u32;
        emit_vote_cast(&env, proposal_id, voter, choice_val);

        Ok(())
    }

    /// Finalize a proposal after the voting period has ended.
    /// Anyone can call this function once the deadline passes.
    pub fn finalize(env: Env, proposal_id: u64) -> Result<ProposalStatus, Error> {
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;

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

        let quorum: u64 = env.storage().instance().get(&DataKey::Quorum).unwrap_or(0);

        // Determine outcome: passes if quorum met AND more for than against
        let new_status = if tally.total_voters >= quorum && tally.votes_for > tally.votes_against {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Failed
        };

        proposal.status = new_status.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        env.storage().persistent().extend_ttl(
            &DataKey::Proposal(proposal_id),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

        // Tally is no longer needed for voting; shorten its TTL to 7 days.
        env.storage().persistent().extend_ttl(
            &DataKey::VoteTally(proposal_id),
            VOTE_CLEANUP_TTL,
            VOTE_CLEANUP_TTL,
        );

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

        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;

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
        } else {
            // Upgrade proposal: invoke upgrade on the target contract with the approved wasm hash.
            let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
            if proposal.new_wasm_hash != zero_hash {
                let client = AnalyticsContractClient::new(&env, &proposal.target_contract);
                let _ = client.upgrade(&proposal.new_wasm_hash);
            }
        }

        proposal.status = ProposalStatus::Executed;
        let target_contract = proposal.target_contract.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        env.storage().persistent().extend_ttl(
            &DataKey::Proposal(proposal_id),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        bump_instance(&env);

        emit_proposal_executed(&env, proposal_id, caller, target_contract);

        Ok(())
    }

    // ========================================================================
    // Admin Parameter Update Functions
    // ========================================================================

    /// Update the quorum threshold. Admin-only.
    pub fn update_quorum(env: Env, caller: Address, new_quorum: u64) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        if caller != admin {
            return Err(Error::UnauthorizedCaller);
        }
        let old_quorum: u64 = env.storage().instance().get(&DataKey::Quorum).unwrap_or(0);
        env.storage().instance().set(&DataKey::Quorum, &new_quorum);
        bump_instance(&env);
        emit_governance_param_changed(
            &env,
            String::from_str(&env, "quorum"),
            old_quorum,
            new_quorum,
            caller,
        );
        Ok(())
    }

    /// Update the voting period. Admin-only.
    pub fn update_voting_period(env: Env, caller: Address, new_period: u64) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        if caller != admin {
            return Err(Error::UnauthorizedCaller);
        }
        let old_period: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VotingPeriod)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::VotingPeriod, &new_period);
        bump_instance(&env);
        emit_governance_param_changed(
            &env,
            String::from_str(&env, "voting_period"),
            old_period,
            new_period,
            caller,
        );
        Ok(())
    }

    /// Transfer governance admin to a new address. Admin-only.
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
        caller.require_auth();
        let old_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        if caller != old_admin {
            return Err(Error::UnauthorizedCaller);
        }
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        bump_instance(&env);
        emit_governance_admin_changed(&env, old_admin, new_admin, caller);
        Ok(())
    }

    // ========================================================================
    // Query Functions
    // ========================================================================

    /// Get a proposal by ID.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<Proposal, Error> {
        let proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;
        env.storage().persistent().extend_ttl(
            &DataKey::Proposal(proposal_id),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        Ok(proposal)
    }

    /// Get the vote tally for a proposal.
    pub fn get_tally(env: Env, proposal_id: u64) -> Result<VoteTally, Error> {
        let tally = env
            .storage()
            .persistent()
            .get(&DataKey::VoteTally(proposal_id))
            .ok_or(Error::ProposalNotFound)?;
        env.storage().persistent().extend_ttl(
            &DataKey::VoteTally(proposal_id),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        Ok(tally)
    }

    /// Check if an address has voted on a proposal — O(1) key lookup.
    pub fn has_voted(env: Env, proposal_id: u64, voter: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Vote(proposal_id, voter))
    }

    /// Remove vote records and tally for a finalized/executed proposal to reclaim storage rent.
    /// Caller provides the list of voters whose records should be removed.
    /// Only callable after the proposal is no longer Active.
    pub fn cleanup_proposal(
        env: Env,
        proposal_id: u64,
        voters: soroban_sdk::Vec<Address>,
    ) -> Result<u32, Error> {
        let proposal: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;

        if proposal.status == ProposalStatus::Active {
            return Err(Error::VotingNotActive);
        }

        let mut removed: u32 = 0;
        for voter in voters.iter() {
            let key = DataKey::Vote(proposal_id, voter);
            if env.storage().persistent().get::<DataKey, VoteChoice>(&key).is_some() {
                env.storage().persistent().remove(&key);
                removed += 1;
            }
        }

        // Remove tally once votes are cleaned up
        if env
            .storage()
            .persistent()
            .get::<DataKey, VoteTally>(&DataKey::VoteTally(proposal_id))
            .is_some()
        {
            env.storage().persistent().remove(&DataKey::VoteTally(proposal_id));
        }

        Ok(removed)
    }

    /// Get the parameter action for a proposal (if it is a parameter-update proposal).
    pub fn get_parameter_action(env: Env, proposal_id: u64) -> Option<ParameterAction> {
        env.storage()
            .persistent()
            .get(&DataKey::ParameterAction(proposal_id))
    }

    /// Get contract configuration (admin, quorum, voting_period, proposal_count).
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

        let proposal_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);

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

#[cfg(test)]
mod fuzz_tests;
