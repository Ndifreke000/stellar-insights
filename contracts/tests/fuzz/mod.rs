//! Fuzz-style property tests for Soroban contracts.
//!
//! Run with (RBAC model fuzzing budget: RBAC_FUZZ_CASES / RBAC_FUZZ_STEPS):
//!   cargo test -p contract-integration-tests --test fuzz

mod payraider_fuzz;
mod rbac_invariants;
