/// Unit tests for payment processing business logic:
///   - `Corridor::new()` canonical ordering
///   - `Corridor::to_string_key()`
///   - `PaymentRecord::settlement_latency_ms()`
///   - `PaymentRecord::get_corridor()`
///   - `compute_median()`
///   - `compute_metrics_from_payments()`
///   - `compute_metrics_by_window()`
use chrono::{Duration, Utc};
use stellar_insights_backend::models::corridor::{compute_median, Corridor, PaymentRecord};
use stellar_insights_backend::services::analytics::{
    compute_metrics_by_window, compute_metrics_from_payments,
};
use uuid::Uuid;

// ── helpers ──────────────────────────────────────────────────────────────────

fn payment(
    src_code: &str,
    src_issuer: &str,
    dst_code: &str,
    dst_issuer: &str,
    amount: f64,
    successful: bool,
    offset_secs: i64,
) -> PaymentRecord {
    let ts = Utc::now() + Duration::seconds(offset_secs);
    PaymentRecord {
        id: Uuid::new_v4(),
        source_asset_code: src_code.into(),
        source_asset_issuer: src_issuer.into(),
        destination_asset_code: dst_code.into(),
        destination_asset_issuer: dst_issuer.into(),
        amount,
        successful,
        timestamp: ts,
        submission_time: None,
        confirmation_time: None,
    }
}

fn payment_with_latency(
    src_code: &str,
    dst_code: &str,
    amount: f64,
    successful: bool,
    latency_ms: i64,
) -> PaymentRecord {
    let now = Utc::now();
    let submission = now - Duration::milliseconds(latency_ms);
    PaymentRecord {
        id: Uuid::new_v4(),
        source_asset_code: src_code.into(),
        source_asset_issuer: "issuerA".into(),
        destination_asset_code: dst_code.into(),
        destination_asset_issuer: "issuerB".into(),
        amount,
        successful,
        timestamp: now,
        submission_time: Some(submission),
        confirmation_time: Some(now),
    }
}

// ── Corridor canonical ordering ───────────────────────────────────────────────

#[test]
fn corridor_orders_assets_lexicographically() {
    // "USDC:issuerA" < "XLM:issuerB", so USDC should be asset_a
    let c = Corridor::new(
        "XLM".into(),
        "issuerB".into(),
        "USDC".into(),
        "issuerA".into(),
    );
    assert_eq!(c.source_asset_code, "USDC");
    assert_eq!(c.destination_asset_code, "XLM");
}

#[test]
fn corridor_preserves_order_when_already_canonical() {
    let c = Corridor::new(
        "EUR".into(),
        "issuer1".into(),
        "NGN".into(),
        "issuer2".into(),
    );
    assert_eq!(c.source_asset_code, "EUR");
    assert_eq!(c.destination_asset_code, "NGN");
}

#[test]
fn corridor_same_code_different_issuer_ordered_by_issuer() {
    let c = Corridor::new(
        "USDC".into(),
        "issuer2".into(),
        "USDC".into(),
        "issuer1".into(),
    );
    assert_eq!(c.source_asset_issuer, "issuer1");
    assert_eq!(c.destination_asset_issuer, "issuer2");
}

#[test]
fn corridor_to_string_key_contains_separator_and_codes() {
    let c = Corridor::new(
        "USDC".into(),
        "issuerA".into(),
        "NGN".into(),
        "issuerB".into(),
    );
    let key = c.to_string_key();
    assert!(key.contains("->"));
    assert!(key.contains("USDC:issuerA"));
    assert!(key.contains("NGN:issuerB"));
}

#[test]
fn corridor_symmetric_construction_yields_same_key() {
    let c1 = Corridor::new("A".into(), "i1".into(), "B".into(), "i2".into());
    let c2 = Corridor::new("B".into(), "i2".into(), "A".into(), "i1".into());
    assert_eq!(c1.to_string_key(), c2.to_string_key());
}

// ── PaymentRecord helpers ─────────────────────────────────────────────────────

#[test]
fn settlement_latency_returns_none_without_times() {
    let p = payment("USDC", "issA", "NGN", "issB", 100.0, true, 0);
    assert_eq!(p.settlement_latency_ms(), None);
}

#[test]
fn settlement_latency_computes_correct_ms() {
    let p = payment_with_latency("USDC", "NGN", 500.0, true, 1_200);
    assert_eq!(p.settlement_latency_ms(), Some(1_200));
}

#[test]
fn settlement_latency_returns_zero_for_instant_confirm() {
    let now = Utc::now();
    let p = PaymentRecord {
        id: Uuid::new_v4(),
        source_asset_code: "USDC".into(),
        source_asset_issuer: "issA".into(),
        destination_asset_code: "NGN".into(),
        destination_asset_issuer: "issB".into(),
        amount: 50.0,
        successful: true,
        timestamp: now,
        submission_time: Some(now),
        confirmation_time: Some(now),
    };
    assert_eq!(p.settlement_latency_ms(), Some(0));
}

#[test]
fn get_corridor_extracts_normalized_corridor() {
    let p = payment("NGN", "issB", "USDC", "issA", 100.0, true, 0);
    let c = p.get_corridor();
    // "NGN:issB" < "USDC:issA" lexicographically (N < U), so NGN is asset_a
    assert_eq!(c.source_asset_code, "NGN");
    assert_eq!(c.destination_asset_code, "USDC");
}

// ── compute_median ────────────────────────────────────────────────────────────

#[test]
fn compute_median_empty_slice() {
    assert_eq!(compute_median(&mut []), None);
}

#[test]
fn compute_median_single_element() {
    assert_eq!(compute_median(&mut [42]), Some(42));
}

#[test]
fn compute_median_odd_count() {
    assert_eq!(compute_median(&mut [10, 3, 7]), Some(7));
}

#[test]
fn compute_median_even_count() {
    // sorted: [2, 4, 6, 8] → average of two middle values = 5
    assert_eq!(compute_median(&mut [8, 2, 6, 4]), Some(5));
}

#[test]
fn compute_median_all_equal() {
    assert_eq!(compute_median(&mut [5, 5, 5, 5]), Some(5));
}

// ── compute_metrics_from_payments ─────────────────────────────────────────────

#[test]
fn metrics_from_empty_payments() {
    let metrics = compute_metrics_from_payments(&[]);
    assert!(metrics.is_empty());
}

#[test]
fn metrics_single_successful_payment() {
    let payments = [payment_with_latency("USDC", "NGN", 1000.0, true, 500)];
    let metrics = compute_metrics_from_payments(&payments);
    assert_eq!(metrics.len(), 1);
    let m = &metrics[0];
    assert_eq!(m.total_transactions, 1);
    assert_eq!(m.successful_transactions, 1);
    assert_eq!(m.failed_transactions, 0);
    assert!((m.success_rate - 100.0).abs() < 1e-9);
    assert!((m.volume_usd - 1000.0).abs() < 1e-9);
    assert_eq!(m.avg_settlement_latency_ms, Some(500));
}

#[test]
fn metrics_single_failed_payment() {
    let payments = [payment("USDC", "issA", "NGN", "issB", 100.0, false, 0)];
    let metrics = compute_metrics_from_payments(&payments);
    assert_eq!(metrics.len(), 1);
    let m = &metrics[0];
    assert_eq!(m.total_transactions, 1);
    assert_eq!(m.successful_transactions, 0);
    assert_eq!(m.failed_transactions, 1);
    assert!((m.success_rate - 0.0).abs() < 1e-9);
    assert!((m.volume_usd - 0.0).abs() < 1e-9);
    assert_eq!(m.avg_settlement_latency_ms, None);
}

#[test]
fn metrics_mixed_success_and_failure_correct_rate() {
    let payments = [
        payment("USDC", "issA", "NGN", "issB", 200.0, true, 0),
        payment("USDC", "issA", "NGN", "issB", 100.0, false, 1),
        payment("USDC", "issA", "NGN", "issB", 300.0, true, 2),
        payment("USDC", "issA", "NGN", "issB", 400.0, false, 3),
    ];
    let metrics = compute_metrics_from_payments(&payments);
    let m = &metrics[0];
    assert_eq!(m.total_transactions, 4);
    assert_eq!(m.successful_transactions, 2);
    assert_eq!(m.failed_transactions, 2);
    assert!((m.success_rate - 50.0).abs() < 1e-9);
    // volume only from successful payments
    assert!((m.volume_usd - 500.0).abs() < 1e-9);
}

#[test]
fn metrics_groups_into_separate_corridors() {
    let payments = [
        payment("USDC", "issA", "NGN", "issB", 100.0, true, 0),
        payment("USDC", "issA", "KES", "issC", 200.0, true, 1),
        payment("USDC", "issA", "NGN", "issB", 150.0, true, 2),
    ];
    let metrics = compute_metrics_from_payments(&payments);
    assert_eq!(metrics.len(), 2);
}

#[test]
fn metrics_volume_only_from_successful_payments() {
    let payments = [
        payment("USDC", "issA", "NGN", "issB", 100.0, true, 0),
        payment("USDC", "issA", "NGN", "issB", 9999.0, false, 1),
    ];
    let metrics = compute_metrics_from_payments(&payments);
    assert!((metrics[0].volume_usd - 100.0).abs() < 1e-9);
}

#[test]
fn metrics_median_latency_of_three_payments() {
    let payments = [
        payment_with_latency("USDC", "NGN", 100.0, true, 1_000),
        payment_with_latency("USDC", "NGN", 100.0, true, 3_000),
        payment_with_latency("USDC", "NGN", 100.0, true, 2_000),
    ];
    let metrics = compute_metrics_from_payments(&payments);
    assert_eq!(metrics.len(), 1);
    // sorted latencies: [1000, 2000, 3000] → median = 2000 ms
    assert_eq!(metrics[0].median_settlement_latency_ms, Some(2_000));
}

// ── compute_metrics_by_window ─────────────────────────────────────────────────

#[test]
fn metrics_by_window_filters_outside_range() {
    let base = Utc::now();
    let window_start = base;
    let window_end = base + Duration::hours(1);

    let payments = [
        payment("USDC", "issA", "NGN", "issB", 100.0, true, 0), // inside
        payment("USDC", "issA", "NGN", "issB", 200.0, true, -3_600), // before
        payment("USDC", "issA", "NGN", "issB", 300.0, true, 7_200), // after
    ];

    let metrics = compute_metrics_by_window(&payments, window_start, window_end);
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].total_transactions, 1);
    assert!((metrics[0].volume_usd - 100.0).abs() < 1e-9);
}

#[test]
fn metrics_by_window_includes_boundary_payments() {
    let base = Utc::now();
    let window_start = base;
    let window_end = base + Duration::hours(1);

    let payments = [
        PaymentRecord {
            id: Uuid::new_v4(),
            source_asset_code: "USDC".into(),
            source_asset_issuer: "issA".into(),
            destination_asset_code: "NGN".into(),
            destination_asset_issuer: "issB".into(),
            amount: 50.0,
            successful: true,
            timestamp: window_start,
            submission_time: None,
            confirmation_time: None,
        },
        PaymentRecord {
            id: Uuid::new_v4(),
            source_asset_code: "USDC".into(),
            source_asset_issuer: "issA".into(),
            destination_asset_code: "NGN".into(),
            destination_asset_issuer: "issB".into(),
            amount: 75.0,
            successful: true,
            timestamp: window_end,
            submission_time: None,
            confirmation_time: None,
        },
    ];

    let metrics = compute_metrics_by_window(&payments, window_start, window_end);
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].total_transactions, 2);
}

#[test]
fn metrics_by_empty_window_returns_empty() {
    let base = Utc::now();
    let payments = [payment("USDC", "issA", "NGN", "issB", 100.0, true, 0)];
    let metrics = compute_metrics_by_window(
        &payments,
        base + Duration::hours(1),
        base + Duration::hours(2),
    );
    assert!(metrics.is_empty());
}
