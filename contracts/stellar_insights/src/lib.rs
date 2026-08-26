#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{emit_admin_transferred, emit_contract_initialized, emit_contract_paused, emit_contract_unpaused, emit_snapshot_submitted};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Map, String};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ~30 days at 5 s/ledger
const LEDGERS_TO_EXTEND: u32 = 518_400;
const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

/// Storage keys for persistent contract data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Administrator address authorized to submit snapshots
    Admin,
    /// Map of epoch -> snapshot hash
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
}

/// Schema version this build reads and writes.
///
/// Bumped whenever the *shape* of stored data changes — not on every release.
/// `migrate()` moves storage from whatever version it is on up to this one, and
/// refuses to run against anything newer than it understands.
pub const CURRENT_STORAGE_VERSION: u32 = 1;

/// Analytics snapshot data structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    /// SHA-256 hash of analytics data
    pub hash: BytesN<32>,
    /// Epoch identifier
    pub epoch: u64,
    /// Ledger timestamp when recorded
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
pub struct StellarInsightsContract;

#[contractimpl]
impl StellarInsightsContract {
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
    /// Only the authorized admin can call this function. Each epoch can only
    /// have one snapshot submitted. Upon successful submission, an event is
    /// emitted for verification purposes.
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
    /// * `Error::UnauthorizedCaller` - If caller is not the admin
    /// * `Error::InvalidEpoch` - If epoch is 0
    /// * `Error::DuplicateEpoch` - If snapshot already exists for this epoch
    /// * `Error::EpochMonotonicityViolated` - If epoch <= latest (out-of-order submission)
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

        // Get admin address from storage
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        // Verify caller is the admin
        if caller != admin {
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

        // Get existing snapshots map or create new one
        let mut snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        // Check for duplicate epoch
        if snapshots.contains_key(epoch) {
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

        // Create snapshot entry
        let snapshot = Snapshot {
            hash: hash.clone(),
            epoch,
            timestamp,
        };

        // Store snapshot
        snapshots.set(epoch, snapshot);
        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);

        // Extend storage TTL (~30 days at 5s per ledger)
        env.storage().persistent().extend_ttl(
            &DataKey::Snapshots,
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

        env.storage().instance().set(&DataKey::LatestEpoch, &epoch);

        // Emit structured event for off-chain indexing
        // Event payload matches stored data exactly:
        // - hash: same as snapshot.hash
        // - epoch: same as snapshot.epoch
        // - timestamp: same as snapshot.timestamp
        // - submitter: the authenticated caller
        emit_snapshot_submitted(&env, hash, epoch, timestamp, caller);

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
        // Extend TTL on read to keep data alive
        if env.storage().persistent().has(&DataKey::Snapshots) {
            env.storage().persistent().extend_ttl(
                &DataKey::Snapshots,
                LEDGERS_TO_EXTEND,
                LEDGERS_TO_EXTEND,
            );
        }
        let snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        snapshots
            .get(epoch)
            .map(|s| s.hash)
            .ok_or(Error::SnapshotNotFound)
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

        if env.storage().persistent().has(&DataKey::Snapshots) {
            env.storage().persistent().extend_ttl(
                &DataKey::Snapshots,
                LEDGERS_TO_EXTEND,
                LEDGERS_TO_EXTEND,
            );
        }

        let snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        let snapshot = snapshots.get(latest_epoch).ok_or(Error::SnapshotNotFound)?;

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

    /// Transfer admin ownership to a new address.
    ///
    /// Only the current admin can call this function.
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `caller` - Current admin address (must match stored admin)
    /// * `new_admin` - Address to transfer admin rights to
    ///
    /// # Errors
    /// * `Error::AdminNotSet` - If admin was not initialized
    /// * `Error::Unauthorized` - If caller is not the current admin
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
        // Require the new admin to also sign to prevent unilateral transfer
        new_admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        bump_instance(&env);
        emit_admin_transferred(&env, old_admin, new_admin);
        Ok(())
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

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if caller != admin {
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

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if caller != admin {
            return Err(Error::Unauthorized);
        }

        env.storage().instance().set(&DataKey::Paused, &false);
        bump_instance(&env);

        emit_contract_unpaused(&env, caller);

        Ok(())
    }

    /// Upgrade the contract Wasm. Admin-only.
    ///
    /// The contract must not be paused to perform an upgrade.
    /// After a successful upgrade the new Wasm is active immediately.
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `new_wasm_hash` - 32-byte hash of the new Wasm blob (must be uploaded first)
    ///
    /// # Errors
    /// * `Error::AdminNotSet` - If admin was not initialized
    /// * `Error::Unauthorized` - If caller is not the admin
    /// * `Error::ContractPaused` - If contract is currently paused
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        // Authorisation stays with the admin. `GovernedContract::upgrade`
        // carries no caller argument, so there is nothing to distinguish a
        // governance invocation from any other — governance drives an upgrade
        // by *being* the admin, which `set_admin_by_governance` allows a vote
        // to arrange.
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

        // A zero hash is never a real Wasm blob; accepting one would replace
        // the contract with nothing and brick it permanently.
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        if new_wasm_hash == zero_hash {
            return Err(Error::InvalidWasmHash);
        }

        // Perform upgrade
        env.deployer().update_current_contract_wasm(new_wasm_hash.clone());
        bump_instance(&env);

        // Emit event
        env.events().publish(
            (symbol_short!("upgrade"),),
            (admin, new_wasm_hash),
        );

        Ok(())
    }

    /// Schema version of the data currently in storage.
    ///
    /// Contracts deployed before versioning existed report `0`, which is what
    /// [`Self::migrate`] uses to decide there is work to do.
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
    pub fn migrate(env: Env, caller: Address) -> Result<u32, Error> {
        caller.require_auth();

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

        env.events().publish(
            (symbol_short!("govset"),),
            (caller, governance),
        );

        Ok(())
    }

    /// The governance contract permitted to update parameters, if configured.
    pub fn get_governance(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Governance)
    }

    /// Transfer admin as the outcome of a passed governance proposal.
    ///
    /// Callable only by the configured governance contract. This is the
    /// counterpart of `set_admin`, which requires the current admin — together
    /// they let control move either by the admin's own hand or by a vote.
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

        env.storage().instance().set(&DataKey::Admin, &new_admin);
        bump_instance(&env);

        emit_admin_transferred(&env, old_admin, new_admin);

        Ok(())
    }

    /// Pause or unpause as the outcome of a passed governance proposal.
    ///
    /// Callable only by the configured governance contract.
    pub fn set_paused_by_governance(
        env: Env,
        caller: Address,
        paused: bool,
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
    // Contract Metadata
    // =========================================================================

    /// Get public contract metadata
    pub fn get_metadata(env: Env) -> PublicMetadata {
        PublicMetadata {
            name: String::from_str(&env, "Stellar Insights Core"),
            version: String::from_str(&env, VERSION),
            author: String::from_str(&env, "Stellar Insights Team"),
            description: String::from_str(
                &env,
                "Core analytics snapshot contract for Stellar network",
            ),
            repository: String::from_str(&env, "https://github.com/stellar-insights/contracts"),
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
