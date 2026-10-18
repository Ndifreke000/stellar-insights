//! Regression tests for the panic-free construction paths hardened in #2324.

use payraider_backend::network::StellarNetwork;
use payraider_backend::observability::job_metrics::{JobExecution, JobStatus};
use payraider_backend::rpc::StellarRpcClient;

#[test]
fn job_completion_records_matching_timestamps() {
    let success = JobExecution::new("unwrap-hardening".to_string()).complete_success();
    assert!(matches!(success.status, JobStatus::Success));
    let completed_at = success.completed_at.expect("completion time is recorded");
    assert_eq!(
        success.duration,
        Some(completed_at.duration_since(success.started_at))
    );

    let failed = JobExecution::new("unwrap-hardening".to_string()).complete_failure("boom".into());
    assert!(matches!(failed.status, JobStatus::Failed(ref e) if e == "boom"));
    assert_eq!(failed.error.as_deref(), Some("boom"));
    assert!(failed.completed_at.is_some() && failed.duration.is_some());

    let timed_out = JobExecution::new("unwrap-hardening".to_string()).complete_timeout();
    assert!(matches!(timed_out.status, JobStatus::Timeout));
    assert!(timed_out.completed_at.is_some() && timed_out.duration.is_some());
}

#[test]
fn rpc_client_can_be_built_fallibly() {
    // Testnet + mock mode needs no mainnet secrets, so construction must succeed
    // and be reachable through the `Result`-returning constructor used by main.rs.
    let client = StellarRpcClient::try_new_with_network(StellarNetwork::Testnet, true);
    assert!(client.is_ok());

    let client = StellarRpcClient::try_new(
        "https://soroban-testnet.stellar.org".to_string(),
        "https://horizon-testnet.stellar.org".to_string(),
        true,
    );
    assert!(client.is_ok());
}
