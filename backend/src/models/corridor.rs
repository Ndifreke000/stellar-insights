use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, sqlx::FromRow)]
pub struct Corridor {
    pub asset_a_code: String,
    pub asset_a_issuer: String,
    pub asset_b_code: String,
    pub asset_b_issuer: String,
}

impl Corridor {
    pub fn new(
        asset_a_code: String,
        asset_a_issuer: String,
        asset_b_code: String,
        asset_b_issuer: String,
    ) -> Self {
        let mut corridor = Corridor {
            asset_a_code,
            asset_a_issuer,
            asset_b_code,
            asset_b_issuer,
        };
        corridor.normalize_ordering();
        corridor
    }

    fn normalize_ordering(&mut self) {
        let asset_a_key = format!("{}:{}", self.asset_a_code, self.asset_a_issuer);
        let asset_b_key = format!("{}:{}", self.asset_b_code, self.asset_b_issuer);

        if asset_a_key > asset_b_key {
            std::mem::swap(&mut self.asset_a_code, &mut self.asset_b_code);
            std::mem::swap(&mut self.asset_a_issuer, &mut self.asset_b_issuer);
        }
    }

    pub fn to_string_key(&self) -> String {
        format!(
            "{}:{}->{}:{}",
            self.asset_a_code, self.asset_a_issuer, self.asset_b_code, self.asset_b_issuer
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CorridorMetrics {
    pub id: String,
    pub corridor_key: String,
    pub asset_a_code: String,
    pub asset_a_issuer: String,
    pub asset_b_code: String,
    pub asset_b_issuer: String,
    pub date: DateTime<Utc>,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub success_rate: f64,
    pub volume_usd: f64,
    pub avg_settlement_latency_ms: Option<i32>,
    /// Median settlement latency in milliseconds
    #[sqlx(default)]
    pub median_settlement_latency_ms: Option<i32>,
    #[serde(default)]
    pub liquidity_depth_usd: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CorridorMetricsHistory {
    pub id: String,
    pub corridor_id: String,
    pub timestamp: DateTime<Utc>,
    pub success_rate: f64,
    pub avg_settlement_latency_ms: i32,
    pub liquidity_depth_usd: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorAnalytics {
    pub corridor: Corridor,
    pub success_rate: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub volume_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRecord {
    pub id: Uuid,
    pub source_asset_code: String,
    pub source_asset_issuer: String,
    pub destination_asset_code: String,
    pub destination_asset_issuer: String,
    pub amount: f64,
    pub successful: bool,
    pub timestamp: DateTime<Utc>,
    /// Time when the transaction was submitted
    pub submission_time: Option<DateTime<Utc>>,
    /// Time when the transaction was confirmed
    pub confirmation_time: Option<DateTime<Utc>>,
}

impl PaymentRecord {
    /// Computes settlement latency in milliseconds between submission and confirmation times.
    pub fn settlement_latency_ms(&self) -> Option<i64> {
        match (self.submission_time, self.confirmation_time) {
            (Some(submitted), Some(confirmed)) => {
                let duration = confirmed.signed_duration_since(submitted);
                Some(duration.num_milliseconds())
            }
            _ => None,
        }
    }

    /// Extracts the corridor from source and destination assets.
    pub fn get_corridor(&self) -> Corridor {
        Corridor::new(
            self.source_asset_code.clone(),
            self.source_asset_issuer.clone(),
            self.destination_asset_code.clone(),
            self.destination_asset_issuer.clone(),
        )
    }
}

/// Computes the median value from a slice of i64 latency measurements.
pub fn compute_median(values: &mut [i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let len = values.len();
    if len % 2 == 0 {
        // Average of two middle values
        Some((values[len / 2 - 1] + values[len / 2]) / 2)
    } else {
        Some(values[len / 2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corridor_normalization() {
        let corridor1 = Corridor::new(
            "USDC".to_string(),
            "issuer1".to_string(),
            "EURC".to_string(),
            "issuer2".to_string(),
        );

        let corridor2 = Corridor::new(
            "EURC".to_string(),
            "issuer2".to_string(),
            "USDC".to_string(),
            "issuer1".to_string(),
        );

        assert_eq!(corridor1, corridor2);
        assert_eq!(corridor1.asset_a_code, "EURC");
        assert_eq!(corridor1.asset_b_code, "USDC");
    }

    #[test]
    fn test_corridor_same_asset_order() {
        let corridor = Corridor::new(
            "USDC".to_string(),
            "issuer1".to_string(),
            "USDC".to_string(),
            "issuer2".to_string(),
        );

        assert_eq!(corridor.asset_a_code, "USDC");
        assert_eq!(corridor.asset_b_code, "USDC");
        assert_eq!(corridor.asset_a_issuer, "issuer1");
        assert_eq!(corridor.asset_b_issuer, "issuer2");
    }

    #[test]
    fn test_corridor_to_string_key() {
        let corridor = Corridor::new(
            "USDC".to_string(),
            "issuer1".to_string(),
            "EURC".to_string(),
            "issuer2".to_string(),
        );

        let key = corridor.to_string_key();
        assert!(key.contains("EURC:issuer2"));
        assert!(key.contains("USDC:issuer1"));
        assert!(key.contains("->"));
    }

    #[test]
    fn test_payment_record_get_corridor() {
        let payment = PaymentRecord {
            id: Uuid::new_v4(),
            source_asset_code: "USDC".to_string(),
            source_asset_issuer: "issuer1".to_string(),
            destination_asset_code: "EURC".to_string(),
            destination_asset_issuer: "issuer2".to_string(),
            amount: 100.0,
            successful: true,
            timestamp: Utc::now(),
            submission_time: None,
            confirmation_time: None,
        };

        let corridor = payment.get_corridor();
        assert_eq!(corridor.asset_a_code, "EURC");
        assert_eq!(corridor.asset_b_code, "USDC");
    }

    #[test]
    fn test_payment_record_settlement_latency() {
        let now = Utc::now();
        let submitted = now - chrono::Duration::milliseconds(1500);
        
        let payment = PaymentRecord {
            id: Uuid::new_v4(),
            source_asset_code: "USDC".to_string(),
            source_asset_issuer: "issuer1".to_string(),
            destination_asset_code: "EURC".to_string(),
            destination_asset_issuer: "issuer2".to_string(),
            amount: 100.0,
            successful: true,
            timestamp: now,
            submission_time: Some(submitted),
            confirmation_time: Some(now),
        };

        assert_eq!(payment.settlement_latency_ms(), Some(1500));
    }

    #[test]
    fn test_payment_record_settlement_latency_missing_times() {
        let payment = PaymentRecord {
            id: Uuid::new_v4(),
            source_asset_code: "USDC".to_string(),
            source_asset_issuer: "issuer1".to_string(),
            destination_asset_code: "EURC".to_string(),
            destination_asset_issuer: "issuer2".to_string(),
            amount: 100.0,
            successful: true,
            timestamp: Utc::now(),
            submission_time: None,
            confirmation_time: None,
        };

        assert_eq!(payment.settlement_latency_ms(), None);
    }

    #[test]
    fn test_compute_median_odd_count() {
        let mut values = vec![1000, 3000, 2000];
        assert_eq!(compute_median(&mut values), Some(2000));
    }

    #[test]
    fn test_compute_median_even_count() {
        let mut values = vec![1000, 2000, 3000, 4000];
        assert_eq!(compute_median(&mut values), Some(2500)); // (2000 + 3000) / 2
    }

    #[test]
    fn test_compute_median_empty() {
        let mut values: Vec<i64> = vec![];
        assert_eq!(compute_median(&mut values), None);
    }

    #[test]
    fn test_compute_median_single() {
        let mut values = vec![5000];
        assert_eq!(compute_median(&mut values), Some(5000));
    }
}
