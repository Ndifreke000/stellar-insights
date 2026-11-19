use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, Symbol};

use crate::RotationReason;

// ============================================================================
// Event Topics - Short symbols for efficient on-chain storage
// ============================================================================

/// Topic for snapshot submission events
pub const SNAPSHOT_SUBMITTED: Symbol = symbol_short!("SNAP_SUB");

/// Topic for snapshot lifecycle events (for filtering)
pub const SNAPSHOT_LIFECYCLE: Symbol = symbol_short!("SNAP_LFE");

/// Topic for contract lifecycle events (init, pause, unpause)
pub const CONTRACT_LIFECYCLE: Symbol = symbol_short!("CTR_LFE");

// ============================================================================
// Event Structures
// ============================================================================

/// Event emitted when an analytics snapshot is successfully submitted.
///
/// This event enables off-chain indexing services to:
/// - Track all snapshot submissions in real-time
/// - Verify data integrity by comparing stored hash with off-chain data
/// - Build historical records of analytics epochs
///
/// # Fields
/// - `hash`: The 32-byte SHA-256 hash of the analytics snapshot data
/// - `epoch`: The epoch identifier for this snapshot (positive integer)
/// - `timestamp`: Ledger timestamp when the snapshot was recorded on-chain
/// - `submitter`: Address of the admin who submitted the snapshot
/// - `chain_hash`: `sha256(prev_chain_hash || epoch_be || hash)` (#2782)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotSubmitted {
    /// SHA-256 hash of the analytics snapshot data
    pub hash: BytesN<32>,
    /// Epoch identifier for this snapshot
    pub epoch: u64,
    /// Ledger timestamp when the snapshot was recorded
    pub timestamp: u64,
    /// Address of the admin who submitted the snapshot
    pub submitter: Address,
    /// Rolling hash-chain commitment over every epoch up to this one (#2782)
    pub chain_hash: BytesN<32>,
}

impl SnapshotSubmitted {
    /// Create and publish a SnapshotSubmitted event
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `hash` - 32-byte SHA-256 hash of the snapshot
    /// * `epoch` - Epoch identifier
    /// * `timestamp` - Ledger timestamp
    /// * `submitter` - Address of the submitter
    ///
    /// # Event Format
    /// Topic: (SNAPSHOT_SUBMITTED, SNAPSHOT_LIFECYCLE)
    /// Data: SnapshotSubmitted struct containing hash, epoch, timestamp, submitter
    pub fn publish(
        env: &Env,
        hash: BytesN<32>,
        epoch: u64,
        timestamp: u64,
        submitter: Address,
        chain_hash: BytesN<32>,
    ) {
        let event = SnapshotSubmitted {
            hash,
            epoch,
            timestamp,
            submitter,
            chain_hash,
        };

        // Publish with multiple topics for flexible filtering
        // Indexers can filter by SNAPSHOT_SUBMITTED or SNAPSHOT_LIFECYCLE
        env.events()
            .publish((SNAPSHOT_SUBMITTED, SNAPSHOT_LIFECYCLE), event);
    }
}

/// Legacy event structure for backwards compatibility
///
/// This event emitted when an analytics snapshot is successfully submitted.
/// Deprecated: Use `SnapshotSubmitted` for new implementations.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyticsSnapshotSubmitted {
    /// Epoch identifier for this snapshot
    pub epoch: u64,
    /// SHA-256 hash of the analytics snapshot
    pub hash: BytesN<32>,
    /// Ledger timestamp when the snapshot was recorded
    pub timestamp: u64,
}

impl AnalyticsSnapshotSubmitted {
    /// Publish this event to the blockchain (legacy format)
    ///
    /// # Deprecated
    /// Use `SnapshotSubmitted::publish()` instead for new implementations.
    pub fn publish(env: &Env, epoch: u64, hash: BytesN<32>, timestamp: u64) {
        let event = AnalyticsSnapshotSubmitted {
            epoch,
            hash: hash.clone(),
            timestamp,
        };

        env.events().publish((SNAPSHOT_SUBMITTED,), event);
    }
}

// ============================================================================
// Event Helper Functions
// ============================================================================

/// Emit a snapshot submitted event with full payload
///
/// This is the primary function for emitting snapshot lifecycle events.
/// It ensures the event payload matches the stored data exactly.
///
/// # Arguments
/// * `env` - Contract environment
/// * `hash` - The exact hash that was stored
/// * `epoch` - The exact epoch that was stored
/// * `timestamp` - The exact timestamp that was stored
/// * `submitter` - The address of the caller who submitted
/// * `chain_hash` - The chain commitment stored with this epoch
pub fn emit_snapshot_submitted(
    env: &Env,
    hash: BytesN<32>,
    epoch: u64,
    timestamp: u64,
    submitter: Address,
    chain_hash: BytesN<32>,
) {
    SnapshotSubmitted::publish(env, hash, epoch, timestamp, submitter, chain_hash);
}

/// Emit an event when the contract is initialized.
pub fn emit_contract_initialized(env: &Env, admin: Address) {
    env.events()
        .publish((symbol_short!("init"), CONTRACT_LIFECYCLE), admin);
}

/// Emit an event when the contract is paused.
pub fn emit_contract_paused(env: &Env, caller: Address) {
    env.events()
        .publish((symbol_short!("paused"), CONTRACT_LIFECYCLE), caller);
}

/// Emit an event when the contract is unpaused.
pub fn emit_contract_unpaused(env: &Env, caller: Address) {
    env.events()
        .publish((symbol_short!("unpaused"), CONTRACT_LIFECYCLE), caller);
}

// ============================================================================
// Admin Transfer Event
// ============================================================================

/// Event emitted when the admin address is transferred to a new owner.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferredEvent {
    pub old_admin: Address,
    pub new_admin: Address,
    pub timestamp: u64,
    pub ledger_sequence: u32,
}

/// Emit an event when admin ownership is transferred.
pub fn emit_admin_transferred(env: &Env, old_admin: Address, new_admin: Address) {
    let event = AdminTransferredEvent {
        old_admin,
        new_admin,
        timestamp: env.ledger().timestamp(),
        ledger_sequence: env.ledger().sequence(),
    };
    env.events()
        .publish((symbol_short!("admin"), CONTRACT_LIFECYCLE), event);
}

// ============================================================================
// Submitter Set Events (#2600)
// ============================================================================

/// Topic for changes to the authorised snapshot-submitter set
pub const SUBMITTER_SET: Symbol = symbol_short!("SUBM_SET");

/// Emit an event when an address is added to the submitter set.
///
/// Topic: `("subm_add", SUBM_SET)`; data: `(admin, submitter)`.
pub fn emit_submitter_added(env: &Env, admin: Address, submitter: Address) {
    env.events().publish(
        (symbol_short!("subm_add"), SUBMITTER_SET),
        (admin, submitter),
    );
}

/// Emit an event when an address is removed from the submitter set.
///
/// Topic: `("subm_rm", SUBM_SET)`; data: `(admin, submitter)`.
pub fn emit_submitter_removed(env: &Env, admin: Address, submitter: Address) {
    env.events().publish(
        (symbol_short!("subm_rm"), SUBMITTER_SET),
        (admin, submitter),
    );
}

/// Emit an event when one submitter key is atomically swapped for another.
///
/// Topic: `("subm_rot", SUBM_SET)`; data: `(admin, old_submitter, new_submitter)`.
pub fn emit_submitter_rotated(
    env: &Env,
    admin: Address,
    old_submitter: Address,
    new_submitter: Address,
) {
    env.events().publish(
        (symbol_short!("subm_rot"), SUBMITTER_SET),
        (admin, old_submitter, new_submitter),
    );
}

// ============================================================================
// Two-step admin transfer and timelocked upgrades (#2783)
// ============================================================================

/// Emit when the admin proposes a new admin. Topic `("adm_prop", CTR_LFE)`,
/// data `(admin, proposed)`.
pub fn emit_admin_proposed(env: &Env, admin: Address, proposed: Address) {
    env.events().publish(
        (symbol_short!("adm_prop"), CONTRACT_LIFECYCLE),
        (admin, proposed),
    );
}

/// Emit when a pending admin transfer is withdrawn. Topic
/// `("adm_cncl", CTR_LFE)`, data `(caller, proposed)`.
pub fn emit_admin_transfer_cancelled(env: &Env, caller: Address, proposed: Address) {
    env.events().publish(
        (symbol_short!("adm_cncl"), CONTRACT_LIFECYCLE),
        (caller, proposed),
    );
}

/// Emit when an upgrade is scheduled. Topic `("upg_schd", CTR_LFE)`, data
/// `(proposer, wasm_hash, executable_at_ledger)`.
pub fn emit_upgrade_scheduled(
    env: &Env,
    proposer: Address,
    wasm_hash: BytesN<32>,
    executable_at: u32,
) {
    env.events().publish(
        (symbol_short!("upg_schd"), CONTRACT_LIFECYCLE),
        (proposer, wasm_hash, executable_at),
    );
}

/// Emit when a scheduled upgrade is cancelled. Topic `("upg_cncl", CTR_LFE)`,
/// data `(caller, wasm_hash)`.
pub fn emit_upgrade_cancelled(env: &Env, caller: Address, wasm_hash: BytesN<32>) {
    env.events().publish(
        (symbol_short!("upg_cncl"), CONTRACT_LIFECYCLE),
        (caller, wasm_hash),
    );
}

/// Emit when a scheduled upgrade is executed. Topic `("upg_exec", CTR_LFE)`,
/// data `(caller, wasm_hash)`.
pub fn emit_upgrade_executed(env: &Env, caller: Address, wasm_hash: BytesN<32>) {
    env.events().publish(
        (symbol_short!("upg_exec"), CONTRACT_LIFECYCLE),
        (caller, wasm_hash),
    );
}

/// Emit when the upgrade delay changes. Topic `("upg_dely", CTR_LFE)`, data
/// `(caller, old_delay, new_delay)` in ledgers.
pub fn emit_upgrade_delay_set(env: &Env, caller: Address, old_delay: u32, new_delay: u32) {
    env.events().publish(
        (symbol_short!("upg_dely"), CONTRACT_LIFECYCLE),
        (caller, old_delay, new_delay),
    );
}

/// Policy details of a submitter rotation (#2786).
///
/// Emitted alongside `subm_rot` so indexers that only know the old tuple keep
/// working. Topic: `("rot_pol", SUBM_SET)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmitterRotationPolicy {
    pub admin: Address,
    pub old_submitter: Address,
    pub new_submitter: Address,
    pub reason: RotationReason,
    /// Ledger sequence at which the old key stops being accepted; `0` means
    /// it was revoked immediately.
    pub overlap_until: u32,
}

/// Emit the policy event for a rotation.
pub fn emit_submitter_rotation_policy(env: &Env, event: SubmitterRotationPolicy) {
    env.events()
        .publish((symbol_short!("rot_pol"), SUBMITTER_SET), event);
}

/// Emit progress of a batched legacy-snapshot migration (#2781).
///
/// Topic: `("migr_part",)`; data: `(caller, migrated_this_call, remaining)`.
pub fn emit_migration_progress(env: &Env, caller: Address, migrated: u32, remaining: u32) {
    env.events()
        .publish((symbol_short!("migr_part"),), (caller, migrated, remaining));
}
