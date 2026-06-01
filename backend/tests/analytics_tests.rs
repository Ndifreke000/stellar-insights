use stellar_insights_backend::{
    analytics::compute_anchor_metrics, analytics::count_assets_per_anchor, models::AnchorStatus,
};

#[test]
fn test_compute_metrics_green_status() {
    let metrics = compute_anchor_metrics(10000, 9900, 100, Some(2000));

    assert_eq!(metrics.total_transactions, 10000);
    assert_eq!(metrics.successful_transactions, 9900);
    assert_eq!(metrics.failed_transactions, 100);
    assert_eq!(metrics.success_rate, 99.0);
    assert_eq!(metrics.failure_rate, 1.0);
    assert!(metrics.reliability_score > 90.0);
    // 99% success and 1% failure should be Green (>98% success AND <=1% failure)
    assert_eq!(metrics.status, AnchorStatus::Green);
}

#[test]
fn test_compute_metrics_yellow_status() {
    let metrics = compute_anchor_metrics(10000, 9600, 400, Some(5000));

    assert_eq!(metrics.success_rate, 96.0);
    assert_eq!(metrics.failure_rate, 4.0);
    assert!(metrics.reliability_score > 60.0 && metrics.reliability_score < 90.0);
    assert_eq!(metrics.status, AnchorStatus::Yellow);
}

#[test]
fn test_compute_metrics_red_status() {
    let metrics = compute_anchor_metrics(10000, 9300, 700, Some(8000));

    assert_eq!(metrics.success_rate, 93.0);
    assert!((metrics.failure_rate - 7.0).abs() < 1e-9);
    assert_eq!(metrics.status, AnchorStatus::Red);
}

#[test]
fn test_compute_metrics_zero_transactions() {
    let metrics = compute_anchor_metrics(0, 0, 0, None);

    assert_eq!(metrics.success_rate, 0.0);
    assert_eq!(metrics.failure_rate, 0.0);
    assert_eq!(metrics.reliability_score, 0.0);
    assert_eq!(metrics.status, AnchorStatus::Red);
}

#[test]
fn test_compute_metrics_fast_settlement() {
    let metrics = compute_anchor_metrics(1000, 990, 10, Some(500));

    // Fast settlement should contribute to high reliability score
    assert!(metrics.reliability_score > 95.0);
}

#[test]
fn test_compute_metrics_slow_settlement() {
    let metrics = compute_anchor_metrics(1000, 990, 10, Some(12000));

    // Slow settlement should lower the reliability score despite high success rate
    assert!(metrics.reliability_score < 95.0);
}

#[test]
fn test_anchor_status_boundary_green_yellow() {
    // Exactly 98% success - should be Yellow (not Green)
    assert_eq!(AnchorStatus::from_metrics(98.0, 2.0), AnchorStatus::Yellow);

    // Just above 98% with <1% failures - should be Green
    assert_eq!(AnchorStatus::from_metrics(98.1, 0.9), AnchorStatus::Green);
}

#[test]
fn test_anchor_status_boundary_yellow_red() {
    // Exactly 95% success - should be Yellow
    assert_eq!(AnchorStatus::from_metrics(95.0, 5.0), AnchorStatus::Yellow);

    // Just below 95% - should be Red
    assert_eq!(AnchorStatus::from_metrics(94.9, 5.1), AnchorStatus::Red);
}

#[test]
fn test_count_assets() {
    let assets = vec![
        "USDC".to_string(),
        "EURC".to_string(),
        "BTC".to_string(),
        "ETH".to_string(),
    ];

    assert_eq!(count_assets_per_anchor(&assets), 4);
}

#[test]
fn test_count_assets_empty() {
    let assets: Vec<String> = vec![];
    assert_eq!(count_assets_per_anchor(&assets), 0);
}

#[test]
fn test_reliability_score_calculation() {
    // Test that reliability score is properly weighted
    let high_success = compute_anchor_metrics(1000, 990, 10, Some(1000));
    let low_success = compute_anchor_metrics(1000, 900, 100, Some(1000));

    // Higher success rate should yield higher reliability score
    assert!(high_success.reliability_score > low_success.reliability_score);
}

#[test]
fn test_settlement_time_impact() {
    // Same success rate, different settlement times
    let fast = compute_anchor_metrics(1000, 950, 50, Some(1000));
    let slow = compute_anchor_metrics(1000, 950, 50, Some(9000));

    // Faster settlement should yield higher reliability score
    assert!(fast.reliability_score > slow.reliability_score);
}

#[test]
fn test_perfect_anchor() {
    let metrics = compute_anchor_metrics(10000, 10000, 0, Some(500));

    assert_eq!(metrics.success_rate, 100.0);
    assert_eq!(metrics.failure_rate, 0.0);
    assert!(metrics.reliability_score >= 99.0);
    assert_eq!(metrics.status, AnchorStatus::Green);
}

#[test]
fn test_completely_failed_anchor() {
    let metrics = compute_anchor_metrics(1000, 0, 1000, Some(20000));

    assert_eq!(metrics.success_rate, 0.0);
    assert_eq!(metrics.failure_rate, 100.0);
    assert_eq!(metrics.status, AnchorStatus::Red);
    assert!(metrics.reliability_score < 30.0);
}
