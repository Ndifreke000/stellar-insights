#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Map};

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
    /// Authorized submitter address (only this address can submit snapshots)
    Admin,
    /// Map of epoch -> snapshot metadata (persistent storage for full history)
    Snapshots,
    /// Latest epoch number (instance storage for quick access)
    LatestEpoch,
    /// Emergency pause state (true = paused, false = active)
    Paused,
    /// Governance contract address (only it can call set_admin_by_governance / set_paused_by_governance)
    Governance,
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

        // Initialize contract as not paused
        storage.set(&DataKey::Paused, &false);

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
    /// * If contract is paused for emergency maintenance
    /// * If admin is not set (contract not initialized)
    /// * If caller is not the authorized admin
    /// * If epoch is 0 (invalid)
    /// * If epoch <= latest (monotonicity violated: out-of-order or duplicate)
    ///
    /// # Returns
    /// * Ledger timestamp when snapshot was recorded
    pub fn submit_snapshot(env: Env, epoch: u64, hash: BytesN<32>, caller: Address) -> u64 {
        // Check if contract is paused
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if is_paused {
            panic!("Contract is paused for emergency maintenance");
        }

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

    /// Emergency pause the contract
    ///
    /// Pauses all snapshot submissions. Only the admin can pause the contract.
    /// Read operations remain available during pause.
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `caller` - Address attempting to pause (must be admin)
    ///
    /// # Panics
    /// * If contract is not initialized (admin not set)
    /// * If caller is not the admin
    pub fn pause(env: Env, caller: Address) {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized: admin not set");

        if caller != admin {
            panic!("Unauthorized: only the admin can pause the contract");
        }

        env.storage().instance().set(&DataKey::Paused, &true);
    }

    /// Unpause the contract
    ///
    /// Resumes normal operations. Only the admin can unpause the contract.
    ///
    /// # Arguments
    /// * `env` - Contract environment
    /// * `caller` - Address attempting to unpause (must be admin)
    ///
    /// # Panics
    /// * If contract is not initialized (admin not set)
    /// * If caller is not the admin
    pub fn unpause(env: Env, caller: Address) {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized: admin not set");

        if caller != admin {
            panic!("Unauthorized: only the admin can unpause the contract");
        }

        env.storage().instance().set(&DataKey::Paused, &false);
    }

    /// Set the governance contract address. Only the admin can set this.
    /// The governance contract can then update admin or pause state via voting.
    pub fn set_governance(env: Env, caller: Address, governance: Address) {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized: admin not set");

        if caller != admin {
            panic!("Unauthorized: only the admin can set governance");
        }

        env.storage()
            .instance()
            .set(&DataKey::Governance, &governance);
    }

    /// Get the current governance contract address (if any).
    pub fn get_governance(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Governance)
    }

    /// Set the admin address. Only the governance contract may call this (after a passed proposal).
    pub fn set_admin_by_governance(env: Env, caller: Address, new_admin: Address) {
        let governance: Address = env
            .storage()
            .instance()
            .get(&DataKey::Governance)
            .expect("Governance not set");

        if caller != governance {
            panic!("Unauthorized: only the governance contract can set admin");
        }

        env.storage().instance().set(&DataKey::Admin, &new_admin);
    }

    /// Set the paused state. Only the governance contract may call this (after a passed proposal).
    pub fn set_paused_by_governance(env: Env, caller: Address, paused: bool) {
        let governance: Address = env
            .storage()
            .instance()
            .get(&DataKey::Governance)
            .expect("Governance not set");

        if caller != governance {
            panic!("Unauthorized: only the governance contract can set paused");
        }

        env.storage().instance().set(&DataKey::Paused, &paused);
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
}

#[cfg(test)]
mod tests;
