use crate::models::corridor::Corridor;
use crate::models::{Anchor, SnapshotRecord};
use crate::websocket::{WsMessage, WsState};
use std::sync::Arc;
use tracing::info;

/// Broadcast a new snapshot to all WebSocket clients
pub fn broadcast_snapshot_update(ws_state: &Arc<WsState>, snapshot: &SnapshotRecord) {
    let message = WsMessage::SnapshotUpdate {
        snapshot_id: snapshot.id.clone(),
        epoch: snapshot.epoch.unwrap_or(0),
        timestamp: snapshot.timestamp.to_rfc3339(),
        hash: snapshot.hash.clone().unwrap_or_default(),
    };

    info!("Broadcasting snapshot update: {}", snapshot.id);
    ws_state.broadcast(message);
}

/// Broadcast corridor metrics update to all WebSocket clients
pub fn broadcast_corridor_update(ws_state: &Arc<WsState>, corridor: &Corridor) {
    let message = WsMessage::CorridorUpdate {
        corridor_key: corridor.to_string_key(),
        asset_a_code: corridor.asset_a_code.clone(),
        asset_a_issuer: corridor.asset_a_issuer.clone(),
        asset_b_code: corridor.asset_b_code.clone(),
        asset_b_issuer: corridor.asset_b_issuer.clone(),
    };

    info!("Broadcasting corridor update: {}", corridor.to_string_key());
    ws_state.broadcast(message);
}

/// Broadcast anchor metrics update to all WebSocket clients
pub fn broadcast_anchor_update(ws_state: &Arc<WsState>, anchor: &Anchor) {
    let message = WsMessage::AnchorUpdate {
        anchor_id: anchor.id.clone(),
        name: anchor.name.clone(),
        reliability_score: anchor.reliability_score,
        status: anchor.status.clone(),
    };

    info!("Broadcasting anchor update: {}", anchor.name);
    ws_state.broadcast(message);
}
