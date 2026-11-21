use async_graphql::*;
use futures::{Stream, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::websocket::{WsMessage, WsState};
use super::types::*;

/// Subscription root for real-time GraphQL updates.
///
/// All subscriptions fan out through the shared `WsState` broadcast channel
/// so that GraphQL clients and native WebSocket clients receive the same
/// messages from a single pipeline — resolving issue #2794.
///
/// Clients can subscribe to:
/// - `corridorUpdates`: Real-time corridor metric changes
/// - `anchorUpdates`: Real-time anchor metric changes
/// - `snapshotUpdates`: New on-chain snapshot notifications
/// - `healthAlerts`: Health alert events for corridors
/// - `newPayments`: New payment events
pub struct SubscriptionRoot {
    /// Shared WebSocket state whose broadcast channel is the single source
    /// of truth for all real-time messages (native WS + GraphQL).
    pub ws_state: Arc<WsState>,
}

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to real-time corridor metric updates.
    ///
    /// Emits a `CorridorUpdateEvent` whenever a corridor's metrics are updated.
    /// Optional `corridor_key` filter to receive updates for a specific corridor.
    async fn corridor_updates(
        &self,
        _ctx: &Context<'_>,
        corridor_key: Option<String>,
    ) -> impl Stream<Item = Result<CorridorUpdateEvent, Error>> {
        let mut rx = self.ws_state.tx.subscribe();
        let filter = corridor_key;

        async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                if let WsMessage::CorridorUpdate {
                    corridor_key,
                    source_asset_code,
                    source_asset_issuer,
                    destination_asset_code,
                    destination_asset_issuer,
                    success_rate,
                    health_score,
                    last_updated,
                } = msg {
                    if let Some(ref filter_key) = filter {
                        if corridor_key != *filter_key {
                            continue;
                        }
                    }
                    yield Ok(CorridorUpdateEvent {
                        corridor_key,
                        source_asset_code,
                        source_asset_issuer,
                        destination_asset_code,
                        destination_asset_issuer,
                        success_rate,
                        health_score,
                        last_updated,
                    });
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
        _ctx: &Context<'_>,
        anchor_id: Option<String>,
    ) -> impl Stream<Item = Result<AnchorUpdateEvent, Error>> {
        let mut rx = self.ws_state.tx.subscribe();
        let filter = anchor_id;

        async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                if let WsMessage::AnchorUpdate {
                    anchor_id,
                    name,
                    reliability_score,
                    status,
                } = msg {
                    if let Some(ref filter_id) = filter {
                        if anchor_id != *filter_id {
                            continue;
                        }
                    }
                    yield Ok(AnchorUpdateEvent {
                        anchor_id,
                        name,
                        reliability_score,
                        status,
                    });
                }
            }
        }
    }

    /// Subscribe to real-time snapshot update notifications.
    ///
    /// Emits a `SnapshotUpdateEvent` whenever a new on-chain snapshot is generated.
    async fn snapshot_updates(
        &self,
        _ctx: &Context<'_>,
    ) -> impl Stream<Item = Result<SnapshotUpdateEvent, Error>> {
        let mut rx = self.ws_state.tx.subscribe();

        async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                if let WsMessage::SnapshotUpdate {
                    snapshot_id,
                    epoch,
                    timestamp,
                    hash,
                } = msg {
                    yield Ok(SnapshotUpdateEvent {
                        snapshot_id,
                        epoch,
                        timestamp,
                        hash,
                    });
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
        _ctx: &Context<'_>,
        severity: Option<String>,
    ) -> impl Stream<Item = Result<HealthAlertEvent, Error>> {
        let mut rx = self.ws_state.tx.subscribe();
        let filter_severity = severity;

        async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                if let WsMessage::HealthAlert {
                    corridor_id,
                    severity,
                    message,
                    timestamp,
                } = msg {
                    if let Some(ref filter_sev) = filter_severity {
                        if severity != *filter_sev {
                            continue;
                        }
                    }
                    yield Ok(HealthAlertEvent {
                        corridor_id,
                        severity,
                        message,
                        timestamp,
                    });
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
        _ctx: &Context<'_>,
        corridor_id: Option<String>,
    ) -> impl Stream<Item = Result<NewPaymentEvent, Error>> {
        let mut rx = self.ws_state.tx.subscribe();
        let filter_corridor = corridor_id;

        async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                if let WsMessage::NewPayment {
                    corridor_id,
                    amount,
                    successful,
                    timestamp,
                } = msg {
                    if let Some(ref filter_id) = filter_corridor {
                        if corridor_id != *filter_id {
                            continue;
                        }
                    }
                    yield Ok(NewPaymentEvent {
                        corridor_id,
                        amount,
                        successful,
                        timestamp,
                    });
                }
            }
        }
    }
}
