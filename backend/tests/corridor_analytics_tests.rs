use backend::services::analytics::{compute_corridor_metrics, CorridorTransaction};

#[test]
fn test_corridor_metrics_basic() {
    let txns = vec![
        CorridorTransaction {
            successful: true,
            settlement_latency_ms: Some(1000),
            amount_usd: 100.0,
        },
        CorridorTransaction {
            successful: true,
            settlement_latency_ms: Some(3000),
            amount_usd: 200.0,
        },
        CorridorTransaction {
            successful: false,
            settlement_latency_ms: None,
            amount_usd: 50.0,
        },
    ];

    let m = compute_corridor_metrics(&txns, None, 1.0);
    assert_eq!(m.total_transactions, 3);
    assert_eq!(m.successful_transactions, 2);
    assert_eq!(m.failed_transactions, 1);
    assert!((m.success_rate - 66.6667).abs() < 0.01);
    assert_eq!(m.avg_settlement_latency_ms, Some(2000));
    assert_eq!(m.liquidity_depth_usd, 300.0);
}

#[test]
fn test_corridor_metrics_empty() {
    let m = compute_corridor_metrics(&[], None, 1.0);
    assert_eq!(m.total_transactions, 0);
    assert_eq!(m.success_rate, 0.0);
    assert_eq!(m.avg_settlement_latency_ms, None);
    assert_eq!(m.liquidity_depth_usd, 0.0);
}

#[test]
fn test_corridor_metrics_all_success_no_latency() {
    let txns = vec![
        CorridorTransaction {
            successful: true,
            settlement_latency_ms: None,
            amount_usd: 10.0,
        },
        CorridorTransaction {
            successful: true,
            settlement_latency_ms: None,
            amount_usd: 20.0,
        },
    ];
    let m = compute_corridor_metrics(&txns, None, 1.0);
    assert_eq!(m.success_rate, 100.0);
    assert_eq!(m.avg_settlement_latency_ms, None);
    assert_eq!(m.liquidity_depth_usd, 30.0);
}
