use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::alerts::AlertManager;
use crate::api::corridors_cached::CorridorResponse;
use crate::cache::CacheManager;
use crate::rpc::StellarRpcClient;

pub struct CorridorMonitor {
    alert_manager: Arc<AlertManager>,
    cache: Arc<CacheManager>,
    rpc_client: Arc<StellarRpcClient>,
    previous_state: tokio::sync::RwLock<HashMap<String, CorridorState>>,
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
