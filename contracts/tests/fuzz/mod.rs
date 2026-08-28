//! Fuzz-style property tests for Soroban contracts.
//!
//! These tests exercise boundary values and invariants that need coverage
//! before mainnet: epoch IDs, token amounts, address inputs, and state
//! transitions.  They run against the soroban test host — no live network
//! required.
//!
//! Run with:
//!   cargo test -p contract-integration-tests --test fuzz

mod escrow_fuzz;
mod governance_fuzz;
mod stellar_insights_fuzz;
