use backend::analytics::corridor::{
    compute_corridor_analytics, compute_corridor_analytics_for_date, get_corridors_by_success_rate,
    get_top_corridors_by_transactions, get_top_corridors_by_volume,
};
use backend::models::corridor::PaymentRecord;
use chrono::{DateTime, Utc};
use uuid::Uuid;

fn create_test_payment(
    source_code: &str,
    source_issuer: &str,
    dest_code: &str,
    dest_issuer: &str,
    amount: f64,
    successful: bool,
    timestamp: DateTime<Utc>,
) -> PaymentRecord {
    PaymentRecord {
        id: Uuid::new_v4(),
        source_asset_code: source_code.to_string(),
        source_asset_issuer: source_issuer.to_string(),
        destination_asset_code: dest_code.to_string(),
        destination_asset_issuer: dest_issuer.to_string(),
        amount,
        successful,
        timestamp,
        submission_time: None,
        confirmation_time: None,
    }
}

#[test]
fn test_known_dataset_expected_success_rates() {
    let base_time = Utc::now();

    let payments = vec![
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 100.0, true, base_time),
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 50.0, true, base_time),
        create_test_payment("EURC", "anchor2", "USDC", "anchor1", 75.0, false, base_time),
        create_test_payment("USDC", "anchor1", "BTC", "anchor3", 200.0, true, base_time),
        create_test_payment("BTC", "anchor3", "USDC", "anchor1", 150.0, true, base_time),
        create_test_payment("BTC", "anchor3", "USDC", "anchor1", 100.0, false, base_time),
    ];

    let analytics = compute_corridor_analytics(&payments);

    assert_eq!(analytics.len(), 2);

    let usdc_eurc_analytics = analytics
        .iter()
        .find(|a| a.corridor.asset_a_code == "EURC" && a.corridor.asset_b_code == "USDC")
        .unwrap();

    assert_eq!(usdc_eurc_analytics.total_transactions, 3);
    assert_eq!(usdc_eurc_analytics.successful_transactions, 2);
    assert_eq!(usdc_eurc_analytics.failed_transactions, 1);
    assert!((usdc_eurc_analytics.success_rate - 66.66666666666667).abs() < 0.0001);
    assert_eq!(usdc_eurc_analytics.volume_usd, 225.0);

    let usdc_btc_analytics = analytics
        .iter()
        .find(|a| a.corridor.asset_a_code == "BTC" && a.corridor.asset_b_code == "USDC")
        .unwrap();

    assert_eq!(usdc_btc_analytics.total_transactions, 3);
    assert_eq!(usdc_btc_analytics.successful_transactions, 2);
    assert_eq!(usdc_btc_analytics.failed_transactions, 1);
    assert!((usdc_btc_analytics.success_rate - 66.66666666666667).abs() < 0.0001);
    assert_eq!(usdc_btc_analytics.volume_usd, 450.0);
}

#[test]
fn test_empty_corridor_edge_case() {
    let payments = vec![];
    let analytics = compute_corridor_analytics(&payments);

    assert_eq!(analytics.len(), 0);
}

#[test]
fn test_single_payment_corridor() {
    let base_time = Utc::now();
    let payments = vec![create_test_payment(
        "USDC", "anchor1", "EURC", "anchor2", 100.0, true, base_time,
    )];

    let analytics = compute_corridor_analytics(&payments);

    assert_eq!(analytics.len(), 1);
    assert_eq!(analytics[0].total_transactions, 1);
    assert_eq!(analytics[0].successful_transactions, 1);
    assert_eq!(analytics[0].failed_transactions, 0);
    assert_eq!(analytics[0].success_rate, 100.0);
    assert_eq!(analytics[0].volume_usd, 100.0);
}

#[test]
fn test_all_successful_payments() {
    let base_time = Utc::now();
    let payments = vec![
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 100.0, true, base_time),
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 50.0, true, base_time),
        create_test_payment("EURC", "anchor2", "USDC", "anchor1", 75.0, true, base_time),
    ];

    let analytics = compute_corridor_analytics(&payments);

    assert_eq!(analytics.len(), 1);
    assert_eq!(analytics[0].success_rate, 100.0);
    assert_eq!(analytics[0].failed_transactions, 0);
    assert_eq!(analytics[0].successful_transactions, 3);
}

#[test]
fn test_all_failed_payments() {
    let base_time = Utc::now();
    let payments = vec![
        create_test_payment(
            "USDC", "anchor1", "EURC", "anchor2", 100.0, false, base_time,
        ),
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 50.0, false, base_time),
        create_test_payment("EURC", "anchor2", "USDC", "anchor1", 75.0, false, base_time),
    ];

    let analytics = compute_corridor_analytics(&payments);

    assert_eq!(analytics.len(), 1);
    assert_eq!(analytics[0].success_rate, 0.0);
    assert_eq!(analytics[0].successful_transactions, 0);
    assert_eq!(analytics[0].failed_transactions, 3);
}

#[test]
fn test_mixed_success_rates_across_corridors() {
    let base_time = Utc::now();
    let payments = vec![
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 100.0, true, base_time),
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 50.0, true, base_time),
        create_test_payment("EURC", "anchor2", "USDC", "anchor1", 25.0, false, base_time),
        create_test_payment("USDC", "anchor1", "BTC", "anchor3", 200.0, false, base_time),
        create_test_payment("BTC", "anchor3", "USDC", "anchor1", 150.0, false, base_time),
        create_test_payment("BTC", "anchor3", "USDC", "anchor1", 100.0, true, base_time),
    ];

    let analytics = compute_corridor_analytics(&payments);

    assert_eq!(analytics.len(), 2);

    let usdc_eurc = analytics
        .iter()
        .find(|a| a.corridor.asset_a_code == "EURC" && a.corridor.asset_b_code == "USDC")
        .unwrap();
    assert!((usdc_eurc.success_rate - 66.66666666666667).abs() < 0.0001);

    let usdc_btc = analytics
        .iter()
        .find(|a| a.corridor.asset_a_code == "BTC" && a.corridor.asset_b_code == "USDC")
        .unwrap();
    assert!((usdc_btc.success_rate - 33.33333333333333).abs() < 0.0001);
}

#[test]
fn test_date_filtering() {
    let base_time = Utc::now();
    let yesterday = base_time.date_naive() - chrono::Duration::days(1);
    let yesterday_time = yesterday.and_hms_opt(12, 0, 0).unwrap().and_utc();

    let payments = vec![
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 100.0, true, base_time),
        create_test_payment(
            "USDC",
            "anchor1",
            "EURC",
            "anchor2",
            50.0,
            true,
            yesterday_time,
        ),
        create_test_payment(
            "EURC",
            "anchor2",
            "USDC",
            "anchor1",
            75.0,
            false,
            yesterday_time,
        ),
    ];

    let today_analytics = compute_corridor_analytics_for_date(&payments, base_time);
    assert_eq!(today_analytics.len(), 1);
    assert_eq!(today_analytics[0].total_transactions, 1);
    assert_eq!(today_analytics[0].success_rate, 100.0);

    let yesterday_analytics = compute_corridor_analytics_for_date(&payments, yesterday_time);
    assert_eq!(yesterday_analytics.len(), 1);
    assert_eq!(yesterday_analytics[0].total_transactions, 2);
    assert_eq!(yesterday_analytics[0].success_rate, 50.0);
}

#[test]
fn test_top_corridors_by_volume() {
    let base_time = Utc::now();
    let payments = vec![
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 100.0, true, base_time),
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 50.0, true, base_time),
        create_test_payment("USDC", "anchor1", "BTC", "anchor3", 500.0, true, base_time),
        create_test_payment("BTC", "anchor3", "USDC", "anchor1", 200.0, true, base_time),
    ];

    let analytics = compute_corridor_analytics(&payments);
    let top_corridors = get_top_corridors_by_volume(&analytics, 2);

    assert_eq!(top_corridors.len(), 2);
    assert!(top_corridors[0].volume_usd >= top_corridors[1].volume_usd);
    assert_eq!(top_corridors[0].volume_usd, 700.0);
    assert_eq!(top_corridors[1].volume_usd, 150.0);
}

#[test]
fn test_top_corridors_by_transactions() {
    let base_time = Utc::now();
    let payments = vec![
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 100.0, true, base_time),
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 50.0, true, base_time),
        create_test_payment("EURC", "anchor2", "USDC", "anchor1", 75.0, false, base_time),
        create_test_payment("USDC", "anchor1", "BTC", "anchor3", 500.0, true, base_time),
    ];

    let analytics = compute_corridor_analytics(&payments);
    let top_corridors = get_top_corridors_by_transactions(&analytics, 2);

    assert_eq!(top_corridors.len(), 2);
    assert_eq!(top_corridors[0].total_transactions, 3);
    assert_eq!(top_corridors[1].total_transactions, 1);
}

#[test]
fn test_corridors_by_success_rate_filter() {
    let base_time = Utc::now();
    let payments = vec![
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 100.0, true, base_time),
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 50.0, true, base_time),
        create_test_payment("EURC", "anchor2", "USDC", "anchor1", 75.0, false, base_time),
        create_test_payment("USDC", "anchor1", "BTC", "anchor3", 500.0, true, base_time),
        create_test_payment("BTC", "anchor3", "USDC", "anchor1", 200.0, true, base_time),
        create_test_payment("BTC", "anchor3", "USDC", "anchor1", 100.0, false, base_time),
    ];

    let analytics = compute_corridor_analytics(&payments);
    let high_success_corridors = get_corridors_by_success_rate(&analytics, 2);

    assert_eq!(high_success_corridors.len(), 2);
    // Both corridors should be returned since they both have >= 2 transactions
}

#[test]
fn test_deterministic_results() {
    let base_time = Utc::now();
    let payments = vec![
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 100.0, true, base_time),
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 50.0, true, base_time),
        create_test_payment("EURC", "anchor2", "USDC", "anchor1", 75.0, false, base_time),
    ];

    let analytics1 = compute_corridor_analytics(&payments);
    let analytics2 = compute_corridor_analytics(&payments);

    assert_eq!(analytics1.len(), analytics2.len());

    for (a1, a2) in analytics1.iter().zip(analytics2.iter()) {
        assert_eq!(a1.corridor, a2.corridor);
        assert_eq!(a1.total_transactions, a2.total_transactions);
        assert_eq!(a1.successful_transactions, a2.successful_transactions);
        assert_eq!(a1.failed_transactions, a2.failed_transactions);
        assert!((a1.success_rate - a2.success_rate).abs() < 0.0001);
        assert!((a1.volume_usd - a2.volume_usd).abs() < 0.0001);
    }
}

#[test]
fn test_corridor_normalization_consistency() {
    let base_time = Utc::now();

    let payments_forward = vec![create_test_payment(
        "USDC", "anchor1", "EURC", "anchor2", 100.0, true, base_time,
    )];

    let payments_reverse = vec![create_test_payment(
        "EURC", "anchor2", "USDC", "anchor1", 100.0, true, base_time,
    )];

    let analytics_forward = compute_corridor_analytics(&payments_forward);
    let analytics_reverse = compute_corridor_analytics(&payments_reverse);

    assert_eq!(analytics_forward.len(), 1);
    assert_eq!(analytics_reverse.len(), 1);

    assert_eq!(analytics_forward[0].corridor, analytics_reverse[0].corridor);
    assert_eq!(
        analytics_forward[0].total_transactions,
        analytics_reverse[0].total_transactions
    );
    assert_eq!(
        analytics_forward[0].success_rate,
        analytics_reverse[0].success_rate
    );
}

#[test]
fn test_zero_volume_edge_case() {
    let base_time = Utc::now();
    let payments = vec![
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 0.0, true, base_time),
        create_test_payment("USDC", "anchor1", "EURC", "anchor2", 0.0, false, base_time),
    ];

    let analytics = compute_corridor_analytics(&payments);

    assert_eq!(analytics.len(), 1);
    assert_eq!(analytics[0].total_transactions, 2);
    assert_eq!(analytics[0].volume_usd, 0.0);
    assert_eq!(analytics[0].success_rate, 50.0);
}
