#![no_std]
// extern crate std;

mod errors;

pub use errors::Error;
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Map, String, Vec,
};

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

const VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Event payloads ────────────────────────────────────────────────────────────

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
pub struct SnapshotDiff {
    pub epoch_a: u64,
    pub epoch_b: u64,
    pub hash_match: bool,
    pub timestamp_diff: i64,
    pub submitter_match: bool,
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
pub struct BatchSubmitEvent {
    pub count: u32,
    pub first_epoch: u64,
    pub last_epoch: u64,
    pub submitter: Address,
    pub timestamp: u64,
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
pub struct AdminTransferEvent {
    pub previous_admin: Address,
    pub new_admin: Address,
    pub transferred_by: Address,
    pub timestamp: u64,
    pub ledger_sequence: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseInfo {
    pub paused: bool,
    pub reason: String,
    pub paused_at: u64,
    pub paused_by: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminChangedEvent {
    pub old_admin: Address,
    pub new_admin: Address,
    pub changed_by: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceChangedEvent {
    pub old_governance: Option<Address>,
    pub new_governance: Address,
    pub changed_by: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigInitializedEvent {
    pub admins: Vec<Address>,
    pub threshold: u32,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminChangeProposedEvent {
    pub action_id: u64,
    pub proposer: Address,
    pub new_admin: Address,
    pub executable_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockActionExecutedEvent {
    pub action_id: u64,
    pub executor: Address,
    pub new_admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockActionCancelledEvent {
    pub action_id: u64,
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotsPrunedEvent {
    pub removed_count: u32,
    pub cutoff_epoch: u64,
    pub caller: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockAction {
    pub action_type: String,
    pub action_data: Address,
    pub proposer: Address,
    pub proposed_at: u64,
    pub executable_at: u64,
    pub executed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotWithProof {
    pub metadata: SnapshotMetadata,
    pub proof: Vec<BytesN<32>>,
}

const TIMELOCK_DELAY: u64 = 172_800; // 48 hours in seconds

/// Multi-sig configuration: list of co-admins and the signing threshold.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigConfig {
    pub admins: Vec<Address>,
    pub threshold: u32,
}

/// An in-flight multi-sig action awaiting enough signatures.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAction {
    pub action_id: u64,
    pub action_type: String,
    pub signatures: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
}

/// Paginated result for snapshot queries.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginatedSnapshots {
    pub snapshots: Vec<SnapshotMetadata>,
    pub total_count: u64,
    pub has_more: bool,
    pub next_cursor: Option<u64>,
}

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    Snapshots,
    LatestEpoch,
    Snapshot(u64),
    Paused,
    PauseInfo,
    Governance,
    NextActionId,
    TimelockAction(u64),
    RateLimit(Address),
    Version,
    /// Multi-sig admin configuration
    MultiSigConfig,
    /// Pending multi-sig action keyed by action ID
    PendingAction(u64),
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn check_rate_limit(env: &Env, caller: &Address) -> Result<(), Error> {
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
        return Err(Error::RateLimitExceeded
            .log_context(env, "check_rate_limit: too many calls in this window"));
    }

    rate_info.call_count += 1;
    rate_info.last_call = now;

    env.storage()
        .temporary()
        .set(&DataKey::RateLimit(caller.clone()), &rate_info);

    Ok(())
}

/// Read the admin address; returns `Err(Error::NotInitialized)` if not set.
fn require_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or_else(|| Error::NotInitialized.log_context(env, "require_admin: admin not set"))
}

/// Check if contract has been initialized.
fn require_initialized(env: &Env) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(
            Error::NotInitialized.log_context(env, "require_initialized: contract not initialized")
        );
    }
    Ok(())
}

/// Validate epoch ordering; returns the current latest epoch on success.
fn validate_epoch(env: &Env, epoch: u64) -> Result<u64, Error> {
    if epoch == 0 {
        return Err(Error::InvalidEpochZero.log_context(env, "validate_epoch: epoch must be > 0"));
    }
    let latest: u64 = env
        .storage()
        .instance()
        .get(&DataKey::LatestEpoch)
        .unwrap_or(0);
    if epoch == latest {
        return Err(Error::DuplicateEpoch.log_context(
            env,
            "validate_epoch: snapshot for this epoch already exists",
        ));
    }
    if epoch < latest {
        return Err(Error::EpochMonotonicityViolated.log_context(
            env,
            "validate_epoch: epoch must be strictly greater than latest",
        ));
    }
    Ok(latest)
}

/// Write one snapshot to per-epoch persistent storage and update the shared map + latest epoch.
const LEDGERS_TO_EXTEND: u32 = 518_400; // ~30 days at 5s per ledger

fn write_snapshot(
    env: &Env,
    epoch: u64,
    metadata: &SnapshotMetadata,
    snapshots: &mut Map<u64, SnapshotMetadata>,
) {
    env.storage()
        .persistent()
        .set(&DataKey::Snapshot(epoch), metadata);
    env.storage().persistent().extend_ttl(
        &DataKey::Snapshot(epoch),
        LEDGERS_TO_EXTEND,
        LEDGERS_TO_EXTEND,
    );
    snapshots.set(epoch, metadata.clone());
    env.storage()
        .persistent()
        .set(&DataKey::Snapshots, snapshots);
    env.storage().persistent().extend_ttl(
        &DataKey::Snapshots,
        LEDGERS_TO_EXTEND,
        LEDGERS_TO_EXTEND,
    );
    env.storage().instance().set(&DataKey::LatestEpoch, &epoch);
}

fn get_next_action_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextActionId)
        .unwrap_or(0);
    env.storage()
        .instance()
        .set(&DataKey::NextActionId, &(id + 1));
    id
}

// ── Contract metadata types ───────────────────────────────────────────────────

/// Extended contract metadata for public disclosure.
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

/// Contract info combining metadata with runtime state.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractInfo {
    pub metadata: PublicMetadata,
    pub initialized: bool,
    pub paused: bool,
    pub admin: Option<Address>,
    pub total_snapshots: u64,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct AnalyticsContract;

#[contractimpl]
impl AnalyticsContract {
    /// Initialize the contract with an admin address.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized
                .log_context(&env, "initialize: contract already initialized"));
        }
        let storage = env.storage().instance();
        storage.set(&DataKey::Admin, &admin);
        storage.set(&DataKey::LatestEpoch, &0u64);
        storage.set(&DataKey::Paused, &false);
        storage.set(&DataKey::Version, &VERSION);
        env.storage().persistent().set(
            &DataKey::Snapshots,
            &Map::<u64, SnapshotMetadata>::new(&env),
        );
        Ok(())
    }

    /// Submit a single snapshot. Returns the ledger timestamp on success.
    pub fn submit_snapshot(
        env: Env,
        epoch: u64,
        hash: BytesN<32>,
        caller: Address,
    ) -> Result<u64, Error> {
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if is_paused {
            return Err(
                Error::ContractPaused.log_context(&env, "submit_snapshot: contract is paused")
            );
        }

        caller.require_auth();
        check_rate_limit(&env, &caller)?;

        let admin = require_admin(&env)?;
        if caller != admin {
            return Err(
                Error::Unauthorized.log_context(&env, "submit_snapshot: caller is not the admin")
            );
        }

        let latest = validate_epoch(&env, epoch)?;

        // ─────────────────────────────────────────────────────────────────────
        // Validate hash is not all zeros (security-critical)
        // ─────────────────────────────────────────────────────────────────────
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        if hash == zero_hash {
            return Err(Error::InvalidHashZero
                .log_context(&env, "submit_snapshot: hash must not be all zeros"));
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

        Ok(timestamp)
    }

    /// Submit a batch of snapshots in a single call.
    /// Epochs must be provided in strictly increasing order.
    pub fn batch_submit(
        env: Env,
        snapshots_input: Vec<(u64, BytesN<32>)>,
        caller: Address,
    ) -> Result<Vec<u64>, Error> {
        Self::batch_submit_snapshots(env, caller, snapshots_input)
    }

    /// Batch submit multiple snapshots
    pub fn batch_submit_snapshots(
        env: Env,
        caller: Address,
        snapshots: Vec<(u64, BytesN<32>)>,
    ) -> Result<Vec<u64>, Error> {
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if is_paused {
            return Err(Error::ContractPaused
                .log_context(&env, "batch_submit_snapshots: contract is paused"));
        }

        caller.require_auth();
        check_rate_limit(&env, &caller)?;

        let admin = require_admin(&env)?;
        if caller != admin {
            return Err(Error::Unauthorized
                .log_context(&env, "batch_submit_snapshots: caller is not the admin"));
        }

        let mut snapshots_map: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        let mut results = Vec::new(&env);
        for (epoch, hash) in snapshots.iter() {
            let previous_epoch = validate_epoch(&env, epoch)?;
            let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
            if hash == zero_hash {
                return Err(Error::InvalidHashZero
                    .log_context(&env, "batch_submit_snapshots: hash must not be all zeros"));
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
            write_snapshot(&env, epoch, &metadata, &mut snapshots_map);
            env.events().publish(
                (symbol_short!("snapshot"), caller.clone()),
                SnapshotSubmittedEvent {
                    epoch,
                    hash,
                    submitter: caller.clone(),
                    timestamp,
                    previous_epoch,
                    ledger_sequence,
                },
            );
            results.push_back(timestamp);
        }

        // Emit batch event
        env.events()
            .publish((symbol_short!("batch"), caller), snapshots.len());

        Ok(results)
    }

    /// Submit a snapshot with an optional TTL (defaults to 90 days).
    pub fn submit_snapshot_with_ttl(
        env: Env,
        epoch: u64,
        hash: BytesN<32>,
        caller: Address,
        ttl_seconds: Option<u64>,
    ) -> Result<u64, Error> {
        caller.require_auth();
        let admin = require_admin(&env)?;
        check_rate_limit(&env, &caller)?;

        if caller != admin {
            return Err(Error::Unauthorized
                .log_context(&env, "submit_snapshot_with_ttl: caller is not the admin"));
        }
        validate_epoch(&env, epoch)?;

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

        let ledgers_to_live = (ttl / LEDGER_SECONDS) as u32;
        env.storage().persistent().extend_ttl(
            &DataKey::Snapshot(epoch),
            ledgers_to_live,
            ledgers_to_live,
        );

        Ok(timestamp)
    }

    /// Remove expired snapshots from storage. Admin-only.
    /// Returns the number of snapshots cleaned.
    pub fn cleanup_expired_snapshots(
        env: Env,
        admin: Address,
        max_to_clean: u32,
    ) -> Result<u32, Error> {
        admin.require_auth();
        let stored_admin = require_admin(&env)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized
                .log_context(&env, "cleanup_expired_snapshots: caller is not the admin"));
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
                        env.storage().persistent().remove(&DataKey::Snapshot(epoch));
                        cleaned += 1;
                    }
                }
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);
        Ok(cleaned)
    }

    /// Check whether a snapshot has expired.
    pub fn is_snapshot_expired(env: Env, epoch: u64) -> Result<bool, Error> {
        require_initialized(&env)?;
        match env
            .storage()
            .persistent()
            .get::<DataKey, SnapshotMetadata>(&DataKey::Snapshot(epoch))
        {
            Some(metadata) => match metadata.expires_at {
                Some(expires_at) => Ok(env.ledger().timestamp() > expires_at),
                None => Ok(false),
            },
            None => Ok(false),
        }
    }

    pub fn get_snapshot(env: Env, epoch: u64) -> Result<Option<SnapshotMetadata>, Error> {
        require_initialized(&env)?;
        if env.storage().persistent().has(&DataKey::Snapshot(epoch)) {
            env.storage().persistent().extend_ttl(
                &DataKey::Snapshot(epoch),
                LEDGERS_TO_EXTEND,
                LEDGERS_TO_EXTEND,
            );
        }
        Ok(env.storage().persistent().get(&DataKey::Snapshot(epoch)))
    }

    /// Get the latest snapshot metadata.
    pub fn get_latest_snapshot(env: Env) -> Result<Option<SnapshotMetadata>, Error> {
        require_initialized(&env)?;
        let latest_epoch: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);
        if latest_epoch == 0 {
            return Ok(None);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::Snapshot(latest_epoch)))
    }

    /// Get the entire snapshot history.
    pub fn get_snapshot_history(env: Env) -> Result<Map<u64, SnapshotMetadata>, Error> {
        require_initialized(&env)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env)))
    }

    /// Get the latest epoch submitted.
    pub fn get_latest_epoch(env: Env) -> Result<u64, Error> {
        require_initialized(&env)?;
        Ok(env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0))
    }

    /// Get all submitted epochs.
    pub fn get_all_epochs(env: Env) -> Result<Vec<u64>, Error> {
        require_initialized(&env)?;
        let snapshots = Self::get_snapshot_history(env.clone())?;
        let mut epochs = Vec::new(&env);
        for (epoch, _) in snapshots.iter() {
            epochs.push_back(epoch);
        }
        Ok(epochs)
    }

    /// Comparison functionality for snapshots
    pub fn compare_snapshots(env: Env, epoch_a: u64, epoch_b: u64) -> Result<SnapshotDiff, Error> {
        require_initialized(&env)?;
        let snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .ok_or(Error::NotInitialized)?;

        let snapshot_a = snapshots.get(epoch_a).ok_or(Error::SnapshotNotFound)?;
        let snapshot_b = snapshots.get(epoch_b).ok_or(Error::SnapshotNotFound)?;

        Ok(SnapshotDiff {
            epoch_a,
            epoch_b,
            hash_match: snapshot_a.hash == snapshot_b.hash,
            timestamp_diff: (snapshot_b.timestamp as i64) - (snapshot_a.timestamp as i64),
            submitter_match: snapshot_a.submitter == snapshot_b.submitter,
        })
    }

    /// Verify monotonicity and integrity of snapshot chain
    pub fn verify_snapshot_chain(
        env: Env,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<bool, Error> {
        require_initialized(&env)?;
        for epoch in start_epoch..end_epoch {
            let current = Self::get_snapshot(env.clone(), epoch)?.ok_or(Error::SnapshotNotFound)?;
            let next =
                Self::get_snapshot(env.clone(), epoch + 1)?.ok_or(Error::SnapshotNotFound)?;

            if next.timestamp <= current.timestamp {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Batch get multiple snapshots
    pub fn batch_get_snapshots(
        env: Env,
        epochs: Vec<u64>,
    ) -> Result<Vec<Option<SnapshotMetadata>>, Error> {
        require_initialized(&env)?;
        let snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        let mut results = Vec::new(&env);

        for epoch in epochs.iter() {
            let metadata = snapshots.get(epoch);
            results.push_back(metadata);
        }

        Ok(results)
    }

    /// Returns a paginated page of snapshots ordered by epoch.
    pub fn get_snapshots_paginated(
        env: Env,
        limit: u32,
        cursor: Option<u64>,
    ) -> Result<PaginatedSnapshots, Error> {
        require_initialized(&env)?;
        let snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        let start_epoch = cursor.unwrap_or(1);
        let latest_epoch: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);

        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut next_cursor: Option<u64> = None;

        for epoch in start_epoch..=latest_epoch {
            if count >= limit {
                next_cursor = Some(epoch);
                break;
            }
            if let Some(metadata) = snapshots.get(epoch) {
                results.push_back(metadata);
                count += 1;
            }
        }

        Ok(PaginatedSnapshots {
            snapshots: results,
            total_count: u64::from(snapshots.len()),
            has_more: next_cursor.is_some(),
            next_cursor,
        })
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        require_initialized(&env)?;
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)
    }

    pub fn getversion(env: Env) -> String {
        String::from_str(&env, VERSION)
    }

    pub fn set_admin(env: Env, current_admin: Address, new_admin: Address) -> Result<(), Error> {
        current_admin.require_auth();
        let old_admin = require_admin(&env)?;
        if current_admin != old_admin {
            return Err(
                Error::Unauthorized.log_context(&env, "set_admin: caller is not the current admin")
            );
        }

        let previous_admin = old_admin.clone();
        env.storage().instance().set(&DataKey::Admin, &new_admin);

        // ✅ EMIT DETAILED EVENT for audit trail
        env.events().publish(
            (symbol_short!("admin"), new_admin.clone()),
            AdminTransferEvent {
                previous_admin: previous_admin.clone(),
                new_admin: new_admin.clone(),
                transferred_by: current_admin.clone(),
                timestamp: env.ledger().timestamp(),
                ledger_sequence: env.ledger().sequence(),
            },
        );

        env.events().publish(
            (symbol_short!("admin"), new_admin.clone()),
            AdminChangedEvent {
                old_admin,
                new_admin,
                changed_by: current_admin,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    /// Emergency pause the contract.
    pub fn pause(env: Env, caller: Address, reason: String) -> Result<(), Error> {
        caller.require_auth();
        let admin = require_admin(&env)?;
        if caller != admin {
            return Err(Error::Unauthorized.log_context(&env, "pause: caller is not the admin"));
        }

        let timestamp = env.ledger().timestamp();

        // Store structured pause info for transparency
        let pause_info = PauseInfo {
            paused: true,
            reason: reason.clone(),
            paused_at: timestamp,
            paused_by: caller.clone(),
        };
        env.storage()
            .instance()
            .set(&DataKey::PauseInfo, &pause_info);
        env.storage().instance().set(&DataKey::Paused, &true);

        env.events().publish(
            (symbol_short!("pause"), caller.clone()),
            PauseEvent {
                paused_by: caller,
                reason,
                timestamp,
                ledger_sequence: env.ledger().sequence(),
            },
        );
        Ok(())
    }

    /// Unpause the contract.
    pub fn unpause(env: Env, caller: Address, reason: String) -> Result<(), Error> {
        caller.require_auth();
        let admin = require_admin(&env)?;
        if caller != admin {
            return Err(Error::Unauthorized.log_context(&env, "unpause: caller is not the admin"));
        }

        let timestamp = env.ledger().timestamp();

        // Update pause info to reflect the unpaused state
        let pause_info = PauseInfo {
            paused: false,
            reason: reason.clone(),
            paused_at: timestamp,
            paused_by: caller.clone(),
        };
        env.storage()
            .instance()
            .set(&DataKey::PauseInfo, &pause_info);
        env.storage().instance().set(&DataKey::Paused, &false);

        env.events().publish(
            (symbol_short!("unpause"), caller.clone()),
            UnpauseEvent {
                unpaused_by: caller,
                reason,
                timestamp,
                ledger_sequence: env.ledger().sequence(),
            },
        );
        Ok(())
    }

    /// Upgrade the contract Wasm. Admin-only.
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        // Only admin can upgrade
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        admin.require_auth();

        // Verify contract is not paused
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);

        if paused {
            return Err(Error::ContractPaused);
        }

        // Perform upgrade
        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());

        // Emit event
        env.events()
            .publish((symbol_short!("upgrade"),), (admin, new_wasm_hash));

        Ok(())
    }

    pub fn set_governance(env: Env, caller: Address, governance: Address) -> Result<(), Error> {
        caller.require_auth();
        let admin = require_admin(&env)?;
        if caller != admin {
            return Err(
                Error::Unauthorized.log_context(&env, "set_governance: caller is not the admin")
            );
        }

        let old_governance: Option<Address> = env.storage().instance().get(&DataKey::Governance);
        env.storage()
            .instance()
            .set(&DataKey::Governance, &governance);

        env.events().publish(
            (symbol_short!("gov"), governance.clone()),
            GovernanceChangedEvent {
                old_governance,
                new_governance: governance,
                changed_by: caller,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn get_governance(env: Env) -> Result<Option<Address>, Error> {
        require_initialized(&env)?;
        Ok(env.storage().instance().get(&DataKey::Governance))
    }

    pub fn set_admin_by_governance(
        env: Env,
        caller: Address,
        new_admin: Address,
    ) -> Result<(), Error> {
        let governance: Address = env
            .storage()
            .instance()
            .get(&DataKey::Governance)
            .ok_or_else(|| {
                Error::GovernanceNotSet
                    .log_context(&env, "set_admin_by_governance: governance not set")
            })?;
        if caller != governance {
            return Err(Error::Unauthorized.log_context(
                &env,
                "set_admin_by_governance: caller is not the governance contract",
            ));
        }

        let old_admin = require_admin(&env)?;
        env.storage().instance().set(&DataKey::Admin, &new_admin);

        env.events().publish(
            (symbol_short!("admin"), new_admin.clone()),
            AdminChangedEvent {
                old_admin,
                new_admin,
                changed_by: caller,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn set_paused_by_governance(env: Env, caller: Address, paused: bool) -> Result<(), Error> {
        let governance: Address = env
            .storage()
            .instance()
            .get(&DataKey::Governance)
            .ok_or_else(|| {
                Error::GovernanceNotSet
                    .log_context(&env, "set_paused_by_governance: governance not set")
            })?;
        if caller != governance {
            return Err(Error::Unauthorized.log_context(
                &env,
                "set_paused_by_governance: caller is not the governance contract",
            ));
        }
        env.storage().instance().set(&DataKey::Paused, &paused);

        if paused {
            env.events().publish(
                (symbol_short!("pause"), caller.clone()),
                PauseEvent {
                    paused_by: caller,
                    reason: String::from_str(&env, "Paused by governance"),
                    timestamp: env.ledger().timestamp(),
                    ledger_sequence: env.ledger().sequence(),
                },
            );
        } else {
            env.events().publish(
                (symbol_short!("unpause"), caller.clone()),
                UnpauseEvent {
                    unpaused_by: caller,
                    reason: String::from_str(&env, "Unpaused by governance"),
                    timestamp: env.ledger().timestamp(),
                    ledger_sequence: env.ledger().sequence(),
                },
            );
        }

        Ok(())
    }

    /// Propose an admin change with a 48-hour timelock.
    pub fn propose_admin_change(
        env: Env,
        proposer: Address,
        new_admin: Address,
    ) -> Result<u64, Error> {
        proposer.require_auth();
        check_rate_limit(&env, &proposer)?;

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if proposer != admin {
            return Err(Error::Unauthorized);
        }

        let action_id = get_next_action_id(&env);
        let now = env.ledger().timestamp();

        let action = TimelockAction {
            action_type: String::from_str(&env, "set_admin"),
            action_data: new_admin.clone(),
            proposer: proposer.clone(),
            proposed_at: now,
            executable_at: now + TIMELOCK_DELAY,
            executed: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::TimelockAction(action_id), &action);

        // Emit event
        env.events().publish(
            (symbol_short!("propose"), proposer),
            (action_id, new_admin, action.executable_at),
        );

        Ok(action_id)
    }

    pub fn execute_timelock_action(
        env: Env,
        executor: Address,
        action_id: u64,
    ) -> Result<(), Error> {
        executor.require_auth();

        let mut action: TimelockAction = env
            .storage()
            .persistent()
            .get(&DataKey::TimelockAction(action_id))
            .ok_or(Error::ActionNotFound)?;

        // ✅ Check timelock has passed
        if env.ledger().timestamp() < action.executable_at {
            return Err(Error::TimelockNotExpired);
        }

        // Check not already executed
        if action.executed {
            return Err(Error::ActionAlreadyExecuted);
        }

        // Execute action based on type
        // Use == instead of .as_str() because soroban_sdk::String does not have .as_str()
        if action.action_type == String::from_str(&env, "set_admin") {
            let new_admin = action.action_data.clone();
            env.storage().instance().set(&DataKey::Admin, &new_admin);
        } else {
            return Err(Error::UnknownActionType);
        }

        // Mark as executed
        action.executed = true;
        env.storage()
            .persistent()
            .set(&DataKey::TimelockAction(action_id), &action);

        // Emit event
        env.events()
            .publish((symbol_short!("execute"), executor), action_id);

        Ok(())
    }

    pub fn cancel_timelock_action(env: Env, admin: Address, action_id: u64) -> Result<(), Error> {
        admin.require_auth();

        // Only admin can cancel
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Remove action
        env.storage()
            .persistent()
            .remove(&DataKey::TimelockAction(action_id));

        // Emit event
        env.events()
            .publish((symbol_short!("cancel"), admin), action_id);

        Ok(())
    }

    pub fn get_timelock_action(env: Env, action_id: u64) -> Result<Option<TimelockAction>, Error> {
        require_initialized(&env)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::TimelockAction(action_id)))
    }

    /// Prune old snapshots, keeping only the last N epochs. Admin-only.
    /// Returns the number of snapshots removed.
    pub fn prune_old_snapshots(env: Env, caller: Address, keep_last_n: u32) -> Result<u32, Error> {
        caller.require_auth();
        let admin = require_admin(&env)?;
        if caller != admin {
            return Err(Error::Unauthorized
                .log_context(&env, "prune_old_snapshots: caller is not the admin"));
        }

        let latest_epoch: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);

        if latest_epoch <= keep_last_n as u64 {
            return Ok(0);
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
                env.storage().persistent().remove(&DataKey::Snapshot(epoch));
                removed += 1;
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);

        env.events().publish(
            (symbol_short!("prune"), caller.clone()),
            SnapshotsPrunedEvent {
                removed_count: removed,
                cutoff_epoch,
                caller,
            },
        );

        Ok(removed)
    }

    pub fn is_paused(env: Env) -> Result<bool, Error> {
        require_initialized(&env)?;
        Ok(env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false))
    }

    /// Get detailed pause information including reason, timestamp, and who paused.
    /// Returns `None` if the contract has never been paused.
    pub fn get_pause_info(env: Env) -> Option<PauseInfo> {
        env.storage().instance().get(&DataKey::PauseInfo)
    }

    // =========================================================================
    // Multi-Sig Admin Support
    // =========================================================================

    /// Initialize multi-sig configuration.
    pub fn initialize_multisig(
        env: Env,
        admins: Vec<Address>,
        threshold: u32,
    ) -> Result<(), Error> {
        if threshold == 0 || threshold > admins.len() {
            return Err(Error::InvalidThreshold.log_context(
                &env,
                "initialize_multisig: threshold must be between 1 and number of admins",
            ));
        }

        let config = MultiSigConfig {
            admins: admins.clone(),
            threshold,
        };
        env.storage()
            .instance()
            .set(&DataKey::MultiSigConfig, &config);

        env.events().publish(
            (symbol_short!("multisig"), symbol_short!("init")),
            MultiSigInitializedEvent {
                admins,
                threshold,
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    pub fn get_multisig_config(env: Env) -> Result<Option<MultiSigConfig>, Error> {
        require_initialized(&env)?;
        Ok(env.storage().instance().get(&DataKey::MultiSigConfig))
    }

    /// Propose a new multi-sig action. The proposer automatically adds their signature.
    /// Returns the new action_id.
    pub fn propose_action(
        env: Env,
        proposer: Address,
        action_type: String,
        _action_data: BytesN<32>,
    ) -> Result<u64, Error> {
        proposer.require_auth();
        check_rate_limit(&env, &proposer)?;

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or_else(|| {
                Error::MultiSigNotInitialized
                    .log_context(&env, "propose_action: multisig not initialized")
            })?;

        if !config.admins.contains(&proposer) {
            return Err(Error::SignerNotAdmin
                .log_context(&env, "propose_action: proposer is not a multisig admin"));
        }

        let action_id = get_next_action_id(&env);

        let mut sigs = Vec::new(&env);
        sigs.push_back(proposer.clone());

        let pending = PendingAction {
            action_id,
            action_type,
            signatures: sigs,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 86_400, // 24 hours
        };

        env.storage()
            .persistent()
            .set(&DataKey::PendingAction(action_id), &pending);

        Ok(action_id)
    }

    /// Sign an existing pending action. Returns `true` if the threshold is now reached.
    pub fn sign_action(env: Env, signer: Address, action_id: u64) -> Result<bool, Error> {
        signer.require_auth();
        check_rate_limit(&env, &signer)?;

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or_else(|| {
                Error::MultiSigNotInitialized
                    .log_context(&env, "sign_action: multisig not initialized")
            })?;

        if !config.admins.contains(&signer) {
            return Err(Error::SignerNotAdmin
                .log_context(&env, "sign_action: signer is not a multisig admin"));
        }

        let mut pending: PendingAction = env
            .storage()
            .persistent()
            .get(&DataKey::PendingAction(action_id))
            .ok_or_else(|| {
                Error::ActionNotFound.log_context(&env, "sign_action: action not found")
            })?;

        if env.ledger().timestamp() > pending.expires_at {
            return Err(Error::ActionExpired.log_context(&env, "sign_action: action has expired"));
        }

        if !pending.signatures.contains(&signer) {
            pending.signatures.push_back(signer);
            env.storage()
                .persistent()
                .set(&DataKey::PendingAction(action_id), &pending);
        }

        Ok(pending.signatures.len() >= config.threshold)
    }

    /// Get a pending action by ID.
    pub fn get_pending_action(env: Env, action_id: u64) -> Result<Option<PendingAction>, Error> {
        require_initialized(&env)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::PendingAction(action_id)))
    }

    // =========================================================================
    // Contract Metadata
    // =========================================================================

    pub fn get_metadata(env: Env) -> PublicMetadata {
        PublicMetadata {
            name: String::from_str(&env, "Stellar Insights Analytics"),
            version: String::from_str(&env, VERSION),
            author: String::from_str(&env, "Stellar Insights Team"),
            description: String::from_str(
                &env,
                "Advanced analytics and data aggregation contract for Stellar network",
            ),
            repository: String::from_str(&env, "https://github.com/stellar-insights/contracts"),
            license: String::from_str(&env, "MIT"),
        }
    }

    pub fn get_contract_info(env: Env) -> ContractInfo {
        ContractInfo {
            metadata: Self::get_metadata(env.clone()),
            initialized: env.storage().instance().has(&DataKey::Admin),
            paused: env
                .storage()
                .instance()
                .get(&DataKey::Paused)
                .unwrap_or(false),
            admin: env.storage().instance().get(&DataKey::Admin),
            total_snapshots: env
                .storage()
                .instance()
                .get(&DataKey::LatestEpoch)
                .unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fuzz_tests;
