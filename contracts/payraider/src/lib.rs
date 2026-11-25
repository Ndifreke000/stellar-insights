#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{
    emit_admin_proposed, emit_admin_transfer_cancelled, emit_admin_transferred,
    emit_contract_initialized, emit_contract_paused, emit_contract_unpaused,
    emit_migration_progress, emit_snapshot_submitted, emit_submitter_added, emit_submitter_removed,
    emit_submitter_rotated, emit_submitter_rotation_policy, emit_upgrade_cancelled,
    emit_upgrade_delay_set, emit_upgrade_executed, emit_upgrade_scheduled, SubmitterRotationPolicy,
};
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN, Env, Map, String,
    Vec,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ~30 days at 5 s/ledger
const LEDGERS_TO_EXTEND: u32 = 518_400;
const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;

/// Upper bound on the authorised submitter set. The set lives in instance
/// storage, which is loaded on every invocation, so it must stay small.
pub const MAX_SUBMITTERS: u32 = 10;

/// Upper bound on the role-holder index (#2784). The index lives in instance
/// storage and `bump_ttl` walks it, so it must stay small.
pub const MAX_ROLE_HOLDERS: u32 = 32;

/// Shortest delay, in ledgers, between `schedule_upgrade` and
/// `execute_upgrade` (~1 hour at 5 s/ledger) (#2783).
pub const MIN_UPGRADE_DELAY_LEDGERS: u32 = 720;

/// Upgrade delay used until one is configured (~24 hours at 5 s/ledger).
pub const DEFAULT_UPGRADE_DELAY_LEDGERS: u32 = 17_280;
/// Longest overlap window `rotate_submitter_with_policy` accepts: ~1 day at
/// 5 s/ledger. Long enough for a backend redeploy, short enough that a
/// forgotten old key stops working on its own (#2786).
pub const MAX_ROTATION_OVERLAP: u32 = 17_280;

/// Legacy snapshots moved per `migrate()` call (#2781).
pub const DEFAULT_MIGRATION_BATCH: u32 = 100;
/// Upper bound for `migrate_batch(max_items)`.
pub const MAX_MIGRATION_BATCH: u32 = 500;

/// Most links `verify_range` walks in one call (#2782).
pub const MAX_VERIFY_STEPS: u32 = 100;
/// Widest epoch span `bump_snapshot_ttl` scans in one call (#2781).
pub const MAX_TTL_BUMP_SPAN: u64 = 200;

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

/// Extend a persistent entry's TTL if it exists.
fn bump_persistent(env: &Env, key: &DataKey) {
    if env.storage().persistent().has(key) {
        env.storage()
            .persistent()
            .extend_ttl(key, LEDGERS_TO_EXTEND, LEDGERS_TO_EXTEND);
    }
}

fn load_submitters(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Submitters)
        .unwrap_or_else(|| Vec::new(env))
}

fn store_submitters(env: &Env, submitters: &Vec<Address>) {
    env.storage()
        .instance()
        .set(&DataKey::Submitters, submitters);
    bump_instance(env);
}

/// Keys rotated out with an overlap window, mapped to the ledger sequence at
/// which they stop being accepted. Expired entries are dropped on load.
fn load_outgoing(env: &Env) -> Map<Address, u32> {
    let stored: Map<Address, u32> = env
        .storage()
        .instance()
        .get(&DataKey::OutgoingSubmitters)
        .unwrap_or_else(|| Map::new(env));
    let now = env.ledger().sequence();
    let mut live = Map::new(env);
    for (addr, until) in stored.iter() {
        if now < until {
            live.set(addr, until);
        }
    }
    live
}

fn store_outgoing(env: &Env, outgoing: &Map<Address, u32>) {
    if outgoing.is_empty() {
        env.storage()
            .instance()
            .remove(&DataKey::OutgoingSubmitters);
    } else {
        env.storage()
            .instance()
            .set(&DataKey::OutgoingSubmitters, outgoing);
    }
    bump_instance(env);
}

/// In the submitter set, or an outgoing key still inside its overlap window.
fn is_submitter(env: &Env, addr: &Address) -> bool {
    if load_submitters(env).contains(addr) {
        return true;
    }
    let until: Option<u32> = env
        .storage()
        .instance()
        .get::<DataKey, Map<Address, u32>>(&DataKey::OutgoingSubmitters)
        .and_then(|m| m.get(addr.clone()));
    matches!(until, Some(u) if env.ledger().sequence() < u)
}

/// Genesis value of the snapshot hash chain (#2782).
fn zero_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0u8; 32])
}

/// `sha256(prev_chain || epoch_be_u64 || snapshot_hash)` — one link of the
/// snapshot hash chain (#2782). The backend verifier
/// (`backend/src/services/snapshot_chain.rs`) computes the same bytes.
pub fn chain_link(
    env: &Env,
    prev_chain: &BytesN<32>,
    epoch: u64,
    snapshot_hash: &BytesN<32>,
) -> BytesN<32> {
    let mut buf = Bytes::from_array(env, &prev_chain.to_array());
    buf.extend_from_array(&epoch.to_be_bytes());
    buf.extend_from_array(&snapshot_hash.to_array());
    env.crypto().sha256(&buf).to_bytes()
}

/// `(prev_epoch, prev_chain)` the next snapshot links to.
fn chain_tip(env: &Env) -> (u64, BytesN<32>) {
    match env
        .storage()
        .instance()
        .get::<DataKey, Snapshot>(&DataKey::Latest)
    {
        Some(latest) => (latest.epoch, latest.chain_hash),
        None => (0, zero_hash(env)),
    }
}

fn load_snapshot(env: &Env, epoch: u64) -> Option<Snapshot> {
    env.storage().persistent().get(&DataKey::Snapshot(epoch))
}

/// Write one epoch under its own key and make it the chain tip.
fn store_snapshot(env: &Env, snapshot: &Snapshot) {
    let key = DataKey::Snapshot(snapshot.epoch);
    env.storage().persistent().set(&key, snapshot);
    env.storage()
        .persistent()
        .extend_ttl(&key, LEDGERS_TO_EXTEND, LEDGERS_TO_EXTEND);
    env.storage().instance().set(&DataKey::Latest, snapshot);
    env.storage()
        .instance()
        .set(&DataKey::ChainHead, &snapshot.chain_hash);
}

/// Move up to `max_items` epochs out of the pre-v2 `DataKey::Snapshots` map
/// into per-epoch keys, chaining them in ascending epoch order. Returns
/// `(migrated, remaining)`. Safe to call repeatedly: processed epochs are
/// removed from the legacy map in the same transaction they are written.
fn migrate_legacy_snapshots(env: &Env, max_items: u32) -> (u32, u32) {
    let mut legacy: Map<u64, LegacySnapshot> =
        match env.storage().persistent().get(&DataKey::Snapshots) {
            Some(m) => m,
            None => return (0, 0),
        };

    let (mut prev_epoch, mut prev_chain) = chain_tip(env);
    let mut migrated = 0u32;
    for epoch in legacy.keys().iter() {
        if migrated >= max_items {
            break;
        }
        if let Some(old) = legacy.get(epoch) {
            let chain_hash = chain_link(env, &prev_chain, old.epoch, &old.hash);
            let snapshot = Snapshot {
                hash: old.hash,
                epoch: old.epoch,
                timestamp: old.timestamp,
                prev_epoch,
                prev_chain_hash: prev_chain,
                chain_hash: chain_hash.clone(),
            };
            store_snapshot(env, &snapshot);
            prev_epoch = snapshot.epoch;
            prev_chain = chain_hash;
        }
        legacy.remove(epoch);
        migrated += 1;
    }

    let remaining = legacy.len();
    if remaining == 0 {
        env.storage().persistent().remove(&DataKey::Snapshots);
    } else {
        env.storage().persistent().set(&DataKey::Snapshots, &legacy);
        bump_persistent(env, &DataKey::Snapshots);
    }
    (migrated, remaining)
}

/// Authenticate `caller` and check it is the stored admin.
fn require_admin(env: &Env, caller: &Address) -> Result<Address, Error> {
    caller.require_auth();
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::AdminNotSet)?;
    if *caller != admin {
        return Err(Error::Unauthorized);
    }
    Ok(admin)
}

fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

fn load_governance(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Governance)
}

// ── Role entries and their TTL (#2784) ──────────────────────────────────────

/// Read `user`'s role list, extending the entry's TTL when it exists.
///
/// Every authorisation check goes through here, so an address that keeps
/// using its role keeps its role entry alive.
fn read_roles(env: &Env, user: &Address) -> Option<Vec<Role>> {
    let key = DataKey::Roles(user.clone());
    let roles: Option<Vec<Role>> = env.storage().persistent().get(&key);
    if roles.is_some() {
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGERS_TO_EXTEND, LEDGERS_TO_EXTEND);
    }
    roles
}

/// Write `user`'s role list (removing the entry when empty) and keep the
/// role-holder index in step with it.
fn write_roles(env: &Env, user: &Address, roles: &Vec<Role>) -> Result<(), Error> {
    let key = DataKey::Roles(user.clone());
    if roles.is_empty() {
        env.storage().persistent().remove(&key);
        unindex_role_holder(env, user);
        return Ok(());
    }
    index_role_holder(env, user)?;
    env.storage().persistent().set(&key, roles);
    env.storage()
        .persistent()
        .extend_ttl(&key, LEDGERS_TO_EXTEND, LEDGERS_TO_EXTEND);
    Ok(())
}

fn load_role_holders(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::RoleHolders)
        .unwrap_or_else(|| Vec::new(env))
}

fn index_role_holder(env: &Env, user: &Address) -> Result<(), Error> {
    let mut holders = load_role_holders(env);
    if holders.contains(user) {
        return Ok(());
    }
    if holders.len() >= MAX_ROLE_HOLDERS {
        return Err(Error::TooManyRoleHolders);
    }
    holders.push_back(user.clone());
    env.storage()
        .instance()
        .set(&DataKey::RoleHolders, &holders);
    Ok(())
}

fn unindex_role_holder(env: &Env, user: &Address) {
    let mut holders = load_role_holders(env);
    if let Some(index) = holders.first_index_of(user) {
        holders.remove(index);
        env.storage()
            .instance()
            .set(&DataKey::RoleHolders, &holders);
    }
}

/// Move the admin from `old_admin` to `new_admin`: instance pointer, role
/// entries, role-holder index and any pending transfer. Shared by
/// `accept_admin` and `set_admin_by_governance`.
fn install_admin(env: &Env, old_admin: &Address, new_admin: &Address) -> Result<(), Error> {
    env.storage().instance().set(&DataKey::Admin, new_admin);
    env.storage().instance().remove(&DataKey::PendingAdmin);

    env.storage()
        .persistent()
        .remove(&DataKey::Roles(old_admin.clone()));
    unindex_role_holder(env, old_admin);

    let mut roles = Vec::new(env);
    roles.push_back(Role::Admin);
    write_roles(env, new_admin, &roles)?;

    bump_instance(env);
    emit_admin_transferred(env, old_admin.clone(), new_admin.clone());
    Ok(())
}

/// Authenticate `caller` and check it may schedule/execute an upgrade: the
/// admin, an `UpgradeManager`, or the governance contract.
fn require_upgrade_authority(env: &Env, caller: &Address) -> Result<(), Error> {
    caller.require_auth();
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::AdminNotSet);
    }
    let allowed = has_role_internal(env, caller, Role::Admin)
        || has_role_internal(env, caller, Role::UpgradeManager)
        || load_governance(env).as_ref() == Some(caller);
    if allowed {
        Ok(())
    } else {
        Err(Error::Unauthorized)
    }
}

/// Authenticate `caller` and check it is the admin or the governance contract.
fn require_admin_or_governance(env: &Env, caller: &Address) -> Result<(), Error> {
    caller.require_auth();
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::AdminNotSet)?;
    if *caller == admin || load_governance(env).as_ref() == Some(caller) {
        Ok(())
    } else {
        Err(Error::Unauthorized)
    }
}

fn has_role_internal(env: &Env, user: &Address, role: Role) -> bool {
    if role == Role::SnapshotSubmitter {
        return is_submitter(env, user);
    }
    read_roles(env, user).is_some_and(|roles| roles.iter().any(|r| r == role))
}

/// Add `submitter` to the set. Errors if already present or the set is full.
fn insert_submitter(env: &Env, submitter: &Address) -> Result<(), Error> {
    let mut submitters = load_submitters(env);
    // Re-adding a key that is only in its overlap window makes it a full
    // member again; it must not also linger as an expiring entry.
    let mut outgoing = load_outgoing(env);
    if outgoing.contains_key(submitter.clone()) && !submitters.contains(submitter) {
        if submitters.len() >= MAX_SUBMITTERS {
            return Err(Error::TooManySubmitters);
        }
        outgoing.remove(submitter.clone());
        store_outgoing(env, &outgoing);
    }
    if submitters.contains(submitter) {
        return Err(Error::SubmitterAlreadyAuthorized);
    }
    if submitters.len() >= MAX_SUBMITTERS {
        return Err(Error::TooManySubmitters);
    }
    submitters.push_back(submitter.clone());
    store_submitters(env, &submitters);
    Ok(())
}

/// Remove `submitter` from the set, or end its overlap window early if it is
/// an outgoing key. Errors if it is neither.
fn delete_submitter(env: &Env, submitter: &Address) -> Result<(), Error> {
    let mut submitters = load_submitters(env);
    if let Some(index) = submitters.first_index_of(submitter) {
        submitters.remove(index);
        store_submitters(env, &submitters);
        return Ok(());
    }
    let mut outgoing = load_outgoing(env);
    if outgoing.contains_key(submitter.clone()) {
        outgoing.remove(submitter.clone());
        store_outgoing(env, &outgoing);
        return Ok(());
    }
    Err(Error::SubmitterNotFound)
}

/// Role-based access control for contract functions
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Role {
    /// Can perform all operations (backwards compat: original admin role)
    Admin = 0,
    /// Can submit snapshots
    SnapshotSubmitter = 1,
    /// Can pause/unpause the contract
    PauseManager = 2,
    /// Can perform upgrades
    UpgradeManager = 3,
}

/// Storage keys for persistent contract data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Administrator address authorized to submit snapshots
    Admin,
    /// Legacy (storage v1) single map of epoch -> snapshot. Only read by the
    /// v1 -> v2 migration; v2 never writes it (#2781).
    Snapshots,
    /// Latest epoch number recorded
    LatestEpoch,
    /// Emergency pause state (true = paused, false = active)
    Paused,
    /// Contract package version at initialization
    Version,
    /// Governance contract permitted to update parameters on-chain (#2137)
    Governance,
    /// Schema version of the data currently in storage (#2133)
    StorageVersion,
    /// Role list for a given address (#2140)
    Roles(Address),
    /// Authorised snapshot-submitter set, `Vec<Address>` in instance storage (#2600).
    /// The only source of truth for `Role::SnapshotSubmitter`.
    Submitters,
    /// Address proposed by `propose_admin`, waiting for `accept_admin` (#2783)
    PendingAdmin,
    /// Upgrade scheduled by `schedule_upgrade`, waiting for its delay (#2783)
    PendingUpgrade,
    /// Ledgers between scheduling and executing an upgrade (#2783)
    UpgradeDelay,
    /// Every address with a `Roles(addr)` entry, `Vec<Address>` in instance
    /// storage, so `bump_ttl` can keep all role entries alive (#2784)
    RoleHolders,
    /// One snapshot per epoch, persistent (#2781).
    Snapshot(u64),
    /// Latest `Snapshot`, instance storage, so the tip survives archival of
    /// older epochs (#2781).
    Latest,
    /// Latest chain hash, instance storage (#2782).
    ChainHead,
    /// `Map<Address, u32>` of rotated-out keys still inside their overlap
    /// window, instance storage (#2786).
    OutgoingSubmitters,
}

/// An upgrade waiting out its timelock (#2783).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    /// Hash of the already-uploaded Wasm that will be installed
    pub wasm_hash: BytesN<32>,
    /// Who scheduled it
    pub proposer: Address,
    /// Ledger sequence at scheduling time
    pub scheduled_at: u32,
    /// First ledger sequence at which `execute_upgrade` succeeds
    pub executable_at: u32,
}

/// Why a submitter key was rotated (#2786). Recorded in the `rot_pol` event.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RotationReason {
    /// Planned rotation; an overlap window is allowed.
    Scheduled = 0,
    /// The old key is (suspected) compromised; it is revoked immediately and
    /// no overlap is allowed.
    Compromise = 1,
    /// Anything else, e.g. decommissioning a host.
    Other = 2,
}

/// Schema version this build reads and writes.
///
/// Bumped whenever the *shape* of stored data changes — not on every release.
/// `migrate()` moves storage from whatever version it is on up to this one, and
/// refuses to run against anything newer than it understands.
///
/// * v1 — every snapshot in one `DataKey::Snapshots` map.
/// * v2 — one `DataKey::Snapshot(epoch)` entry per epoch, hash-chained
///   (#2781, #2782).
pub const CURRENT_STORAGE_VERSION: u32 = 2;

/// Analytics snapshot data structure (storage v2).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    /// SHA-256 hash of analytics data
    pub hash: BytesN<32>,
    /// Epoch identifier
    pub epoch: u64,
    /// Ledger timestamp when recorded
    pub timestamp: u64,
    /// Epoch this one chains to; `0` for the first stored epoch
    pub prev_epoch: u64,
    /// Chain hash of `prev_epoch`; all zeros for the first stored epoch
    pub prev_chain_hash: BytesN<32>,
    /// `sha256(prev_chain_hash || epoch_be || hash)`
    pub chain_hash: BytesN<32>,
}

/// Snapshot shape inside the storage-v1 `DataKey::Snapshots` map. Kept only
/// so the migration can decode it.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacySnapshot {
    pub hash: BytesN<32>,
    pub epoch: u64,
    pub timestamp: u64,
}

/// Extended contract metadata for public disclosure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub repository: String,
    pub license: String,
}

/// Represents an optional admin address in contract info
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaybeAddress {
    None,
    Some(Address),
}

/// Contract info combining metadata with runtime state
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInfo {
    pub metadata: PublicMetadata,
    pub initialized: bool,
    pub paused: bool,
    pub admin: MaybeAddress,
    pub total_snapshots: u64,
}

#[contract]
pub struct PayRaiderContract;

#[contractimpl]
impl PayRaiderContract {
    /// Initialize the contract with an admin address
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `admin` - Address that will be authorized to submit snapshots
    ///
    /// # Returns
    /// * Success confirmation
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        // Verify admin doesn't already exist to prevent re-initialization
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        // `stellar contract deploy` and `initialize` are separate
        // transactions, so without this anyone watching the network could
        // initialise a freshly deployed contract first and make themselves
        // admin. The admin must sign its own installation.
        admin.require_auth();

        // Store the admin address
        env.storage().instance().set(&DataKey::Admin, &admin);

        // Initialize latest epoch to 0
        env.storage().instance().set(&DataKey::LatestEpoch, &0u64);

        // Stamp the schema version so a fresh deploy is never mistaken for
        // pre-versioning state that still needs migrating.
        env.storage()
            .instance()
            .set(&DataKey::StorageVersion, &CURRENT_STORAGE_VERSION);

        // Initialize contract as not paused
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage()
            .instance()
            .set(&DataKey::Version, &String::from_str(&env, VERSION));
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);

        // Grant Admin role to initial admin (and index it for bump_ttl)
        let mut roles = Vec::new(&env);
        roles.push_back(Role::Admin);
        write_roles(&env, &admin, &roles)?;

        emit_contract_initialized(&env, admin);

        Ok(())
    }

    pub fn get_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or_else(|| String::from_str(&env, VERSION))
    }

    /// Submit a cryptographic hash of an analytics snapshot on-chain
    ///
    /// Only the admin or a member of the authorised submitter set (see
    /// [`Self::add_submitter`]) can call this function; every other address is
    /// rejected. Each epoch can only have one snapshot submitted. Upon
    /// successful submission, an event is emitted for verification purposes.
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `epoch` - Epoch identifier (must be positive and unique)
    /// * `hash` - 32-byte SHA-256 hash of the analytics snapshot
    /// * `caller` - Address attempting to submit the snapshot
    ///
    /// # Errors
    /// * `Error::ContractPaused` - If contract is in emergency pause state
    /// * `Error::AdminNotSet` - If admin was not initialized
    /// * `Error::Unauthorized` - If caller is neither the admin nor in the submitter set
    /// * `Error::InvalidEpochZero` - If epoch is 0
    /// * `Error::DuplicateEpoch` - If snapshot already exists for this epoch
    /// * `Error::EpochMonotonicityViolated` - If epoch <= latest (out-of-order submission)
    /// * `Error::MigrationRequired` - If storage has not been migrated to v2 yet
    ///
    /// # Storage (#2781, #2782)
    /// Each epoch is written to its own `DataKey::Snapshot(epoch)` entry, and
    /// the tip (`Latest`, `ChainHead`, `LatestEpoch`) lives in instance
    /// storage, so the cost of a submission does not depend on how many
    /// epochs came before it. Every entry carries
    /// `chain_hash = sha256(prev_chain_hash || epoch_be || hash)`.
    ///
    /// # Returns
    /// * Ledger timestamp when the snapshot was recorded
    pub fn submit_snapshot(
        env: Env,
        epoch: u64,
        hash: BytesN<32>,
        caller: Address,
    ) -> Result<u64, Error> {
        // Check if contract is paused
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if is_paused {
            return Err(Error::ContractPaused);
        }

        // Verify caller is authenticated
        caller.require_auth();

        // Verify contract is initialized (admin is set)
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AdminNotSet);
        }

        // An upgraded deployment must finish moving its legacy snapshot map
        // first, otherwise the new epoch would chain to an incomplete history.
        if Self::get_storage_version(env.clone()) < CURRENT_STORAGE_VERSION {
            return Err(Error::MigrationRequired);
        }

        // Deny by default: only the submitter set or an Admin may anchor.
        // The set is checked first because it lives in instance storage,
        // which is already loaded.
        let in_submitter_set = is_submitter(&env, &caller);
        let is_admin =
            !in_submitter_set && Self::has_role(env.clone(), caller.clone(), Role::Admin);
        if !in_submitter_set && !is_admin {
            return Err(Error::Unauthorized);
        }

        // Validate epoch is not zero
        if epoch == 0 {
            return Err(Error::InvalidEpochZero);
        }

        // Prevent u64 overflow on mainnet by capping at u64::MAX
        if epoch == u64::MAX {
            return Err(Error::EpochOverflow);
        }

        // Validate hash is not all zeros (security-critical — the
        // Error::InvalidHashZero variant existed but was never actually
        // checked, so a degenerate all-zero hash silently passed through).
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        if hash == zero_hash {
            return Err(Error::InvalidHashZero);
        }

        // Check for duplicate epoch
        if env.storage().persistent().has(&DataKey::Snapshot(epoch)) {
            return Err(Error::DuplicateEpoch);
        }

        // Enforce monotonic epoch increase to prevent rollback attacks
        let current_latest: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);
        if epoch <= current_latest {
            return Err(Error::EpochMonotonicityViolated);
        }

        // Get current ledger timestamp
        let timestamp = env.ledger().timestamp();

        // Link to the previous tip (#2782)
        let (prev_epoch, prev_chain_hash) = chain_tip(&env);
        let chain_hash = chain_link(&env, &prev_chain_hash, epoch, &hash);

        let snapshot = Snapshot {
            hash: hash.clone(),
            epoch,
            timestamp,
            prev_epoch,
            prev_chain_hash,
            chain_hash: chain_hash.clone(),
        };

        // One persistent entry per epoch, TTL ~30 days (#2781)
        store_snapshot(&env, &snapshot);

        env.storage().instance().set(&DataKey::LatestEpoch, &epoch);

        // Keep the contract instance (admin, submitter set, latest epoch)
        // and the caller's role entry alive alongside the snapshot (#2599).
        // The admin's role entry was already extended by the `has_role`
        // check above (#2784).
        bump_instance(&env);

        // Emit structured event for off-chain indexing
        // Event payload matches stored data exactly:
        // - hash: same as snapshot.hash
        // - epoch: same as snapshot.epoch
        // - timestamp: same as snapshot.timestamp
        // - submitter: the authenticated caller
        // - chain_hash: same as snapshot.chain_hash (#2782)
        emit_snapshot_submitted(&env, hash, epoch, timestamp, caller, chain_hash);

        Ok(timestamp)
    }

    /// Retrieve a snapshot hash for a specific epoch
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `epoch` - Epoch to retrieve
    ///
    /// # Errors
    /// * `Error::SnapshotNotFound` - If no snapshot exists for the epoch
    ///
    /// # Returns
    /// * The 32-byte hash stored for that epoch
    pub fn get_snapshot(env: Env, epoch: u64) -> Result<BytesN<32>, Error> {
        Self::get_snapshot_record(env, epoch).map(|s| s.hash)
    }

    /// Full stored record for `epoch`, including its chain links (#2782).
    ///
    /// Extends the entry's TTL when submitted as a transaction. An epoch
    /// whose entry has been archived must be restored first (see README,
    /// "Restoring an archived epoch").
    pub fn get_snapshot_record(env: Env, epoch: u64) -> Result<Snapshot, Error> {
        let key = DataKey::Snapshot(epoch);
        let snapshot = load_snapshot(&env, epoch).ok_or(Error::SnapshotNotFound)?;
        bump_persistent(&env, &key);
        Ok(snapshot)
    }

    /// `(latest_epoch, chain_hash)` of the chain tip. `(0, zeros)` when no
    /// snapshot has been stored yet (#2782).
    pub fn get_chain_head(env: Env) -> (u64, BytesN<32>) {
        chain_tip(&env)
    }

    /// Recompute every chain link from `to` back to the first stored epoch
    /// `>= from` and return the chain hash at `to` (#2782).
    ///
    /// Walks at most `MAX_VERIFY_STEPS` links; verify a longer history in
    /// several calls, or off-chain with the backend verifier. When `to` is the
    /// latest epoch, the result is also checked against `ChainHead`.
    ///
    /// # Errors
    /// * `Error::InvalidRange` - `from == 0` or `from > to`
    /// * `Error::SnapshotNotFound` - `to`, or an epoch on the path, is missing
    /// * `Error::ChainBroken` - a stored link does not recompute
    /// * `Error::InvalidRange` - the walk needs more than `MAX_VERIFY_STEPS`
    pub fn verify_range(env: Env, from: u64, to: u64) -> Result<BytesN<32>, Error> {
        if from == 0 || from > to {
            return Err(Error::InvalidRange);
        }
        let top = load_snapshot(&env, to).ok_or(Error::SnapshotNotFound)?;
        let mut cur = top.clone();
        let mut steps = 0u32;
        loop {
            if chain_link(&env, &cur.prev_chain_hash, cur.epoch, &cur.hash) != cur.chain_hash {
                return Err(Error::ChainBroken);
            }
            if cur.prev_epoch == 0 || cur.prev_epoch < from {
                break;
            }
            steps += 1;
            if steps >= MAX_VERIFY_STEPS {
                return Err(Error::InvalidRange);
            }
            let prev = load_snapshot(&env, cur.prev_epoch).ok_or(Error::SnapshotNotFound)?;
            if prev.epoch != cur.prev_epoch || prev.chain_hash != cur.prev_chain_hash {
                return Err(Error::ChainBroken);
            }
            cur = prev;
        }
        let (latest_epoch, head) = chain_tip(&env);
        if latest_epoch == to && head != top.chain_hash {
            return Err(Error::ChainBroken);
        }
        Ok(top.chain_hash)
    }

    /// Get the most recent snapshot
    ///
    /// # Arguments
    /// * `env` - Contract environment
    ///
    /// # Errors
    /// * `Error::SnapshotNotFound` - If no snapshots exist
    ///
    /// # Returns
    /// * Tuple of (hash, epoch, timestamp) for the latest snapshot
    pub fn latest_snapshot(env: Env) -> Result<(BytesN<32>, u64, u64), Error> {
        let latest_epoch: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);

        if latest_epoch == 0 {
            return Err(Error::SnapshotNotFound);
        }

        // The tip lives in instance storage (#2781): O(1) and never archived
        // while the contract is alive.
        let snapshot: Snapshot = env
            .storage()
            .instance()
            .get(&DataKey::Latest)
            .ok_or(Error::SnapshotNotFound)?;
        bump_persistent(&env, &DataKey::Snapshot(latest_epoch));

        Ok((snapshot.hash, snapshot.epoch, snapshot.timestamp))
    }

    /// Get the current admin address
    ///
    /// # Arguments
    /// * `env` - Contract environment
    ///
    /// # Errors
    /// * `Error::AdminNotSet` - If admin was not initialized
    ///
    /// # Returns
    /// * The admin address
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)
    }

    /// Propose `new_admin` as the next admin — step one of a two-step
    /// transfer (#2783). Admin-only.
    ///
    /// Nothing changes hands until `new_admin` calls [`Self::accept_admin`],
    /// so a typo'd address can never take control. Proposing again replaces
    /// the pending proposal.
    ///
    /// # Errors
    /// * `Error::ContractPaused` - If contract is in emergency pause state
    /// * `Error::AdminNotSet` - If admin was not initialized
    /// * `Error::Unauthorized` - If caller is not the current admin
    ///
    /// # Events
    /// * `("adm_prop", CTR_LFE)` with data `(admin, new_admin)`
    pub fn propose_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
        if is_paused(&env) {
            return Err(Error::ContractPaused);
        }
        let admin = require_admin(&env, &caller)?;

        env.storage()
            .instance()
            .set(&DataKey::PendingAdmin, &new_admin);
        bump_instance(&env);

        emit_admin_proposed(&env, admin, new_admin);
        Ok(())
    }

    /// Accept a pending admin transfer — step two (#2783). Must be signed by
    /// the proposed address.
    ///
    /// # Errors
    /// * `Error::ContractPaused` - If contract is in emergency pause state
    /// * `Error::NoPendingAdmin` - If no transfer has been proposed
    /// * `Error::Unauthorized` - If `new_admin` is not the proposed address
    ///
    /// # Events
    /// * `("admin", CTR_LFE)` with an `AdminTransferredEvent`
    pub fn accept_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        if is_paused(&env) {
            return Err(Error::ContractPaused);
        }
        new_admin.require_auth();

        let pending: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .ok_or(Error::NoPendingAdmin)?;
        if pending != new_admin {
            return Err(Error::Unauthorized);
        }
        let old_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        install_admin(&env, &old_admin, &new_admin)
    }

    /// Withdraw a pending admin transfer. Admin or governance.
    ///
    /// # Errors
    /// * `Error::Unauthorized` - If caller is neither admin nor governance
    /// * `Error::NoPendingAdmin` - If no transfer has been proposed
    ///
    /// # Events
    /// * `("adm_cncl", CTR_LFE)` with data `(caller, proposed)`
    pub fn cancel_admin_transfer(env: Env, caller: Address) -> Result<(), Error> {
        require_admin_or_governance(&env, &caller)?;
        let pending: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .ok_or(Error::NoPendingAdmin)?;
        env.storage().instance().remove(&DataKey::PendingAdmin);
        bump_instance(&env);

        emit_admin_transfer_cancelled(&env, caller, pending);
        Ok(())
    }

    /// The address proposed as next admin, if a transfer is pending.
    pub fn get_pending_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::PendingAdmin)
    }

    /// Get the latest epoch number
    ///
    /// # Arguments
    /// * `env` - Contract environment
    ///
    /// # Returns
    /// * The latest epoch number (0 if no snapshots)
    pub fn get_latest_epoch(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0)
    }

    /// Emergency pause the contract
    ///
    /// Pauses all snapshot submissions. Only the admin can pause the contract.
    /// Read operations remain available during pause.
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `caller` - Address attempting to pause (must be admin)
    ///
    /// # Errors
    /// * `Error::AdminNotSet` - If admin was not initialized
    /// * `Error::Unauthorized` - If caller is not the admin
    pub fn pause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();

        // Check if caller has Admin or PauseManager role
        let has_admin_role = Self::has_role(env.clone(), caller.clone(), Role::Admin);
        let has_pause_manager_role =
            Self::has_role(env.clone(), caller.clone(), Role::PauseManager);

        if !has_admin_role && !has_pause_manager_role {
            return Err(Error::Unauthorized);
        }

        env.storage().instance().set(&DataKey::Paused, &true);
        bump_instance(&env);

        emit_contract_paused(&env, caller);

        Ok(())
    }

    /// Unpause the contract
    ///
    /// Resumes normal operations. Only the admin can unpause the contract.
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `caller` - Address attempting to unpause (must be admin)
    ///
    /// # Errors
    /// * `Error::AdminNotSet` - If admin was not initialized
    /// * `Error::Unauthorized` - If caller is not the admin
    pub fn unpause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();

        // Check if caller has Admin or PauseManager role
        let has_admin_role = Self::has_role(env.clone(), caller.clone(), Role::Admin);
        let has_pause_manager_role =
            Self::has_role(env.clone(), caller.clone(), Role::PauseManager);

        if !has_admin_role && !has_pause_manager_role {
            return Err(Error::Unauthorized);
        }

        env.storage().instance().set(&DataKey::Paused, &false);
        bump_instance(&env);

        emit_contract_unpaused(&env, caller);

        Ok(())
    }

    /// Schedule a Wasm upgrade — phase one of a timelocked upgrade (#2783).
    ///
    /// Callable by the admin, an `UpgradeManager` or the governance contract.
    /// The upgrade can be executed once `get_upgrade_delay()` ledgers have
    /// passed, giving observers time to inspect the new code. Scheduling again
    /// replaces the pending upgrade and restarts the delay.
    ///
    /// # Errors
    /// * `Error::AdminNotSet` - If admin was not initialized
    /// * `Error::Unauthorized` - If caller may not upgrade
    /// * `Error::ContractPaused` - If contract is currently paused
    /// * `Error::InvalidWasmHash` - If the hash is all zeros
    ///
    /// # Events
    /// * `("upg_schd", CTR_LFE)` with data `(caller, wasm_hash, executable_at)`
    pub fn schedule_upgrade(
        env: Env,
        caller: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<PendingUpgrade, Error> {
        require_upgrade_authority(&env, &caller)?;
        if is_paused(&env) {
            return Err(Error::ContractPaused);
        }

        // A zero hash is never a real Wasm blob; accepting one would replace
        // the contract with nothing and brick it permanently.
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        if new_wasm_hash == zero_hash {
            return Err(Error::InvalidWasmHash);
        }

        let now = env.ledger().sequence();
        let pending = PendingUpgrade {
            wasm_hash: new_wasm_hash.clone(),
            proposer: caller.clone(),
            scheduled_at: now,
            executable_at: now.saturating_add(Self::get_upgrade_delay(env.clone())),
        };
        env.storage()
            .instance()
            .set(&DataKey::PendingUpgrade, &pending);
        bump_instance(&env);

        emit_upgrade_scheduled(&env, caller, new_wasm_hash, pending.executable_at);
        Ok(pending)
    }

    /// Install the scheduled Wasm once its delay has passed — phase two.
    ///
    /// Callable by the admin, an `UpgradeManager` or the governance contract.
    /// Run `migrate()` against the new Wasm afterwards if the storage schema
    /// changed.
    ///
    /// # Errors
    /// * `Error::Unauthorized` - If caller may not upgrade
    /// * `Error::ContractPaused` - If contract is currently paused
    /// * `Error::NoPendingUpgrade` - If nothing is scheduled
    /// * `Error::TimelockNotExpired` - If the delay has not yet passed
    ///
    /// # Events
    /// * `("upg_exec", CTR_LFE)` with data `(caller, wasm_hash)`
    /// * `("upgrade",)` with data `(caller, wasm_hash)` (kept for existing indexers)
    pub fn execute_upgrade(env: Env, caller: Address) -> Result<(), Error> {
        require_upgrade_authority(&env, &caller)?;
        if is_paused(&env) {
            return Err(Error::ContractPaused);
        }

        let pending: PendingUpgrade = env
            .storage()
            .instance()
            .get(&DataKey::PendingUpgrade)
            .ok_or(Error::NoPendingUpgrade)?;
        if env.ledger().sequence() < pending.executable_at {
            return Err(Error::TimelockNotExpired);
        }

        env.storage().instance().remove(&DataKey::PendingUpgrade);
        bump_instance(&env);

        emit_upgrade_executed(&env, caller.clone(), pending.wasm_hash.clone());
        env.events().publish(
            (symbol_short!("upgrade"),),
            (caller, pending.wasm_hash.clone()),
        );

        env.deployer()
            .update_current_contract_wasm(pending.wasm_hash);
        Ok(())
    }

    /// Cancel the scheduled upgrade. Admin or governance.
    ///
    /// # Errors
    /// * `Error::Unauthorized` - If caller is neither admin nor governance
    /// * `Error::NoPendingUpgrade` - If nothing is scheduled
    ///
    /// # Events
    /// * `("upg_cncl", CTR_LFE)` with data `(caller, wasm_hash)`
    pub fn cancel_upgrade(env: Env, caller: Address) -> Result<(), Error> {
        require_admin_or_governance(&env, &caller)?;
        let pending: PendingUpgrade = env
            .storage()
            .instance()
            .get(&DataKey::PendingUpgrade)
            .ok_or(Error::NoPendingUpgrade)?;
        env.storage().instance().remove(&DataKey::PendingUpgrade);
        bump_instance(&env);

        emit_upgrade_cancelled(&env, caller, pending.wasm_hash);
        Ok(())
    }

    /// The upgrade waiting out its delay, if any.
    pub fn get_pending_upgrade(env: Env) -> Option<PendingUpgrade> {
        env.storage().instance().get(&DataKey::PendingUpgrade)
    }

    /// Ledgers between `schedule_upgrade` and `execute_upgrade`.
    pub fn get_upgrade_delay(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::UpgradeDelay)
            .unwrap_or(DEFAULT_UPGRADE_DELAY_LEDGERS)
    }

    /// Change the upgrade delay. The admin may only lengthen it; governance
    /// may set any value at or above `MIN_UPGRADE_DELAY_LEDGERS`. An upgrade
    /// already scheduled keeps its original `executable_at`.
    ///
    /// # Errors
    /// * `Error::Unauthorized` - If caller is neither admin nor governance
    /// * `Error::InvalidUpgradeDelay` - Below the minimum, or an admin shortening it
    ///
    /// # Events
    /// * `("upg_dely", CTR_LFE)` with data `(caller, old_delay, new_delay)`
    pub fn set_upgrade_delay(env: Env, caller: Address, ledgers: u32) -> Result<(), Error> {
        require_admin_or_governance(&env, &caller)?;
        let current = Self::get_upgrade_delay(env.clone());
        let is_governance = load_governance(&env).as_ref() == Some(&caller);
        if ledgers < MIN_UPGRADE_DELAY_LEDGERS || (!is_governance && ledgers < current) {
            return Err(Error::InvalidUpgradeDelay);
        }
        env.storage()
            .instance()
            .set(&DataKey::UpgradeDelay, &ledgers);
        bump_instance(&env);

        emit_upgrade_delay_set(&env, caller, current, ledgers);
        Ok(())
    }

    /// Schema version of the data currently in storage.
    ///
    /// Contracts deployed before versioning existed report `0`, which is what
    /// [`Self::migrate`] uses to decide there is work to do.
    /// Legacy snapshots still waiting for `migrate()` (#2781).
    pub fn pending_migration(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get::<DataKey, Map<u64, LegacySnapshot>>(&DataKey::Snapshots)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    pub fn get_storage_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::StorageVersion)
            .unwrap_or(0)
    }

    /// Bring stored data up to the schema this build expects.
    ///
    /// `upgrade()` swaps the Wasm but cannot touch storage in the same
    /// invocation — the new code is not running yet. Migration is therefore a
    /// separate admin-invoked step, run once against the *new* Wasm:
    ///
    /// ```text
    ///   upgrade(new_wasm_hash)   →  new code active, storage still old shape
    ///   migrate()                →  storage transformed, version stamped
    /// ```
    ///
    /// Idempotent by construction: it refuses with `MigrationNotNeeded` once
    /// storage is already current, so a retry after a partial failure is safe
    /// and a double-invocation cannot corrupt state.
    ///
    /// # Errors
    /// * `Error::AdminNotSet` — admin was never initialized
    /// * `Error::Unauthorized` — caller is neither admin nor governance
    /// * `Error::MigrationNotNeeded` — storage is already current
    /// * `Error::StorageVersionTooNew` — storage was written by a newer build,
    ///   which means a downgrade happened; migrating would lose data
    ///
    /// Moves at most `DEFAULT_MIGRATION_BATCH` legacy snapshots per call; see
    /// [`Self::migrate_batch`]. Returns the storage version after the call —
    /// keep calling until it returns `CURRENT_STORAGE_VERSION`.
    pub fn migrate(env: Env, caller: Address) -> Result<u32, Error> {
        Self::migrate_batch(env, caller, DEFAULT_MIGRATION_BATCH)
    }

    /// [`Self::migrate`] with an explicit per-call batch size (#2781).
    ///
    /// The v1 -> v2 step moves up to `max_items` epochs from the legacy
    /// `DataKey::Snapshots` map into per-epoch entries, chaining them in
    /// ascending epoch order from the zero genesis, and removes them from the
    /// map in the same transaction. While items remain the version stays at 1,
    /// `submit_snapshot` returns `MigrationRequired`, and the call returns `1`.
    /// The call that empties the map deletes it and stamps version 2.
    ///
    /// # Errors
    /// * as [`Self::migrate`], plus `Error::InvalidRange` when `max_items` is
    ///   `0` or above `MAX_MIGRATION_BATCH`
    pub fn migrate_batch(env: Env, caller: Address, max_items: u32) -> Result<u32, Error> {
        caller.require_auth();

        if max_items == 0 || max_items > MAX_MIGRATION_BATCH {
            return Err(Error::InvalidRange);
        }

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        let governance: Option<Address> = env.storage().instance().get(&DataKey::Governance);

        let authorized = caller == admin || governance.as_ref() == Some(&caller);
        if !authorized {
            return Err(Error::Unauthorized);
        }

        let from = Self::get_storage_version(env.clone());

        if from > CURRENT_STORAGE_VERSION {
            return Err(Error::StorageVersionTooNew);
        }
        if from == CURRENT_STORAGE_VERSION {
            return Err(Error::MigrationNotNeeded);
        }

        // ── Migration steps ──────────────────────────────────────────────
        // Each step moves storage forward exactly one version and must be
        // safe to re-run. Add the next as `if from < N { … }` below, in order.

        if from < 1 {
            // v0 → v1: pre-versioning deployments never stamped LatestEpoch
            // when they had no snapshots, so a missing key is read as 0. The
            // rollback guard in submit_snapshot depends on that value being
            // present and correct, so it is materialised here rather than
            // left to `unwrap_or(0)`.
            let latest: u64 = env
                .storage()
                .instance()
                .get(&DataKey::LatestEpoch)
                .unwrap_or(0);
            env.storage().instance().set(&DataKey::LatestEpoch, &latest);
            env.storage()
                .instance()
                .set(&DataKey::StorageVersion, &1u32);
        }

        if from < 2 {
            // v1 -> v2: split the single snapshot map into per-epoch entries
            // and build the hash chain over them (#2781, #2782).
            let (migrated, remaining) = migrate_legacy_snapshots(&env, max_items);
            if remaining > 0 {
                env.storage()
                    .instance()
                    .set(&DataKey::StorageVersion, &1u32);
                bump_instance(&env);
                emit_migration_progress(&env, caller, migrated, remaining);
                return Ok(1);
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::StorageVersion, &CURRENT_STORAGE_VERSION);
        bump_instance(&env);

        env.events().publish(
            (symbol_short!("migrate"),),
            (caller, from, CURRENT_STORAGE_VERSION),
        );

        Ok(CURRENT_STORAGE_VERSION)
    }

    /// Point the contract at the governance contract allowed to update its
    /// parameters on-chain. Admin-only.
    ///
    /// Setting this is what makes a passed governance proposal executable
    /// against this contract; until then the `*_by_governance` entrypoints
    /// reject every caller.
    pub fn set_governance(env: Env, caller: Address, governance: Address) -> Result<(), Error> {
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
            .set(&DataKey::Governance, &governance);
        bump_instance(&env);

        env.events()
            .publish((symbol_short!("govset"),), (caller, governance));

        Ok(())
    }

    /// The governance contract permitted to update parameters, if configured.
    pub fn get_governance(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Governance)
    }

    /// Transfer admin as the outcome of a passed governance proposal.
    ///
    /// Callable only by the configured governance contract. This is the
    /// counterpart of `propose_admin`/`accept_admin` — together they let
    /// control move either by the admin's own hand or by a vote.
    pub fn set_admin_by_governance(
        env: Env,
        caller: Address,
        new_admin: Address,
    ) -> Result<(), Error> {
        caller.require_auth();

        let governance: Address = env
            .storage()
            .instance()
            .get(&DataKey::Governance)
            .ok_or(Error::GovernanceNotSet)?;
        if caller != governance {
            return Err(Error::Unauthorized);
        }

        let old_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        // Immediate: the governance vote is the delay. Clears any pending
        // `propose_admin` so a stale proposal can't later override the vote.
        install_admin(&env, &old_admin, &new_admin)
    }

    /// Pause or unpause as the outcome of a passed governance proposal.
    ///
    /// Callable only by the configured governance contract.
    pub fn set_paused_by_governance(env: Env, caller: Address, paused: bool) -> Result<(), Error> {
        caller.require_auth();

        let governance: Address = env
            .storage()
            .instance()
            .get(&DataKey::Governance)
            .ok_or(Error::GovernanceNotSet)?;
        if caller != governance {
            return Err(Error::Unauthorized);
        }

        env.storage().instance().set(&DataKey::Paused, &paused);
        bump_instance(&env);

        if paused {
            emit_contract_paused(&env, caller);
        } else {
            emit_contract_unpaused(&env, caller);
        }

        Ok(())
    }

    /// Check if contract is paused
    ///
    /// # Arguments
    /// * `env` - Contract environment
    ///
    /// # Returns
    /// * `true` if contract is paused, `false` otherwise
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    // =========================================================================
    // Authorised Submitter Set (#2600)
    // =========================================================================

    /// Authorise `submitter` to anchor snapshots. Admin-only.
    ///
    /// Allowed while paused so a compromised key can be replaced during an
    /// incident.
    ///
    /// # Errors
    /// * `Error::AdminNotSet` - If admin was not initialized
    /// * `Error::Unauthorized` - If caller is not the admin
    /// * `Error::SubmitterAlreadyAuthorized` - If `submitter` is already in the set
    /// * `Error::TooManySubmitters` - If the set already holds `MAX_SUBMITTERS`
    ///
    /// # Events
    /// * `("subm_add", "SUBM_SET")` with data `(admin, submitter)`
    pub fn add_submitter(env: Env, caller: Address, submitter: Address) -> Result<(), Error> {
        let admin = require_admin(&env, &caller)?;
        insert_submitter(&env, &submitter)?;
        emit_submitter_added(&env, admin, submitter);
        Ok(())
    }

    /// Revoke `submitter`'s right to anchor snapshots. Admin-only.
    ///
    /// # Errors
    /// * `Error::AdminNotSet` - If admin was not initialized
    /// * `Error::Unauthorized` - If caller is not the admin
    /// * `Error::SubmitterNotFound` - If `submitter` is not in the set
    ///
    /// # Events
    /// * `("subm_rm", "SUBM_SET")` with data `(admin, submitter)`
    pub fn remove_submitter(env: Env, caller: Address, submitter: Address) -> Result<(), Error> {
        let admin = require_admin(&env, &caller)?;
        delete_submitter(&env, &submitter)?;
        emit_submitter_removed(&env, admin, submitter);
        Ok(())
    }

    /// Atomically replace `old_submitter` with `new_submitter` (key rotation).
    /// Admin-only, and `new_submitter` must also sign so a typo'd or
    /// uncontrolled key can never be installed.
    ///
    /// Equivalent to [`Self::rotate_submitter_with_policy`] with no overlap
    /// and `RotationReason::Scheduled`: the old key loses submission rights in
    /// the same transaction the new key gains them.
    ///
    /// # Errors
    /// * as [`Self::rotate_submitter_with_policy`]
    ///
    /// # Events
    /// * `("subm_rot", "SUBM_SET")` with data `(admin, old_submitter, new_submitter)`
    /// * `("rot_pol", "SUBM_SET")` with a `SubmitterRotationPolicy`
    pub fn rotate_submitter(
        env: Env,
        caller: Address,
        old_submitter: Address,
        new_submitter: Address,
    ) -> Result<(), Error> {
        Self::rotate_submitter_with_policy(
            env,
            caller,
            old_submitter,
            new_submitter,
            0,
            RotationReason::Scheduled,
        )
        .map(|_| ())
    }

    /// Key rotation with an explicit policy (#2786).
    ///
    /// Policy (see README, "Submitter rotation policy"):
    /// * Only the admin may rotate, and `new_submitter` must sign.
    /// * The old key's consent is **not** required — rotation is the response
    ///   to a lost or leaked key, which cannot be relied on to sign.
    /// * `new_submitter` must not be the old key, a current submitter, a key
    ///   still inside an overlap window, or the admin / an `Admin` role holder
    ///   (submitting and administering stay separate keys).
    /// * With `overlap_ledgers > 0` the old key keeps submitting until ledger
    ///   `now + overlap_ledgers` (exclusive). `MAX_ROTATION_OVERLAP` caps it.
    ///   `remove_submitter(old)` ends the window early.
    /// * `RotationReason::Compromise` forbids any overlap.
    /// * Pending epochs: Soroban transactions are atomic, so nothing is half
    ///   written. A transaction signed by the old key that lands after its
    ///   window closes fails with `Unauthorized`; the backend resubmits the
    ///   same epoch with the new key. Epoch monotonicity is unaffected.
    ///
    /// Returns the ledger sequence at which the old key expires (`0` = now).
    ///
    /// # Errors
    /// * `Error::AdminNotSet` / `Error::Unauthorized` - caller is not the admin
    /// * `Error::InvalidOverlap` - overlap too long, or overlap on a compromise
    /// * `Error::SubmitterNotFound` - `old_submitter` is not in the set
    /// * `Error::SubmitterAlreadyAuthorized` - `new_submitter` is already in the set
    /// * `Error::InvalidRotationTarget` - `new_submitter` is the old key, an
    ///   admin, or an outgoing key still in its window
    /// * `Error::TooManySubmitters` - too many keys already in overlap windows
    pub fn rotate_submitter_with_policy(
        env: Env,
        caller: Address,
        old_submitter: Address,
        new_submitter: Address,
        overlap_ledgers: u32,
        reason: RotationReason,
    ) -> Result<u32, Error> {
        let admin = require_admin(&env, &caller)?;

        if overlap_ledgers > MAX_ROTATION_OVERLAP
            || (reason == RotationReason::Compromise && overlap_ledgers > 0)
        {
            return Err(Error::InvalidOverlap);
        }

        let mut submitters = load_submitters(&env);
        let index = submitters
            .first_index_of(&old_submitter)
            .ok_or(Error::SubmitterNotFound)?;
        if new_submitter == old_submitter {
            return Err(Error::InvalidRotationTarget);
        }
        if submitters.contains(&new_submitter) {
            return Err(Error::SubmitterAlreadyAuthorized);
        }
        if new_submitter == admin || Self::has_role(env.clone(), new_submitter.clone(), Role::Admin)
        {
            return Err(Error::InvalidRotationTarget);
        }
        let mut outgoing = load_outgoing(&env);
        if outgoing.contains_key(new_submitter.clone()) {
            return Err(Error::InvalidRotationTarget);
        }
        if overlap_ledgers > 0 && outgoing.len() >= MAX_SUBMITTERS {
            return Err(Error::TooManySubmitters);
        }

        new_submitter.require_auth();

        submitters.set(index, new_submitter.clone());
        store_submitters(&env, &submitters);

        let overlap_until = if overlap_ledgers > 0 {
            let until = env.ledger().sequence().saturating_add(overlap_ledgers);
            outgoing.set(old_submitter.clone(), until);
            until
        } else {
            0
        };
        store_outgoing(&env, &outgoing);

        emit_submitter_rotated(
            &env,
            admin.clone(),
            old_submitter.clone(),
            new_submitter.clone(),
        );
        emit_submitter_rotation_policy(
            &env,
            SubmitterRotationPolicy {
                admin,
                old_submitter,
                new_submitter,
                reason,
                overlap_until,
            },
        );
        Ok(overlap_until)
    }

    /// Rotated-out keys still inside their overlap window, mapped to the
    /// ledger sequence at which each stops being accepted (#2786).
    pub fn get_outgoing_submitters(env: Env) -> Map<Address, u32> {
        load_outgoing(&env)
    }

    /// The current authorised submitter set, in insertion order.
    ///
    /// The admin can also submit and is not listed here.
    pub fn get_submitters(env: Env) -> Vec<Address> {
        load_submitters(&env)
    }

    // =========================================================================
    // State archival (#2599)
    // =========================================================================

    /// Extend the TTL of the contract instance, the snapshot store and every
    /// role entry, so anchored hashes stay verifiable and no role is silently
    /// revoked by archival (#2599, #2784).
    ///
    /// Role entries are found through the `RoleHolders` index; the admin is
    /// always included, even on deployments that predate the index. The
    /// submitter set lives in instance storage and is covered by the instance
    /// extension.
    /// Extend the TTL of the contract instance, the latest snapshot entry
    /// and the admin's role entry so anchored hashes stay verifiable.
    ///
    /// Older epochs are deliberately *not* bumped here: each lives in its own
    /// entry and may expire (#2781). Use [`Self::bump_snapshot_ttl`] to keep a
    /// range alive, or restore an archived entry (see README).
    ///
    /// Read-only calls (`get_snapshot`, `latest_snapshot`) also extend the
    /// snapshot store, but only when submitted as a transaction — a simulated
    /// read changes nothing on-chain. Run this periodically (well within
    /// `LEDGERS_TO_EXTEND`, ~30 days) in quiet periods with no submissions.
    ///
    /// Permissionless: it changes no state other than TTLs and the caller pays
    /// the fee. No event is emitted.
    pub fn bump_ttl(env: Env) {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
        let latest = Self::get_latest_epoch(env.clone());
        if latest > 0 {
            bump_persistent(&env, &DataKey::Snapshot(latest));
        }
        let holders = load_role_holders(&env);
        for holder in holders.iter() {
            bump_persistent(&env, &DataKey::Roles(holder));
        }
        if let Some(admin) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
        {
            if !holders.contains(&admin) {
                bump_persistent(&env, &DataKey::Roles(admin));
            }
        }
    }

    /// Every address holding a per-address role entry (the `RoleHolders`
    /// index `bump_ttl` walks). Submitters are listed by `get_submitters`.
    pub fn get_role_holders(env: Env) -> Vec<Address> {
        load_role_holders(&env)
    }

    /// Extend the TTL of every stored epoch in `from..=to` (#2781).
    /// Permissionless; the span is capped at `MAX_TTL_BUMP_SPAN` epochs so a
    /// call's footprint stays bounded. Missing epochs are skipped. Returns how
    /// many entries were extended.
    ///
    /// # Errors
    /// * `Error::InvalidRange` - `from > to` or the span exceeds the cap
    pub fn bump_snapshot_ttl(env: Env, from: u64, to: u64) -> Result<u32, Error> {
        if from > to || to - from >= MAX_TTL_BUMP_SPAN {
            return Err(Error::InvalidRange);
        }
        let mut bumped = 0u32;
        let mut epoch = from;
        loop {
            let key = DataKey::Snapshot(epoch);
            if env.storage().persistent().has(&key) {
                env.storage()
                    .persistent()
                    .extend_ttl(&key, LEDGERS_TO_EXTEND, LEDGERS_TO_EXTEND);
                bumped += 1;
            }
            if epoch == to {
                break;
            }
            epoch += 1;
        }
        Ok(bumped)
    }

    // =========================================================================
    // Role-Based Access Control (#2140)
    // =========================================================================

    /// Grant a role to an address. Only admin can grant roles.
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `caller` - Address attempting to grant role (must be admin)
    /// * `user` - Address to grant role to
    /// * `role` - Role to grant
    ///
    /// # Errors
    /// * `Error::Unauthorized` - If caller is not admin
    /// * `Error::AdminNotSet` - If admin not initialized
    /// * `Error::AdminRoleNotGrantable` - If `role` is `Role::Admin`
    pub fn grant_role(env: Env, caller: Address, user: Address, role: Role) -> Result<(), Error> {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if caller != admin {
            return Err(Error::Unauthorized);
        }

        // Exactly one admin at all times (#2785): Admin moves via set_admin
        // only, which revokes it from the previous holder.
        if role == Role::Admin {
            return Err(Error::AdminRoleNotGrantable);
        }

        // Submitter rights live only in the submitter set (#2600). Granting to
        // an existing member is a no-op, as it is for every other role.
        if role == Role::SnapshotSubmitter {
            match insert_submitter(&env, &user) {
                Ok(()) => emit_submitter_added(&env, caller.clone(), user.clone()),
                Err(Error::SubmitterAlreadyAuthorized) => {}
                Err(e) => return Err(e),
            }
            env.events().publish(
                (symbol_short!("role_grnt"), user.clone()),
                (caller, user, role),
            );
            return Ok(());
        }

        let mut roles = read_roles(&env, &user).unwrap_or_else(|| Vec::new(&env));

        if !roles.iter().any(|r| r == role) {
            roles.push_back(role);
        }

        write_roles(&env, &user, &roles)?;
        bump_instance(&env);

        env.events().publish(
            (symbol_short!("role_grnt"), user.clone()),
            (caller, user, role),
        );

        Ok(())
    }

    /// Revoke a role from an address. Only admin can revoke roles.
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `caller` - Address attempting to revoke role (must be admin)
    /// * `user` - Address to revoke role from
    /// * `role` - Role to revoke
    ///
    /// # Errors
    /// * `Error::Unauthorized` - If caller is not admin
    /// * `Error::AdminNotSet` - If admin not initialized
    pub fn revoke_role(env: Env, caller: Address, user: Address, role: Role) -> Result<(), Error> {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if caller != admin {
            return Err(Error::Unauthorized);
        }

        // Stripping the admin's own Admin role would leave the contract with
        // a stored admin that fails every role check (#2785).
        if role == Role::Admin && user == admin {
            return Err(Error::AdminRoleNotGrantable);
        }

        if role == Role::SnapshotSubmitter {
            match delete_submitter(&env, &user) {
                Ok(()) => {
                    emit_submitter_removed(&env, caller.clone(), user.clone());
                    env.events().publish(
                        (symbol_short!("role_rvk"), user.clone()),
                        (caller, user, role),
                    );
                }
                Err(Error::SubmitterNotFound) => {}
                Err(e) => return Err(e),
            }
            return Ok(());
        }

        if let Some(roles) = read_roles(&env, &user) {
            let mut new_roles = Vec::new(&env);
            for r in roles.iter() {
                if r != role {
                    new_roles.push_back(r);
                }
            }
            // An empty list removes the entry and drops `user` from the
            // role-holder index.
            write_roles(&env, &user, &new_roles)?;
            bump_instance(&env);

            env.events().publish(
                (symbol_short!("role_rvk"), user.clone()),
                (caller, user, role),
            );
        }

        Ok(())
    }

    /// Check if an address has a specific role
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `user` - Address to check
    /// * `role` - Role to check for
    ///
    /// # Returns
    /// * `true` if user has the role, `false` otherwise
    ///
    /// `Role::SnapshotSubmitter` is answered from the submitter set only;
    /// legacy per-address role entries for it are ignored (deny by default).
    ///
    /// Reading an existing role entry extends its TTL (#2784), so every
    /// authorisation check that goes through here keeps the entry alive.
    /// `Role::SnapshotSubmitter` is answered from the submitter set (plus
    /// outgoing keys inside their overlap window) only; legacy per-address
    /// role entries for it are ignored (deny by default).
    pub fn has_role(env: Env, user: Address, role: Role) -> bool {
        has_role_internal(&env, &user, role)
    }

    // =========================================================================
    // Contract Metadata
    // =========================================================================

    /// Get public contract metadata
    pub fn get_metadata(env: Env) -> PublicMetadata {
        PublicMetadata {
            name: String::from_str(&env, "PayRaider Core"),
            version: String::from_str(&env, VERSION),
            author: String::from_str(&env, "PayRaider Team"),
            description: String::from_str(
                &env,
                "Core analytics snapshot contract for Stellar network",
            ),
            repository: String::from_str(&env, "https://github.com/payraider/contracts"),
            license: String::from_str(&env, "MIT"),
        }
    }

    /// Get comprehensive contract information
    pub fn get_contract_info(env: Env) -> ContractInfo {
        let initialized = env.storage().instance().has(&DataKey::Admin);
        let admin = if initialized {
            match env.storage().instance().get(&DataKey::Admin) {
                Some(addr) => MaybeAddress::Some(addr),
                None => MaybeAddress::None,
            }
        } else {
            MaybeAddress::None
        };

        ContractInfo {
            metadata: Self::get_metadata(env.clone()),
            initialized,
            paused: env
                .storage()
                .instance()
                .get(&DataKey::Paused)
                .unwrap_or(false),
            admin,
            total_snapshots: env
                .storage()
                .instance()
                .get(&DataKey::LatestEpoch)
                .unwrap_or(0),
        }
    }
}

mod test;
