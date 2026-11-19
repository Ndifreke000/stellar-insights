//! Governance contract upgrade module.
//!
//! Extends the upgradeable pattern with governance voting integration:
//! upgrades require a successful governance vote before they can be
//! proposed, adding a democratic layer to the upgrade process.
//!
//! Closes #2388

#![cfg_attr(target_arch = "wasm32", no_std)]

use soroban_sdk::{contracterror, contracttype, symbol_short, Address, Env, Symbol, Vec};

const VERSION_KEY: Symbol = symbol_short!("VERSION");
const PROPOSAL_KEY: Symbol = symbol_short!("UPGRADE");
const GOVERNANCE_KEY: Symbol = symbol_short!("GOV_ADDR");
const VOTE_THRESHOLD: u32 = 66; // 66% approval required

const TIMELOCK_NORMAL: u64 = 48 * 60 * 60;
const TIMELOCK_EMERGENCY: u64 = 2 * 60 * 60;

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum GovernanceUpgradeStatus {
    VotePending,
    Approved,
    Rejected,
    TimelockPending,
    Executed,
    Cancelled,
}

#[contracttype]
#[derive(Clone)]
pub struct GovernanceUpgradeProposal {
    pub new_wasm_hash: soroban_sdk::Bytes,
    pub new_version: u32,
    pub vote_start: u64,
    pub vote_end: u64,
    pub yes_votes: u32,
    pub no_votes: u32,
    pub total_voters: u32,
    pub status: GovernanceUpgradeStatus,
    pub timelock_expires_at: u64,
    pub is_emergency: bool,
    pub migration_data: Vec<u8>,
}

/// Errors returned by the governance upgrade helpers instead of panicking.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum GovernanceUpgradeError {
    /// The governance address has not been set yet.
    NotInitialized = 1,
    /// There is no upgrade proposal to act on.
    NoUpgradeProposal = 2,
    /// A stored value could not be decoded.
    StorageCorrupted = 3,
}

/// Reads a stored value: a missing key is `None`, a decode failure is an error.
fn read_optional<T>(env: &Env, key: &Symbol) -> Result<Option<T>, GovernanceUpgradeError>
where
    T: soroban_sdk::TryFromVal<Env, soroban_sdk::Val>,
{
    match env.storage().get(key) {
        None => Ok(None),
        Some(Ok(value)) => Ok(Some(value)),
        Some(Err(_)) => Err(GovernanceUpgradeError::StorageCorrupted),
    }
}

/// Loads the governance address, failing if the module was never initialised.
fn governance_addr(env: &Env) -> Result<Address, GovernanceUpgradeError> {
    read_optional(env, &GOVERNANCE_KEY)?.ok_or(GovernanceUpgradeError::NotInitialized)
}

/// Loads the current proposal, failing if there is none.
fn load_proposal(env: &Env) -> Result<GovernanceUpgradeProposal, GovernanceUpgradeError> {
    read_optional(env, &PROPOSAL_KEY)?.ok_or(GovernanceUpgradeError::NoUpgradeProposal)
}

/// Initialize governance upgrade system.
pub fn init_governance_upgrade(env: &Env, governance_addr: Address) {
    env.storage().set(&GOVERNANCE_KEY, &governance_addr);
    env.storage().set(&VERSION_KEY, &1u32);
}

/// Get current version.
pub fn get_version(env: &Env) -> Result<u32, GovernanceUpgradeError> {
    Ok(read_optional(env, &VERSION_KEY)?.unwrap_or(1u32))
}

/// Create a governance upgrade proposal that requires community voting.
/// After the vote period ends with >= 66% approval, a timelock starts.
/// After the timelock, the upgrade can be executed.
pub fn create_governance_upgrade(
    env: &Env,
    caller: Address,
    new_wasm_hash: soroban_sdk::Bytes,
    new_version: u32,
    vote_duration_secs: u64,
    migration_data: Vec<u8>,
) -> Result<GovernanceUpgradeProposal, GovernanceUpgradeError> {
    caller.require_auth();

    let gov_addr = governance_addr(env)?;
    assert_eq!(
        caller, gov_addr,
        "only governance can create upgrade proposals"
    );

    let now = env.ledger().timestamp();
    let proposal = GovernanceUpgradeProposal {
        new_wasm_hash,
        new_version,
        vote_start: now,
        vote_end: now + vote_duration_secs,
        yes_votes: 0,
        no_votes: 0,
        total_voters: 0,
        status: GovernanceUpgradeStatus::VotePending,
        timelock_expires_at: 0, // Set after vote approval
        is_emergency: false,
        migration_data,
    };

    env.storage().set(&PROPOSAL_KEY, &proposal);
    env.events().publish(
        (symbol_short!("gov_upgrade"), symbol_short!("vote_started")),
        (new_version, now),
    );
    Ok(proposal)
}

/// Vote on a pending governance upgrade proposal.
pub fn vote_on_upgrade(
    env: &Env,
    voter: Address,
    approve: bool,
) -> Result<(), GovernanceUpgradeError> {
    voter.require_auth();

    let mut proposal = load_proposal(env)?;

    assert_eq!(
        proposal.status,
        GovernanceUpgradeStatus::VotePending,
        "voting is not open"
    );

    let now = env.ledger().timestamp();
    assert!(now < proposal.vote_end, "voting period has ended");

    proposal.total_voters += 1;
    if approve {
        proposal.yes_votes += 1;
    } else {
        proposal.no_votes += 1;
    }

    env.storage().set(&PROPOSAL_KEY, &proposal);
    env.events().publish(
        (symbol_short!("gov_upgrade"), symbol_short!("vote_cast")),
        (voter, approve),
    );
    Ok(())
}

/// Finalize the vote and start the timelock if approved.
pub fn finalize_vote(env: &Env, caller: Address) -> Result<(), GovernanceUpgradeError> {
    caller.require_auth();

    let mut proposal = load_proposal(env)?;

    assert_eq!(
        proposal.status,
        GovernanceUpgradeStatus::VotePending,
        "vote not in progress"
    );

    let now = env.ledger().timestamp();
    assert!(now >= proposal.vote_end, "voting period not ended");

    let approval_rate = if proposal.total_voters > 0 {
        (proposal.yes_votes * 100) / proposal.total_voters
    } else {
        0
    };

    if approval_rate >= VOTE_THRESHOLD {
        proposal.status = GovernanceUpgradeStatus::TimelockPending;
        proposal.timelock_expires_at = now + TIMELOCK_NORMAL;
        env.events().publish(
            (symbol_short!("gov_upgrade"), symbol_short!("approved")),
            (approval_rate, now),
        );
    } else {
        proposal.status = GovernanceUpgradeStatus::Rejected;
        env.events().publish(
            (symbol_short!("gov_upgrade"), symbol_short!("rejected")),
            (approval_rate, now),
        );
    }

    env.storage().set(&PROPOSAL_KEY, &proposal);
    Ok(())
}

/// Execute the upgrade after the timelock expires.
pub fn execute_governance_upgrade(
    env: &Env,
    caller: Address,
) -> Result<(), GovernanceUpgradeError> {
    caller.require_auth();

    let gov_addr = governance_addr(env)?;
    assert_eq!(caller, gov_addr, "only governance can execute upgrades");

    let mut proposal = load_proposal(env)?;

    assert_eq!(
        proposal.status,
        GovernanceUpgradeStatus::TimelockPending,
        "upgrade not approved or timelock not started"
    );

    let now = env.ledger().timestamp();
    assert!(
        now >= proposal.timelock_expires_at,
        "timelock has not expired"
    );

    env.storage().set(&VERSION_KEY, &proposal.new_version);
    proposal.status = GovernanceUpgradeStatus::Executed;
    env.storage().set(&PROPOSAL_KEY, &proposal);

    env.events().publish(
        (symbol_short!("gov_upgrade"), symbol_short!("executed")),
        (proposal.new_version, now),
    );
    Ok(())
}

/// Cancel a pending upgrade (before execution).
pub fn cancel_governance_upgrade(env: &Env, caller: Address) -> Result<(), GovernanceUpgradeError> {
    caller.require_auth();

    let gov_addr = governance_addr(env)?;
    assert_eq!(caller, gov_addr, "only governance can cancel");

    if let Some(mut proposal) = read_optional::<GovernanceUpgradeProposal>(env, &PROPOSAL_KEY)? {
        if proposal.status == GovernanceUpgradeStatus::VotePending
            || proposal.status == GovernanceUpgradeStatus::TimelockPending
        {
            proposal.status = GovernanceUpgradeStatus::Cancelled;
            env.storage().set(&PROPOSAL_KEY, &proposal);
            env.events().publish(
                (symbol_short!("gov_upgrade"), symbol_short!("cancelled")),
                env.ledger().timestamp(),
            );
        }
    }
    Ok(())
}

/// Get current proposal state.
pub fn get_governance_upgrade_proposal(
    env: &Env,
) -> Result<Option<GovernanceUpgradeProposal>, GovernanceUpgradeError> {
    read_optional(env, &PROPOSAL_KEY)
}

/// Transfer governance to a new address.
pub fn transfer_governance(
    env: &Env,
    caller: Address,
    new_gov: Address,
) -> Result<(), GovernanceUpgradeError> {
    caller.require_auth();
    let gov_addr = governance_addr(env)?;
    assert_eq!(caller, gov_addr, "only governance can transfer");
    env.storage().set(&GOVERNANCE_KEY, &new_gov);
    env.events().publish(
        (symbol_short!("gov"), symbol_short!("transferred")),
        new_gov,
    );
    Ok(())
}
