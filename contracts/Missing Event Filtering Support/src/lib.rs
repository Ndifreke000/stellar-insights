#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Result};

// Event type enum for standardization
#[contracttype]
#[derive(Clone, Copy)]
pub enum EventType {
    SnapshotSubmitted = 0,
    AdminChanged = 1,
    ContractPaused = 2,
    ContractUnpaused = 3,
    GovernanceSet = 4,
}

// Event struct for snapshot submissions
#[contracttype]
pub struct SnapshotSubmittedEvent {
    pub epoch: u64,
    pub hash: BytesN<32>,
    pub submitter: Address,
    pub timestamp: u64,
    pub ledger_sequence: u32,
}

#[contract]
pub struct SnapshotContract;

#[contractimpl]
impl SnapshotContract {
    /// Submit a snapshot with proper event filtering metadata
    ///
    /// # Arguments
    /// * `env` - The environment context
    /// * `epoch` - The epoch number for this snapshot
    /// * `hash` - The 32-byte hash of the snapshot data
    /// * `caller` - The address of the submitter
    ///
    /// # Returns
    /// * `Ok(u64)` - The timestamp of the submission
    /// * `Err(Error)` - If submission fails
    pub fn submit_snapshot(
        env: Env,
        epoch: u64,
        hash: BytesN<32>,
        caller: Address,
    ) -> Result<u64> {
        // Verify caller is authorized (can be extended with admin checks)
        caller.require_auth();

        // Get current timestamp before any state mutations
        let timestamp = env.ledger().timestamp();
        let sequence = env.ledger().sequence();

        // ✅ EMIT WITH TOPICS FOR FILTERING
        // Topics enable efficient querying:
        // Topic 0: Event type (snapshot)
        // Topic 1: Submitter address (caller)
        // Topic 2: Epoch number
        env.events().publish(
            (
                symbol_short!("snapshot"),  // Event type
                caller.clone(),             // Topic 1: submitter
                epoch,                      // Topic 2: epoch
            ),
            SnapshotSubmittedEvent {
                epoch,
                hash,
                submitter: caller,
                timestamp,
                ledger_sequence: sequence,
            },
        );

        Ok(timestamp)
    }

    /// Emit an admin change event with filtering support
    pub fn emit_admin_changed(env: Env, new_admin: Address) {
        // Topic 0: admin_changed
        // Topic 1: new_admin address
        env.events().publish(
            (symbol_short!("admin_ch"), new_admin.clone()),
            (symbol_short!("new_admin"),),
        );
    }

    /// Emit contract paused event
    pub fn emit_contract_paused(env: Env) {
        // Topic 0: paused event marker
        env.events().publish(
            (symbol_short!("pause"),),
            (symbol_short!("contract_paused"),),
        );
    }

    /// Emit contract unpaused event
    pub fn emit_contract_unpaused(env: Env) {
        // Topic 0: unpaused event marker
        env.events().publish(
            (symbol_short!("unpause"),),
            (symbol_short!("contract_unpaused"),),
        );
    }

    /// Emit governance set event with filtering support
    pub fn emit_governance_set(env: Env, governance_address: Address) {
        // Topic 0: governance_set
        // Topic 1: governance address
        env.events().publish(
            (symbol_short!("gov_set"), governance_address.clone()),
            (symbol_short!("governance"),),
        );
    }
}
