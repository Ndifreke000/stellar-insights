#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Map};

const DEFAULT_SNAPSHOT_TTL: u64 = 7_776_000; // 90 days in seconds
const LEDGER_SECONDS: u64 = 5; // ~5 seconds per ledger

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
pub enum DataKey {
    /// Authorized submitter address (only this address can submit snapshots)
    Admin,
    /// Map of epoch -> snapshot metadata (persistent storage for full history)
    Snapshots,
    /// Latest epoch number (instance storage for quick access)
    LatestEpoch,
    /// Per-epoch individual storage key for TTL management
    Snapshot(u64),
}

#[contract]
pub struct AnalyticsContract;

#[contractimpl]
impl AnalyticsContract {
    /// Initialize contract storage with an authorized admin address
    /// Sets up empty snapshot history and initializes latest epoch to 0
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `admin` - Address authorized to submit snapshots
    ///
    /// # Panics
    /// * If contract is already initialized (admin already set)
    pub fn initialize(env: Env, admin: Address) {
        let storage = env.storage().instance();

        // Prevent re-initialization if admin is already set
        if storage.has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        // Store the authorized admin address
        storage.set(&DataKey::Admin, &admin);

        // Initialize latest epoch to 0
        storage.set(&DataKey::LatestEpoch, &0u64);

        // Initialize empty snapshots map
        let persistent_storage = env.storage().persistent();
        let empty_snapshots = Map::<u64, SnapshotMetadata>::new(&env);
        persistent_storage.set(&DataKey::Snapshots, &empty_snapshots);
    }

    /// Submit a new snapshot for a specific epoch.
    /// Stores the snapshot in the historical map and updates latest epoch.
    /// Epochs must be submitted in strictly increasing order (monotonicity).
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `epoch` - Epoch identifier (must be positive and strictly greater than latest)
    /// * `hash` - 32-byte hash of the analytics snapshot
    /// * `caller` - Address attempting to submit (must be the authorized admin)
    ///
    /// # Panics
    /// * If admin is not set (contract not initialized)
    /// * If caller is not the authorized admin
    /// * If epoch is 0 (invalid)
    /// * If epoch <= latest (monotonicity violated: out-of-order or duplicate)
    ///
    /// # Returns
    /// * Ledger timestamp when snapshot was recorded
    pub fn submit_snapshot(env: Env, epoch: u64, hash: BytesN<32>, caller: Address) -> u64 {
        // Require authentication from the caller
        caller.require_auth();

        // Verify caller is the authorized admin
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized: admin not set");

        if caller != admin {
            panic!("Unauthorized: only the admin can submit snapshots");
        }

        if epoch == 0 {
            panic!("Invalid epoch: must be greater than 0");
        }

        let latest: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);

        if epoch <= latest {
            if epoch == latest {
                panic!("Snapshot for epoch {} already exists", epoch);
            } else {
                panic!(
                    "Epoch monotonicity violated: epoch {} must be strictly greater than latest {}",
                    epoch, latest
                );
            }
        }

        let timestamp = env.ledger().timestamp();
        let metadata = SnapshotMetadata {
            epoch,
            timestamp,
            hash,
            submitter: caller.clone(),
            ledger_sequence: env.ledger().sequence(),
            expires_at: None,
        };

        let mut snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        snapshots.set(epoch, metadata.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);
        env.storage().instance().set(&DataKey::LatestEpoch, &epoch);

        // Also store per-epoch key for TTL management
        env.storage().persistent().set(&DataKey::Snapshot(epoch), &metadata);

        timestamp
    }

    /// Submit a snapshot with an optional TTL (defaults to 90 days).
    /// Stores expiry metadata and sets Soroban storage TTL accordingly.
    pub fn submit_snapshot_with_ttl(
        env: Env,
        epoch: u64,
        hash: BytesN<32>,
        caller: Address,
        ttl_seconds: Option<u64>,
    ) -> u64 {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized: admin not set");

        if caller != admin {
            panic!("Unauthorized: only the admin can submit snapshots");
        }

        if epoch == 0 {
            panic!("Invalid epoch: must be greater than 0");
        }

        let latest: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);

        if epoch <= latest {
            if epoch == latest {
                panic!("Snapshot for epoch {} already exists", epoch);
            } else {
                panic!(
                    "Epoch monotonicity violated: epoch {} must be strictly greater than latest {}",
                    epoch, latest
                );
            }
        }

        let timestamp = env.ledger().timestamp();
        let ttl = ttl_seconds.unwrap_or(DEFAULT_SNAPSHOT_TTL);
        let expires_at = timestamp + ttl;

        let metadata = SnapshotMetadata {
            epoch,
            timestamp,
            hash,
            submitter: caller.clone(),
            ledger_sequence: env.ledger().sequence(),
            expires_at: Some(expires_at),
        };

        // Store in the shared map
        let mut snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));
        snapshots.set(epoch, metadata.clone());
        env.storage().persistent().set(&DataKey::Snapshots, &snapshots);
        env.storage().instance().set(&DataKey::LatestEpoch, &epoch);

        // Store per-epoch key and set Soroban storage TTL
        let ledgers_to_live = (ttl / LEDGER_SECONDS) as u32;
        env.storage().persistent().set(&DataKey::Snapshot(epoch), &metadata);
        env.storage().persistent().extend_ttl(
            &DataKey::Snapshot(epoch),
            ledgers_to_live,
            ledgers_to_live,
        );

        timestamp
    }

    /// Remove expired snapshots from the shared map.
    /// Admin-only. Iterates up to `max_to_clean` epochs and removes those past expiry.
    /// Returns the number of snapshots cleaned.
    pub fn cleanup_expired_snapshots(env: Env, admin: Address, max_to_clean: u32) -> u32 {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized: admin not set");

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
                        env.storage().persistent().remove(&DataKey::Snapshot(epoch));
                        cleaned += 1;
                    }
                }
            }
        }

        env.storage().persistent().set(&DataKey::Snapshots, &snapshots);
        cleaned
    }

    /// Check whether a snapshot has expired.
    pub fn is_snapshot_expired(env: Env, epoch: u64) -> bool {
        let snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        match snapshots.get(epoch) {
            Some(metadata) => match metadata.expires_at {
                Some(expires_at) => env.ledger().timestamp() > expires_at,
                None => false,
            },
            None => false,
        }
    }

    /// Get snapshot metadata for a specific epoch
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `epoch` - Epoch to retrieve
    ///
    /// # Returns
    /// * Snapshot metadata for the epoch, or None if not found
    pub fn get_snapshot(env: Env, epoch: u64) -> Option<SnapshotMetadata> {
        let snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        snapshots.get(epoch)
    }

    /// Get the latest snapshot metadata
    ///
    /// # Arguments
    /// * `env` - Contract environment
    ///
    /// # Returns
    /// * Latest snapshot metadata, or None if no snapshots exist
    pub fn get_latest_snapshot(env: Env) -> Option<SnapshotMetadata> {
        let latest_epoch: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);

        if latest_epoch == 0 {
            return None;
        }

        Self::get_snapshot(env, latest_epoch)
    }

    /// Get the complete snapshot history as a Map
    ///
    /// # Arguments
    /// * `env` - Contract environment
    ///
    /// # Returns
    /// * Map of all snapshots keyed by epoch
    pub fn get_snapshot_history(env: Env) -> Map<u64, SnapshotMetadata> {
        env.storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env))
    }

    /// Get the latest epoch number
    ///
    /// # Arguments
    /// * `env` - Contract environment
    ///
    /// # Returns
    /// * Latest epoch number (0 if no snapshots)
    pub fn get_latest_epoch(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0)
    }

    /// Get all epochs that have snapshots (for iteration purposes)
    ///
    /// # Arguments
    /// * `env` - Contract environment
    ///
    /// # Returns
    /// * Vector of all epochs with stored snapshots
    pub fn get_all_epochs(env: Env) -> soroban_sdk::Vec<u64> {
        let snapshots = Self::get_snapshot_history(env.clone());
        let mut epochs = soroban_sdk::Vec::new(&env);

        for (epoch, _) in snapshots.iter() {
            epochs.push_back(epoch);
        }

        epochs
    }

    /// Get the current authorized admin address
    ///
    /// # Arguments
    /// * `env` - Contract environment
    ///
    /// # Returns
    /// * The admin address if set, None otherwise
    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    /// Update the authorized admin address
    /// Only the current admin can transfer admin rights to a new address
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `current_admin` - Current admin address (must authenticate)
    /// * `new_admin` - New address to set as admin
    ///
    /// # Panics
    /// * If contract is not initialized (admin not set)
    /// * If caller is not the current admin
    pub fn set_admin(env: Env, current_admin: Address, new_admin: Address) {
        // Require authentication from the current admin
        current_admin.require_auth();

        // Verify caller is the current admin
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized: admin not set");

        if current_admin != admin {
            panic!("Unauthorized: only the current admin can set a new admin");
        }

        // Update admin address
        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }
}

#[cfg(test)]
mod tests;
