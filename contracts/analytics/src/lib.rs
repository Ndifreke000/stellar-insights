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
mod test {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_initialization() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        // Verify initial state
        assert_eq!(client.get_latest_epoch(), 0);
        let history = client.get_snapshot_history();
        assert_eq!(history.len(), 0);
        assert_eq!(client.get_latest_snapshot(), None);
    }

    #[test]
    fn test_submit_single_snapshot() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        let epoch = 1u64;
        let hash = BytesN::from_array(&env, &[1u8; 32]);

        let timestamp = client.submit_snapshot(&epoch, &hash);

        // Verify snapshot was stored
        let snapshot = client.get_snapshot(&epoch).unwrap();
        assert_eq!(snapshot.epoch, epoch);
        assert_eq!(snapshot.hash, hash);
        assert_eq!(snapshot.timestamp, timestamp);

        // Verify latest epoch updated
        assert_eq!(client.get_latest_epoch(), epoch);

        // Verify latest snapshot
        let latest = client.get_latest_snapshot().unwrap();
        assert_eq!(latest.epoch, epoch);
        assert_eq!(latest.hash, hash);
    }

    #[test]
    fn test_multiple_snapshots_strictly_increasing_epochs() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        let epoch1 = 1u64;
        let hash1 = BytesN::from_array(&env, &[1u8; 32]);
        client.submit_snapshot(&epoch1, &hash1);

        let epoch2 = 2u64;
        let hash2 = BytesN::from_array(&env, &[2u8; 32]);
        client.submit_snapshot(&epoch2, &hash2);

        let epoch3 = 3u64;
        let hash3 = BytesN::from_array(&env, &[3u8; 32]);
        client.submit_snapshot(&epoch3, &hash3);

        let snapshot1 = client.get_snapshot(&epoch1).unwrap();
        assert_eq!(snapshot1.epoch, epoch1);
        assert_eq!(snapshot1.hash, hash1);

        let snapshot2 = client.get_snapshot(&epoch2).unwrap();
        assert_eq!(snapshot2.epoch, epoch2);
        assert_eq!(snapshot2.hash, hash2);

        let snapshot3 = client.get_snapshot(&epoch3).unwrap();
        assert_eq!(snapshot3.epoch, epoch3);
        assert_eq!(snapshot3.hash, hash3);

        assert_eq!(client.get_latest_epoch(), epoch3);

        let latest = client.get_latest_snapshot().unwrap();
        assert_eq!(latest.epoch, epoch3);
        assert_eq!(latest.hash, hash3);

        let history = client.get_snapshot_history();
        assert_eq!(history.len(), 3);

        let all_epochs = client.get_all_epochs();
        assert_eq!(all_epochs.len(), 3);
        assert!(all_epochs.contains(epoch1));
        assert!(all_epochs.contains(epoch2));
        assert!(all_epochs.contains(epoch3));
    }

    #[test]
    fn test_historical_data_integrity_after_new_submissions() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        // Submit initial snapshots
        let epoch1 = 1u64;
        let hash1 = BytesN::from_array(&env, &[1u8; 32]);
        let timestamp1 = client.submit_snapshot(&epoch1, &hash1);

        let epoch2 = 2u64;
        let hash2 = BytesN::from_array(&env, &[2u8; 32]);
        let timestamp2 = client.submit_snapshot(&epoch2, &hash2);

        // Verify initial state
        let snapshot1_before = client.get_snapshot(&epoch1).unwrap();
        let snapshot2_before = client.get_snapshot(&epoch2).unwrap();

        // Submit new snapshot
        let epoch3 = 5u64;
        let hash3 = BytesN::from_array(&env, &[5u8; 32]);
        client.submit_snapshot(&epoch3, &hash3);

        // Verify historical data remains intact
        let snapshot1_after = client.get_snapshot(&epoch1).unwrap();
        let snapshot2_after = client.get_snapshot(&epoch2).unwrap();

        assert_eq!(snapshot1_before.epoch, snapshot1_after.epoch);
        assert_eq!(snapshot1_before.hash, snapshot1_after.hash);
        assert_eq!(snapshot1_before.timestamp, snapshot1_after.timestamp);
        assert_eq!(snapshot1_after.timestamp, timestamp1);

        assert_eq!(snapshot2_before.epoch, snapshot2_after.epoch);
        assert_eq!(snapshot2_before.hash, snapshot2_after.hash);
        assert_eq!(snapshot2_before.timestamp, snapshot2_after.timestamp);
        assert_eq!(snapshot2_after.timestamp, timestamp2);

        // Verify new snapshot is accessible
        let snapshot3 = client.get_snapshot(&epoch3).unwrap();
        assert_eq!(snapshot3.epoch, epoch3);
        assert_eq!(snapshot3.hash, hash3);

        // Verify latest epoch updated correctly
        assert_eq!(client.get_latest_epoch(), epoch3);
    }

    #[test]
    fn test_get_nonexistent_snapshot() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        // Try to get snapshot for non-existent epoch
        assert_eq!(client.get_snapshot(&999), None);
    }

    #[test]
    #[should_panic(expected = "Invalid epoch: must be greater than 0")]
    fn test_invalid_epoch_zero() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        let hash = BytesN::from_array(&env, &[1u8; 32]);
        client.submit_snapshot(&0, &hash);
    }

    #[test]
    #[should_panic(expected = "must be strictly greater than latest")]
    fn test_epoch_monotonicity_error_message() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        client.submit_snapshot(&3u64, &BytesN::from_array(&env, &[1u8; 32]));
        client.submit_snapshot(&1u64, &BytesN::from_array(&env, &[2u8; 32]));
    }

    #[test]
    #[should_panic(expected = "already exists")]
    fn test_duplicate_epoch_fails() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        let epoch = 1u64;
        let hash1 = BytesN::from_array(&env, &[1u8; 32]);
        let hash2 = BytesN::from_array(&env, &[2u8; 32]);

        client.submit_snapshot(&epoch, &hash1);
        client.submit_snapshot(&epoch, &hash2); // Should panic
    }

    #[test]
    #[should_panic(expected = "Epoch monotonicity violated")]
    fn test_older_epoch_rejected() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        let epoch_new = 10u64;
        let hash_new = BytesN::from_array(&env, &[10u8; 32]);
        client.submit_snapshot(&epoch_new, &hash_new);
        assert_eq!(client.get_latest_epoch(), epoch_new);

        let epoch_old = 5u64;
        let hash_old = BytesN::from_array(&env, &[5u8; 32]);
        client.submit_snapshot(&epoch_old, &hash_old);
    }

    #[test]
    fn test_non_sequential_epochs_monotonic_order() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        let epochs = [1u64, 5u64, 10u64];
        for (i, &epoch) in epochs.iter().enumerate() {
            let mut hash_bytes = [0u8; 32];
            hash_bytes[0] = (i + 1) as u8;
            let hash = BytesN::from_array(&env, &hash_bytes);
            client.submit_snapshot(&epoch, &hash);
        }

        for (i, &epoch) in epochs.iter().enumerate() {
            let snapshot = client.get_snapshot(&epoch).unwrap();
            assert_eq!(snapshot.epoch, epoch);
            let mut expected = [0u8; 32];
            expected[0] = (i + 1) as u8;
            assert_eq!(snapshot.hash, BytesN::from_array(&env, &expected));
        }

        assert_eq!(client.get_latest_epoch(), 10u64);

        let history = client.get_snapshot_history();
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_bounded_storage_growth_simulation() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        client.initialize();

        // Simulate many epochs to test storage behavior
        let num_epochs = 100u64;
        for epoch in 1..=num_epochs {
            let mut hash_bytes = [0u8; 32];
            hash_bytes[0] = (epoch % 256) as u8;
            hash_bytes[1] = ((epoch / 256) % 256) as u8;
            let hash = BytesN::from_array(&env, &hash_bytes);
            client.submit_snapshot(&epoch, &hash);
        }

        // Verify all epochs are still accessible
        for epoch in 1..=num_epochs {
            assert!(client.get_snapshot(&epoch).is_some());
        }

        // Verify latest epoch
        assert_eq!(client.get_latest_epoch(), num_epochs);

        // Verify history size
        let history = client.get_snapshot_history();
        assert_eq!(history.len(), num_epochs as u32);

        // Verify all epochs list
        let all_epochs = client.get_all_epochs();
        assert_eq!(all_epochs.len(), num_epochs as u32);
    }
}
