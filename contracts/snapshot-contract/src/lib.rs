#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Bytes, Env, Map};

const HASH_SIZE: u32 = 32;

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
    /// Submit a snapshot hash for an epoch with input validation
    ///
    /// # Arguments
    /// * `hash` - 32-byte SHA-256 hash of analytics snapshot
    /// * `epoch` - Epoch identifier (must be positive)
    ///
    /// # Panics
    /// * If hash is not exactly 32 bytes
    /// * If epoch is 0
    /// * If snapshot already exists for this epoch
    ///
    /// # Returns
    /// * Ledger timestamp when snapshot was recorded
    pub fn submit_snapshot(env: Env, hash: Bytes, epoch: u64) -> u64 {
        // Validate inputs
        if hash.len() != HASH_SIZE {
            panic!(
                "Invalid hash size: expected {} bytes, got {}",
                HASH_SIZE,
                hash.len()
            );
        }

        if epoch == 0 {
            panic!("Invalid epoch: must be greater than 0");
        }

        let timestamp = env.ledger().timestamp();

        let snapshot = Snapshot {
            hash: hash.clone(),
            epoch,
            timestamp,
        };

        let mut snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        // Prevent overwriting existing snapshots
        if snapshots.contains_key(epoch) {
            panic!("Snapshot for epoch {} already exists", epoch);
        }

        snapshots.set(epoch, snapshot);

        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);

        // Update latest epoch if this is newer
        let current_latest: Option<u64> = env.storage().persistent().get(&DataKey::LatestEpoch);
        if current_latest.is_none() || epoch > current_latest.unwrap() {
            env.storage()
                .persistent()
                .set(&DataKey::LatestEpoch, &epoch);
        }

        // Emit event
        // Emit event exactly once
        env.events()
            .publish((symbol_short!("SNAP_SUB"),), (hash, epoch, timestamp));

        timestamp
    }

    /// Get snapshot data for a specific epoch
    pub fn get_snapshot(env: Env, epoch: u64) -> Bytes {
        let snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        match snapshots.get(epoch) {
            Some(snapshot) => snapshot.hash, // return raw hash bytes
            None => panic!("No snapshot found for epoch {}", epoch),
        }
    }

    pub fn latest_snapshot(env: Env) -> Option<Snapshot> {
        let latest_epoch: Option<u64> = env.storage().persistent().get(&DataKey::LatestEpoch);

        match latest_epoch {
            Some(epoch) => {
                let snapshots: Map<u64, Snapshot> = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Snapshots)
                    .unwrap_or_else(|| Map::new(&env));

                snapshots.get(epoch)
            }
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
        match Self::latest_snapshot(env.clone()) {
            Some(snapshot) => snapshot.hash == hash,
            None => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{bytes, testutils::Events, Env};

    #[test]
    fn test_submit_and_retrieve() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash = bytes!(
            &env,
            0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
        );
        let epoch = 42u64;

        let _timestamp = client.submit_snapshot(&hash, &epoch);

        let retrieved_hash = client.get_snapshot(&epoch);
        assert_eq!(retrieved_hash, hash);
    }

    #[test]
    fn test_snapshot_submitted_event() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash = bytes!(
            &env,
            0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
        );
        let epoch = 100u64;

        client.submit_snapshot(&hash, &epoch);

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let ev = events.get(0).unwrap();
        assert_eq!(ev.0, contract_id);

        // Verify event topics
        let topics = ev.1;
        assert_eq!(topics.len(), 1);
    }

    #[test]
    #[should_panic(expected = "Invalid hash size")]
    fn test_invalid_hash_size() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let short_hash = bytes!(&env, 0x1234);
        client.submit_snapshot(&short_hash, &1);
    }

    #[test]
    #[should_panic(expected = "Invalid epoch")]
    fn test_invalid_epoch_zero() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash = bytes!(
            &env,
            0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
        );
        client.submit_snapshot(&hash, &0);
    }

    #[test]
    #[should_panic(expected = "already exists")]
    fn test_duplicate_epoch_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash1 = bytes!(
            &env,
            0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
        );
        let hash2 = bytes!(
            &env,
            0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
        );

        client.submit_snapshot(&hash1, &1);
        client.submit_snapshot(&hash2, &1);
    }

    #[test]
    fn test_multiple_snapshots() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash1 = bytes!(
            &env,
            0x1111111111111111111111111111111111111111111111111111111111111111
        );
        let epoch1 = 1u64;
        client.submit_snapshot(&hash1, &epoch1);

        let hash2 = bytes!(
            &env,
            0x2222222222222222222222222222222222222222222222222222222222222222
        );
        let epoch2 = 2u64;
        client.submit_snapshot(&hash2, &epoch2);

        assert_eq!(client.get_snapshot(&epoch1), hash1);
        assert_eq!(client.get_snapshot(&epoch2), hash2);
    }

    #[test]
    fn test_latest_snapshot() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.submit_snapshot(
            &bytes!(
                &env,
                0x1111111111111111111111111111111111111111111111111111111111111111
            ),
            &1,
        );
        client.submit_snapshot(
            &bytes!(
                &env,
                0x2222222222222222222222222222222222222222222222222222222222222222
            ),
            &3,
        );
        client.submit_snapshot(
            &bytes!(
                &env,
                0x3333333333333333333333333333333333333333333333333333333333333333
            ),
            &7,
        );

        let snapshot = client.latest_snapshot().unwrap();
        assert_eq!(snapshot.epoch, 7);
        assert_eq!(
            snapshot.hash,
            bytes!(
                &env,
                0x3333333333333333333333333333333333333333333333333333333333333333
            )
        );
    }

    #[test]
    fn test_latest_snapshot_empty() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        assert_eq!(client.latest_snapshot(), None);
    }

    #[test]
    fn test_verify_found() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash = bytes!(
            &env,
            0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
        );
        client.submit_snapshot(&hash, &100);

        assert!(client.verify_snapshot(&hash));
    }

    #[test]
    fn test_verify_not_found() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.submit_snapshot(
            &bytes!(
                &env,
                0x1111111111111111111111111111111111111111111111111111111111111111
            ),
            &5,
        );

        assert!(!client.verify_snapshot(&bytes!(
            &env,
            0x9999999999999999999999999999999999999999999999999999999999999999
        )));
    }
}
