#![no_std]

mod errors;
mod events;

use errors::Error;
use events::emit_snapshot_submitted;
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, BytesN, Env, Map,
};

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
}

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
    pub fn initialize(env: Env, admin: Address) {
        // Verify admin doesn't already exist to prevent re-initialization
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        // Store the admin address
        env.storage().instance().set(&DataKey::Admin, &admin);
        
        // Initialize latest epoch to 0
        env.storage().instance().set(&DataKey::LatestEpoch, &0u64);
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
    /// * `Error::AdminNotSet` - If admin was not initialized
    /// * `Error::UnauthorizedCaller` - If caller is not the admin
    /// * `Error::InvalidEpoch` - If epoch is 0
    /// * `Error::DuplicateEpoch` - If snapshot already exists for this epoch
    /// 
    /// # Returns
    /// * Ledger timestamp when the snapshot was recorded
    pub fn submit_snapshot(
        env: Env,
        epoch: u64,
        hash: BytesN<32>,
        caller: Address,
    ) -> Result<u64, Error> {
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
            return Err(Error::UnauthorizedCaller);
        }

        // Validate epoch is not zero
        if epoch == 0 {
            return Err(Error::InvalidEpoch);
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

        // Update latest epoch if this is newer
        let current_latest: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);

        if epoch > current_latest {
            env.storage().instance().set(&DataKey::LatestEpoch, &epoch);
        }

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
}

mod test;
