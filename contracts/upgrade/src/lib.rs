#![no_std]
extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, BytesN, Env,
};

/// Storage keys for instance storage
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    /// Governance address authorized to manage upgrades
    Governance,
    /// Current contract version
    Version,
    /// Total count of upgrade proposals
    ProposalCount,
    /// Flag indicating if contract migration has been completed
    MigrationComplete,
    /// Upgrade proposal storage (keyed by proposal ID)
    Proposal(u32),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeProposal {
    pub id: u32,
    pub new_wasm_hash: BytesN<32>,
    pub proposer: Address,
    pub created_at: u64,
    pub timelock_until: u64,
    pub status: u32,
}

#[contract]
pub struct UpgradeManager;

/// Helper function to ensure migration has been completed.
///
/// Gating all non-upgrade entrypoints behind this check prevents calling
/// contract logic before the new WASM binary is ready to handle requests.
fn require_migration_complete(env: &Env) {
    let migration_complete: bool = env
        .storage()
        .instance()
        .get(&DataKey::MigrationComplete)
        .unwrap_or(false);

    if !migration_complete {
        panic!("Contract migration not yet complete; upgrade in progress");
    }
}

#[contractimpl]
impl UpgradeManager {
    /// Initialize the upgrade manager with governance address and mark migration as complete.
    ///
    /// This must be called immediately after a WASM upgrade to gate further operations.
    pub fn init(env: &Env, governance: Address) {
        env.storage()
            .instance()
            .set(&DataKey::Governance, &governance);
        env.storage()
            .instance()
            .set(&DataKey::Version, &ContractVersion {
                major: 1,
                minor: 0,
                patch: 0,
            });
        env.storage()
            .instance()
            .set(&DataKey::ProposalCount, &0u32);
        env.storage()
            .instance()
            .set(&DataKey::MigrationComplete, &true);
    }

    pub fn version(env: &Env) -> ContractVersion {
        require_migration_complete(env);

        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or(ContractVersion {
                major: 1,
                minor: 0,
                patch: 0,
            })
    }

    pub fn propose_upgrade(env: &Env, governance: Address, new_wasm_hash: BytesN<32>) -> u32 {
        require_migration_complete(env);
        governance.require_auth();

        let proposal_count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);

        let proposal_id = proposal_count + 1;
        let timelock_until = env.ledger().timestamp() + (48 * 3600);

        let proposal = UpgradeProposal {
            id: proposal_id,
            new_wasm_hash: new_wasm_hash.clone(),
            proposer: governance.clone(),
            created_at: env.ledger().timestamp(),
            timelock_until,
            status: 0,
        };

        env.storage()
            .instance()
            .set(&DataKey::ProposalCount, &proposal_id);

        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        proposal_id
    }

    pub fn approve_upgrade(env: &Env, governance: Address, proposal_id: u32) {
        require_migration_complete(env);
        governance.require_auth();

        let mut proposal: UpgradeProposal = env
            .storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found");

        if proposal.status != 0 {
            panic!("Proposal is not pending");
        }

        proposal.status = 1;
        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);
    }

    pub fn execute_upgrade(env: &Env, proposal_id: u32) {
        require_migration_complete(env);

        let mut proposal: UpgradeProposal = env
            .storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found");

        if proposal.status != 1 {
            panic!("Proposal is not approved");
        }

        if env.ledger().timestamp() < proposal.timelock_until {
            panic!("Timelock period not expired");
        }

        let old_version = Self::version_internal(env);

        env.deployer()
            .update_current_contract_wasm(proposal.new_wasm_hash.clone());

        // Reset migration flag for the new WASM binary
        env.storage()
            .instance()
            .set(&DataKey::MigrationComplete, &false);

        proposal.status = 2;
        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        let new_version = ContractVersion {
            major: old_version.major,
            minor: old_version.minor + 1,
            patch: 0,
        };
        env.storage()
            .instance()
            .set(&DataKey::Version, &new_version);
    }

    pub fn emergency_upgrade(env: &Env, governance: Address, new_wasm_hash: BytesN<32>) {
        require_migration_complete(env);
        governance.require_auth();

        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());

        // Reset migration flag for the new WASM binary
        env.storage()
            .instance()
            .set(&DataKey::MigrationComplete, &false);

        let new_version = ContractVersion {
            major: 1,
            minor: 0,
            patch: 1,
        };
        env.storage()
            .instance()
            .set(&DataKey::Version, &new_version);
    }

    pub fn get_proposal(env: &Env, proposal_id: u32) -> UpgradeProposal {
        require_migration_complete(env);

        env.storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found")
    }

    /// Internal version retrieval without migration check.
    /// Used by execute_upgrade to read version before setting migration flag to false.
    fn version_internal(env: &Env) -> ContractVersion {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or(ContractVersion {
                major: 1,
                minor: 0,
                patch: 0,
            })
    }
}
