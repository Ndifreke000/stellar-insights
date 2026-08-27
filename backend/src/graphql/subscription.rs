use async_graphql::*;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::broadcast;

use super::types::*;

/// Subscription root for real-time GraphQL updates.
///
/// Clients can subscribe to:
/// - `corridorUpdates`: Real-time corridor metric changes
/// - `anchorUpdates`: Real-time anchor metric changes
/// - `snapshotUpdates`: New on-chain snapshot notifications
/// - `healthAlerts`: Health alert events for corridors
/// - `newPayments`: New payment events
pub struct SubscriptionRoot {
    pub broadcast_rx: broadcast::Sender<String>,
}

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to real-time corridor metric updates.
    ///
    /// Emits a `CorridorUpdateEvent` whenever a corridor's metrics are updated.
    /// Optional `corridor_key` filter to receive updates for a specific corridor.
    async fn corridor_updates(
        &self,
        ctx: &Context<'_>,
        corridor_key: Option<String>,
    ) -> impl Stream<Item = Result<CorridorUpdateEvent, Error>> {
        let mut rx = self.broadcast_rx.subscribe();
        let filter = corridor_key;

        async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                if let Ok(ws_msg) = serde_json::from_str::<serde_json::Value>(&msg) {
                    if ws_msg.get("type").and_then(|t| t.as_str()) == Some("corridor_update") {
                        if let Some(data) = ws_msg.get("data") {
                            let event = CorridorUpdateEvent {
                                corridor_key: data.get("corridor_key")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                source_asset_code: data.get("source_asset_code")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                source_asset_issuer: data.get("source_asset_issuer")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                destination_asset_code: data.get("destination_asset_code")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                destination_asset_issuer: data.get("destination_asset_issuer")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                success_rate: data.get("success_rate")
                                    .and_then(|v| v.as_f64()),
                                health_score: data.get("health_score")
                                    .and_then(|v| v.as_f64()),
                                last_updated: data.get("last_updated")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                            };

                            // Apply optional filter
                            if let Some(ref filter_key) = filter {
                                if event.corridor_key != *filter_key {
                                    continue;
                                }
                            }

                            yield Ok(event);
                        }
                    }
                }
            }
        }
    }

    /// Subscribe to real-time anchor metric updates.
    ///
    /// Emits an `AnchorUpdateEvent` whenever an anchor's metrics are updated.
    /// Optional `anchor_id` filter to receive updates for a specific anchor.
    async fn anchor_updates(
        &self,
        ctx: &Context<'_>,
        anchor_id: Option<String>,
    ) -> impl Stream<Item = Result<AnchorUpdateEvent, Error>> {
        let mut rx = self.broadcast_rx.subscribe();
        let filter = anchor_id;

        async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                if let Ok(ws_msg) = serde_json::from_str::<serde_json::Value>(&msg) {
                    if ws_msg.get("type").and_then(|t| t.as_str()) == Some("anchor_update") {
                        if let Some(data) = ws_msg.get("data") {
                            let event = AnchorUpdateEvent {
                                anchor_id: data.get("anchor_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                name: data.get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                reliability_score: data.get("reliability_score")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0),
                                status: data.get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            };

                            if let Some(ref filter_id) = filter {
                                if event.anchor_id != *filter_id {
                                    continue;
                                }
                            }

                            yield Ok(event);
                        }
                    }
                }
            }
        }
    }

    /// Subscribe to real-time snapshot update notifications.
    ///
    /// Emits a `SnapshotUpdateEvent` whenever a new on-chain snapshot is generated.
    async fn snapshot_updates(
        &self,
        ctx: &Context<'_>,
    ) -> impl Stream<Item = Result<SnapshotUpdateEvent, Error>> {
        let mut rx = self.broadcast_rx.subscribe();

        async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                if let Ok(ws_msg) = serde_json::from_str::<serde_json::Value>(&msg) {
                    if ws_msg.get("type").and_then(|t| t.as_str()) == Some("snapshot_update") {
                        if let Some(data) = ws_msg.get("data") {
                            yield Ok(SnapshotUpdateEvent {
                                snapshot_id: data.get("snapshot_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                epoch: data.get("epoch")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0),
                                timestamp: data.get("timestamp")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                hash: data.get("hash")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Subscribe to health alert events.
    ///
    /// Emits a `HealthAlertEvent` when a corridor's health degrades.
    /// Optional `severity` filter (e.g., "critical", "warning").
    async fn health_alerts(
        &self,
        ctx: &Context<'_>,
        severity: Option<String>,
    ) -> impl Stream<Item = Result<HealthAlertEvent, Error>> {
        let mut rx = self.broadcast_rx.subscribe();
        let filter_severity = severity;

        async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                if let Ok(ws_msg) = serde_json::from_str::<serde_json::Value>(&msg) {
                    if ws_msg.get("type").and_then(|t| t.as_str()) == Some("health_alert") {
                        if let Some(data) = ws_msg.get("data") {
                            let event = HealthAlertEvent {
                                corridor_id: data.get("corridor_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                severity: data.get("severity")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                message: data.get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                timestamp: data.get("timestamp")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            };

                            if let Some(ref filter_sev) = filter_severity {
                                if event.severity != *filter_sev {
                                    continue;
                                }
                            }

                            yield Ok(event);
                        }
                    }
                }
            }
        }
    }

    /// Subscribe to new payment events.
    ///
    /// Emits a `NewPaymentEvent` whenever a new payment is recorded.
    /// Optional `corridor_id` filter to receive updates for a specific corridor.
    async fn new_payments(
        &self,
        ctx: &Context<'_>,
        corridor_id: Option<String>,
    ) -> impl Stream<Item = Result<NewPaymentEvent, Error>> {
        let mut rx = self.broadcast_rx.subscribe();
        let filter_corridor = corridor_id;

        async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                if let Ok(ws_msg) = serde_json::from_str::<serde_json::Value>(&msg) {
                    if ws_msg.get("type").and_then(|t| t.as_str()) == Some("new_payment") {
                        if let Some(data) = ws_msg.get("data") {
                            let event = NewPaymentEvent {
                                corridor_id: data.get("corridor_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                amount: data.get("amount")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0),
                                successful: data.get("successful")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false),
                                timestamp: data.get("timestamp")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            };

                            if let Some(ref filter_id) = filter_corridor {
                                if event.corridor_id != *filter_id {
                                    continue;
                                }
                            }

                            yield Ok(event);
                        }
                    }
                }
            }
        }
    }
}
