#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Bytes, Env, Map};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub hash: Bytes,
    pub epoch: u64,
    pub timestamp: u64,
}

#[contracttype]
pub enum DataKey {
    Snapshots,
    LatestEpoch,
}

#[contract]
pub struct SnapshotContract;

#[contractimpl]
impl SnapshotContract {
    /// Submit a snapshot hash for verification
    ///
    /// # Arguments
    /// * `hash` - The analytics hash to store
    /// * `epoch` - The epoch identifier for the snapshot
    ///
    /// # Returns
    /// The timestamp when the snapshot was submitted
    pub fn submit_snapshot(env: Env, hash: Bytes, epoch: u64) -> u64 {
        let timestamp = env.ledger().timestamp();

        // Create snapshot
        let snapshot = Snapshot {
            hash: hash.clone(),
            epoch,
            timestamp,
        };

        // Store snapshot in persistent storage
        let mut snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or(Map::new(&env));

        snapshots.set(epoch, snapshot);
        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);

        // Update latest epoch if this is newer
        let current_latest: Option<u64> = env.storage().persistent().get(&DataKey::LatestEpoch);
        if current_latest.is_none() || epoch > current_latest.unwrap() {
            env.storage().persistent().set(&DataKey::LatestEpoch, &epoch);
        }

        // Emit event
        env.events()
            .publish((symbol_short!("SNAP_SUB"),), (hash, epoch, timestamp));

        timestamp
    }

    /// Get a snapshot by epoch
    ///
    /// # Arguments
    /// * `epoch` - The epoch identifier
    ///
    /// # Returns
    /// The snapshot data if it exists
    pub fn get_snapshot(env: Env, epoch: u64) -> Option<Snapshot> {
        let snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or(Map::new(&env));

        snapshots.get(epoch)
    }

    /// Get the latest snapshot
    ///
    /// # Returns
    /// The most recent snapshot if any exist
    pub fn get_latest_snapshot(env: Env) -> Option<Snapshot> {
        let latest_epoch: Option<u64> = env.storage().persistent().get(&DataKey::LatestEpoch);
        
        match latest_epoch {
            Some(epoch) => Self::get_snapshot(env, epoch),
            None => None,
        }
    }

    /// Verify if a snapshot hash is canonical (exists in stored snapshots)
    ///
    /// This function checks the provided hash against:
    /// 1. The latest snapshot
    /// 2. All historical snapshots
    ///
    /// # Arguments
    /// * `hash` - The snapshot hash to verify
    ///
    /// # Returns
    /// `true` if the hash matches any stored snapshot, `false` otherwise
    pub fn verify_snapshot(env: Env, hash: Bytes) -> bool {
        let snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or(Map::new(&env));

        // Iterate through all snapshots and check if any hash matches
        for (_, snapshot) in snapshots.iter() {
            if snapshot.hash == hash {
                return true;
            }
        }

        false
    }

    /// Verify if a snapshot hash matches a specific epoch
    ///
    /// # Arguments
    /// * `hash` - The snapshot hash to verify
    /// * `epoch` - The specific epoch to check against
    ///
    /// # Returns
    /// `true` if the hash matches the snapshot at the given epoch, `false` otherwise
    pub fn verify_snapshot_at_epoch(env: Env, hash: Bytes, epoch: u64) -> bool {
        let snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or(Map::new(&env));

        match snapshots.get(epoch) {
            Some(snapshot) => snapshot.hash == hash,
            None => false,
        }
    }

    /// Verify if a snapshot hash matches the latest snapshot
    ///
    /// # Arguments
    /// * `hash` - The snapshot hash to verify
    ///
    /// # Returns
    /// `true` if the hash matches the latest snapshot, `false` otherwise
    pub fn verify_latest_snapshot(env: Env, hash: Bytes) -> bool {
        match Self::get_latest_snapshot(env) {
            Some(snapshot) => snapshot.hash == hash,
            None => false,
        }
    }
}

mod test;
