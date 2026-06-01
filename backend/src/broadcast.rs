use crate::models::corridor::Corridor;
use crate::models::Anchor;
use crate::websocket::{WsMessage, WsState};
use std::sync::Arc;

/// Broadcast an anchor update to all WebSocket clients
pub fn broadcast_anchor_update(ws_state: &Arc<WsState>, anchor: &Anchor) {
    let message = WsMessage::AnchorUpdate {
        anchor_id: anchor.id.clone(),
        name: anchor.name.clone(),
        reliability_score: anchor.reliability_score,
        status: anchor.status.clone(),
    };
    ws_state.broadcast(message);
}

/// Broadcast a corridor update to all WebSocket clients
pub fn broadcast_corridor_update(ws_state: &Arc<WsState>, corridor: &Corridor) {
    let message = WsMessage::CorridorUpdate {
        corridor_key: corridor.to_string_key(),
        asset_a_code: corridor.asset_a_code.clone(),
        asset_a_issuer: corridor.asset_a_issuer.clone(),
        asset_b_code: corridor.asset_b_code.clone(),
        asset_b_issuer: corridor.asset_b_issuer.clone(),
        success_rate: None,
        health_score: None,
        last_updated: None,
    };
    ws_state.broadcast(message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_broadcast_anchor_update() {
        let ws_state = Arc::new(WsState::new());
        let anchor = Anchor {
            id: "test-id".to_string(),
            name: "Test Anchor".to_string(),
            stellar_account: "GA123".to_string(),
            home_domain: None,
            total_transactions: 100,
            successful_transactions: 95,
            failed_transactions: 5,
            total_volume_usd: 1000.0,
            avg_settlement_time_ms: 500,
            reliability_score: 95.0,
            status: "active".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Should not panic
        broadcast_anchor_update(&ws_state, &anchor);
    }

    #[test]
    fn test_broadcast_corridor_update() {
        let ws_state = Arc::new(WsState::new());
        let corridor = Corridor::new(
            "USD".to_string(),
            "GA123".to_string(),
            "EUR".to_string(),
            "GA456".to_string(),
        );

        // Should not panic
        broadcast_corridor_update(&ws_state, &corridor);
    }
}
