#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, BytesN, Env, Map, String,
    Vec,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorEvent {
    pub error_code: u32,
    pub error_message: String,
    pub function_name: String,
    pub caller: Address,
    pub timestamp: u64,
    pub ledger_sequence: u32,
    pub context: String,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractError {
    ContractPaused = 1,
    Unauthorized = 2,
    InvalidEpoch = 3,
    EpochAlreadyExists = 4,
    EpochMonotonicityViolated = 5,
    SnapshotImmutabilityViolated = 6,
}

fn emit_error_event(
    env: &Env,
    error: ContractError,
    function_name: &str,
    caller: &Address,
    context: &str,
) {
    let msg = match error {
        ContractError::ContractPaused => "Contract is paused",
        ContractError::Unauthorized => "Unauthorized caller",
        ContractError::InvalidEpoch => "Invalid epoch value",
        ContractError::EpochAlreadyExists => "Epoch already exists",
        ContractError::EpochMonotonicityViolated => "Epoch monotonicity violated",
        ContractError::SnapshotImmutabilityViolated => "Snapshot immutability violated",
    };
    env.events().publish(
        (symbol_short!("error"), caller.clone()),
        ErrorEvent {
            error_code: error as u32,
            error_message: String::from_str(env, msg),
            function_name: String::from_str(env, function_name),
            caller: caller.clone(),
            timestamp: env.ledger().timestamp(),
            ledger_sequence: env.ledger().sequence(),
            context: String::from_str(env, context),
        },
    );
}

const DEFAULT_SNAPSHOT_TTL: u64 = 7_776_000; // 90 days in seconds
const LEDGER_SECONDS: u64 = 5; // ~5 seconds per ledger

const RATE_LIMIT_WINDOW: u64 = 3600; // 1 hour
const MAX_CALLS_PER_WINDOW: u32 = 100;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotMetadata {
    pub epoch: u64,
    pub timestamp: u64,
    pub hash: BytesN<32>,
    pub submitter: Address,
    pub ledger_sequence: u32,
    pub expires_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitInfo {
    pub last_call: u64,
    pub call_count: u32,
    pub window_start: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotSubmittedEvent {
    pub epoch: u64,
    pub hash: BytesN<32>,
    pub submitter: Address,
    pub timestamp: u64,
    pub previous_epoch: u64,
    pub ledger_sequence: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseEvent {
    pub paused_by: Address,
    pub reason: String,
    pub timestamp: u64,
    pub ledger_sequence: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnpauseEvent {
    pub unpaused_by: Address,
    pub reason: String,
    pub timestamp: u64,
    pub ledger_sequence: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockAction {
    pub action_type: String,
    pub new_admin: Address,
    pub proposer: Address,
    pub proposed_at: u64,
    pub executable_at: u64,
    pub executed: bool,
}

const TIMELOCK_DELAY: u64 = 172800; // 48 hours in seconds

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigConfig {
    pub admins: Vec<Address>,
    pub threshold: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAction {
    pub action_id: u64,
    pub action_type: String,
    pub signatures: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Snapshots,
    LatestEpoch,
    Snapshot(u64),
    Paused,
    Governance,
    /// Auto-incrementing ID counter for timelock actions
    NextActionId,
    /// Timelock action keyed by action ID
    TimelockAction(u64),
    /// Per-caller rate limit tracking
    RateLimit(Address),
    Version,
    MultiSigConfig,
    PendingAction(u64),
}

fn check_rate_limit(env: &Env, caller: &Address) {
    let now = env.ledger().timestamp();

    let mut rate_info: RateLimitInfo = env
        .storage()
        .temporary()
        .get(&DataKey::RateLimit(caller.clone()))
        .unwrap_or(RateLimitInfo {
            last_call: 0,
            call_count: 0,
            window_start: now,
        });

    if now - rate_info.window_start > RATE_LIMIT_WINDOW {
        rate_info.call_count = 0;
        rate_info.window_start = now;
    }

    if rate_info.call_count >= MAX_CALLS_PER_WINDOW {
        panic!("Rate limit exceeded: too many calls in this window");
    }

    rate_info.call_count += 1;
    rate_info.last_call = now;

    env.storage()
        .temporary()
        .set(&DataKey::RateLimit(caller.clone()), &rate_info);
}

// ── Private helpers ──────────────────────────────────────────────────────────

/// Read the admin address; panics if the contract is not initialized.
fn require_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("Contract not initialized: admin not set"))
}

/// Validate epoch ordering and return the current latest epoch.
fn validate_epoch(env: &Env, epoch: u64) -> u64 {
    if epoch == 0 {
        panic!("Invalid epoch: must be greater than 0");
    }
    let latest: u64 = env
        .storage()
        .instance()
        .get(&DataKey::LatestEpoch)
        .unwrap_or(0);
    if epoch == latest {
        panic!("Snapshot for epoch {} already exists", epoch);
    }
    if epoch < latest {
        panic!(
            "Epoch monotonicity violated: epoch {} must be strictly greater than latest {}",
            epoch, latest
        );
    }
    latest
}

/// Write one snapshot to per-epoch persistent storage and update the shared map + latest epoch.
fn write_snapshot(
    env: &Env,
    epoch: u64,
    metadata: &SnapshotMetadata,
    snapshots: &mut Map<u64, SnapshotMetadata>,
) {
    env.storage()
        .persistent()
        .set(&DataKey::Snapshot(epoch), metadata);
    snapshots.set(epoch, metadata.clone());
    env.storage()
        .persistent()
        .set(&DataKey::Snapshots, snapshots);
    env.storage().instance().set(&DataKey::LatestEpoch, &epoch);
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct AnalyticsContract;

#[contractimpl]
impl AnalyticsContract {
    pub fn initialize(env: Env, admin: Address) {
        let storage = env.storage().instance();
        if storage.has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }
        storage.set(&DataKey::Admin, &admin);
        storage.set(&DataKey::LatestEpoch, &0u64);
        storage.set(&DataKey::Paused, &false);
        storage.set(&DataKey::Version, &VERSION);
        env.storage().persistent().set(
            &DataKey::Snapshots,
            &Map::<u64, SnapshotMetadata>::new(&env),
        );
    }

    /// Submit a single snapshot. Panics on invalid input or unauthorized caller.
    pub fn submit_snapshot(env: Env, epoch: u64, hash: BytesN<32>, caller: Address) -> u64 {
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if is_paused {
            emit_error_event(
                &env,
                ContractError::ContractPaused,
                "submit_snapshot",
                &caller,
                "Contract is paused for emergency maintenance",
            );
            panic!("Contract is paused for emergency maintenance");
        }

        caller.require_auth();

        check_rate_limit(&env, &caller);

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Contract not initialized: admin not set"));

        if caller != admin {
            emit_error_event(
                &env,
                ContractError::Unauthorized,
                "submit_snapshot",
                &caller,
                "Only the admin can submit snapshots",
            );
            panic!("Unauthorized: only the admin can submit snapshots");
        }

        if epoch == 0 {
            emit_error_event(
                &env,
                ContractError::InvalidEpoch,
                "submit_snapshot",
                &caller,
                "Epoch must be greater than 0",
            );
            panic!("Invalid epoch: must be greater than 0");
        }

        let latest: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);

        if epoch == latest {
            emit_error_event(
                &env,
                ContractError::EpochAlreadyExists,
                "submit_snapshot",
                &caller,
                "Snapshot for this epoch already exists",
            );
            panic!("Snapshot for epoch {} already exists", epoch);
        }
        if epoch < latest {
            emit_error_event(
                &env,
                ContractError::EpochMonotonicityViolated,
                "submit_snapshot",
                &caller,
                "Epoch must be strictly greater than the latest epoch",
            );
            panic!(
                "Epoch monotonicity violated: epoch {} must be strictly greater than latest {}",
                epoch, latest
            );
        }

        let timestamp = env.ledger().timestamp();
        let ledger_sequence = env.ledger().sequence();
        let metadata = SnapshotMetadata {
            epoch,
            timestamp,
            hash: hash.clone(),
            submitter: caller.clone(),
            ledger_sequence,
            expires_at: None,
        };

        let mut snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        write_snapshot(&env, epoch, &metadata, &mut snapshots);

        env.events().publish(
            (symbol_short!("snapshot"), caller),
            SnapshotSubmittedEvent {
                epoch,
                hash,
                submitter: metadata.submitter,
                timestamp,
                previous_epoch: latest,
                ledger_sequence,
            },
        );

        timestamp
    }

    /// Submit a batch of snapshots in a single call (single auth + admin check).
    /// Epochs must be provided in strictly increasing order.
    pub fn batch_submit(
        env: Env,
        snapshots_input: Vec<(u64, BytesN<32>)>,
        caller: Address,
    ) -> Vec<u64> {
        if env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
        {
            panic!("Contract is paused for emergency maintenance");
        }
        caller.require_auth();
        let admin = require_admin(&env);
        if caller != admin {
            panic!("Unauthorized: only the admin can submit snapshots");
        }

        let mut snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        let mut results = Vec::new(&env);
        for (epoch, hash) in snapshots_input.iter() {
            validate_epoch(&env, epoch);
            let timestamp = env.ledger().timestamp();
            let metadata = SnapshotMetadata {
                epoch,
                timestamp,
                hash,
                submitter: caller.clone(),
                ledger_sequence: env.ledger().sequence(),
                expires_at: None,
            };
            write_snapshot(&env, epoch, &metadata, &mut snapshots);
            results.push_back(timestamp);
        }
        results
    }

    /// Submit a snapshot with an optional TTL (defaults to 90 days).
    pub fn submit_snapshot_with_ttl(
        env: Env,
        epoch: u64,
        hash: BytesN<32>,
        caller: Address,
        ttl_seconds: Option<u64>,
    ) -> u64 {
        caller.require_auth();
        let admin = require_admin(&env);

        check_rate_limit(&env, &caller);

        if caller != admin {
            panic!("Unauthorized: only the admin can submit snapshots");
        }
        validate_epoch(&env, epoch);

        let timestamp = env.ledger().timestamp();
        let ttl = ttl_seconds.unwrap_or(DEFAULT_SNAPSHOT_TTL);
        let metadata = SnapshotMetadata {
            epoch,
            timestamp,
            hash,
            submitter: caller,
            ledger_sequence: env.ledger().sequence(),
            expires_at: Some(timestamp + ttl),
        };

        let mut snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));
        write_snapshot(&env, epoch, &metadata, &mut snapshots);

        // Extend per-epoch key's Soroban storage TTL
        let ledgers_to_live = (ttl / LEDGER_SECONDS) as u32;
        env.storage().persistent().extend_ttl(
            &DataKey::Snapshot(epoch),
            ledgers_to_live,
            ledgers_to_live,
        );

        timestamp
    }

    /// Remove expired snapshots from storage.
    /// Admin-only. Iterates up to `max_to_clean` epochs and removes those past expiry.
    /// Returns the number of snapshots cleaned.
    pub fn cleanup_expired_snapshots(env: Env, admin: Address, max_to_clean: u32) -> u32 {
        admin.require_auth();
        let stored_admin = require_admin(&env);
        if admin != stored_admin {
            panic!("Unauthorized: only the admin can clean up snapshots");
        }

        let now = env.ledger().timestamp();
        let mut cleaned = 0u32;

        let latest_epoch: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);

        let mut snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        for epoch in 1..=latest_epoch {
            if cleaned >= max_to_clean {
                break;
            }
            if let Some(metadata) = snapshots.get(epoch) {
                if let Some(expires_at) = metadata.expires_at {
                    if now > expires_at {
                        snapshots.remove(epoch);
                        env.storage()
                            .persistent()
                            .remove(&DataKey::Snapshot(epoch));
                        cleaned += 1;
                    }
                }
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);
        cleaned
    }

    /// Check whether a snapshot has expired.
    pub fn is_snapshot_expired(env: Env, epoch: u64) -> bool {
        match env
            .storage()
            .persistent()
            .get::<DataKey, SnapshotMetadata>(&DataKey::Snapshot(epoch))
        {
            Some(metadata) => match metadata.expires_at {
                Some(expires_at) => env.ledger().timestamp() > expires_at,
                None => false,
            },
            None => false,
        }
    }

    /// Get snapshot metadata for a specific epoch.
    pub fn get_snapshot(env: Env, epoch: u64) -> Option<SnapshotMetadata> {
        env.storage().persistent().get(&DataKey::Snapshot(epoch))
    }

    pub fn get_latest_snapshot(env: Env) -> Option<SnapshotMetadata> {
        let latest_epoch: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);
        if latest_epoch == 0 {
            return None;
        }
        env.storage()
            .persistent()
            .get(&DataKey::Snapshot(latest_epoch))
    }

    pub fn get_snapshot_history(env: Env) -> Map<u64, SnapshotMetadata> {
        env.storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env))
    }

    pub fn get_latest_epoch(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0)
    }

    pub fn get_all_epochs(env: Env) -> Vec<u64> {
        let snapshots = Self::get_snapshot_history(env.clone());
        let mut epochs = Vec::new(&env);
        for (epoch, _) in snapshots.iter() {
            epochs.push_back(epoch);
        }
        epochs
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    pub fn getversion(env: Env) -> String {
        String::from_str(&env, VERSION)
    }

    pub fn set_admin(env: Env, current_admin: Address, new_admin: Address) {
        current_admin.require_auth();
        let admin = require_admin(&env);
        if current_admin != admin {
            panic!("Unauthorized: only the current admin can set a new admin");
        }
        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }

    /// Emergency pause the contract.
    pub fn pause(env: Env, caller: Address, reason: String) {
        caller.require_auth();
        let admin = require_admin(&env);
        if caller != admin {
            panic!("Unauthorized: only the admin can pause the contract");
        }
        env.storage().instance().set(&DataKey::Paused, &true);

        env.events().publish(
            (symbol_short!("pause"), caller.clone()),
            PauseEvent {
                paused_by: caller,
                reason,
                timestamp: env.ledger().timestamp(),
                ledger_sequence: env.ledger().sequence(),
            },
        );
    }

    /// Unpause the contract.
    pub fn unpause(env: Env, caller: Address, reason: String) {
        caller.require_auth();
        let admin = require_admin(&env);
        if caller != admin {
            panic!("Unauthorized: only the admin can unpause the contract");
        }
        env.storage().instance().set(&DataKey::Paused, &false);

        env.events().publish(
            (symbol_short!("unpause"), caller.clone()),
            UnpauseEvent {
                unpaused_by: caller,
                reason,
                timestamp: env.ledger().timestamp(),
                ledger_sequence: env.ledger().sequence(),
            },
        );
    }

    pub fn set_governance(env: Env, caller: Address, governance: Address) {
        caller.require_auth();
        let admin = require_admin(&env);
        if caller != admin {
            panic!("Unauthorized: only the admin can set governance");
        }
        env.storage()
            .instance()
            .set(&DataKey::Governance, &governance);
    }

    pub fn get_governance(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Governance)
    }

    pub fn set_admin_by_governance(env: Env, caller: Address, new_admin: Address) {
        let governance: Address = env
            .storage()
            .instance()
            .get(&DataKey::Governance)
            .unwrap_or_else(|| panic!("Governance not set"));
        if caller != governance {
            panic!("Unauthorized: only the governance contract can set admin");
        }
        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }

    pub fn set_paused_by_governance(env: Env, caller: Address, paused: bool) {
        let governance: Address = env
            .storage()
            .instance()
            .get(&DataKey::Governance)
            .unwrap_or_else(|| panic!("Governance not set"));
        if caller != governance {
            panic!("Unauthorized: only the governance contract can set paused");
        }
        env.storage().instance().set(&DataKey::Paused, &paused);
    }

    /// Batch submit multiple snapshots in a single transaction.
    pub fn batch_submit_snapshots(
        env: Env,
        caller: Address,
        snapshots: Vec<(u64, BytesN<32>)>,
    ) -> Vec<u64> {
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if is_paused {
            panic!("Contract is paused for emergency maintenance");
        }

        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Contract not initialized: admin not set"));

        if caller != admin {
            panic!("Unauthorized: only the admin can submit snapshots");
        }

        let mut snapshots_map: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        let mut timestamps = Vec::new(&env);
        for (epoch, hash) in snapshots.iter() {
            validate_epoch(&env, epoch);

            let timestamp = env.ledger().timestamp();
            let metadata = SnapshotMetadata {
                epoch,
                timestamp,
                hash,
                submitter: caller.clone(),
                ledger_sequence: env.ledger().sequence(),
                expires_at: None,
            };
            write_snapshot(&env, epoch, &metadata, &mut snapshots_map);
            timestamps.push_back(timestamp);
        }

        env.events()
            .publish((symbol_short!("batch"), caller), snapshots.len());

        timestamps
    }

    /// Batch get multiple snapshots by epoch.
    pub fn batch_get_snapshots(env: Env, epochs: Vec<u64>) -> Vec<Option<SnapshotMetadata>> {
        let snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        let mut results = Vec::new(&env);
        for epoch in epochs.iter() {
            results.push_back(snapshots.get(epoch));
        }
        results
    }

    /// Propose an admin change with a 48-hour timelock.
    pub fn propose_admin_change(env: Env, proposer: Address, new_admin: Address) -> u64 {
        proposer.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Contract not initialized: admin not set"));

        if proposer != admin {
            panic!("Unauthorized: only the admin can propose changes");
        }

        let action_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextActionId)
            .unwrap_or(0u64);

        let now = env.ledger().timestamp();
        let action = TimelockAction {
            action_type: String::from_str(&env, "set_admin"),
            new_admin: new_admin.clone(),
            proposer: proposer.clone(),
            proposed_at: now,
            executable_at: now + TIMELOCK_DELAY,
            executed: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::TimelockAction(action_id), &action);
        env.storage()
            .instance()
            .set(&DataKey::NextActionId, &(action_id + 1));

        env.events().publish(
            (symbol_short!("propose"), proposer),
            (action_id, new_admin, action.executable_at),
        );

        action_id
    }

    /// Execute a timelock action after the delay has passed.
    pub fn execute_timelock_action(env: Env, executor: Address, action_id: u64) {
        executor.require_auth();

        let mut action: TimelockAction = env
            .storage()
            .persistent()
            .get(&DataKey::TimelockAction(action_id))
            .unwrap_or_else(|| panic!("Action not found"));

        if env.ledger().timestamp() < action.executable_at {
            panic!("Timelock not expired: action cannot be executed yet");
        }

        if action.executed {
            panic!("Action already executed");
        }

        env.storage()
            .instance()
            .set(&DataKey::Admin, &action.new_admin);

        action.executed = true;
        env.storage()
            .persistent()
            .set(&DataKey::TimelockAction(action_id), &action);

        env.events()
            .publish((symbol_short!("execute"), executor), action_id);
    }

    /// Cancel a pending timelock action. Only the current admin can cancel.
    pub fn cancel_timelock_action(env: Env, admin: Address, action_id: u64) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Contract not initialized: admin not set"));

        if admin != stored_admin {
            panic!("Unauthorized: only the admin can cancel actions");
        }

        env.storage()
            .persistent()
            .remove(&DataKey::TimelockAction(action_id));

        env.events()
            .publish((symbol_short!("cancel"), admin), action_id);
    }

    /// Get a timelock action by ID.
    pub fn get_timelock_action(env: Env, action_id: u64) -> Option<TimelockAction> {
        env.storage()
            .persistent()
            .get(&DataKey::TimelockAction(action_id))
    }

    /// Prune old snapshots, keeping only the last N epochs.
    /// Only admin can call this.
    /// Returns the number of snapshots removed.
    pub fn prune_old_snapshots(env: Env, caller: Address, keep_last_n: u32) -> u32 {
        caller.require_auth();
        let admin = require_admin(&env);
        if caller != admin {
            panic!("Unauthorized: only the admin can prune snapshots");
        }

        let latest_epoch: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);

        if latest_epoch <= keep_last_n as u64 {
            return 0;
        }

        let cutoff_epoch = latest_epoch - keep_last_n as u64;

        let mut snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        let mut removed = 0u32;
        for epoch in 1..=cutoff_epoch {
            if snapshots.contains_key(epoch) {
                snapshots.remove(epoch);
                env.storage()
                    .persistent()
                    .remove(&DataKey::Snapshot(epoch));
                removed += 1;
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);

        env.events().publish(
            (symbol_short!("prune"), caller),
            (removed, cutoff_epoch),
        );

        removed
    }

    /// Check if contract is paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    // =========================================================================
    // Multi-Sig Admin Support
    // =========================================================================

    /// Initialize multi-sig configuration with a list of admins and a signing threshold.
    /// Only the current single admin can call this.
    pub fn initialize_multisig(
        env: Env,
        caller: Address,
        admins: Vec<Address>,
        threshold: u32,
    ) {
        caller.require_auth();

        let admin = require_admin(&env);

        if caller != admin {
            panic!("Unauthorized: only the admin can initialize multisig");
        }

        if threshold == 0 || threshold > admins.len() {
            panic!("Invalid threshold: must be > 0 and <= number of admins");
        }

        let config = MultiSigConfig { admins, threshold };
        env.storage()
            .instance()
            .set(&DataKey::MultiSigConfig, &config);
    }

    /// Get the current multi-sig configuration.
    pub fn get_multisig_config(env: Env) -> Option<MultiSigConfig> {
        env.storage().instance().get(&DataKey::MultiSigConfig)
    }

    /// Propose a new multi-sig action. The proposer automatically adds their signature.
    /// Returns the new action_id.
    pub fn propose_action(
        env: Env,
        proposer: Address,
        action_type: String,
        _action_data: BytesN<32>,
    ) -> u64 {
        proposer.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .unwrap_or_else(|| panic!("MultiSig not initialized"));

        if !config.admins.contains(&proposer) {
            panic!("Unauthorized: proposer is not a multisig admin");
        }

        let action_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextActionId)
            .unwrap_or(0u64);

        let mut sigs = Vec::new(&env);
        sigs.push_back(proposer);

        let pending = PendingAction {
            action_id,
            action_type,
            signatures: sigs,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 86400,
        };

        env.storage()
            .persistent()
            .set(&DataKey::PendingAction(action_id), &pending);

        env.storage()
            .instance()
            .set(&DataKey::NextActionId, &(action_id + 1));

        action_id
    }

    /// Sign an existing pending action. Returns `true` if the threshold is now reached.
    pub fn sign_action(env: Env, signer: Address, action_id: u64) -> bool {
        signer.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .unwrap_or_else(|| panic!("MultiSig not initialized"));

        if !config.admins.contains(&signer) {
            panic!("Unauthorized: signer is not a multisig admin");
        }

        let mut pending: PendingAction = env
            .storage()
            .persistent()
            .get(&DataKey::PendingAction(action_id))
            .unwrap_or_else(|| panic!("Action not found"));

        if env.ledger().timestamp() > pending.expires_at {
            panic!("Action expired");
        }

        if !pending.signatures.contains(&signer) {
            pending.signatures.push_back(signer);
            env.storage()
                .persistent()
                .set(&DataKey::PendingAction(action_id), &pending);
        }

        pending.signatures.len() >= config.threshold
    }

    /// Get a pending action by ID.
    pub fn get_pending_action(env: Env, action_id: u64) -> Option<PendingAction> {
        env.storage()
            .persistent()
            .get(&DataKey::PendingAction(action_id))
    }

    /// Emergency withdrawal of tokens from the contract.
    /// Only the admin can call this, and the contract must be paused.
    pub fn emergency_withdraw(
        env: Env,
        admin: Address,
        token_addr: Address,
        amount: i128,
        recipient: Address,
    ) {
        admin.require_auth();
        let stored_admin = require_admin(&env);
        if admin != stored_admin {
            panic!("Unauthorized: only the admin can perform emergency withdrawal");
        }

        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if !paused {
            panic!("Contract must be paused for emergency withdrawal");
        }

        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);

        env.events().publish(
            (symbol_short!("emergency"), admin),
            (token_addr, amount, recipient),
        );
    }
}

#[cfg(test)]
mod tests;
