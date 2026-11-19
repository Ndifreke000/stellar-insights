//! Multi-provider RPC failover with health-weighted selection (#2776).
//!
//! # Design
//!
//! * **Data model** - each network has a list of [`ProviderConfig`]s (name,
//!   RPC URL, Horizon URL, weight, optional per-provider request budget).
//!   [`ProviderPool`] keeps a [`ProviderHealth`] per provider: latency EWMA,
//!   error-rate EWMA, last observed `latestLedger` and a state
//!   (`Healthy` / `Ejected` / `HalfOpen`).
//! * **Selection** - weighted random among providers that are healthy (or
//!   half-open and not already probing) and under their rate budget. The
//!   effective weight is `weight * (1 - error_ewma) / (1 + latency_ewma / 100ms)`
//!   so slow or flaky providers get proportionally less traffic.
//! * **Ejection** - a provider is ejected when its error EWMA exceeds
//!   `error_threshold` or when its `latestLedger` lags the freshest provider by
//!   more than `max_ledger_lag`. After `ejection_cooldown` it becomes
//!   half-open: one probe request is admitted; success re-admits it, failure
//!   ejects it again with a doubled cooldown (capped at 16x).
//! * **Failure modes** - if every provider is ejected, [`ProviderPool::select`]
//!   falls back to the provider whose cooldown expires soonest so that the
//!   service degrades instead of hard-failing.
//!
//! [`FailoverRpcClient`] wires the pool to one [`StellarRpcClient`] per
//! provider (each with its own circuit breaker) and implements
//! [`StellarRpcClientTrait`], so it is usable everywhere the trait is.
//!
//! # Configuration
//!
//! `STELLAR_RPC_PROVIDERS_<NETWORK>` (e.g. `STELLAR_RPC_PROVIDERS_TESTNET`):
//! semicolon-separated entries `name|rpc_url|horizon_url|weight|max_rps`
//! (`weight` defaults to 1, `max_rps` is optional).
//! `RPC_PROVIDER_MAX_LEDGER_LAG` (default 10), `RPC_PROVIDER_ERROR_THRESHOLD`
//! (default 0.5), `RPC_PROVIDER_EJECTION_COOLDOWN_SECS` (default 30).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use lazy_static::lazy_static;
use prometheus::{
    register_gauge_vec, register_int_counter_vec, register_int_gauge_vec, GaugeVec, IntCounterVec,
    IntGaugeVec,
};
use serde::Serialize;

use crate::network::{NetworkConfig, StellarNetwork};
use crate::rpc::circuit_breaker::new_rpc_circuit_breaker;
use crate::rpc::client_trait::StellarRpcClientTrait;
use crate::rpc::error::RpcError;
use crate::rpc::stellar::{
    GetLedgersResult, HealthResponse, HorizonAsset, HorizonLiquidityPool, HorizonOperation,
    HorizonTransaction, LedgerInfo, OrderBook, Payment, StellarRpcClient, Trade,
};

lazy_static! {
    static ref PROVIDER_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "rpc_provider_requests_total",
        "RPC requests per provider and outcome",
        &["network", "provider", "outcome"]
    )
    .expect("rpc_provider_requests_total metric");
    static ref PROVIDER_LATENCY: GaugeVec = register_gauge_vec!(
        "rpc_provider_latency_ewma_ms",
        "Latency EWMA per RPC provider (ms)",
        &["network", "provider"]
    )
    .expect("rpc_provider_latency_ewma_ms metric");
    static ref PROVIDER_STATE: IntGaugeVec = register_int_gauge_vec!(
        "rpc_provider_state",
        "Provider state (0=healthy, 1=ejected, 2=half-open)",
        &["network", "provider"]
    )
    .expect("rpc_provider_state metric");
}

/// One configured upstream provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderConfig {
    pub name: String,
    pub rpc_url: String,
    pub horizon_url: String,
    pub weight: u32,
    /// Maximum requests per second sent to this provider (None = unlimited).
    pub max_rps: Option<u32>,
}

/// Tunables for health tracking.
#[derive(Debug, Clone)]
pub struct ProviderPoolConfig {
    /// Smoothing factor for EWMAs (0 < alpha <= 1).
    pub ewma_alpha: f64,
    /// Error EWMA above which a provider is ejected.
    pub error_threshold: f64,
    /// Ledgers a provider may trail the freshest provider before ejection.
    pub max_ledger_lag: u64,
    /// How long an ejected provider waits before a half-open probe.
    pub ejection_cooldown: Duration,
}

impl Default for ProviderPoolConfig {
    fn default() -> Self {
        Self {
            ewma_alpha: 0.3,
            error_threshold: 0.5,
            max_ledger_lag: 10,
            ejection_cooldown: Duration::from_secs(30),
        }
    }
}

impl ProviderPoolConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let d = Self::default();
        let parse = |k: &str| std::env::var(k).ok();
        Self {
            ewma_alpha: d.ewma_alpha,
            error_threshold: parse("RPC_PROVIDER_ERROR_THRESHOLD")
                .and_then(|v| v.parse::<f64>().ok())
                .map_or(d.error_threshold, |v| v.clamp(0.05, 1.0)),
            max_ledger_lag: parse("RPC_PROVIDER_MAX_LEDGER_LAG")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(d.max_ledger_lag),
            ejection_cooldown: parse("RPC_PROVIDER_EJECTION_COOLDOWN_SECS")
                .and_then(|v| v.parse::<u64>().ok())
                .map_or(d.ejection_cooldown, |s| {
                    Duration::from_secs(s.clamp(1, 3600))
                }),
        }
    }
}

/// Parse `name|rpc_url|horizon_url|weight|max_rps;...`.
///
/// Returns an error describing the first malformed entry so misconfiguration
/// fails loudly at startup.
pub fn parse_providers(spec: &str) -> Result<Vec<ProviderConfig>, String> {
    let mut out = Vec::new();
    for (idx, raw) in spec.split(';').map(str::trim).enumerate() {
        if raw.is_empty() {
            continue;
        }
        let parts: Vec<&str> = raw.split('|').map(str::trim).collect();
        if parts.len() < 3 {
            return Err(format!(
                "provider #{idx}: expected name|rpc_url|horizon_url[|weight[|max_rps]]"
            ));
        }
        let (name, rpc_url, horizon_url) = (parts[0], parts[1], parts[2]);
        if name.is_empty() {
            return Err(format!("provider #{idx}: empty name"));
        }
        for url in [rpc_url, horizon_url] {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(format!("provider {name}: invalid URL {url:?}"));
            }
        }
        let weight = match parts.get(3).filter(|s| !s.is_empty()) {
            Some(w) => w
                .parse::<u32>()
                .map_err(|_| format!("provider {name}: invalid weight {w:?}"))?,
            None => 1,
        };
        if weight == 0 {
            return Err(format!("provider {name}: weight must be > 0"));
        }
        let max_rps = match parts.get(4).filter(|s| !s.is_empty()) {
            Some(r) => Some(
                r.parse::<u32>()
                    .map_err(|_| format!("provider {name}: invalid max_rps {r:?}"))?,
            ),
            None => None,
        };
        if out.iter().any(|p: &ProviderConfig| p.name == name) {
            return Err(format!("provider {name}: duplicate name"));
        }
        out.push(ProviderConfig {
            name: name.to_string(),
            rpc_url: rpc_url.to_string(),
            horizon_url: horizon_url.to_string(),
            weight,
            max_rps,
        });
    }
    Ok(out)
}

/// Read the provider list for `network` from the environment.
pub fn providers_from_env(network: StellarNetwork) -> Result<Vec<ProviderConfig>, String> {
    let key = format!(
        "STELLAR_RPC_PROVIDERS_{}",
        format!("{network:?}").to_uppercase()
    );
    std::env::var(&key).map_or_else(|_| Ok(Vec::new()), |spec| parse_providers(&spec))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderState {
    Healthy,
    Ejected,
    HalfOpen,
}

impl ProviderState {
    const fn gauge(self) -> i64 {
        match self {
            Self::Healthy => 0,
            Self::Ejected => 1,
            Self::HalfOpen => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub latency_ewma_ms: f64,
    pub error_ewma: f64,
    pub latest_ledger: Option<u64>,
    pub state: ProviderState,
    pub ejected_until: Option<Instant>,
    pub ejection_reason: Option<String>,
    /// Multiplier applied to the cooldown on repeated ejections.
    cooldown_factor: u32,
    /// True while a half-open probe is in flight.
    probing: bool,
    window_start: Instant,
    window_count: u32,
    pub requests: u64,
    pub failures: u64,
}

impl ProviderHealth {
    fn new(now: Instant) -> Self {
        Self {
            latency_ewma_ms: 0.0,
            error_ewma: 0.0,
            latest_ledger: None,
            state: ProviderState::Healthy,
            ejected_until: None,
            ejection_reason: None,
            cooldown_factor: 1,
            probing: false,
            window_start: now,
            window_count: 0,
            requests: 0,
            failures: 0,
        }
    }
}

/// Serializable per-provider view for `/health` and metrics.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub network: String,
    pub name: String,
    pub state: ProviderState,
    pub weight: u32,
    pub latency_ewma_ms: f64,
    pub error_rate: f64,
    pub latest_ledger: Option<u64>,
    pub ejection_reason: Option<String>,
    pub requests: u64,
    pub failures: u64,
}

/// Health-weighted provider selector for one network.
pub struct ProviderPool {
    network: String,
    providers: Vec<ProviderConfig>,
    config: ProviderPoolConfig,
    health: Mutex<Vec<ProviderHealth>>,
    rng: AtomicU64,
}

impl ProviderPool {
    /// # Panics
    /// Panics if `providers` is empty.
    #[must_use]
    pub fn new(
        network: impl Into<String>,
        providers: Vec<ProviderConfig>,
        config: ProviderPoolConfig,
    ) -> Self {
        assert!(
            !providers.is_empty(),
            "ProviderPool needs at least one provider"
        );
        let now = Instant::now();
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0x9E37_79B9_7F4A_7C15, |d| d.as_nanos() as u64)
            | 1;
        Self {
            network: network.into(),
            health: Mutex::new(providers.iter().map(|_| ProviderHealth::new(now)).collect()),
            providers,
            config,
            rng: AtomicU64::new(seed),
        }
    }

    #[must_use]
    pub fn providers(&self) -> &[ProviderConfig] {
        &self.providers
    }

    fn next_random(&self) -> f64 {
        // xorshift64*: fast, lock-free and good enough for load spreading.
        let mut x = self.rng.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng.store(x, Ordering::Relaxed);
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<ProviderHealth>> {
        self.health
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn effective_weight(&self, idx: usize, h: &ProviderHealth) -> f64 {
        let base = f64::from(self.providers[idx].weight);
        base * (1.0 - h.error_ewma).max(0.01) / (1.0 + h.latency_ewma_ms / 100.0)
    }

    fn publish(&self, idx: usize, h: &ProviderHealth) {
        let name = self.providers[idx].name.as_str();
        PROVIDER_LATENCY
            .with_label_values(&[self.network.as_str(), name])
            .set(h.latency_ewma_ms);
        PROVIDER_STATE
            .with_label_values(&[self.network.as_str(), name])
            .set(h.state.gauge());
    }

    fn eject(&self, idx: usize, h: &mut ProviderHealth, now: Instant, reason: String) {
        let cooldown = self.config.ejection_cooldown * h.cooldown_factor;
        h.state = ProviderState::Ejected;
        h.ejected_until = Some(now + cooldown);
        h.ejection_reason = Some(reason);
        h.probing = false;
        h.cooldown_factor = (h.cooldown_factor * 2).min(16);
        tracing::warn!(
            network = %self.network,
            provider = %self.providers[idx].name,
            reason = h.ejection_reason.as_deref().unwrap_or(""),
            "RPC provider ejected"
        );
        self.publish(idx, h);
    }

    /// Pick a provider index, excluding `exclude` (already tried in this call).
    pub fn select(&self, exclude: &[usize]) -> Option<usize> {
        self.select_at(Instant::now(), exclude, self.next_random())
    }

    /// Deterministic selection used by [`Self::select`] and tests.
    pub fn select_at(&self, now: Instant, exclude: &[usize], r: f64) -> Option<usize> {
        let mut health = self.lock();
        let mut candidates: Vec<(usize, f64)> = Vec::new();
        for (idx, h) in health.iter_mut().enumerate() {
            if exclude.contains(&idx) {
                continue;
            }
            if h.state == ProviderState::Ejected && h.ejected_until.is_some_and(|t| now >= t) {
                h.state = ProviderState::HalfOpen;
                h.probing = false;
            }
            let admissible = match h.state {
                ProviderState::Healthy => true,
                ProviderState::HalfOpen => !h.probing,
                ProviderState::Ejected => false,
            };
            if !admissible {
                continue;
            }
            if let Some(max) = self.providers[idx].max_rps {
                if now.duration_since(h.window_start) >= Duration::from_secs(1) {
                    h.window_start = now;
                    h.window_count = 0;
                }
                if h.window_count >= max {
                    continue;
                }
            }
            candidates.push((idx, self.effective_weight(idx, h)));
        }

        let chosen = if candidates.is_empty() {
            // Everything ejected or saturated: degrade to the provider that
            // recovers soonest rather than failing outright.
            health
                .iter()
                .enumerate()
                .filter(|(i, _)| !exclude.contains(i))
                .min_by_key(|(_, h)| h.ejected_until.unwrap_or(now))
                .map(|(i, _)| i)?
        } else {
            let total: f64 = candidates.iter().map(|(_, w)| w).sum();
            let mut target = r.clamp(0.0, 0.999_999) * total;
            let mut pick = candidates[candidates.len() - 1].0;
            for (idx, w) in &candidates {
                if target < *w {
                    pick = *idx;
                    break;
                }
                target -= w;
            }
            pick
        };

        let h = &mut health[chosen];
        h.window_count = h.window_count.saturating_add(1);
        if h.state == ProviderState::HalfOpen {
            h.probing = true;
        }
        Some(chosen)
    }

    /// Record a successful call.
    pub fn record_success(&self, idx: usize, latency: Duration) {
        let alpha = self.config.ewma_alpha;
        let mut health = self.lock();
        let Some(h) = health.get_mut(idx) else { return };
        let ms = latency.as_secs_f64() * 1000.0;
        h.latency_ewma_ms = if h.requests == 0 {
            ms
        } else {
            alpha.mul_add(ms, (1.0 - alpha) * h.latency_ewma_ms)
        };
        h.error_ewma *= 1.0 - alpha;
        h.requests += 1;
        if h.state == ProviderState::HalfOpen {
            h.state = ProviderState::Healthy;
            h.ejected_until = None;
            h.ejection_reason = None;
            h.cooldown_factor = 1;
            h.error_ewma = 0.0;
            tracing::info!(network = %self.network, provider = %self.providers[idx].name, "RPC provider re-admitted");
        }
        h.probing = false;
        PROVIDER_REQUESTS
            .with_label_values(&[
                self.network.as_str(),
                self.providers[idx].name.as_str(),
                "success",
            ])
            .inc();
        let snapshot = h.clone();
        drop(health);
        self.publish(idx, &snapshot);
    }

    /// Record a failed call.
    pub fn record_failure(&self, idx: usize) {
        self.record_failure_at(idx, Instant::now());
    }

    pub fn record_failure_at(&self, idx: usize, now: Instant) {
        let alpha = self.config.ewma_alpha;
        let mut health = self.lock();
        let Some(h) = health.get_mut(idx) else { return };
        h.error_ewma = alpha.mul_add(1.0, (1.0 - alpha) * h.error_ewma);
        h.requests += 1;
        h.failures += 1;
        PROVIDER_REQUESTS
            .with_label_values(&[
                self.network.as_str(),
                self.providers[idx].name.as_str(),
                "failure",
            ])
            .inc();
        if h.state == ProviderState::HalfOpen {
            self.eject(idx, h, now, "half-open probe failed".to_string());
        } else if h.state == ProviderState::Healthy && h.error_ewma > self.config.error_threshold {
            let reason = format!("error rate {:.2} above threshold", h.error_ewma);
            self.eject(idx, h, now, reason);
        } else {
            h.probing = false;
        }
    }

    /// Record the `latestLedger` reported by a provider and eject laggards.
    pub fn record_latest_ledger(&self, idx: usize, latest: u64) {
        self.record_latest_ledger_at(idx, latest, Instant::now());
    }

    pub fn record_latest_ledger_at(&self, idx: usize, latest: u64, now: Instant) {
        let mut health = self.lock();
        if let Some(h) = health.get_mut(idx) {
            h.latest_ledger = Some(latest);
        }
        let freshest = health
            .iter()
            .filter_map(|h| h.latest_ledger)
            .max()
            .unwrap_or(0);
        for (i, h) in health.iter_mut().enumerate() {
            let Some(seen) = h.latest_ledger else {
                continue;
            };
            let lag = freshest.saturating_sub(seen);
            if lag > self.config.max_ledger_lag && h.state == ProviderState::Healthy {
                self.eject(
                    i,
                    h,
                    now,
                    format!("lagging {lag} ledgers behind {freshest}"),
                );
            }
        }
    }

    #[must_use]
    pub fn statuses(&self) -> Vec<ProviderStatus> {
        let health = self.lock();
        self.providers
            .iter()
            .zip(health.iter())
            .map(|(p, h)| ProviderStatus {
                network: self.network.clone(),
                name: p.name.clone(),
                state: h.state,
                weight: p.weight,
                latency_ewma_ms: h.latency_ewma_ms,
                error_rate: h.error_ewma,
                latest_ledger: h.latest_ledger,
                ejection_reason: h.ejection_reason.clone(),
                requests: h.requests,
                failures: h.failures,
            })
            .collect()
    }

    /// Run `op` against a selected provider, failing over to others on
    /// retryable errors. Every attempt updates provider health.
    pub async fn execute<T, F, Fut>(&self, op: F) -> Result<T, RpcError>
    where
        F: Fn(usize) -> Fut,
        Fut: std::future::Future<Output = Result<T, RpcError>>,
    {
        let mut tried = Vec::new();
        let mut last_err = None;
        while tried.len() < self.providers.len() {
            let Some(idx) = self.select(&tried) else {
                break;
            };
            tried.push(idx);
            let start = Instant::now();
            match op(idx).await {
                Ok(v) => {
                    self.record_success(idx, start.elapsed());
                    return Ok(v);
                }
                Err(e) => {
                    self.record_failure(idx);
                    if !e.is_retryable() {
                        return Err(e);
                    }
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| RpcError::NetworkError("no RPC provider available".into())))
    }
}

/// Process-wide registry so `/health` can report every pool.
fn registry() -> &'static Mutex<HashMap<String, Arc<ProviderPool>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<ProviderPool>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_pool(pool: Arc<ProviderPool>) {
    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(pool.network.clone(), pool);
}

/// Provider breakdown for all registered pools (empty when single-provider).
#[must_use]
pub fn registered_statuses() -> Vec<ProviderStatus> {
    let pools: Vec<Arc<ProviderPool>> = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .values()
        .cloned()
        .collect();
    let mut out: Vec<ProviderStatus> = pools.iter().flat_map(|p| p.statuses()).collect();
    out.sort_by(|a, b| (&a.network, &a.name).cmp(&(&b.network, &b.name)));
    out
}

/// [`StellarRpcClientTrait`] implementation that spreads calls across
/// several providers.
pub struct FailoverRpcClient {
    pool: Arc<ProviderPool>,
    clients: Vec<Arc<dyn StellarRpcClientTrait>>,
    network_config: NetworkConfig,
}

impl FailoverRpcClient {
    /// Build from explicit per-provider clients (used by tests and `from_env`).
    ///
    /// # Panics
    /// Panics if `clients.len()` differs from the pool's provider count.
    #[must_use]
    pub fn new(
        pool: Arc<ProviderPool>,
        clients: Vec<Arc<dyn StellarRpcClientTrait>>,
        network_config: NetworkConfig,
    ) -> Self {
        assert_eq!(pool.providers().len(), clients.len());
        Self {
            pool,
            clients,
            network_config,
        }
    }

    /// Build from `STELLAR_RPC_PROVIDERS_<NETWORK>`. Returns `Ok(None)` when no
    /// provider list is configured (single-provider mode).
    pub fn from_env(network: StellarNetwork) -> Result<Option<Self>, String> {
        let providers = providers_from_env(network)?;
        if providers.is_empty() {
            return Ok(None);
        }
        let mut clients: Vec<Arc<dyn StellarRpcClientTrait>> = Vec::new();
        for p in &providers {
            let client = StellarRpcClient::try_new(p.rpc_url.clone(), p.horizon_url.clone(), false)
                .map_err(|e| format!("provider {}: {e}", p.name))?
                .with_circuit_breaker(new_rpc_circuit_breaker());
            clients.push(Arc::new(client));
        }
        let pool = Arc::new(ProviderPool::new(
            format!("{network:?}").to_lowercase(),
            providers,
            ProviderPoolConfig::from_env(),
        ));
        register_pool(pool.clone());
        Ok(Some(Self::new(
            pool,
            clients,
            NetworkConfig::for_network(network),
        )))
    }

    #[must_use]
    pub fn pool(&self) -> Arc<ProviderPool> {
        self.pool.clone()
    }

    /// Probe every provider's `latestLedger` once and eject laggards.
    pub async fn check_freshness(&self) {
        for (idx, client) in self.clients.iter().enumerate() {
            let start = Instant::now();
            match client.check_health().await {
                Ok(h) => {
                    self.pool.record_success(idx, start.elapsed());
                    self.pool.record_latest_ledger(idx, h.latest_ledger);
                }
                Err(_) => self.pool.record_failure(idx),
            }
        }
    }

    /// Spawn a background task running [`Self::check_freshness`] every `every`.
    pub fn spawn_freshness_monitor(
        self: Arc<Self>,
        every: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(every);
            loop {
                ticker.tick().await;
                self.check_freshness().await;
            }
        })
    }
}

#[async_trait::async_trait]
impl StellarRpcClientTrait for FailoverRpcClient {
    fn network_config(&self) -> &NetworkConfig {
        &self.network_config
    }
    async fn check_health(&self) -> Result<HealthResponse, RpcError> {
        self.pool
            .execute(|i| {
                let c = self.clients[i].clone();
                let pool = self.pool.clone();
                async move {
                    let h = c.check_health().await?;
                    pool.record_latest_ledger(i, h.latest_ledger);
                    Ok(h)
                }
            })
            .await
    }
    async fn fetch_latest_ledger(&self) -> Result<LedgerInfo, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_latest_ledger())
            .await
    }
    async fn fetch_ledger_by_sequence(&self, sequence: u64) -> Result<LedgerInfo, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_ledger_by_sequence(sequence))
            .await
    }
    async fn fetch_ledgers(
        &self,
        start_ledger: Option<u64>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<GetLedgersResult, RpcError> {
        self.pool
            .execute(|i| {
                let c = self.clients[i].clone();
                let pool = self.pool.clone();
                async move {
                    let r = c.fetch_ledgers(start_ledger, limit, cursor).await?;
                    pool.record_latest_ledger(i, r.latest_ledger);
                    Ok(r)
                }
            })
            .await
    }
    async fn fetch_transactions_for_ledger(
        &self,
        ledger_sequence: u64,
    ) -> Result<Vec<HorizonTransaction>, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_transactions_for_ledger(ledger_sequence))
            .await
    }
    async fn fetch_payments_for_ledger(
        &self,
        ledger_sequence: u64,
    ) -> Result<Vec<Payment>, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_payments_for_ledger(ledger_sequence))
            .await
    }
    async fn fetch_account_payments(
        &self,
        account_id: &str,
        limit: u32,
    ) -> Result<Vec<Payment>, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_account_payments(account_id, limit))
            .await
    }
    async fn fetch_all_account_payments(
        &self,
        account_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Payment>, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_all_account_payments(account_id, limit))
            .await
    }
    async fn fetch_operations_for_ledger(
        &self,
        ledger_sequence: u64,
    ) -> Result<Vec<HorizonOperation>, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_operations_for_ledger(ledger_sequence))
            .await
    }
    async fn fetch_trades(&self, limit: u32, cursor: Option<&str>) -> Result<Vec<Trade>, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_trades(limit, cursor))
            .await
    }
    async fn fetch_order_book(
        &self,
        selling_asset: &crate::rpc::stellar::Asset,
        buying_asset: &crate::rpc::stellar::Asset,
        limit: u32,
    ) -> Result<OrderBook, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_order_book(selling_asset, buying_asset, limit))
            .await
    }
    async fn fetch_liquidity_pools(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<Vec<HorizonLiquidityPool>, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_liquidity_pools(limit, cursor))
            .await
    }
    async fn fetch_pool_trades(&self, pool_id: &str, limit: u32) -> Result<Vec<Trade>, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_pool_trades(pool_id, limit))
            .await
    }
    async fn fetch_assets(
        &self,
        limit: u32,
        sponsored: bool,
    ) -> Result<Vec<HorizonAsset>, RpcError> {
        self.pool
            .execute(|i| self.clients[i].fetch_assets(limit, sponsored))
            .await
    }
}

#[cfg(test)]
#[path = "provider_pool_tests.rs"]
mod tests;
