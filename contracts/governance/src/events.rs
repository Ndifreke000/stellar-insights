use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

// ============================================================================
// Event Topics - Short symbols for efficient on-chain storage
// ============================================================================

/// Topic for proposal creation events
pub const PROPOSAL_CREATED: Symbol = symbol_short!("PROP_CRT");

/// Topic for vote cast events
pub const VOTE_CAST: Symbol = symbol_short!("VOTE_CST");

/// Topic for proposal finalized events
pub const PROP_FINALIZED: Symbol = symbol_short!("PROP_FIN");

/// Topic for governance lifecycle events (for filtering)
pub const GOV_LIFECYCLE: Symbol = symbol_short!("GOV_LFE");

// ============================================================================
// Event Structures
// ============================================================================

/// Event emitted when a new proposal is created.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalCreated {
    pub proposal_id: u64,
    pub proposer: Address,
    pub target_contract: Address,
    pub voting_ends_at: u64,
}

impl ProposalCreated {
    pub fn publish(
        env: &Env,
        proposal_id: u64,
        proposer: Address,
        target_contract: Address,
        voting_ends_at: u64,
    ) {
        let event = ProposalCreated {
            proposal_id,
            proposer,
            target_contract,
            voting_ends_at,
        };
        env.events()
            .publish((PROPOSAL_CREATED, GOV_LIFECYCLE), event);
    }
}

/// Event emitted when a vote is cast on a proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteCastEvent {
    pub proposal_id: u64,
    pub voter: Address,
    pub choice: u32,
}

impl VoteCastEvent {
    pub fn publish(env: &Env, proposal_id: u64, voter: Address, choice: u32) {
        let event = VoteCastEvent {
            proposal_id,
            voter,
            choice,
        };
        env.events().publish((VOTE_CAST, GOV_LIFECYCLE), event);
    }
}

/// Event emitted when a proposal is finalized.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalFinalizedEvent {
    pub proposal_id: u64,
    pub status: u32,
    pub votes_for: u64,
    pub votes_against: u64,
    pub total_voters: u64,
}

impl ProposalFinalizedEvent {
    pub fn publish(
        env: &Env,
        proposal_id: u64,
        status: u32,
        votes_for: u64,
        votes_against: u64,
        total_voters: u64,
    ) {
        let event = ProposalFinalizedEvent {
            proposal_id,
            status,
            votes_for,
            votes_against,
            total_voters,
        };
        env.events().publish((PROP_FINALIZED, GOV_LIFECYCLE), event);
    }
}

// ============================================================================
// Event Helper Functions
// ============================================================================

pub fn emit_proposal_created(
    env: &Env,
    proposal_id: u64,
    proposer: Address,
    target_contract: Address,
    voting_ends_at: u64,
) {
    ProposalCreated::publish(env, proposal_id, proposer, target_contract, voting_ends_at);
}

pub fn emit_vote_cast(env: &Env, proposal_id: u64, voter: Address, choice: u32) {
    VoteCastEvent::publish(env, proposal_id, voter, choice);
}

pub fn emit_proposal_finalized(
    env: &Env,
    proposal_id: u64,
    status: u32,
    votes_for: u64,
    votes_against: u64,
    total_voters: u64,
) {
    ProposalFinalizedEvent::publish(
        env,
        proposal_id,
        status,
        votes_for,
        votes_against,
        total_voters,
    );
}
