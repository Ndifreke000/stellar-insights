//! Integration test harness for testnet verification.
//!
//! Run with:
//!   STELLAR_RPC_URL_TESTNET=https://soroban-testnet.stellar.org \
//!   cargo test -p contract-integration-tests --features testnet-integration

#![cfg(feature = "testnet-integration")]

mod governance_test;
mod stellar_insights_test;
mod escrow_test;

use std::env;

/// Returns the testnet RPC URL from the environment, skipping the test if unset.
pub(crate) fn rpc_url() -> String {
    env::var("STELLAR_RPC_URL_TESTNET").unwrap_or_else(|_| {
        panic!(
            "STELLAR_RPC_URL_TESTNET must be set to run testnet integration tests, \
             e.g. https://soroban-testnet.stellar.org"
        )
    })
}

/// Returns a deployed contract ID from the environment, skipping the test if unset.
pub(crate) fn contract_id(var: &str) -> String {
    env::var(var).unwrap_or_else(|_| {
        panic!(
            "{var} must be set to run testnet integration tests. \
             Run scripts/deploy-contracts-testnet.sh first, then source contracts/.env.testnet"
        )
    })
}
