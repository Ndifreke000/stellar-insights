use crate::models::corridor::CorridorMetrics;
use crate::models::{AnchorMetrics, AnchorStatus};
use crate::services::broadcaster_port::BroadcasterPort;
use crate::services::data_port::DataPort;
use crate::services::webhook_event_service::WebhookEventService;
use crate::websocket::{WsMessage, WsState};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Real-time broadcaster service for WebSocket updates.
/// Depends on traits (BroadcasterPort, DataPort) — not concrete types.
pub struct RealtimeBroadcaster {
    ws_state:       Arc<WsState>,
    data:           Arc<dyn DataPort>,
    subscriptions:  Arc<DashMap<Uuid, HashSet<String>>>,
    webhook_events: Arc<WebhookEventService>,
    shutdown_rx:    Option<tokio::sync::oneshot::Receiver<()>>,
    shutdown_tx:    std::sync::Mutex<Option<tokio::sync::oneshot::Sder<()>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubscriptionMessage {
    Subscribe { channels: Vec<String> },
    Unsubscribe { channels: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BroadcastMessage {
    CorridorUpdate { corridor: CorridorMetrics, channel: String },
    AnchorStatusChange { anchor: AnchorMetrics, old_status: String, channel: String },
    NewPayment { corridor_key: String, amount: f64, successful: bool, timestamp: String, channel: String },
    HealthAlert { corridor_id: String, severity: String, message: String, timestamp: String },
    ConnectionStatus { status: String },
}

impl RealtimeBroadcaster {
    /// Create a new `RealtimeBroadcaster`.
    /// Accepts trait objects — callers pass `Arc<dyn DataPort>` and `Arc<WebhookEventService>`.
    #[must_use]
    pub fn new(
        ws_state: Arc<WsState>,
        data: Arc<dyn DataPort>        webhook_events: Arc<WebhookEventService>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        Self {
            ws_state,
            data,
            subscriptions: Arc::new(DashMap::new()),
            webhook_events,
            shutdown_rx: Some(shutdown_rx),
            shutdown_tx: std::sync::Mutex::new(Some(shutdown_tx)),
        }
    }

    /// Start the broadcaster background tasks.
    pub async fn start(&mut self) {
        info!("Starting RealtimeBroadcaster service");

        let shutdown_rx = self
            .shutdown_rx
            .take()
            .expect("Shutdown receiver already taken");

        let corridor_task  = self.start_corridor_broadcast_task();
        let subscription_task = self.start_subscription_management_task();

        tokio::select! {
            _ = shutdown_rx => { info!("RealtimeBroadcaster received shutdown signal"); }
            _ = corridor_task => { warn!("Corridor broadcast task completed unexpectedly"); }
            _ = subscription_task => { warn!("Subscription management task completed unexpectedly"); }
        }

        info!("RealtimeBroadcaster service stopped");
    }

    fn start_corridor_broadcast_task(&self) -> tokio::task::JoinHandle<()> {
        let ws_state      = Arc::clone(&self.ws_state);
        let data          = Arc::clone(&self.data);
        let subscriptions = Arc::clone(&self.subscriptions);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                match data.fetch_corridor_updates().await {
                    Ok(corridors) => {
                        for corridor in corridors {
                            let channel = format!("corridor:{}", corridor.corridor_key);
                            let message = BroadcastMessage::CorridorUpdate {
                                corridor: corridor.clone(),
                                channel: channel.clone(),
                            };
                            Self::broadcast_to_subscribers(&ws_state, &subscriptions, &channel, message).await;
                        }
                    }
                    Err(e) => { error!("Failed to fetch corridor updates: {}", e); }
                }
            }
        })
    }

    fn start_subscription_management_task(&self) -> tokio::task::JoinHandle<()> {
        let ws_state      = Arc::clone(&self.ws_state);
        let subscriptions = Arc::clone(&self.subscriptions);

        tokio::spawn(async move {
            info!("Subscription management task started");
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                let active: HashSet<Uuid> = ws_state.connections.iter().map(|e| *e.key()).collect();
                subscriptions.retain(|id, _| active.contains(id));
            }
        })
    }

    async fn broadcast_to_subscribers(
        ws_state:      &Arc<WsState>,
        subscriptions: &Arc<DashMap<Uuid, HashSet<String>>>,
        channel:       &str,
        message:       BroadcastMessage,
    ) {
        let ws_message = WsMessage::from_broadcast_message(message);
        let targets: Vec<Uuid> = subscriptions
            .iter()
            .filter(|e| e.value().contains(channel))
            .map(|e| *e.key())
            .collect();

        for id in targets {
            if let Some(sender) = ws_state.connections.get(&id) {
                if let Err(e) = sender.send(ws_message.clone()).await {
                    warn!("Failed to send message to connection {}: {}", id, e);
                }
            }
        }
    }

    /// Subscribe a connection to channels.
    pub fn subscribe_connection(&self, connection_id: Uuid, channels: Vec<String>) {
        let mut set = self.subscriptions.entry(connection_id).or_default();
        for ch in channels {
            info!("Connection {} subscribed to channel: {}", connection_id, ch);
            set.insert(ch);
        }
    }

    /// Unsubscribe a connection from channels.
    pub fn unsubscribe_connection(&self, connection_id: Uuid, channels: Vec<String>) {
        if let Some(mut set) = self.subscriptions.get_mut(&connection_id) {
            for ch in channels {
                info!("Connection {} unsubscribed from channel: {}", connection_id, ch);
                set.remove(&ch);
            }
        }
    }

    /// Shutdown the broadcaster.
    pub fn shutdown(&self) {
        if let Ok(mut guard) = self.shutdown_tx.lock() {
            if let Some(tx) = guard.take() {
                if tx.send(()).is_err() {
                    warn!("Failed to send shutdown signal - receiver may have been dropped");
                }
            }
        }
    }
}

// --- BroadcasterPort impl ---------------------------------------------------

#[async_trait]
impl BroadcasterPort for RealtimeBroadcaster {
    async fn broadcast_corridor_update(&self, corridor: CorridorMetrics) {
        let channel = format!("corridor:{}", corridor.corridor_key);
        let message = BroadcastMessage::CorridorUpdate { corridor, channel: channel.clone() };
        Self::broadcast_to_subscribers(&self.ws_state, &self.subscriptions, &channel, message).await;
    }

    async fn broadcast_anchor_status(
        &self,
        anchor_id: String,
        anchor_name: String,
        anchor: AnchorMetrics,
        old_status: String,
    ) {
        let channel = "anchor:status".to_string();
        let message = BroadcastMessage::AnchorStatusChange {
            anchor: anchor.clone(),
            old_status: old_status.clone(),
            channel: channel.clone(),
        };
        Self::broadcast_to_subscribers(&self.ws_state, &self.subscriptions, &channel, message).await;

        let webhook_events  = Arc::clone(&self.webhook_events);
        let new_status      = anchor.status.as_str().to_string();
        let reliability     = anchor.reliability_score;
        let failed_txn      = anchor.failed_transactions;
        tokio::spawn(async move {
            if let Err(e) = webhook_events
                .trigger_anchor_status_changed(&anchor_id, &anchor_name, &old_status, &new_status, reliability, failed_txn)
                .await
            {
                warn!("Failed to trigger anchor_status_changed webhook: {}", e);
            }
        });
    }

    async fn broadcast_payment(&self, corridor_key: String, amount: f64, successful: bool, timestamp: String) {
        let channel = format!("corridor:{corridor_key}");
        let message = BroadcastMessage::NewPayment {
            corridor_key,
            amount,
            successful,
            timestamp,
            channel: channel.clone(),
        };
        Self::broadcast_to_subscribers(&self.ws_state, &self.subscriptions, &channel, message).await;
    }

    async fn broadcast_health_alert(&self, corridor_id: String, severity: String, message: String) {
    /// Broadcast health alert to all clients
    pub fn broadcast_health_alert(
        &self,
        corridor_id: String,
        severity: String,
        message: String,
    ) {
        let alert = BroadcastMessage::HealthAlert {
            corridor_id: corridor_id.clone(),
            severity: severity.clone(),
            message: message.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.ws_state.broadcast(WsMessage::from_broadcast_message(alert));

        let webhook_events = Arc::clone(&self.webhook_events);
        tokio::spawn(async move {
            use crate::webhooks::events::CorridorMetrics as EventMetrics;
            let empty = EventMetrics {
                success_rate: 0.0, avg_latency_ms: 0.0, p95_latency_ms: 0.0,
                p99_latency_ms: 0.0, liquidity_depth_usd: 0.0,
                liquidity_volume_24h_usd: 0.0, total_attempts: 0,
                successful_payments: 0, failed_payments: 0,
            };
            if let Err(e) = webhook_events
                .trigger_corridor_health_degraded(&corridor_id, &empty, &empty, &severity, vec![message])
                .await
            {
                warn!("Failed to trigger corridor_health_degraded webhook: {}", e);
            }
        });
    }

    fn connection_count(&self) -> usize {
    /// Subscribe a connection to specific channels
    pub fn subscribe_connection(&self, connection_id: Uuid, channels: Vec<String>) {
        let mut subscription_set = self.subscriptions.entry(connection_id).or_default();

        for channel in channels {
            subscription_set.insert(channel.clone());
            info!(
                "Connection {} subscribed to channel: {}",
                connection_id, channel
            );
        }
    }

    /// Unsubscribe a connection from specific channels
    pub fn unsubscribe_connection(&self, connection_id: Uuid, channels: Vec<String>) {
        if let Some(mut subscription_set) = self.subscriptions.get_mut(&connection_id) {
            for channel in channels {
                subscription_set.remove(&channel);
                info!(
                    "Connection {} unsubscribed from channel: {}",
                    connection_id, channel
                );
            }
        }
    }

    /// Broadcast message to subscribers of a specific channel
    async fn broadcast_to_subscribers(
        ws_state: &Arc<WsState>,
        subscriptions: &Arc<DashMap<Uuid, HashSet<String>>>,
        channel: &str,
        message: BroadcastMessage,
    ) {
        let ws_message = WsMessage::from_broadcast_message(message);

        let target_connections: Vec<Uuid> = subscriptions
            .iter()
            .filter(|e| e.value().contains(channel))
            .map(|e| *e.key())
            .collect();

        for connection_id in target_connections {
            if let Some(sender) = ws_state.connections.get(&connection_id) {
                if let Err(e) = sender.send(ws_message.clone()).await {
                    warn!("Failed to send message to connection {}: {}", connection_id, e);
                }
            }
        }
    }

    /// Shutdown the broadcaster
    pub fn shutdown(&self) {
        if let Ok(mut tx_guard) = self.shutdown_tx.lock() {
            if let Some(tx) = tx_guard.take() {
                if tx.send(()).is_err() {
                    warn!("Failed to send shutdown signal - receiver may have been dropped");
                }
            }
        }
    }

    /// Get connection count
    pub fn connection_count(&self) -> usize {
        self.ws_state.connection_count()
    }

    fn channel_subscription_count(&self, channel: &str) -> usize {
        self.subscriptions.iter().filter(|e| e.value().contains(channel)).count()
    }
}

// --- WsMessage conversion ---------------------------------------------------

impl WsMessage {
    pub fn from_broadcast_message(msg: BroadcastMessage) -> Self {
        match msg {
            BroadcastMessage::CorridorUpdate { corridor, .. } => Self::CorridorUpdate {
                corridor_key:          corridor.corridor_key,
                source_asset_code:     corridor.source_asset_code,
                source_asset_issuer:   corridor.source_asset_issuer,
                destination_asset_code: corridor.destination_asset_code,
                destination_asset_issuer: corridor.destination_asset_issuer,
                success_rate:  Some(corridor.success_rate),
                health_score:  Some(corridor.success_rate * 100.0),
                last_updated:  Some(corridor.updated_at.to_rfc3339()),
            },
            BroadcastMessage::AnchorStatusChange { anchor, .. } => Self::AnchorUpdate {
                anchor_id:         "unknown".to_string(),
                name:              "unknown".to_string(),
                reliability_score: anchor.reliability_score,
                status: AnchorStatus::from_metrics(anchor.success_rate, anchor.failure_rate)
                    .as_str()
                    .to_string(),
            },
            BroadcastMessage::NewPayment { corridor_key, amount, successful, timestamp, .. } => {
                Self::NewPayment { corridor_id: corridor_key, amount, successful, timestamp }
            }
            BroadcastMessage::HealthAlert { corridor_id, severity, message, timestamp } => {
                Self::HealthAlert { corridor_id, severity, message, timestamp }
            }
            BroadcastMessage::ConnectionStatus { status } => Self::ConnectionStatus { status },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_message_serialization() {
        let msg = SubscriptionMessage::Subscribe {
            channels: vec!["corridor:XLM-USDC".to_string()],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("subscribe"));
        assert!(json.contains("XLM-USDC"));
    }

    #[test]
    fn test_broadcast_message_serialization() {
        let msg = BroadcastMessage::ConnectionStatus {
            status: "connected".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("connection_status"));
        assert!(json.contains("connected"));
    fn test_subscription_management() {
        let _ws_state = Arc::new(WsState::new());
        let _rpc_client = Arc::new(StellarRpcClient::new(
            "test".to_string(),
            "test".to_string(),
            true,
        ));
        // Note: This test would need a mock CacheManager before testing subscription logic.
    }
}
