use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use stellar_insights_backend::websocket::{WsMessage, WsState};
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_websocket_subscription_flow() {
    // Create WebSocket state
    let ws_state = Arc::new(WsState::new());

    // Test subscription message parsing
    let subscribe_msg = json!({
        "type": "subscribe",
        "channels": ["corridor:USDC-XLM", "anchor:GXXX"]
    });

    let parsed: Result<WsMessage, _> = serde_json::from_value(subscribe_msg);
    assert!(parsed.is_ok());

    if let Ok(WsMessage::Subscribe { channels }) = parsed {
        assert_eq!(channels.len(), 2);
        assert!(channels.contains(&"corridor:USDC-XLM".to_string()));
        assert!(channels.contains(&"anchor:GXXX".to_string()));
    } else {
        panic!("Failed to parse subscribe message");
    }
}

#[tokio::test]
async fn test_corridor_update_message_serialization() {
    let corridor_update = WsMessage::CorridorUpdate {
        corridor_key: "USDC-XLM".to_string(),
        asset_a_code: "USDC".to_string(),
        asset_a_issuer: "issuer1".to_string(),
        asset_b_code: "XLM".to_string(),
        asset_b_issuer: "native".to_string(),
        success_rate: Some(95.5),
        health_score: Some(92.0),
        last_updated: Some("2026-02-20T10:30:00Z".to_string()),
    };

    let json = serde_json::to_string(&corridor_update).unwrap();
    assert!(json.contains("corridor_update"));
    assert!(json.contains("USDC-XLM"));
    assert!(json.contains("95.5"));
}

#[tokio::test]
async fn test_health_alert_message_serialization() {
    let health_alert = WsMessage::HealthAlert {
        corridor_id: "USDC-PHP".to_string(),
        severity: "warning".to_string(),
        message: "Success rate dropped below 85%".to_string(),
        timestamp: "2026-02-20T10:30:00Z".to_string(),
    };

    let json = serde_json::to_string(&health_alert).unwrap();
    assert!(json.contains("health_alert"));
    assert!(json.contains("USDC-PHP"));
    assert!(json.contains("warning"));
    assert!(json.contains("Success rate dropped below 85%"));
}

#[tokio::test]
async fn test_ws_state_subscription_management() {
    let ws_state = WsState::new();
    let connection_id = uuid::Uuid::new_v4();

    // Test subscription
    let channels = vec!["corridor:USDC-XLM".to_string(), "anchor:GXXX".to_string()];
    ws_state.subscribe_connection(connection_id, channels.clone());

    // Verify subscription count
    assert_eq!(ws_state.channel_subscription_count("corridor:USDC-XLM"), 1);
    assert_eq!(ws_state.channel_subscription_count("anchor:GXXX"), 1);
    assert_eq!(ws_state.channel_subscription_count("nonexistent"), 0);

    // Test unsubscription
    ws_state.unsubscribe_connection(connection_id, vec!["corridor:USDC-XLM".to_string()]);
    assert_eq!(ws_state.channel_subscription_count("corridor:USDC-XLM"), 0);
    assert_eq!(ws_state.channel_subscription_count("anchor:GXXX"), 1);

    // Test cleanup
    ws_state.cleanup_connection(connection_id);
    assert_eq!(ws_state.channel_subscription_count("anchor:GXXX"), 0);
}

#[tokio::test]
async fn test_new_payment_message_serialization() {
    let payment_msg = WsMessage::NewPayment {
        corridor_id: "USDC-XLM".to_string(),
        amount: 1000.50,
        successful: true,
        timestamp: "2026-02-20T10:30:00Z".to_string(),
    };

    let json = serde_json::to_string(&payment_msg).unwrap();
    assert!(json.contains("new_payment"));
    assert!(json.contains("USDC-XLM"));
    assert!(json.contains("1000.5"));
    assert!(json.contains("true"));
}
