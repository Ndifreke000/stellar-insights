#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN, Env,
    Map, String, Symbol,
};

const HASH_SIZE: u32 = 32;
const CONTRACT_VERSION: u32 = 1;
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    ContractStopped = 4,
    ContractPaused = 5,
    InvalidHashSize = 6,
    InvalidEpoch = 7,
    EpochAlreadyExists = 8,
    EpochMonotonicityViolated = 9,
    SnapshotNotFound = 10,
    InvalidMigration = 11,
    InvalidWasmHash = 12,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub hash: Bytes,
    pub epoch: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotSubmittedEvent {
    pub epoch: u64,
    pub hash: Bytes,
    pub timestamp: u64,
    pub previous_epoch: u64, // 0 means no previous epoch
    pub ledger_sequence: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseEvent {
    pub paused_by: Address,
    pub timestamp: u64,
    pub ledger_sequence: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnpauseEvent {
    pub unpaused_by: Address,
    pub timestamp: u64,
    pub ledger_sequence: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMetadata {
    pub version: u32,
    pub upgrade_timestamp: u64,
}

/// Extended contract metadata for public disclosure
#[contracttype]
#[derive(Clone, Debug)]
pub struct PublicMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub repository: String,
    pub license: String,
}

/// Contract info combining metadata with runtime state
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractInfo {
    pub metadata: PublicMetadata,
    pub initialized: bool,
    pub paused: bool,
    pub admin: Option<Address>,
    pub total_snapshots: u64,
}

#[contracttype]
pub enum DataKey {
    Snapshots,
    LatestEpoch,
    Metadata,
    Admin,
    Stopped,
    Paused,
}

#[contract]
pub struct SnapshotContract;

#[contractimpl]
impl SnapshotContract {
    /// Internal: check if contract is stopped. Returns Err if stopped.
    fn require_not_stopped(env: &Env) -> Result<(), Error> {
        if env
            .storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::Stopped)
            .unwrap_or(false)
        {
            return Err(Error::ContractStopped);
        }
        Ok(())
    }

    /// Internal: get admin or return NotInitialized error.
    fn get_admin(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)
    }

    /// Admin-only: stop contract operations
    pub fn stop_contract(env: Env) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Stopped, &true);
        env.events().publish((symbol_short!("STOPPED"),), (admin,));
        Ok(())
    }

    /// Admin-only: resume contract operations
    pub fn resume_contract(env: Env) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Stopped, &false);
        env.events().publish((symbol_short!("RESUMED"),), (admin,));
        Ok(())
    }

    /// Initialize the contract with an admin address
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);

        let metadata = ContractMetadata {
            version: CONTRACT_VERSION,
            upgrade_timestamp: env.ledger().timestamp(),
        };
        env.storage().instance().set(&DataKey::Metadata, &metadata);
        env.storage().instance().set(&DataKey::Stopped, &false);

        env.events()
            .publish((symbol_short!("INIT"),), (admin, CONTRACT_VERSION));
        Ok(())
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
    pub fn get_admin_addr(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    /// Check if an address is the admin
    pub fn is_admin(env: Env, addr: Address) -> bool {
        match Self::get_admin_addr(env) {
            Some(admin) => admin == addr,
            None => false,
        }
    }

    /// Check if an address has permission for a function (role-based)
    pub fn check_permission(env: Env, addr: Address, _function: Symbol) -> bool {
        Self::is_admin(env, addr)
    }

    /// Transfer admin rights to a new address (only callable by existing admin)
    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        Self::require_not_stopped(&env)?;
        let current_admin = Self::get_admin(&env)?;
        current_admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &new_admin);

        env.events()
            .publish((symbol_short!("ADM_XFER"),), (current_admin, new_admin));
        Ok(())
    }

    /// Prepare for contract upgrade by validating the new WASM hash
    pub fn prepare_upgrade(env: Env, new_wasm_hash: Bytes) -> Result<(), Error> {
        Self::require_not_stopped(&env)?;
        let admin = Self::get_admin(&env)?;
        admin.require_auth();

        if new_wasm_hash.len() != HASH_SIZE {
            return Err(Error::InvalidHashSize);
        }

        env.events()
            .publish((symbol_short!("UPG_PREP"),), (new_wasm_hash,));
        Ok(())
    }

    /// Execute contract upgrade
    pub fn upgrade(env: Env, new_wasm_hash: Bytes) -> Result<(), Error> {
        Self::require_not_stopped(&env)?;
        let admin = Self::get_admin(&env)?;
        admin.require_auth();

        if new_wasm_hash.len() != HASH_SIZE {
            return Err(Error::InvalidHashSize);
        }

        let hash_bytes: BytesN<32> = new_wasm_hash
            .clone()
            .try_into()
            .map_err(|_| Error::InvalidWasmHash)?;
        env.deployer().update_current_contract_wasm(hash_bytes);

        let mut metadata: ContractMetadata = env
            .storage()
            .instance()
            .get(&DataKey::Metadata)
            .unwrap_or(ContractMetadata {
                version: CONTRACT_VERSION,
                upgrade_timestamp: env.ledger().timestamp(),
            });
        metadata.version += 1;
        metadata.upgrade_timestamp = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Metadata, &metadata);

        env.events().publish(
            (symbol_short!("UPGRADED"),),
            (new_wasm_hash, metadata.version),
        );
        Ok(())
    }

    /// Migrate data from old version to new version
    pub fn migrate(env: Env, from_version: u32) -> Result<(), Error> {
        Self::require_not_stopped(&env)?;
        let admin = Self::get_admin(&env)?;
        admin.require_auth();

        let current_version = Self::version(env.clone());
        if from_version >= current_version {
            return Err(Error::InvalidMigration);
        }

        env.events().publish(
            (symbol_short!("MIGRATED"),),
            (from_version, current_version),
        );
        Ok(())
    }

    /// Submit a snapshot hash for an epoch with input validation
    pub fn submit_snapshot(env: Env, hash: Bytes, epoch: u64) -> Result<u64, Error> {
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if is_paused {
            return Err(Error::ContractPaused);
        }

        if hash.len() != HASH_SIZE {
            return Err(Error::InvalidHashSize);
        }

        if epoch == 0 {
            return Err(Error::InvalidEpoch);
        }

        let current_latest: Option<u64> = env.storage().persistent().get(&DataKey::LatestEpoch);
        if let Some(latest) = current_latest {
            if epoch <= latest {
                if epoch == latest {
                    return Err(Error::EpochAlreadyExists);
                } else {
                    return Err(Error::EpochMonotonicityViolated);
                }
            }
        }

        let timestamp = env.ledger().timestamp();
        let ledger_sequence = env.ledger().sequence();

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

        if snapshots.contains_key(epoch) {
            return Err(Error::EpochAlreadyExists);
        }

        snapshots.set(epoch, snapshot);
        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);
        env.storage()
            .persistent()
            .set(&DataKey::LatestEpoch, &epoch);

        env.events().publish(
            (symbol_short!("SNAP_SUB"),),
            SnapshotSubmittedEvent {
                epoch,
                hash,
                timestamp,
                previous_epoch: current_latest.unwrap_or(0),
                ledger_sequence,
            },
        );

        Ok(timestamp)
    }

    /// Get snapshot hash for a specific epoch
    pub fn get_snapshot(env: Env, epoch: u64) -> Result<Bytes, Error> {
        Self::require_not_stopped(&env)?;
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

    /// Get the latest snapshot
    pub fn latest_snapshot(env: Env) -> Result<Snapshot, Error> {
        Self::require_not_stopped(&env)?;
        let latest_epoch: Option<u64> = env.storage().persistent().get(&DataKey::LatestEpoch);

        let epoch = latest_epoch.ok_or(Error::SnapshotNotFound)?;
        let snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        snapshots.get(epoch).ok_or(Error::SnapshotNotFound)
    }

    /// Verify if a hash matches any stored snapshot
    pub fn verify_snapshot(env: Env, hash: Bytes) -> bool {
        Self::require_not_stopped(&env).is_ok_and(|_| {
            let snapshots: Map<u64, Snapshot> = env
                .storage()
                .persistent()
                .get(&DataKey::Snapshots)
                .unwrap_or(Map::new(&env));

            for (_, snapshot) in snapshots.iter() {
                if snapshot.hash == hash {
                    return true;
                }
            }
            false
        })
    }

    /// Verify if a hash matches the snapshot at a specific epoch
    pub fn verify_snapshot_at_epoch(env: Env, hash: Bytes, epoch: u64) -> bool {
        if Self::require_not_stopped(&env).is_err() {
            return false;
        }
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

    /// Verify if a hash matches the latest snapshot
    pub fn verify_latest_snapshot(env: Env, hash: Bytes) -> bool {
        match Self::latest_snapshot(env) {
            Ok(snapshot) => snapshot.hash == hash,
            Err(_) => false,
        }
    }

    /// Emergency pause the contract
    pub fn pause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        let admin = Self::get_admin(&env)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Paused, &true);
        env.events().publish(
            (symbol_short!("pause"), caller.clone()),
            PauseEvent {
                paused_by: caller,
                timestamp: env.ledger().timestamp(),
                ledger_sequence: env.ledger().sequence(),
            },
        );
        Ok(())
    }

    /// Unpause the contract
    pub fn unpause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        let admin = Self::get_admin(&env)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Paused, &false);
        env.events().publish(
            (symbol_short!("unpause"), caller.clone()),
            UnpauseEvent {
                unpaused_by: caller,
                timestamp: env.ledger().timestamp(),
                ledger_sequence: env.ledger().sequence(),
            },
        );
        Ok(())
    }

    /// Check if contract is paused
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Get public contract metadata
    ///
    /// Returns metadata information including name, version, author,
    /// description, repository, and license.
    pub fn get_metadata(env: Env) -> PublicMetadata {
        PublicMetadata {
            name: String::from_str(&env, "Stellar Insights Snapshot"),
            version: String::from_str(&env, VERSION),
            author: String::from_str(&env, "Stellar Insights Team"),
            description: String::from_str(
                &env,
                "Snapshot management and verification contract for Stellar analytics",
            ),
            repository: String::from_str(&env, "https://github.com/stellar-insights/contracts"),
            license: String::from_str(&env, "MIT"),
        }
    }

    /// Get comprehensive contract information
    ///
    /// Returns both metadata and current runtime state including
    /// initialization status, pause state, admin address, and snapshot count.
    pub fn get_contract_info(env: Env) -> ContractInfo {
        ContractInfo {
            metadata: Self::get_metadata(env.clone()),
            initialized: env.storage().instance().has(&DataKey::Admin),
            paused: env
                .storage()
                .instance()
                .get(&DataKey::Paused)
                .unwrap_or(false),
            admin: env.storage().instance().get(&DataKey::Admin),
            total_snapshots: env
                .storage()
                .persistent()
                .get(&DataKey::LatestEpoch)
                .unwrap_or(0),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
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
        assert_eq!(client.get_admin_addr(), Some(admin));
    }

    #[test]
    fn test_initialize_twice_fails() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.initialize(&admin);
        let result = client.try_initialize(&admin);
        assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
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

        assert_eq!(client.get_admin_addr(), Some(new_admin));
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
    fn test_prepare_upgrade_invalid_hash() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        client.initialize(&admin);

        let invalid_hash = bytes!(&env, 0x1234);
        let result = client.try_prepare_upgrade(&invalid_hash);
        assert_eq!(result, Err(Ok(Error::InvalidHashSize)));
    }

    #[test]
    fn test_migrate() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

        client.initialize(&admin);

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
    fn test_migrate_invalid_version() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

        client.initialize(&admin);

        let result = client.try_migrate(&CONTRACT_VERSION);
        assert_eq!(result, Err(Ok(Error::InvalidMigration)));
    }

    #[test]
    fn test_submit_and_retrieve() {
        let env = Env::default();
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

        let hash = bytes!(
            &env,
            0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
        );
        let epoch = 42u64;

        let _timestamp = client.submit_snapshot(&hash, &epoch);

        let retrieved_hash = client.get_snapshot(&epoch);
        assert_eq!(retrieved_hash, Ok(hash));
    }

    #[test]
    fn test_snapshot_submitted_event() {
        let env = Env::default();
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

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
    fn test_invalid_hash_size() {
        let env = Env::default();
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

        let short_hash = bytes!(&env, 0x1234);
        let result = client.try_submit_snapshot(&short_hash, &1);
        assert_eq!(result, Err(Ok(Error::InvalidHashSize)));
    }

    #[test]
    fn test_invalid_epoch_zero() {
        let env = Env::default();
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

        let hash = bytes!(
            &env,
            0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
        );
        let result = client.try_submit_snapshot(&hash, &0);
        assert_eq!(result, Err(Ok(Error::InvalidEpoch)));
    }

    #[test]
    fn test_older_epoch_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

        let hash1 = bytes!(
            &env,
            0x1111111111111111111111111111111111111111111111111111111111111111
        );
        let hash2 = bytes!(
            &env,
            0x2222222222222222222222222222222222222222222222222222222222222222
        );

        client.submit_snapshot(&hash1, &10);
        let latest = client.latest_snapshot().unwrap();
        assert_eq!(latest.epoch, 10);

        let result = client.try_submit_snapshot(&hash2, &5);
        assert_eq!(result, Err(Ok(Error::EpochMonotonicityViolated)));
    }

    #[test]
    fn test_multiple_snapshots() {
        let env = Env::default();
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

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

        assert_eq!(client.get_snapshot(&epoch1), Ok(hash1));
        assert_eq!(client.get_snapshot(&epoch2), Ok(hash2));
    }

    #[test]
    fn test_latest_snapshot() {
        let env = Env::default();
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

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

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

        let result = client.try_latest_snapshot();
        assert_eq!(result, Err(Ok(Error::SnapshotNotFound)));
    }

    #[test]
    fn test_verify_found() {
        let env = Env::default();
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

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

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

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
        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

        assert_eq!(client.version(), CONTRACT_VERSION);
    }

    #[test]
    fn test_upgrade_mechanism_with_snapshots() {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

        client.initialize(&admin);

        let hash1 = bytes!(
            &env,
            0x1111111111111111111111111111111111111111111111111111111111111111
        );
        client.submit_snapshot(&hash1, &1);

        let wasm_hash = bytes!(
            &env,
            0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
        );
        client.prepare_upgrade(&wasm_hash);

        assert_eq!(client.get_snapshot(&1), Ok(hash1));
        assert!(client.verify_snapshot(&hash1));
    }

    #[test]
    fn test_verify_snapshot_at_epoch() {
        let env = Env::default();
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

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

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

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
