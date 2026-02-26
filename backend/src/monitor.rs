use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::alerts::AlertManager;
use crate::api::corridors_cached::CorridorResponse;
use crate::cache::CacheManager;
use crate::rpc::StellarRpcClient;
use crate::webhooks::events::CorridorMetrics;

pub struct CorridorMonitor {
    alert_manager: Arc<AlertManager>,
    cache: Arc<CacheManager>,
    rpc_client: Arc<StellarRpcClient>,
    previous_state: tokio::sync::RwLock<HashMap<String, CorridorState>>,
    webhook_event_service: Option<Arc<crate::services::webhook_event_service::WebhookEventService>>,
}

#[derive(Clone)]
struct CorridorState {
    success_rate: f64,
    latency: f64,
    liquidity: f64,
}

impl CorridorMonitor {
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
        }
    }

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
        }
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
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut corridor_map: HashMap<String, Vec<&crate::rpc::Payment>> = HashMap::new();
        for payment in &payments {
            let key = format!(
                "{}:{}->XLM:native",
                payment.get_asset_code().as_deref().unwrap_or("XLM"),
                payment.get_asset_issuer().as_deref().unwrap_or("native")
            );
            corridor_map
                .entry(key)
                .or_insert_with(Vec::new)
                .push(payment);
        }

        let mut prev_state = self.previous_state.write().await;

        for (corridor_id, payments) in corridor_map {
            let success_rate = 100.0;
            let latency = 400.0 + (success_rate * 2.0);
            let liquidity: f64 = payments
                .iter()
                .filter_map(|p| p.get_amount().parse::<f64>().ok())
                .sum();

            if let Some(old_state) = prev_state.get(&corridor_id) {
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
                corridor_id,
                CorridorState {
                    success_rate,
                    latency,
                    liquidity,
                },
            );
        }

        Ok(())
    }
}
