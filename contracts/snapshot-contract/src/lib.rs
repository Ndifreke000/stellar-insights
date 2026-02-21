#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN, Env, Map,
};

const HASH_SIZE: u32 = 32;
const CONTRACT_VERSION: u32 = 1;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub hash: Bytes,
    pub epoch: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMetadata {
    pub version: u32,
    pub admin: Address,
    pub upgrade_timestamp: u64,
}

#[contracttype]
pub enum DataKey {
    Snapshots,
    LatestEpoch,
    Metadata,
    Admin,
}

#[contract]
pub struct SnapshotContract;

#[contractimpl]
impl SnapshotContract {
    /// Initialize the contract with an admin address
    ///
    /// # Arguments
    /// * `admin` - Address that will have upgrade privileges
    ///
    /// # Panics
    /// * If contract is already initialized
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }

        admin.require_auth();

        let metadata = ContractMetadata {
            version: CONTRACT_VERSION,
            admin: admin.clone(),
            upgrade_timestamp: env.ledger().timestamp(),
        };

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Metadata, &metadata);

        env.events()
            .publish((symbol_short!("INIT"),), (admin, CONTRACT_VERSION));
    }

    /// Get the current contract version
    pub fn version(env: Env) -> u32 {
        let metadata: Option<ContractMetadata> = env.storage().instance().get(&DataKey::Metadata);
        match metadata {
            Some(m) => m.version,
            None => CONTRACT_VERSION,
        }
    }

    /// Get the contract admin address
    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    /// Transfer admin privileges to a new address
    ///
    /// # Arguments
    /// * `new_admin` - New admin address
    ///
    /// # Panics
    /// * If caller is not the current admin
    pub fn transfer_admin(env: Env, new_admin: Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");

        admin.require_auth();
        new_admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &new_admin);

        let mut metadata: ContractMetadata = env
            .storage()
            .instance()
            .get(&DataKey::Metadata)
            .expect("Metadata not found");

        metadata.admin = new_admin.clone();
        env.storage().instance().set(&DataKey::Metadata, &metadata);

        env.events()
            .publish((symbol_short!("ADM_XFER"),), (admin, new_admin));
    }

    /// Prepare for contract upgrade by validating the new WASM hash
    ///
    /// # Arguments
    /// * `new_wasm_hash` - Hash of the new contract WASM
    ///
    /// # Panics
    /// * If caller is not the admin
    /// * If hash is invalid
    pub fn prepare_upgrade(env: Env, new_wasm_hash: Bytes) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");

        admin.require_auth();

        if new_wasm_hash.len() != HASH_SIZE {
            panic!("Invalid WASM hash size");
        }

        env.events()
            .publish((symbol_short!("UPG_PREP"),), (new_wasm_hash,));
    }

    /// Execute contract upgrade
    ///
    /// # Arguments
    /// * `new_wasm_hash` - Hash of the new contract WASM
    ///
    /// # Panics
    /// * If caller is not the admin
    pub fn upgrade(env: Env, new_wasm_hash: Bytes) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");

        admin.require_auth();

        if new_wasm_hash.len() != HASH_SIZE {
            panic!("Invalid WASM hash size");
        }

        let hash_bytes: BytesN<32> = new_wasm_hash.clone().try_into().unwrap();
        env.deployer().update_current_contract_wasm(hash_bytes);

        let mut metadata: ContractMetadata = env
            .storage()
            .instance()
            .get(&DataKey::Metadata)
            .unwrap_or(ContractMetadata {
                version: CONTRACT_VERSION,
                admin: admin.clone(),
                upgrade_timestamp: env.ledger().timestamp(),
            });

        metadata.version += 1;
        metadata.upgrade_timestamp = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Metadata, &metadata);

        env.events().publish(
            (symbol_short!("UPGRADED"),),
            (new_wasm_hash, metadata.version),
        );
    }

    /// Migrate data from old version to new version
    /// This is a placeholder that can be extended in future versions
    ///
    /// # Arguments
    /// * `from_version` - Version to migrate from
    ///
    /// # Panics
    /// * If caller is not the admin
    pub fn migrate(env: Env, from_version: u32) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");

        admin.require_auth();

        let current_version = Self::version(env.clone());

        if from_version >= current_version {
            panic!("Invalid migration: from_version must be less than current version");
        }

        // Migration logic would go here based on version differences
        // For now, this is a no-op that validates the migration is possible

        env.events().publish(
            (symbol_short!("MIGRATED"),),
            (from_version, current_version),
        );
    }

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
            Some(snapshot) => snapshot.hash,
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
    use soroban_sdk::{
        bytes,
        testutils::{Address as _, Events},
        Env, TryIntoVal,
    };

    #[test]
    fn test_initialize() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.initialize(&admin);

        assert_eq!(client.version(), CONTRACT_VERSION);
        assert_eq!(client.get_admin(), Some(admin));
    }

    #[test]
    #[should_panic(expected = "Contract already initialized")]
    fn test_initialize_twice_fails() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.initialize(&admin);
        client.initialize(&admin);
    }

    #[test]
    fn test_transfer_admin() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let new_admin = Address::generate(&env);
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.initialize(&admin);
        client.transfer_admin(&new_admin);

        assert_eq!(client.get_admin(), Some(new_admin));
    }

    #[test]
    fn test_prepare_upgrade() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.initialize(&admin);

        let wasm_hash = bytes!(
            &env,
            0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
        );

        client.prepare_upgrade(&wasm_hash);

        let events = env.events().all();
        let upgrade_event = events.iter().find(|e| {
            if !e.1.is_empty() {
                let topic: soroban_sdk::Symbol = e.1.get_unchecked(0).try_into_val(&env).unwrap();
                topic == symbol_short!("UPG_PREP")
            } else {
                false
            }
        });
        assert!(upgrade_event.is_some());
    }

    #[test]
    #[should_panic(expected = "Invalid WASM hash size")]
    fn test_prepare_upgrade_invalid_hash() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.initialize(&admin);

        let invalid_hash = bytes!(&env, 0x1234);
        client.prepare_upgrade(&invalid_hash);
    }

    #[test]
    fn test_migrate() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.initialize(&admin);

        // Simulate migration from version 0 to current
        client.migrate(&0);

        let events = env.events().all();
        let migrate_event = events.iter().find(|e| {
            if !e.1.is_empty() {
                let topic: soroban_sdk::Symbol = e.1.get_unchecked(0).try_into_val(&env).unwrap();
                topic == symbol_short!("MIGRATED")
            } else {
                false
            }
        });
        assert!(migrate_event.is_some());
    }

    #[test]
    #[should_panic(expected = "Invalid migration")]
    fn test_migrate_invalid_version() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.initialize(&admin);

        // Try to migrate from current or future version
        client.migrate(&CONTRACT_VERSION);
    }

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
        let snap_event = events.iter().find(|e| {
            if !e.1.is_empty() {
                let topic: soroban_sdk::Symbol = e.1.get_unchecked(0).try_into_val(&env).unwrap();
                topic == symbol_short!("SNAP_SUB")
            } else {
                false
            }
        });
        assert!(snap_event.is_some());
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

    #[test]
    fn test_version_without_init() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        assert_eq!(client.version(), CONTRACT_VERSION);
    }

    #[test]
    fn test_upgrade_mechanism_with_snapshots() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.initialize(&admin);

        // Submit snapshots before upgrade
        let hash1 = bytes!(
            &env,
            0x1111111111111111111111111111111111111111111111111111111111111111
        );
        client.submit_snapshot(&hash1, &1);

        // Prepare and verify upgrade readiness
        let wasm_hash = bytes!(
            &env,
            0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
        );
        client.prepare_upgrade(&wasm_hash);

        // Verify snapshots are still accessible
        assert_eq!(client.get_snapshot(&1), hash1);
        assert!(client.verify_snapshot(&hash1));
    }

    #[test]
    fn test_verify_snapshot_at_epoch() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash1 = bytes!(
            &env,
            0x1111111111111111111111111111111111111111111111111111111111111111
        );
        let hash2 = bytes!(
            &env,
            0x2222222222222222222222222222222222222222222222222222222222222222
        );

        client.submit_snapshot(&hash1, &1);
        client.submit_snapshot(&hash2, &2);

        assert!(client.verify_snapshot_at_epoch(&hash1, &1));
        assert!(!client.verify_snapshot_at_epoch(&hash1, &2));
        assert!(client.verify_snapshot_at_epoch(&hash2, &2));
    }

    #[test]
    fn test_verify_latest_snapshot() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let hash1 = bytes!(
            &env,
            0x1111111111111111111111111111111111111111111111111111111111111111
        );
        let hash2 = bytes!(
            &env,
            0x2222222222222222222222222222222222222222222222222222222222222222
        );

        client.submit_snapshot(&hash1, &1);
        assert!(client.verify_latest_snapshot(&hash1));

        client.submit_snapshot(&hash2, &2);
        assert!(!client.verify_latest_snapshot(&hash1));
        assert!(client.verify_latest_snapshot(&hash2));
    }
}
