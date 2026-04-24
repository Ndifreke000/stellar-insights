#![no_std]
extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env,
};

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

#[contractimpl]
impl UpgradeManager {
    pub fn init(env: &Env, governance: Address) {
        env.storage()
            .instance()
            .set(&symbol_short!("gov"), &governance);
        env.storage()
            .instance()
            .set(&symbol_short!("version"), &ContractVersion {
                major: 1,
                minor: 0,
                patch: 0,
            });
        env.storage()
            .instance()
            .set(&symbol_short!("proposal_count"), &0u32);
    }

    pub fn version(env: &Env) -> ContractVersion {
        env.storage()
            .instance()
            .get(&symbol_short!("version"))
            .unwrap_or(ContractVersion {
                major: 1,
                minor: 0,
                patch: 0,
            })
    }

    pub fn propose_upgrade(env: &Env, new_wasm_hash: BytesN<32>) -> u32 {
        let governance: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("gov"))
            .expect("Governance not initialized");

        if env.invoker() != governance {
            panic!("Only governance can propose upgrades");
        }

        let proposal_count: u32 = env
            .storage()
            .instance()
            .get(&symbol_short!("proposal_count"))
            .unwrap_or(0);

        let proposal_id = proposal_count + 1;
        let timelock_until = env.ledger().timestamp() + (48 * 3600);

        let proposal = UpgradeProposal {
            id: proposal_id,
            new_wasm_hash: new_wasm_hash.clone(),
            proposer: env.invoker(),
            created_at: env.ledger().timestamp(),
            timelock_until,
            status: 0,
        };

        env.storage()
            .instance()
            .set(&symbol_short!("proposal_count"), &proposal_id);

        let key = format!("proposal_{}", proposal_id);
        env.storage()
            .instance()
            .set(&symbol_short!(&key), &proposal);

        proposal_id
    }

    pub fn approve_upgrade(env: &Env, proposal_id: u32) {
        let governance: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("gov"))
            .expect("Governance not initialized");

        if env.invoker() != governance {
            panic!("Only governance can approve upgrades");
        }

        let key = format!("proposal_{}", proposal_id);
        let mut proposal: UpgradeProposal = env
            .storage()
            .instance()
            .get(&symbol_short!(&key))
            .expect("Proposal not found");

        if proposal.status != 0 {
            panic!("Proposal is not pending");
        }

        proposal.status = 1;
        env.storage()
            .instance()
            .set(&symbol_short!(&key), &proposal);
    }

    pub fn execute_upgrade(env: &Env, proposal_id: u32) {
        let key = format!("proposal_{}", proposal_id);
        let mut proposal: UpgradeProposal = env
            .storage()
            .instance()
            .get(&symbol_short!(&key))
            .expect("Proposal not found");

        if proposal.status != 1 {
            panic!("Proposal is not approved");
        }

        if env.ledger().timestamp() < proposal.timelock_until {
            panic!("Timelock period not expired");
        }

        let old_version = Self::version(env);

        env.deployer()
            .update_current_contract_wasm(proposal.new_wasm_hash.clone());

        proposal.status = 2;
        env.storage()
            .instance()
            .set(&symbol_short!(&key), &proposal);

        let new_version = ContractVersion {
            major: old_version.major,
            minor: old_version.minor + 1,
            patch: 0,
        };
        env.storage()
            .instance()
            .set(&symbol_short!("version"), &new_version);
    }

    pub fn emergency_upgrade(env: &Env, new_wasm_hash: BytesN<32>) {
        let governance: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("gov"))
            .expect("Governance not initialized");

        if env.invoker() != governance {
            panic!("Only governance can trigger emergency upgrade");
        }

        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());

        let new_version = ContractVersion {
            major: 1,
            minor: 0,
            patch: 1,
        };
        env.storage()
            .instance()
            .set(&symbol_short!("version"), &new_version);
    }

    pub fn get_proposal(env: &Env, proposal_id: u32) -> UpgradeProposal {
        let key = format!("proposal_{}", proposal_id);
        env.storage()
            .instance()
            .get(&symbol_short!(&key))
            .expect("Proposal not found")
    }
}
