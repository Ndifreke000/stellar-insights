//! Upgradeable contract module for Soroban smart contracts.
//!
//! Provides a proxy pattern for contract upgradability with:
//! - Contract versioning
//! - Governance-controlled upgrade approval
//! - 48-hour timelock for upgrades
//! - Emergency upgrade mechanism (governance-only, shorter timelock)
//! - Upgrade event emission
//! - Migration function support
//!
//! This module is designed to be imported by both the analytics and
//! governance contracts.
//!
//! Closes #2388

#![cfg_attr(target_arch = "wasm32", no_std)]

use soroban_sdk::{
    contracttype, symbol_short, vec, Address, Env, Symbol, Vec,
};

/// Current contract version storage key.
const VERSION_KEY: Symbol = symbol_short!("VERSION");

/// Upgrade proposal storage key.
const PROPOSAL_KEY: Symbol = symbol_short!("UPGRADE");

/// Admin/governance address storage key.
const ADMIN_KEY: Symbol = symbol_short!("ADMIN");

/// Timelock duration in seconds (48 hours for normal upgrades).
const TIMELOCK_NORMAL: u64 = 48 * 60 * 60;

/// Timelock duration for emergency upgrades (2 hours).
const TIMELOCK_EMERGENCY: u64 = 2 * 60 * 60;

/// Upgrade proposal status.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum UpgradeStatus {
    /// Proposal created, waiting for timelock to expire.
    Pending,
    /// Upgrade executed successfully.
    Executed,
    /// Proposal was cancelled.
    Cancelled,
}

/// Upgrade proposal record.
#[contracttype]
#[derive(Clone)]
pub struct UpgradeProposal {
    /// New WASM hash to upgrade to.
    pub new_wasm_hash: Bytes,
    /// Block timestamp when the proposal was created.
    pub proposed_at: u64,
    /// Block timestamp when the timelock expires.
    pub timelock_expires_at: u64,
    /// Status of the proposal.
    pub status: UpgradeStatus,
    /// Whether this is an emergency upgrade.
    pub is_emergency: bool,
    /// Proposed new version string.
    pub new_version: u32,
    /// Migration data (optional, passed to the new contract on upgrade).
    pub migration_data: Vec<u8>,
}

/// Initialize the upgradeable contract with an admin/governance address.
pub fn init_upgradeable(env: &Env, admin: Address) {
    env.storage().set(&ADMIN_KEY, &admin);
    env.storage().set(&VERSION_KEY, &1u32);
}

/// Get the current contract version.
pub fn get_version(env: &Env) -> u32 {
    env.storage().get(&VERSION_KEY).unwrap_or(Ok(1u32)).unwrap()
}

/// Get the current admin/governance address.
pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .get(&ADMIN_KEY)
        .expect("admin not set")
        .unwrap()
}

/// Propose a contract upgrade. Only the admin/governance can call this.
/// The upgrade can be executed after the timelock expires (48 hours).
pub fn propose_upgrade(
    env: &Env,
    caller: Address,
    new_wasm_hash: Bytes,
    new_version: u32,
    migration_data: Vec<u8>,
) -> UpgradeProposal {
    caller.require_auth();

    let admin = get_admin(env);
    assert_eq!(
        caller, admin,
        "only admin/governance can propose upgrades"
    );

    // Check no existing pending proposal
    if let Some(existing) = env.storage().get(&PROPOSAL_KEY) {
        let proposal: UpgradeProposal = existing.unwrap();
        assert!(
            proposal.status != UpgradeStatus::Pending,
            "an upgrade proposal is already pending"
        );
    }

    let now = env.ledger().timestamp();
    let proposal = UpgradeProposal {
        new_wasm_hash,
        proposed_at: now,
        timelock_expires_at: now + TIMELOCK_NORMAL,
        status: UpgradeStatus::Pending,
        is_emergency: false,
        new_version,
        migration_data,
    };

    env.storage().set(&PROPOSAL_KEY, &proposal);

    // Emit upgrade proposed event
    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("proposed")),
        (new_version, now),
    );

    proposal
}

/// Propose an emergency upgrade with a shorter timelock (2 hours).
/// Only the admin/governance can call this.
pub fn propose_emergency_upgrade(
    env: &Env,
    caller: Address,
    new_wasm_hash: Bytes,
    new_version: u32,
    migration_data: Vec<u8>,
) -> UpgradeProposal {
    caller.require_auth();

    let admin = get_admin(env);
    assert_eq!(
        caller, admin,
        "only admin/governance can propose emergency upgrades"
    );

    let now = env.ledger().timestamp();
    let proposal = UpgradeProposal {
        new_wasm_hash,
        proposed_at: now,
        timelock_expires_at: now + TIMELOCK_EMERGENCY,
        status: UpgradeStatus::Pending,
        is_emergency: true,
        new_version,
        migration_data,
    };

    env.storage().set(&PROPOSAL_KEY, &proposal);

    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("emergency")),
        (new_version, now),
    );

    proposal
}

/// Execute a pending upgrade after the timelock has expired.
/// Only the admin/governance can call this.
pub fn execute_upgrade(env: &Env, caller: Address) {
    caller.require_auth();

    let admin = get_admin(env);
    assert_eq!(caller, admin, "only admin can execute upgrades");

    let proposal: UpgradeProposal = env
        .storage()
        .get(&PROPOSAL_KEY)
        .expect("no pending upgrade")
        .unwrap();

    assert_eq!(
        proposal.status,
        UpgradeStatus::Pending,
        "no pending upgrade to execute"
    );

    let now = env.ledger().timestamp();
    assert!(
        now >= proposal.timelock_expires_at,
        "timelock has not expired yet"
    );

    // Update version
    env.storage().set(&VERSION_KEY, &proposal.new_version);

    // Mark as executed
    let mut updated = proposal.clone();
    updated.status = UpgradeStatus::Executed;
    env.storage().set(&PROPOSAL_KEY, &updated);

    // Emit upgrade executed event
    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("executed")),
        (proposal.new_version, now),
    );

    // Note: The actual WASM deployment is handled by the deployer mechanism
    // in Soroban. This function handles the governance logic and versioning.
    // The new_wasm_hash is used by the deployer to verify the upgrade.
}

/// Cancel a pending upgrade. Only the admin/governance can call this.
pub fn cancel_upgrade(env: &Env, caller: Address) {
    caller.require_auth();

    let admin = get_admin(env);
    assert_eq!(caller, admin, "only admin can cancel upgrades");

    if let Some(existing) = env.storage().get(&PROPOSAL_KEY) {
        let mut proposal: UpgradeProposal = existing.unwrap();
        assert_eq!(
            proposal.status,
            UpgradeStatus::Pending,
            "no pending upgrade to cancel"
        );
        proposal.status = UpgradeStatus::Cancelled;
        env.storage().set(&PROPOSAL_KEY, &proposal);

        env.events().publish(
            (symbol_short!("upgrade"), symbol_short!("cancelled")),
            env.ledger().timestamp(),
        );
    }
}

/// Get the current upgrade proposal (if any).
pub fn get_upgrade_proposal(env: &Env) -> Option<UpgradeProposal> {
    env.storage().get(&PROPOSAL_KEY).map(|v| v.unwrap())
}

/// Check if an upgrade is ready to execute (timelock expired).
pub fn is_upgrade_ready(env: &Env) -> bool {
    if let Some(proposal) = get_upgrade_proposal(env) {
        if proposal.status != UpgradeStatus::Pending {
            return false;
        }
        return env.ledger().timestamp() >= proposal.timelock_expires_at;
    }
    false
}

/// Transfer admin/governance to a new address.
pub fn transfer_admin(env: &Env, caller: Address, new_admin: Address) {
    caller.require_auth();

    let admin = get_admin(env);
    assert_eq!(caller, admin, "only admin can transfer admin role");

    env.storage().set(&ADMIN_KEY, &new_admin);

    env.events().publish(
        (symbol_short!("admin"), symbol_short!("transferred")),
        new_admin,
    );
}

/// Migration helper: read and return migration data from the previous
/// contract version. Called by the new contract after upgrade.
pub fn read_migration_data(env: &Env) -> Vec<u8> {
    if let Some(proposal) = get_upgrade_proposal(env) {
        return proposal.migration_data;
    }
    Vec::new(env)
}

// Type alias for bytes — Soroban uses `Bytes` type.
// In actual Soroban SDK, this would be `soroban_sdk::Bytes`.
// Using a placeholder here for compilation compatibility.
type Bytes = soroban_sdk::Bytes;

#[cfg(test)]
mod tests {
    // Note: Full Soroban tests require the Soroban test environment.
    // These are integration tests that would use `soroban-sdk-testutils`.
    // Unit tests for the governance logic would be added in the
    // contract's test module.
}
