use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::alerts::{Alert, AlertManager};

pub async fn alert_websocket_handler(
    ws: WebSocketUpgrade,
    State(alert_manager): State<Arc<AlertManager>>,
) -> Response {
    ws.on_upgrade(|socket| handle_alert_socket(socket, alert_manager))
}

async fn handle_alert_socket(socket: WebSocket, alert_manager: Arc<AlertManager>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = alert_manager.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(alert) = rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&alert) {
                if sender.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(_)) = receiver.next().await {}
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}
