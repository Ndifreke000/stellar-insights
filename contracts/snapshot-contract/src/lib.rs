#![no_std]
extern crate std;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Bytes, BytesN, Env,
    Map, String, Symbol, Vec,
};

const HASH_SIZE: u32 = 32;
const CONTRACT_VERSION: u32 = 1;
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ~30 days at 5 s/ledger
const LEDGERS_TO_EXTEND: u32 = 518_400;
const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

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
    MultiSigNotInitialized = 13,
    InvalidThreshold = 14,
    SignerNotAdmin = 15,
    ActionNotFound = 16,
    ActionExpired = 17,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiSigConfig {
    pub admins: Vec<Address>,
    pub threshold: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAction {
    pub action_id: u64,
    pub action_type: String,
    pub signatures: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
}

#[contracttype]
pub enum DataKey {
    Snapshots,
    LatestEpoch,
    Metadata,
    Admin,
    Stopped,
    Paused,
    ReentrancyGuard,
    MultiSigConfig,
    PendingAction(u64),
    NextActionId,
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

    /// Internal: check and set reentrancy guard
    fn check_and_set_reentrancy_guard(env: &Env) -> Result<(), &'static str> {
        let is_locked: bool = env
            .storage()
            .instance()
            .get(&DataKey::ReentrancyGuard)
            .unwrap_or(false);

        if is_locked {
            return Err("Reentrancy detected: function already in execution");
        }

        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);
        Ok(())
    }

    /// Internal: clear reentrancy guard
    fn clear_reentrancy_guard(env: &Env) {
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &false);
    }

    fn get_next_action_id(env: &Env) -> u64 {
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextActionId)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::NextActionId, &(id + 1));
        id
    }

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
        bump_instance(&env);
        env.events().publish((symbol_short!("STOPPED"),), (admin,));
        Ok(())
    }

    /// Admin-only: resume contract operations
    pub fn resume_contract(env: Env) -> Result<(), Error> {
        let admin = Self::get_admin(&env)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Stopped, &false);
        bump_instance(&env);
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
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);

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
        bump_instance(&env);

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
        bump_instance(&env);

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

        bump_instance(&env);

        env.events().publish(
            (symbol_short!("MIGRATED"),),
            (from_version, current_version),
        );
        Ok(())
    }

    /// Submit a snapshot hash for an epoch with input validation
    ///
    /// # Arguments
    /// * `hash` - 32-byte SHA-256 hash of analytics snapshot
    /// * `epoch` - Epoch identifier (must be positive)
    ///
    /// # Panics
    /// * If contract is paused for emergency maintenance
    /// * If hash is not exactly 32 bytes
    /// * If epoch is 0
    /// * If a snapshot already exists for this epoch
    /// * If epoch <= latest (monotonicity violated: out-of-order or duplicate)
    /// * If reentrancy is detected
    ///
    /// # Returns
    /// * Ledger timestamp when snapshot was recorded
    pub fn submit_snapshot(env: Env, hash: Bytes, epoch: u64) -> Result<u64, Error> {
        // Check reentrancy guard
        if let Err(e) = Self::check_and_set_reentrancy_guard(&env) {
            panic!("{}", e);
        }

        // Check if contract is paused
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if is_paused {
            Self::clear_reentrancy_guard(&env);
            return Err(Error::ContractPaused);
        }

        // Validate inputs
        if hash.len() != HASH_SIZE {
            Self::clear_reentrancy_guard(&env);
            return Err(Error::InvalidHashSize);
        }

        if epoch == 0 {
            Self::clear_reentrancy_guard(&env);
            return Err(Error::InvalidEpoch);
        }

        let current_latest: Option<u64> = env.storage().persistent().get(&DataKey::LatestEpoch);
        if let Some(latest) = current_latest {
            if epoch <= latest {
                Self::clear_reentrancy_guard(&env);
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
            Self::clear_reentrancy_guard(&env);
            return Err(Error::EpochAlreadyExists);
        }

        snapshots.set(epoch, snapshot);
        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);
        env.storage()
            .persistent()
            .set(&DataKey::LatestEpoch, &epoch);

        // Extend storage TTL (~30 days at 5s per ledger)
        const LEDGERS_TO_EXTEND: u32 = 518_400;
        env.storage().persistent().extend_ttl(
            &DataKey::Snapshots,
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        env.storage().persistent().extend_ttl(
            &DataKey::LatestEpoch,
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

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

        // Clear guard before returning
        Self::clear_reentrancy_guard(&env);

        Ok(timestamp)
    }

    /// Get snapshot hash for a specific epoch
    pub fn get_snapshot(env: Env, epoch: u64) -> Result<Bytes, Error> {
        Self::require_not_stopped(&env)?;
        const LEDGERS_TO_EXTEND: u32 = 518_400;
        if env.storage().persistent().has(&DataKey::Snapshots) {
            env.storage().persistent().extend_ttl(
                &DataKey::Snapshots,
                LEDGERS_TO_EXTEND,
                LEDGERS_TO_EXTEND,
            );
        }
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
        if env.storage().persistent().has(&DataKey::LatestEpoch) {
            env.storage().persistent().extend_ttl(
                &DataKey::LatestEpoch,
                LEDGERS_TO_EXTEND,
                LEDGERS_TO_EXTEND,
            );
        }
        let latest_epoch: Option<u64> = env.storage().persistent().get(&DataKey::LatestEpoch);

        let epoch = latest_epoch.ok_or(Error::SnapshotNotFound)?;
        if env.storage().persistent().has(&DataKey::Snapshots) {
            env.storage().persistent().extend_ttl(
                &DataKey::Snapshots,
                LEDGERS_TO_EXTEND,
                LEDGERS_TO_EXTEND,
            );
        }
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
        bump_instance(&env);
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
        bump_instance(&env);
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

    pub fn initialize_multisig(
        env: Env,
        admins: Vec<Address>,
        threshold: u32,
    ) -> Result<(), Error> {
        if threshold == 0 || threshold > admins.len() as u32 {
            return Err(Error::InvalidThreshold);
        }

        let config = MultiSigConfig { admins, threshold };
        env.storage()
            .instance()
            .set(&DataKey::MultiSigConfig, &config);
        bump_instance(&env);

        Ok(())
    }

    pub fn propose_action(
        env: Env,
        proposer: Address,
        action_type: String,
        _action_data: BytesN<32>,
    ) -> Result<u64, Error> {
        proposer.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&proposer) {
            return Err(Error::Unauthorized);
        }

        let action_id = Self::get_next_action_id(&env);

        let mut sigs = Vec::new(&env);
        sigs.push_back(proposer);

        let pending = PendingAction {
            action_id,
            action_type,
            signatures: sigs,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 86_400,
        };

        env.storage()
            .persistent()
            .set(&DataKey::PendingAction(action_id), &pending);
        env.storage().persistent().extend_ttl(
            &DataKey::PendingAction(action_id),
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

        Ok(action_id)
    }

    pub fn sign_action(env: Env, signer: Address, action_id: u64) -> Result<bool, Error> {
        signer.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&signer) {
            return Err(Error::Unauthorized);
        }

        let mut pending: PendingAction = env
            .storage()
            .persistent()
            .get(&DataKey::PendingAction(action_id))
            .ok_or(Error::ActionNotFound)?;

        if env.ledger().timestamp() > pending.expires_at {
            return Err(Error::ActionExpired);
        }

        if !pending.signatures.contains(&signer) {
            pending.signatures.push_back(signer);
            env.storage()
                .persistent()
                .set(&DataKey::PendingAction(action_id), &pending);
            env.storage().persistent().extend_ttl(
                &DataKey::PendingAction(action_id),
                LEDGERS_TO_EXTEND,
                LEDGERS_TO_EXTEND,
            );
        }

        Ok(pending.signatures.len() >= config.threshold)
    }

    pub fn get_multisig_config(env: Env) -> Option<MultiSigConfig> {
        env.storage().instance().get(&DataKey::MultiSigConfig)
    }

    pub fn get_pending_action(env: Env, action_id: u64) -> Option<PendingAction> {
        env.storage()
            .persistent()
            .get(&DataKey::PendingAction(action_id))
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
        vec, Env, TryIntoVal,
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
        assert_eq!(retrieved_hash, hash);
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
        let latest = client.latest_snapshot();
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

        assert_eq!(client.get_snapshot(&epoch1), hash1);
        assert_eq!(client.get_snapshot(&epoch2), hash2);
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

        let snapshot = client.latest_snapshot();
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

        assert_eq!(client.get_snapshot(&1), hash1);
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

    #[test]
    #[should_panic(expected = "Reentrancy detected")]
    fn test_reentrancy_protection() {
        let env = Env::default();
        env.mock_all_auths();

        let client =
            SnapshotContractClient::new(&env, &env.register_contract(None, SnapshotContract));

        let hash = bytes!(
            &env,
            0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
        );

        // Manually set the reentrancy guard to simulate a reentrant call
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);

        // This should panic due to reentrancy detection
        client.submit_snapshot(&hash, &1);
    }

    #[test]
    fn test_multisig_initialization() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        let admins = vec![&env, admin1.clone(), admin2.clone()];
        let threshold = 2;

        client.initialize_multisig(&admins, &threshold);

        let config = client.get_multisig_config().unwrap();
        assert_eq!(config.admins.len(), 2);
        assert_eq!(config.threshold, 2);
        assert!(config.admins.contains(&admin1));
        assert!(config.admins.contains(&admin2));
    }

    #[test]
    fn test_multisig_proposal() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        let admins = vec![&env, admin1.clone(), admin2.clone()];
        client.initialize_multisig(&admins, &2);

        let action_type = soroban_sdk::String::from_str(&env, "upgrade");
        let action_data = BytesN::from_array(&env, &[0u8; 32]);
        let action_id = client.propose_action(&admin1, &action_type, &action_data);

        let pending = client.get_pending_action(&action_id).unwrap();
        assert_eq!(pending.action_id, action_id);
        assert_eq!(pending.signatures.len(), 1);
        assert_eq!(pending.signatures.get(0).unwrap(), admin1);
    }

    #[test]
    fn test_multisig_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SnapshotContract);
        let client = SnapshotContractClient::new(&env, &contract_id);

        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        let admins = vec![&env, admin1.clone(), admin2.clone()];
        client.initialize_multisig(&admins, &2);

        let action_type = soroban_sdk::String::from_str(&env, "test");
        let action_data = BytesN::from_array(&env, &[0u8; 32]);
        let action_id = client.propose_action(&admin1, &action_type, &action_data);

        // First signature already added by proposer
        let reached_first = client.sign_action(&admin1, &action_id);
        assert!(!reached_first); // Already signed, still 1/2

        // Second signature
        let reached_second = client.sign_action(&admin2, &action_id);
        assert!(reached_second); // Now 2/2

        let pending = client.get_pending_action(&action_id).unwrap();
        assert_eq!(pending.signatures.len(), 2);
    }
}
