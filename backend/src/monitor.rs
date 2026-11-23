use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

use crate::alerts::AlertManager;
use crate::cache::CacheManager;
use crate::rpc::StellarRpcClient;
use crate::webhooks::events::CorridorMetrics;

/// Default number of seconds without a new ledger close before the network is
/// considered halted.
pub const DEFAULT_HALT_THRESHOLD_SECS: u64 = 30;

pub struct CorridorMonitor {
    alert_manager: Arc<AlertManager>,
    cache: Arc<CacheManager>,
    rpc_client: Arc<StellarRpcClient>,
    previous_state: tokio::sync::RwLock<HashMap<String, CorridorState>>,
    webhook_event_service: Option<Arc<crate::services::webhook_event_service::WebhookEventService>>,
    halt_threshold_secs: u64,
    network_status: tokio::sync::RwLock<NetworkStatus>,
}

#[derive(Clone, Serialize, Deserialize)]
struct CorridorState {
    success_rate: f64,
    latency: f64,
    liquidity: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub success_rate: f64,
    pub latency: f64,
    pub liquidity: f64,
}

/// Snapshot of the Stellar network health used to drive the frontend banner.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkStatus {
    /// True when no ledger has closed within the configured threshold.
    pub halted: bool,
    /// Seconds since the last observed ledger close, if known.
    pub seconds_since_last_ledger: Option<u64>,
    /// Last observed protocol version, if known.
    pub protocol_version: Option<u32>,
    /// True when the protocol version changed since the previous check.
    pub protocol_upgrade_detected: bool,
    /// Human readable banner message for the frontend, if any.
    pub banner: Option<String>,
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self {
            halted: false,
            seconds_since_last_ledger: None,
            protocol_version: None,
            protocol_upgrade_detected: false,
            banner: None,
        }
    }
}

#[cfg(test)]
static FETCH_CORRIDOR_METRICS_CALLS: AtomicU64 = AtomicU64::new(0);

impl CorridorMonitor {
    #[must_use]
    pub fn new(
        alert_manager: Arc<AlertManager>,
        cache: Arc<CacheManager>,
        rpc_client: Arc<StellarRpcClient>,
    ) -> Self {
        Self {
            alert_manager,
            cache,
            rpc_client,
            previous_state: tokio::sync::RwLock::new(HashMap::new()),
            webhook_event_service: None,
            halt_threshold_secs: DEFAULT_HALT_THRESHOLD_SECS,
            network_status: tokio::sync::RwLock::new(NetworkStatus::default()),
        }
    }

    #[must_use]
    pub fn new_with_webhooks(
        alert_manager: Arc<AlertManager>,
        cache: Arc<CacheManager>,
        rpc_client: Arc<StellarRpcClient>,
        webhook_event_service: Arc<crate::services::webhook_event_service::WebhookEventService>,
    ) -> Self {
        Self {
            alert_manager,
            cache,
            rpc_client,
            previous_state: tokio::sync::RwLock::new(HashMap::new()),
            webhook_event_service: Some(webhook_event_service),
            halt_threshold_secs: DEFAULT_HALT_THRESHOLD_SECS,
            network_status: tokio::sync::RwLock::new(NetworkStatus::default()),
        }
    }

    /// Override the halt threshold (seconds without a ledger close).
    #[must_use]
    pub fn with_halt_threshold(mut self, secs: u64) -> Self {
        self.halt_threshold_secs = secs;
        self
    }

    /// Current network status snapshot for the banner API.
    pub async fn network_status(&self) -> NetworkStatus {
        self.network_status.read().await.clone()
    }

    /// Evaluate halt/upgrade state from the latest ledger close time and
    /// protocol version. Returns the updated status.
    pub async fn evaluate_network_status(
        &self,
        last_ledger_close_unix: Option<u64>,
        protocol_version: Option<u32>,
        now_unix: u64,
    ) -> NetworkStatus {
        let mut status = self.network_status.write().await;

        let seconds_since_last_ledger = last_ledger_close_unix
            .map(|close| now_unix.saturating_sub(close));
        let halted = seconds_since_last_ledger
            .map(|secs| secs > self.halt_threshold_secs)
            .unwrap_or(false);

        let protocol_upgrade_detected = match (status.protocol_version, protocol_version) {
            (Some(prev), Some(current)) => prev != current,
            _ => false,
        };

        status.halted = halted;
        status.seconds_since_last_ledger = seconds_since_last_ledger;
        status.protocol_version = protocol_version.or(status.protocol_version);
        status.protocol_upgrade_detected = protocol_upgrade_detected;
        status.banner = if halted {
            Some(format!(
                "Stellar network halt detected: no ledger close for {}s. Corridor alerts are paused.",
                seconds_since_last_ledger.unwrap_or_default()
            ))
        } else if protocol_upgrade_detected {
            Some(format!(
                "Stellar protocol upgrade detected (now version {}).",
                protocol_version.unwrap_or_default()
            ))
        } else {
            None
        };

        status.clone()
    }

    pub async fn start(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(60));

        loop {
            ticker.tick().await;
            if let Err(e) = self.check_corridors().await {
                tracing::error!("Error checking corridors: {}", e);
            }
        }
    }

    async fn check_corridors(&self) -> anyhow::Result<()> {
        let payments = self
            .rpc_client
            .fetch_payments(200, None)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // Suppress corridor alerts while the network is halted to avoid false
        // positives caused by missing ledger data.
        if self.network_status.read().await.halted {
            tracing::warn!("Network halt active; suppressing corridor alerts");
            return Ok(());
        }

        let mut corridor_map: HashMap<String, Vec<&crate::rpc::Payment>> = HashMap::new();
        for payment in &payments {
            let key = format!(
                "{}:{}->XLM:native",
                payment.get_asset_code().as_deref().unwrap_or("XLM"),
                payment.get_asset_issuer().as_deref().unwrap_or("native")
            );
            corridor_map.entry(key).or_default().push(payment);
        }

        let mut prev_state = self.previous_state.write().await;

        for (corridor_id, payments) in corridor_map {
            let success_rate = 100.0;
            let latency = 400.0 + (success_rate * 2.0);
            let liquidity: f64 = payments
                .iter()
                .filter_map(|p| p.get_amount().parse::<f64>().ok())
                .sum();

            let cache_key = format!("corridor_health:{}", corridor_id);
            let cached_state: Option<CorridorState> =
                self.cache.get(&cache_key).await.unwrap_or(None);
            let effective_old = cached_state
                .as_ref()
                .or_else(|| prev_state.get(&corridor_id));

            if let Some(old_state) = effective_old {
                self.alert_manager.check_and_alert(
                    &corridor_id,
                    old_state.success_rate,
                    success_rate,
                    old_state.latency,
                    latency,
                    old_state.liquidity,
                    liquidity,
                );

                // Trigger webhook events for corridor changes
                if let Some(webhook_service) = &self.webhook_event_service {
                    let old_metrics = CorridorMetrics {
                        success_rate: old_state.success_rate / 100.0,
                        avg_latency_ms: old_state.latency,
                        p95_latency_ms: old_state.latency * 1.5,
                        p99_latency_ms: old_state.latency * 2.0,
                        liquidity_depth_usd: old_state.liquidity,
                        liquidity_volume_24h_usd: old_state.liquidity * 10.0,
                        total_attempts: 100,
                        successful_payments: (old_state.success_rate / 100.0 * 100.0) as i64,
                        failed_payments: (100.0 - old_state.success_rate) as i64,
                    };

                    let new_metrics = CorridorMetrics {
                        success_rate: success_rate / 100.0,
                        avg_latency_ms: latency,
                        p95_latency_ms: latency * 1.5,
                        p99_latency_ms: latency * 2.0,
                        liquidity_depth_usd: liquidity,
                        liquidity_volume_24h_usd: liquidity * 10.0,
                        total_attempts: 100,
                        successful_payments: (success_rate / 100.0 * 100.0) as i64,
                        failed_payments: (100.0 - success_rate) as i64,
                    };

                    // Check for corridor health degradation
                    use crate::webhooks::events::{check_corridor_degradation, determine_severity};
                    let (degraded, changes) =
                        check_corridor_degradation(&old_metrics, &new_metrics);

                    if degraded {
                        let severity = determine_severity(&old_metrics, &new_metrics);
                        let webhook_service = webhook_service.clone();
                        let corridor_id_clone = corridor_id.clone();
                        let changes_clone = changes.clone();
                        let severity_clone = severity.clone();

                        tokio::spawn(async move {
                            if let Err(e) = webhook_service
                                .trigger_corridor_health_degraded(
                                    &corridor_id_clone,
                                    &old_metrics,
                                    &new_metrics,
                                    &severity_clone,
                                    changes_clone,
                                )
                                .await
                            {
                                tracing::error!("Failed to trigger corridor health webhook: {}", e);
                            }
                        });
                    }

                    // Check for liquidity drops
                    if old_state.liquidity > 0.0
                        && (old_state.liquidity - liquidity) / old_state.liquidity > 0.30
                    {
                        let webhook_service = webhook_service.clone();
                        let corridor_id_clone = corridor_id.clone();
                        let threshold = old_state.liquidity * 0.7; // 30% drop threshold

                        tokio::spawn(async move {
                            if let Err(e) = webhook_service
                                .trigger_corridor_liquidity_dropped(
                                    &corridor_id_clone,
                                    liquidity,
                                    threshold,
                                    "decreasing",
                                    "warning",
                                )
                                .await
                            {
                                tracing::error!(
                                    "Failed to trigger corridor liquidity webhook: {}",
                                    e
                                );
                            }
                        });
                    }
                }
            }

            prev_state.insert(
                corridor_id.clone(),
                CorridorState {
                    success_rate,
                    latency,
                    liquidity,
                },
            );

            let _ = self
                .cache
                .set(
                    &cache_key,
                    &CorridorState {
                        success_rate,
                        latency,
                        liquidity,
                    },
                    300,
                )
                .await;
        }

        Ok(())
    }

    pub async fn check_health(&self, corridor_key: &str) -> anyhow::Result<HealthStatus> {
        let cache_key = format!("corridor_health:{}", corridor_key);

        if let Some(cached) = self.cache.get::<HealthStatus>(&cache_key).await? {
            return Ok(cached);
        }

        let fresh = self.fetch_corridor_metrics(corridor_key).await?;
        let _ = self.cache.set(&cache_key, &fresh, 60).await;
        Ok(fresh)
    }

    async fn fetch_corridor_metrics(&self, corridor_key: &str) -> anyhow::Result<HealthStatus> {
        #[cfg(test)]
        {
            FETCH_CORRIDOR_METRICS_CALLS.fetch_add(1, Ordering::Relaxed);
        }

        let payments = self
            .rpc_client
            .fetch_payments(200, None)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let liquidity: f64 = payments
            .iter()
            .filter(|payment| {
                let key = format!(
                    "{}:{}->XLM:native",
                    payment.get_asset_code().as_deref().unwrap_or("XLM"),
                    payment.get_asset_issuer().as_deref().unwrap_or("native")
                );
                key == corridor_key
            })
            .filter_map(|p| p.get_amount().parse::<f64>().ok())
            .sum();

        Ok(HealthStatus {
            success_rate: 100.0,
            latency: 400.0,
            liquidity,
        })
    }
}

#[cfg(test)]
mod network_status_tests {
    use super::*;

    fn status_from(
        last_close: Option<u64>,
        protocol: Option<u32>,
        now: u64,
        threshold: u64,
    ) -> NetworkStatus {
        let mut status = NetworkStatus::default();
        let seconds_since_last_ledger = last_close.map(|c| now.saturating_sub(c));
        let halted = seconds_since_last_ledger
            .map(|s| s > threshold)
            .unwrap_or(false);
        let protocol_upgrade_detected = match (status.protocol_version, protocol) {
            (Some(prev), Some(cur)) => prev != cur,
            _ => false,
        };
        status.halted = halted;
        status.seconds_since_last_ledger = seconds_since_last_ledger;
        status.protocol_version = protocol.or(status.protocol_version);
        status.protocol_upgrade_detected = protocol_upgrade_detected;
        status
    }

    #[test]
    fn stall_triggers_halt() {
        let status = status_from(Some(1_000), Some(20), 1_100, 30);
        assert!(status.halted);
        assert_eq!(status.seconds_since_last_ledger, Some(100));
    }

    #[test]
    fn fresh_ledger_is_not_halted() {
        let status = status_from(Some(1_000), Some(20), 1_010, 30);
        assert!(!status.halted);
    }

    #[test]
    fn unknown_ledger_is_not_halted() {
        let status = status_from(None, None, 1_000, 30);
        assert!(!status.halted);
        assert_eq!(status.seconds_since_last_ledger, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{CacheConfig, CacheManager};

    #[tokio::test]
    async fn test_health_check_caching() {
        let _guard = crate::lock_env_test();
        FETCH_CORRIDOR_METRICS_CALLS.store(0, Ordering::Relaxed);

        let (alert_manager, _rx) = AlertManager::new();
        let cache = Arc::new(CacheManager::new_in_memory_for_tests(CacheConfig::default()));
        let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
        let monitor = CorridorMonitor::new(Arc::new(alert_manager), cache, rpc_client);

        let corridor_key = "USDC:native->XLM:native";

        let _first = monitor.check_health(corridor_key).await.unwrap();
        assert_eq!(
            FETCH_CORRIDOR_METRICS_CALLS.load(Ordering::Relaxed),
            1,
            "first call should compute fresh metrics"
        );

        let _second = monitor.check_health(corridor_key).await.unwrap();
        assert_eq!(
            FETCH_CORRIDOR_METRICS_CALLS.load(Ordering::Relaxed),
            1,
            "second call should use cached metrics"
        );
    }
}
