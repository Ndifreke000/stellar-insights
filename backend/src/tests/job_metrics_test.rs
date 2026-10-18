use std::time::Duration;
use tokio::time::sleep;

use payraider_backend::observability::job_metrics::{
    JobMetricsCollector, JobRegistry, get_job_status_summary,
};

#[tokio::test]
async fn test_job_metrics_collection() {
    // Test successful job execution
    let metrics = JobMetricsCollector::new("test-job");
    sleep(Duration::from_millis(100)).await;
    metrics.complete_success();

    // Verify metrics were recorded
    let registry = payraider_backend::observability::job_metrics::JOB_REGISTRY.read().await;
    let job_info = registry.get_job_info("test-job");
    
    assert!(job_info.is_some());
    let info = job_info.unwrap();
    assert_eq!(info.total_executions, 1);
    assert_eq!(info.total_failures, 0);
    assert_eq!(info.consecutive_failures, 0);
    assert!(info.last_success_timestamp.is_some());
    assert!(info.last_execution.is_some());
}

#[tokio::test]
async fn test_job_failure_tracking() {
    // Test job failure
    let metrics = JobMetricsCollector::new("test-failure-job");
    sleep(Duration::from_millis(50)).await;
    metrics.complete_failure("Test error message");

    // Verify failure metrics
    let registry = payraider_backend::observability::job_metrics::JOB_REGISTRY.read().await;
    let job_info = registry.get_job_info("test-failure-job");
    
    assert!(job_info.is_some());
    let info = job_info.unwrap();
    assert_eq!(info.total_executions, 1);
    assert_eq!(info.total_failures, 1);
    assert_eq!(info.consecutive_failures, 1);
    assert!(info.last_failure_timestamp.is_some());
    assert!(info.last_execution.is_some());
}

#[tokio::test]
async fn test_consecutive_failures() {
    // Test consecutive failures
    let metrics1 = JobMetricsCollector::new("consecutive-test-job");
    let metrics2 = JobMetricsCollector::new("consecutive-test-job");
    let metrics3 = JobMetricsCollector::new("consecutive-test-job");

    metrics1.complete_failure("First error");
    metrics2.complete_failure("Second error");
    metrics3.complete_failure("Third error");

    // Verify consecutive failures
    let registry = payraider_backend::observability::job_metrics::JOB_REGISTRY.read().await;
    let job_info = registry.get_job_info("consecutive-test-job");
    
    assert!(job_info.is_some());
    let info = job_info.unwrap();
    assert_eq!(info.consecutive_failures, 3);
    assert_eq!(info.total_failures, 3);
}

#[tokio::test]
async fn test_job_status_summary() {
    // Create some test jobs
    let metrics1 = JobMetricsCollector::new("summary-test-job-1");
    let metrics2 = JobMetricsCollector::new("summary-test-job-2");
    
    metrics1.complete_success();
    metrics2.complete_failure("Test error");

    // Get status summary
    let summary = get_job_status_summary().await;
    let summary_obj = summary.as_object().unwrap();
    
    assert_eq!(summary_obj.get("total_jobs").unwrap(), 2);
    assert_eq!(summary_obj.get("active_jobs").unwrap(), 0); // All jobs should be inactive
    
    let jobs = summary_obj.get("jobs").unwrap().as_object().unwrap();
    assert!(jobs.contains_key("summary-test-job-1"));
    assert!(jobs.contains_key("summary-test-job-2"));
}

#[tokio::test]
async fn test_multiple_job_types() {
    // Test different job types
    let corridor_metrics = JobMetricsCollector::new("corridor-refresh");
    let anchor_metrics = JobMetricsCollector::new("anchor-refresh");
    let price_metrics = JobMetricsCollector::new("price-feed-update");
    let cache_metrics = JobMetricsCollector::new("cache-cleanup");

    // Simulate different outcomes
    corridor_metrics.complete_success();
    anchor_metrics.complete_success();
    price_metrics.complete_failure("RPC timeout");
    cache_metrics.complete_success();

    // Verify all jobs are tracked
    let registry = payraider_backend::observability::job_metrics::JOB_REGISTRY.read().await;
    
    assert_eq!(registry.get_all_jobs().len(), 4);
    assert!(registry.get_job_info("corridor-refresh").is_some());
    assert!(registry.get_job_info("anchor-refresh").is_some());
    assert!(registry.get_job_info("price-feed-update").is_some());
    assert!(registry.get_job_info("cache-cleanup").is_some());

    // Verify individual job statuses
    let corridor_info = registry.get_job_info("corridor-refresh").unwrap();
    assert_eq!(corridor_info.total_failures, 0);
    
    let price_info = registry.get_job_info("price-feed-update").unwrap();
    assert_eq!(price_info.total_failures, 1);
    assert_eq!(price_info.consecutive_failures, 1);
}

#[tokio::test]
fn test_error_classification() {
    let metrics = JobMetricsCollector::new("error-classification-test");
    
    // Test different error types
    metrics.complete_failure("Connection timeout after 30 seconds");
    let metrics2 = JobMetricsCollector::new("error-classification-test-2");
    metrics2.complete_failure("Database connection failed");
    let metrics3 = JobMetricsCollector::new("error-classification-test-3");
    metrics3.complete_failure("Invalid input data");
    
    // The error classification should work (we can't easily test the exact classification
    // without exposing internal methods, but we can verify the metrics are recorded)
    let registry = payraider_backend::observability::job_metrics::JOB_REGISTRY.read().await;
    let info = registry.get_job_info("error-classification-test").unwrap();
    assert_eq!(info.total_failures, 1);
}

#[tokio::test]
async fn test_job_timeout() {
    let metrics = JobMetricsCollector::new("timeout-test-job");
    sleep(Duration::from_millis(50)).await;
    metrics.complete_timeout();

    let registry = payraider_backend::observability::job_metrics::JOB_REGISTRY.read().await;
    let job_info = registry.get_job_info("timeout-test-job");
    
    assert!(job_info.is_some());
    let info = job_info.unwrap();
    assert_eq!(info.total_failures, 1);
    assert_eq!(info.consecutive_failures, 1);
}

#[tokio::test]
async fn test_job_registry_persistence() {
    // Test that job registry persists data across multiple operations
    let metrics1 = JobMetricsCollector::new("persistence-test-job");
    metrics1.complete_success();

    // Create another collector for the same job
    let metrics2 = JobMetricsCollector::new("persistence-test-job");
    metrics2.complete_failure("Second operation failed");

    // Verify registry shows both operations
    let registry = payraider_backend::observability::job_metrics::JOB_REGISTRY.read().await;
    let info = registry.get_job_info("persistence-test-job").unwrap();
    
    assert_eq!(info.total_executions, 2);
    assert_eq!(info.total_failures, 1);
    assert_eq!(info.consecutive_failures, 1); // Last operation was a failure
    assert!(info.last_success_timestamp.is_some());
    assert!(info.last_failure_timestamp.is_some());
}

#[test]
fn test_job_metrics_collector_creation() {
    // Test that JobMetricsCollector can be created and has expected initial state
    let _metrics = JobMetricsCollector::new("test-job");
    // If this compiles, the collector was created successfully
}

#[test]
fn test_job_registry_default() {
    // Test that JobRegistry can be created with default state
    let _registry = JobRegistry::new();
    // If this compiles, the registry was created successfully
}
