/// Webhook event definitions and payloads
use serde::{Deserialize, Serialize};

/// Corridor Health Degradation Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorHealthDegradedEvent {
    pub corridor_key: String,
    pub old_metrics: CorridorMetrics,
    pub new_metrics: CorridorMetrics,
    pub severity: String,     // "warning" | "critical"
    pub changes: Vec<String>, // e.g., ["success_rate_dropped", "latency_increased"]
}

/// Anchor Status Change Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorStatusChangedEvent {
    pub anchor_id: String,
    pub name: String,
    pub old_status: String,
    pub new_status: String,
    pub reliability_score: f64,
    pub failed_txn_count: i64,
}

/// Payment Created Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentCreatedEvent {
    pub payment_id: String,
    pub source: String,
    pub destination: String,
    pub asset_code: String,
    pub asset_issuer: String,
    pub amount: f64,
    pub timestamp: String,
}

/// Corridor Liquidity Dropped Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorLiquidityDroppedEvent {
    pub corridor_key: String,
    pub liquidity_depth_usd: f64,
    pub threshold: f64,
    pub liquidity_trend: String, // "increasing" | "stable" | "decreasing"
    pub severity: String,        // "warning" | "critical"
}

/// Corridor Metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorMetrics {
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub liquidity_depth_usd: f64,
    pub liquidity_volume_24h_usd: f64,
    pub total_attempts: i64,
    pub successful_payments: i64,
    pub failed_payments: i64,
}

/// Check if corridor metrics have degraded
pub fn check_corridor_degradation(
    old: &CorridorMetrics,
    new: &CorridorMetrics,
) -> (bool, Vec<String>) {
    let mut degraded = false;
    let mut changes = Vec::new();

    // Check success rate drop (>10%)
    if (old.success_rate - new.success_rate) > 0.10 {
        degraded = true;
        changes.push(format!(
            "success_rate_dropped: {:.1}% -> {:.1}%",
            old.success_rate * 100.0,
            new.success_rate * 100.0
        ));
    }

    // Check latency increase (>50%)
    if (new.p95_latency_ms - old.p95_latency_ms) / old.p95_latency_ms > 0.50 {
        degraded = true;
        changes.push(format!(
            "latency_increased: {:.0}ms -> {:.0}ms",
            old.p95_latency_ms, new.p95_latency_ms
        ));
    }

    // Check liquidity decrease (>30%)
    if (old.liquidity_depth_usd - new.liquidity_depth_usd) / old.liquidity_depth_usd > 0.30 {
        degraded = true;
        changes.push(format!(
            "liquidity_dropped: ${:.0} -> ${:.0}",
            old.liquidity_depth_usd, new.liquidity_depth_usd
        ));
    }

    (degraded, changes)
}

/// Determine severity based on degradation magnitude
pub fn determine_severity(old: &CorridorMetrics, new: &CorridorMetrics) -> String {
    // Critical: success rate dropped >25% or liquidity dropped >50%
    if (old.success_rate - new.success_rate) > 0.25 {
        return "critical".to_string();
    }
    if (old.liquidity_depth_usd - new.liquidity_depth_usd) / old.liquidity_depth_usd > 0.50 {
        return "critical".to_string();
    }

    // Warning: other degradations
    "warning".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corridor_degradation_detection() {
        let old = CorridorMetrics {
            success_rate: 0.95,
            avg_latency_ms: 100.0,
            p95_latency_ms: 150.0,
            p99_latency_ms: 200.0,
            liquidity_depth_usd: 1000000.0,
            liquidity_volume_24h_usd: 500000.0,
            total_attempts: 1000,
            successful_payments: 950,
            failed_payments: 50,
        };

        let mut new = old.clone();
        new.success_rate = 0.84; // 11% drop

        let (degraded, changes) = check_corridor_degradation(&old, &new);
        assert!(degraded);
        assert!(!changes.is_empty());
    }
}
