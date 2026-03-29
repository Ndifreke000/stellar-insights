// Moved from analytics.rs

use crate::models::AnchorMetrics;
use crate::models::AnchorStatus;

pub fn compute_anchor_metrics(
    total_transactions: i64,
    successful_transactions: i64,
    failed_transactions: i64,
    avg_settlement_time_ms: Option<i32>,
) -> AnchorMetrics {
    // Full implementation from original analytics.rs
    if total_transactions == 0 {
        return AnchorMetrics {
            success_rate: 0.0,
            failure_rate: 0.0,
            reliability_score: 0.0,
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            avg_settlement_time_ms: None,
            status: AnchorStatus::Red,
        };
    }

    let success_rate = (successful_transactions as f64 / total_transactions as f64) * 100.0;
    let failure_rate = (failed_transactions as f64 / total_transactions as f64) * 100.0;

    let success_rate = (success_rate * 100.0).round() / 100.0;
    let failure_rate = (failure_rate * 100.0).round() / 100.0;

    let settlement_time_score = calculate_settlement_time_score(avg_settlement_time_ms);
    let reliability_score = (success_rate * 0.7) + (settlement_time_score * 0.3);

    let status = AnchorStatus::from_metrics(success_rate, failure_rate);

    AnchorMetrics {
        success_rate,
        failure_rate,
        reliability_score,
        total_transactions,
        successful_transactions,
        failed_transactions,
        avg_settlement_time_ms,
        status,
    }
}

fn calculate_settlement_time_score(avg_settlement_time_ms: Option<i32>) -> f64 {
    const MAX_SETTLEMENT_TIME_MS: f64 = 10_000.0;
    const MIN_SETTLEMENT_TIME_MS: f64 = 1_000.0;

    match avg_settlement_time_ms {
        Some(time_ms) if time_ms <= MIN_SETTLEMENT_TIME_MS as i32 => 100.0,
        Some(time_ms) if time_ms >= MAX_SETTLEMENT_TIME_MS as i32 => 0.0,
        Some(time_ms) => {
            let normalized = (MAX_SETTLEMENT_TIME_MS - f64::from(time_ms))
                / (MAX_SETTLEMENT_TIME_MS - MIN_SETTLEMENT_TIME_MS);
            normalized * 100.0
        }
        None => 50.0,
    }
}

