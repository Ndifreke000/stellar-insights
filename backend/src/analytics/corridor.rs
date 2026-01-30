use crate::models::corridor::{Corridor, CorridorAnalytics, PaymentRecord};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub fn compute_corridor_analytics(payments: &[PaymentRecord]) -> Vec<CorridorAnalytics> {
    let mut corridor_payments: HashMap<String, Vec<&PaymentRecord>> = HashMap::new();

    for payment in payments {
        let corridor = payment.get_corridor();
        let corridor_key = corridor.to_string_key();
        corridor_payments
            .entry(corridor_key)
            .or_default()
            .push(payment);
    }

    let mut analytics = Vec::new();

    for (corridor_key, corridor_payment_records) in corridor_payments {
        let total_transactions = corridor_payment_records.len() as i64;
        let successful_transactions = corridor_payment_records
            .iter()
            .filter(|p| p.successful)
            .count() as i64;
        let failed_transactions = total_transactions - successful_transactions;

        let success_rate = if total_transactions > 0 {
            (successful_transactions as f64 / total_transactions as f64) * 100.0
        } else {
            0.0
        };

        let volume_usd: f64 = corridor_payment_records.iter().map(|p| p.amount).sum();

        let corridor = parse_corridor_key(&corridor_key);

        analytics.push(CorridorAnalytics {
            corridor,
            success_rate,
            total_transactions,
            successful_transactions,
            failed_transactions,
            volume_usd,
        });
    }

    analytics.sort_by(|a, b| b.total_transactions.cmp(&a.total_transactions));
    analytics
}

pub fn compute_corridor_analytics_for_date(
    payments: &[PaymentRecord],
    target_date: DateTime<Utc>,
) -> Vec<CorridorAnalytics> {
    let filtered_payments: Vec<PaymentRecord> = payments
        .iter()
        .filter(|p| p.timestamp.date_naive() == target_date.date_naive())
        .cloned()
        .collect();

    compute_corridor_analytics(&filtered_payments)
}

pub fn get_top_corridors_by_volume(
    analytics: &[CorridorAnalytics],
    limit: usize,
) -> Vec<&CorridorAnalytics> {
    let mut sorted_analytics = analytics.iter().collect::<Vec<_>>();
    sorted_analytics.sort_by(|a, b| b.volume_usd.partial_cmp(&a.volume_usd).unwrap());
    sorted_analytics.truncate(limit);
    sorted_analytics
}

pub fn get_top_corridors_by_transactions(
    analytics: &[CorridorAnalytics],
    limit: usize,
) -> Vec<&CorridorAnalytics> {
    let mut sorted_analytics = analytics.iter().collect::<Vec<_>>();
    sorted_analytics.sort_by(|a, b| b.total_transactions.cmp(&a.total_transactions));
    sorted_analytics.truncate(limit);
    sorted_analytics
}

pub fn get_corridors_by_success_rate(
    analytics: &[CorridorAnalytics],
    min_transactions: i64,
) -> Vec<&CorridorAnalytics> {
    analytics
        .iter()
        .filter(|a| a.total_transactions >= min_transactions)
        .collect::<Vec<_>>()
        .into_iter()
        .filter(|a| a.total_transactions > 0)
        .collect()
}

fn parse_corridor_key(corridor_key: &str) -> Corridor {
    let parts: Vec<&str> = corridor_key.split("->").collect();
    let asset_a_parts: Vec<&str> = parts[0].split(':').collect();
    let asset_b_parts: Vec<&str> = parts[1].split(':').collect();

    Corridor::new(
        asset_a_parts[0].to_string(),
        asset_a_parts[1].to_string(),
        asset_b_parts[0].to_string(),
        asset_b_parts[1].to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_payment(
        source_code: &str,
        source_issuer: &str,
        dest_code: &str,
        dest_issuer: &str,
        amount: f64,
        successful: bool,
    ) -> PaymentRecord {
        PaymentRecord {
            id: Uuid::new_v4(),
            source_asset_code: source_code.to_string(),
            source_asset_issuer: source_issuer.to_string(),
            destination_asset_code: dest_code.to_string(),
            destination_asset_issuer: dest_issuer.to_string(),
            amount,
            successful,
            timestamp: Utc::now(),
            submission_time: None,
            confirmation_time: None,
        }
    }

    #[test]
    fn test_compute_corridor_analytics_basic() {
        let payments = vec![
            create_test_payment("USDC", "issuer1", "EURC", "issuer2", 100.0, true),
            create_test_payment("USDC", "issuer1", "EURC", "issuer2", 50.0, true),
            create_test_payment("EURC", "issuer2", "USDC", "issuer1", 75.0, false),
        ];

        let analytics = compute_corridor_analytics(&payments);
        assert_eq!(analytics.len(), 1);

        let corridor_analytics = &analytics[0];
        assert_eq!(corridor_analytics.total_transactions, 3);
        assert_eq!(corridor_analytics.successful_transactions, 2);
        assert_eq!(corridor_analytics.failed_transactions, 1);
        assert!((corridor_analytics.success_rate - 66.66666666666667).abs() < 0.0001);
        assert_eq!(corridor_analytics.volume_usd, 225.0);
    }

    #[test]
    fn test_compute_corridor_analytics_multiple_corridors() {
        let payments = vec![
            create_test_payment("USDC", "issuer1", "EURC", "issuer2", 100.0, true),
            create_test_payment("USDC", "issuer1", "BTC", "issuer3", 200.0, false),
            create_test_payment("EURC", "issuer2", "BTC", "issuer3", 150.0, true),
        ];

        let analytics = compute_corridor_analytics(&payments);
        assert_eq!(analytics.len(), 3);
    }

    #[test]
    fn test_compute_corridor_analytics_empty() {
        let payments = vec![];
        let analytics = compute_corridor_analytics(&payments);
        assert_eq!(analytics.len(), 0);
    }

    #[test]
    fn test_compute_corridor_analytics_all_successful() {
        let payments = vec![
            create_test_payment("USDC", "issuer1", "EURC", "issuer2", 100.0, true),
            create_test_payment("USDC", "issuer1", "EURC", "issuer2", 50.0, true),
        ];

        let analytics = compute_corridor_analytics(&payments);
        assert_eq!(analytics.len(), 1);
        assert_eq!(analytics[0].success_rate, 100.0);
        assert_eq!(analytics[0].failed_transactions, 0);
    }

    #[test]
    fn test_compute_corridor_analytics_all_failed() {
        let payments = vec![
            create_test_payment("USDC", "issuer1", "EURC", "issuer2", 100.0, false),
            create_test_payment("USDC", "issuer1", "EURC", "issuer2", 50.0, false),
        ];

        let analytics = compute_corridor_analytics(&payments);
        assert_eq!(analytics.len(), 1);
        assert_eq!(analytics[0].success_rate, 0.0);
        assert_eq!(analytics[0].successful_transactions, 0);
    }

    #[test]
    fn test_get_top_corridors_by_volume() {
        let payments = vec![
            create_test_payment("USDC", "issuer1", "EURC", "issuer2", 100.0, true),
            create_test_payment("USDC", "issuer1", "BTC", "issuer3", 200.0, true),
            create_test_payment("EURC", "issuer2", "BTC", "issuer3", 150.0, true),
        ];

        let analytics = compute_corridor_analytics(&payments);
        let top_corridors = get_top_corridors_by_volume(&analytics, 2);

        assert_eq!(top_corridors.len(), 2);
        assert!(top_corridors[0].volume_usd >= top_corridors[1].volume_usd);
    }

    #[test]
    fn test_get_corridors_by_success_rate_filter() {
        let payments = vec![
            create_test_payment("USDC", "issuer1", "EURC", "issuer2", 100.0, true),
            create_test_payment("USDC", "issuer1", "EURC", "issuer2", 50.0, true),
            create_test_payment("USDC", "issuer1", "BTC", "issuer3", 10.0, false),
        ];

        let analytics = compute_corridor_analytics(&payments);
        let filtered_corridors = get_corridors_by_success_rate(&analytics, 2);

        assert_eq!(filtered_corridors.len(), 1);
        assert_eq!(filtered_corridors[0].success_rate, 100.0);
    }
}
