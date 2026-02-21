use crate::cache::CacheManager;
use crate::database::Database;
use crate::models::corridor::CorridorMetrics;
use crate::models::{AnchorMetrics, PaymentRecord};
use crate::rpc::StellarRpcClient;
use crate::websocket::{WsMessage, WsState};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Real-time broadcaster service for WebSocket updates
pub struct RealtimeBroadcaster {
    /// WebSocket state for managing connections
    ws_state: Arc<WsState>,
    /// Database for fetching data
    db: Arc<Database>,
    /// RPC client for fetching data
    _rpc_client: Arc<StellarRpcClient>,
    /// Cache manager for data access
    _cache: Arc<CacheManager>,
    /// Per-connection subscriptions
    subscriptions: Arc<DashMap<Uuid, HashSet<String>>>,
    /// Shutdown signal receiver
    shutdown_rx: Option<tokio::sync::oneshot::Receiver<()>>,
    /// Shutdown signal sender
    shutdown_tx: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
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
    CorridorUpdate {
        corridor: CorridorMetrics,
        channel: String,
    },
    AnchorStatusChange {
        anchor: AnchorMetrics,
        old_status: String,
        channel: String,
    },
    NewPayment {
        payment: PaymentRecord,
        channel: String,
    },
    HealthAlert {
        corridor_id: String,
        severity: String,
        message: String,
        timestamp: String,
    },
    ConnectionStatus {
        status: String,
    },
}

impl RealtimeBroadcaster {
    /// Create a new realtime broadcaster
    pub fn new(
        ws_state: Arc<WsState>,
        db: Arc<Database>,
        rpc_client: Arc<StellarRpcClient>,
        cache: Arc<CacheManager>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        Self {
            ws_state,
            db,
            _rpc_client: rpc_client,
            _cache: cache,
            subscriptions: Arc::new(DashMap::new()),
            shutdown_rx: Some(shutdown_rx),
            shutdown_tx: std::sync::Mutex::new(Some(shutdown_tx)),
        }
    }

    /// Start the broadcaster background tasks
    pub async fn start(&mut self) {
        info!("Starting RealtimeBroadcaster service");

        let shutdown_rx = self
            .shutdown_rx
            .take()
            .expect("Shutdown receiver already taken");

        // Start corridor metrics broadcasting task
        let corridor_task = self.start_corridor_broadcast_task();

        // Start subscription management task
        let subscription_task = self.start_subscription_management_task();

        // Wait for shutdown signal or task completion
        tokio::select! {
            _ = shutdown_rx => {
                info!("RealtimeBroadcaster received shutdown signal");
            }
            _ = corridor_task => {
                warn!("Corridor broadcast task completed unexpectedly");
            }
            _ = subscription_task => {
                warn!("Subscription management task completed unexpectedly");
            }
        }

        info!("RealtimeBroadcaster service stopped");
    }

    /// Start the corridor metrics broadcasting task
    fn start_corridor_broadcast_task(&self) -> tokio::task::JoinHandle<()> {
        let ws_state = Arc::clone(&self.ws_state);
        let db = Arc::clone(&self.db);
        let subscriptions = Arc::clone(&self.subscriptions);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Fetch latest corridor metrics from database
                match Self::fetch_corridor_updates(&db).await {
                    Ok(corridors) => {
                        for corridor in corridors {
                            let channel = format!("corridor:{}", corridor.corridor_key);
                            let message = BroadcastMessage::CorridorUpdate {
                                corridor: corridor.clone(),
                                channel: channel.clone(),
                            };

                            Self::broadcast_to_subscribers(
                                &ws_state,
                                &subscriptions,
                                &channel,
                                message,
                            )
                            .await;
                        }
                    }
                    Err(e) => {
                        error!("Failed to fetch corridor updates: {}", e);
                    }
                }
            }
        })
    }

    /// Start the subscription management task
    fn start_subscription_management_task(&self) -> tokio::task::JoinHandle<()> {
        let ws_state = Arc::clone(&self.ws_state);
        let subscriptions = Arc::clone(&self.subscriptions);

        tokio::spawn(async move {
            // This task would handle incoming subscription messages
            // For now, we'll implement basic subscription tracking
            info!("Subscription management task started");

            // Keep the task alive
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                // Clean up subscriptions for disconnected clients
                let active_connections: HashSet<Uuid> = ws_state
                    .connections
                    .iter()
                    .map(|entry| *entry.key())
                    .collect();

                subscriptions.retain(|connection_id, _| active_connections.contains(connection_id));
            }
        })
    }

    /// Fetch corridor updates from database
    async fn fetch_corridor_updates(
        db: &Arc<Database>,
    ) -> Result<Vec<CorridorMetrics>, Box<dyn std::error::Error + Send + Sync>> {
        match db.list_corridors(50, 0).await {
            Ok(corridors) => {
                let mut corridor_metrics = Vec::new();
                for corridor in corridors {
                    let now = chrono::Utc::now();
                    let corridor_key = corridor.to_string_key();
                    let metrics = CorridorMetrics {
                        id: corridor_key.clone(),
                        corridor_key,
                        asset_a_code: corridor.asset_a_code,
                        asset_a_issuer: corridor.asset_a_issuer,
                        asset_b_code: corridor.asset_b_code,
                        asset_b_issuer: corridor.asset_b_issuer,
                        date: now,
                        total_transactions: 0,
                        successful_transactions: 0,
                        failed_transactions: 0,
                        success_rate: 0.0,
                        volume_usd: 0.0,
                        avg_settlement_latency_ms: None,
                        median_settlement_latency_ms: None,
                        liquidity_depth_usd: 0.0,
                        created_at: now,
                        updated_at: now,
                    };
                    corridor_metrics.push(metrics);
                }
                Ok(corridor_metrics)
            }
            Err(e) => {
                warn!("Failed to fetch corridors from database: {}", e);
                Ok(vec![])
            }
        }
    }

    /// Broadcast corridor update to all subscribed clients
    pub async fn broadcast_corridor_update(&self, corridor: CorridorMetrics) {
        let channel = format!("corridor:{}", corridor.corridor_key);
        let message = BroadcastMessage::CorridorUpdate {
            corridor,
            channel: channel.clone(),
        };

        Self::broadcast_to_subscribers(&self.ws_state, &self.subscriptions, &channel, message)
            .await;
    }

    /// Broadcast anchor status change to all subscribed clients
    pub async fn broadcast_anchor_status(&self, anchor: AnchorMetrics, old_status: String) {
        let channel = "anchor:status".to_string();
        let message = BroadcastMessage::AnchorStatusChange {
            anchor,
            old_status,
            channel: channel.clone(),
        };

        Self::broadcast_to_subscribers(&self.ws_state, &self.subscriptions, &channel, message)
            .await;
    }

    /// Broadcast new payment to all subscribed clients
    pub async fn broadcast_payment(&self, payment: PaymentRecord) {
        let corridor = payment.get_corridor();
        let channel = format!("corridor:{}", corridor.to_string_key());
        let message = BroadcastMessage::NewPayment {
            payment,
            channel: channel.clone(),
        };

        Self::broadcast_to_subscribers(&self.ws_state, &self.subscriptions, &channel, message)
            .await;
    }

    /// Broadcast health alert to all clients
    pub async fn broadcast_health_alert(
        &self,
        corridor_id: String,
        severity: String,
        message: String,
    ) {
        let alert = BroadcastMessage::HealthAlert {
            corridor_id,
            severity,
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // Broadcast to all connections for health alerts
        self.ws_state
            .broadcast(WsMessage::from_broadcast_message(alert));
    }

    /// Subscribe a connection to specific channels
    pub fn subscribe_connection(&self, connection_id: Uuid, channels: Vec<String>) {
        let mut subscription_set = self
            .subscriptions
            .entry(connection_id)
            .or_insert_with(HashSet::new);

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

        // Find all connections subscribed to this channel
        let mut target_connections = Vec::new();
        for entry in subscriptions.iter() {
            let (connection_id, channels) = entry.pair();
            if channels.contains(channel) {
                target_connections.push(*connection_id);
            }
        }

        // Send to targeted connections
        for connection_id in target_connections {
            if let Some(sender) = ws_state.connections.get(&connection_id) {
                if let Err(e) = sender.send(ws_message.clone()).await {
                    warn!(
                        "Failed to send message to connection {}: {}",
                        connection_id, e
                    );
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

    /// Get subscription count for a channel
    pub fn channel_subscription_count(&self, channel: &str) -> usize {
        self.subscriptions
            .iter()
            .filter(|entry| entry.value().contains(channel))
            .count()
    }
}

impl WsMessage {
    /// Convert BroadcastMessage to WsMessage
    fn from_broadcast_message(broadcast_msg: BroadcastMessage) -> Self {
        match broadcast_msg {
            BroadcastMessage::CorridorUpdate { corridor, .. } => {
                WsMessage::CorridorUpdate {
                    corridor_key: corridor.corridor_key,
                    asset_a_code: corridor.asset_a_code,
                    asset_a_issuer: corridor.asset_a_issuer,
                    asset_b_code: corridor.asset_b_code,
                    asset_b_issuer: corridor.asset_b_issuer,
                    success_rate: Some(corridor.success_rate),
                    health_score: Some(corridor.success_rate * 100.0), // Simple health score calculation
                    last_updated: Some(corridor.updated_at.to_rfc3339()),
                }
            }
            BroadcastMessage::AnchorStatusChange {
                anchor,
                old_status: _,
                ..
            } => WsMessage::AnchorUpdate {
                anchor_id: "unknown".to_string(),
                name: "unknown".to_string(),
                reliability_score: anchor.reliability_score,
                status: anchor.status.as_str().to_string(),
            },
            BroadcastMessage::NewPayment { payment, .. } => {
                let corridor = payment.get_corridor();
                WsMessage::NewPayment {
                    corridor_id: corridor.to_string_key(),
                    amount: payment.amount,
                    successful: payment.successful,
                    timestamp: payment
                        .timestamp
                        .map(|value| value.to_rfc3339())
                        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                }
            }
            BroadcastMessage::HealthAlert {
                corridor_id,
                severity,
                message,
                timestamp,
            } => WsMessage::HealthAlert {
                corridor_id,
                severity,
                message,
                timestamp,
            },
            BroadcastMessage::ConnectionStatus { status } => WsMessage::ConnectionStatus { status },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_management() {
        let _ws_state = Arc::new(WsState::new());
        let _rpc_client = Arc::new(StellarRpcClient::new(
            "test".to_string(),
            "test".to_string(),
            true,
        ));
        // Note: This test would need a mock CacheManager
        // let cache = Arc::new(CacheManager::new_mock());
        // let broadcaster = RealtimeBroadcaster::new(ws_state, rpc_client, cache);

        // Test subscription logic here
    }
}
