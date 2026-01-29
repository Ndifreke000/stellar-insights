#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, BytesN, Env, Map};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotMetadata {
    pub epoch: u64,
    pub timestamp: u64,
    pub hash: BytesN<32>,
    // Extendable for future fields
}

#[contracttype]
pub enum DataKey {
    /// Map of epoch -> snapshot metadata (persistent storage for full history)
    Snapshots,
    /// Latest epoch number (instance storage for quick access)
    LatestEpoch,
}

#[contract]
pub struct AnalyticsContract;

#[contractimpl]
impl AnalyticsContract {
    /// Initialize contract storage
    /// Sets up empty snapshot history and initializes latest epoch to 0
    pub fn initialize(env: Env) {
        let storage = env.storage().instance();

        // Initialize latest epoch to 0 if not already set
        if !storage.has(&DataKey::LatestEpoch) {
            storage.set(&DataKey::LatestEpoch, &0u64);
        }

        // Initialize empty snapshots map if not already set
        let persistent_storage = env.storage().persistent();
        if !persistent_storage.has(&DataKey::Snapshots) {
            let empty_snapshots = Map::<u64, SnapshotMetadata>::new(&env);
            persistent_storage.set(&DataKey::Snapshots, &empty_snapshots);
        }
    }

    /// Submit a new snapshot for a specific epoch.
    /// Stores the snapshot in the historical map and updates latest epoch.
    /// Epochs must be submitted in strictly increasing order (monotonicity).
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `epoch` - Epoch identifier (must be positive and strictly greater than latest)
    /// * `hash` - 32-byte hash of the analytics snapshot
    ///
    /// # Panics
    /// * If epoch is 0 (invalid)
    /// * If epoch <= latest (monotonicity violated: out-of-order or duplicate)
    ///
    /// # Returns
    /// * Ledger timestamp when snapshot was recorded
    pub fn submit_snapshot(env: Env, epoch: u64, hash: BytesN<32>) -> u64 {
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
        };

        let mut snapshots: Map<u64, SnapshotMetadata> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        snapshots.set(epoch, metadata);
        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);
        env.storage().instance().set(&DataKey::LatestEpoch, &epoch);

        timestamp
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
}

#[cfg(test)]
mod tests;
