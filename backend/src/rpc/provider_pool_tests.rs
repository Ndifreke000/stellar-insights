//! Tests for the provider pool (#2776).

use super::*;
use crate::rpc::MockStellarRpcClient;
use std::sync::atomic::AtomicUsize;

fn provider(name: &str, weight: u32) -> ProviderConfig {
    ProviderConfig {
        name: name.into(),
        rpc_url: format!("https://{name}.example/rpc"),
        horizon_url: format!("https://{name}.example/horizon"),
        weight,
        max_rps: None,
    }
}

fn pool3() -> ProviderPool {
    ProviderPool::new(
        "testnet",
        vec![provider("a", 1), provider("b", 1), provider("c", 1)],
        ProviderPoolConfig {
            ewma_alpha: 0.5,
            error_threshold: 0.5,
            max_ledger_lag: 5,
            ejection_cooldown: Duration::from_secs(10),
        },
    )
}

#[test]
fn parses_provider_list() {
    let p = parse_providers(
        "sdf|https://rpc.sdf|https://h.sdf|3|50; alt|https://rpc.alt|https://h.alt",
    )
    .unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].weight, 3);
    assert_eq!(p[0].max_rps, Some(50));
    assert_eq!(p[1].weight, 1);
    assert_eq!(p[1].max_rps, None);
}

#[test]
fn rejects_bad_provider_specs() {
    assert!(parse_providers("only|https://x").is_err());
    assert!(parse_providers("a|ftp://x|https://y").is_err());
    assert!(parse_providers("a|https://x|https://y|0").is_err());
    assert!(parse_providers("a|https://x|https://y;a|https://x|https://y").is_err());
    assert!(parse_providers("").unwrap().is_empty());
}

#[test]
fn failing_and_lagging_providers_are_ejected_traffic_goes_to_healthy() {
    let pool = pool3();
    let now = Instant::now();
    // "a" fails repeatedly.
    pool.record_failure_at(0, now);
    pool.record_failure_at(0, now);
    // "b" lags, "c" is fresh.
    pool.record_latest_ledger_at(1, 100, now);
    pool.record_latest_ledger_at(2, 120, now);

    let s = pool.statuses();
    assert_eq!(s[0].state, ProviderState::Ejected);
    assert_eq!(s[1].state, ProviderState::Ejected);
    assert!(s[1].ejection_reason.as_deref().unwrap().contains("lagging"));
    assert_eq!(s[2].state, ProviderState::Healthy);

    for i in 0..20 {
        let r = f64::from(i) / 20.0;
        assert_eq!(pool.select_at(now, &[], r), Some(2));
    }
}

#[test]
fn failed_provider_readmitted_after_half_open_probe() {
    let pool = pool3();
    let now = Instant::now();
    pool.record_failure_at(0, now);
    pool.record_failure_at(0, now);
    assert_eq!(pool.statuses()[0].state, ProviderState::Ejected);

    // After the cooldown, a probe is admitted exactly once.
    let later = now + Duration::from_secs(11);
    let picked = pool.select_at(later, &[1, 2], 0.0);
    assert_eq!(picked, Some(0));
    assert_eq!(pool.statuses()[0].state, ProviderState::HalfOpen);
    pool.record_success(0, Duration::from_millis(20));
    assert_eq!(pool.statuses()[0].state, ProviderState::Healthy);
}

#[test]
fn failed_half_open_probe_ejects_again_with_longer_cooldown() {
    let pool = pool3();
    let now = Instant::now();
    pool.record_failure_at(0, now);
    pool.record_failure_at(0, now);
    let later = now + Duration::from_secs(11);
    assert_eq!(pool.select_at(later, &[1, 2], 0.0), Some(0));
    pool.record_failure_at(0, later);
    assert_eq!(pool.statuses()[0].state, ProviderState::Ejected);
    // Doubled cooldown (20s): still ejected after 15s.
    assert_ne!(
        pool.select_at(later + Duration::from_secs(15), &[], 0.0),
        Some(0)
    );
}

#[test]
fn weights_bias_selection() {
    let pool = ProviderPool::new(
        "testnet",
        vec![provider("heavy", 9), provider("light", 1)],
        ProviderPoolConfig::default(),
    );
    let now = Instant::now();
    assert_eq!(pool.select_at(now, &[], 0.5), Some(0));
    assert_eq!(pool.select_at(now, &[], 0.95), Some(1));
}

#[test]
fn per_provider_rate_limit_spills_over() {
    let mut a = provider("a", 100);
    a.max_rps = Some(1);
    let pool = ProviderPool::new(
        "testnet",
        vec![a, provider("b", 1)],
        ProviderPoolConfig::default(),
    );
    let now = Instant::now();
    assert_eq!(pool.select_at(now, &[], 0.0), Some(0));
    assert_eq!(pool.select_at(now, &[], 0.0), Some(1));
}

#[test]
fn all_ejected_degrades_to_soonest_recovery() {
    let pool = pool3();
    let now = Instant::now();
    for i in 0..3 {
        pool.record_failure_at(i, now);
        pool.record_failure_at(i, now);
    }
    assert!(pool.select_at(now, &[], 0.3).is_some());
}

/// Provider that always fails with a retryable error.
struct Failing(AtomicUsize);

#[async_trait::async_trait]
impl StellarRpcClientTrait for Failing {
    fn network_config(&self) -> &NetworkConfig {
        unreachable!()
    }
    async fn check_health(&self) -> Result<HealthResponse, RpcError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_latest_ledger(&self) -> Result<LedgerInfo, RpcError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_ledger_by_sequence(&self, _: u64) -> Result<LedgerInfo, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_ledgers(
        &self,
        _: Option<u64>,
        _: u32,
        _: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_transactions_for_ledger(
        &self,
        _: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_payments_for_ledger(&self, _: u64) -> Result<Vec<Payment>, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_account_payments(&self, _: &str, _: u32) -> Result<Vec<Payment>, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_all_account_payments(
        &self,
        _: &str,
        _: Option<u32>,
    ) -> Result<Vec<Payment>, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_operations_for_ledger(&self, _: u64) -> Result<Vec<HorizonOperation>, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_trades(&self, _: u32, _: Option<&str>) -> Result<Vec<Trade>, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_order_book(
        &self,
        _: &crate::rpc::stellar::Asset,
        _: &crate::rpc::stellar::Asset,
        _: u32,
    ) -> Result<OrderBook, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_liquidity_pools(
        &self,
        _: u32,
        _: Option<&str>,
    ) -> Result<Vec<HorizonLiquidityPool>, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_pool_trades(&self, _: &str, _: u32) -> Result<Vec<Trade>, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
    async fn fetch_assets(&self, _: u32, _: bool) -> Result<Vec<HorizonAsset>, RpcError> {
        Err(RpcError::NetworkError("down".into()))
    }
}

#[tokio::test]
async fn failover_client_routes_around_failing_provider() {
    let failing = Arc::new(Failing(AtomicUsize::new(0)));
    let pool = Arc::new(ProviderPool::new(
        "testnet",
        vec![provider("bad", 1_000), provider("good", 1)],
        ProviderPoolConfig::default(),
    ));
    let client = FailoverRpcClient::new(
        pool.clone(),
        vec![
            failing.clone() as Arc<dyn StellarRpcClientTrait>,
            Arc::new(MockStellarRpcClient::testnet()),
        ],
        NetworkConfig::for_network(StellarNetwork::Testnet),
    );
    for _ in 0..5 {
        let ledger = client.fetch_latest_ledger().await.expect("failover");
        assert!(ledger.sequence > 0);
    }
    // "bad" is tried at most until it is ejected, then skipped.
    assert!(failing.0.load(Ordering::SeqCst) <= 3);
}
