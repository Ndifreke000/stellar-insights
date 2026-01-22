#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Bytes, Env, Map
};

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
            panic!("Invalid hash size: expected {} bytes, got {}", HASH_SIZE, hash.len());
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

        // Emit event exactly once
        env.events()
            .publish(
                (symbol_short!("SNAP_SUB"),),
                (hash, epoch, timestamp)
            );

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
            Some(snapshot) => snapshot.hash,           // return raw hash bytes
            None => panic!("No snapshot found for epoch {}", epoch),
        }
    }

    /// Return the most recent snapshot (by epoch number)
    /// Returns: (hash, epoch, timestamp)
    pub fn latest_snapshot(env: Env) -> (Bytes, u64, u64) {
        let snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        if snapshots.is_empty() {
            panic!("No snapshots exist");
        }

        let mut max_epoch: u64 = 0;
        let mut latest: Option<Snapshot> = None;

        // We have to iterate — Soroban maps don't have max_by_key yet
        for (epoch, snap) in snapshots.iter() {
            if epoch > max_epoch {
                max_epoch = epoch;
                latest = Some(snap);
            }
        }

        let snap = latest.unwrap(); // safe because we checked !is_empty()

        (snap.hash, snap.epoch, snap.timestamp)
    }

    /// Verify whether a provided hash matches the stored snapshot for that epoch
    pub fn verify_snapshot(env: Env, hash: Bytes) -> bool {
        // We assume the client knows which epoch → hash pair to verify.
        // If you want epoch-specific verification, change signature to:
        // verify_snapshot(env: Env, epoch: u64, hash: Bytes) -> bool

        let snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        // Naive version: check if this exact hash exists anywhere
        // (works if hashes are unique — common when including merkle root + epoch)
        for (_, snap) in snapshots.iter() {
            if snap.hash == hash {
                return true;
            }
        }

        false

        // Alternative (recommended) — require epoch:
        // match snapshots.get(epoch) {
        //     Some(snap) => snap.hash == hash,
        //     None => false,
        // }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        bytes, testutils::Events, Env,
    };

    #[test]
    fn test_submit_and_retrieve() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash = bytes!(&env, 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef);
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

        let hash = bytes!(&env, 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890);
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

        let hash = bytes!(&env, 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef);
        client.submit_snapshot(&hash, &0);
    }

    #[test]
    #[should_panic(expected = "already exists")]
    fn test_duplicate_epoch_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash1 = bytes!(&env, 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef);
        let hash2 = bytes!(&env, 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890);

        client.submit_snapshot(&hash1, &1);
        client.submit_snapshot(&hash2, &1);
    }

    #[test]
    fn test_multiple_snapshots() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash1 = bytes!(&env, 0x1111111111111111111111111111111111111111111111111111111111111111);
        let epoch1 = 1u64;
        client.submit_snapshot(&hash1, &epoch1);

        let hash2 = bytes!(&env, 0x2222222222222222222222222222222222222222222222222222222222222222);
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

        client.submit_snapshot(&bytes!(&env, 0x1111111111111111111111111111111111111111111111111111111111111111), &1);
        client.submit_snapshot(&bytes!(&env, 0x2222222222222222222222222222222222222222222222222222222222222222), &3);
        client.submit_snapshot(&bytes!(&env, 0x3333333333333333333333333333333333333333333333333333333333333333), &7);

        let (h, e, _t) = client.latest_snapshot();
        assert_eq!(e, 7);
        assert_eq!(h, bytes!(&env, 0x3333333333333333333333333333333333333333333333333333333333333333));
    }

    #[test]
    fn test_verify_found() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash = bytes!(&env, 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890);
        client.submit_snapshot(&hash, &100);

        assert!(client.verify_snapshot(&hash));
    }

    #[test]
    fn test_verify_not_found() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.submit_snapshot(&bytes!(&env, 0x1111111111111111111111111111111111111111111111111111111111111111), &5);

        assert!(!client.verify_snapshot(&bytes!(&env, 0x9999999999999999999999999999999999999999999999999999999999999999)));
    }
}
