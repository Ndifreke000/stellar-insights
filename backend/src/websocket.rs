use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::{IntoResponse, Response},
    Json,
};
use dashmap::DashMap;
use futures::{sink::SinkExt, stream::SplitSink, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

const MAX_CONCURRENT_CONNECTIONS: usize = 1_000;
const MAX_MESSAGES_PER_WINDOW: u32 = 100;
const MESSAGE_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

type SharedWebSocketSender = Arc<tokio::sync::Mutex<SplitSink<WebSocket, Message>>>;

#[derive(Debug, Clone, Copy)]
struct MessageRateLimit {
    window_started_at: Instant,
    message_count: u32,
}

struct ConnectionPermit {
    state: Arc<WsState>,
}

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        self.state.active_connections.fetch_sub(1, Ordering::AcqRel);
    }
}

/// WebSocket connection state
/// Maximum number of concurrent WebSocket connections the server will accept.
const MAX_CONNECTIONS: usize = 1000;

/// Number of messages a single connection may send per rate-limit window.
const MAX_MESSAGES_PER_WINDOW: u32 = 100;

/// Duration of the rate-limit sliding window.
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

// ── Rate limiting ─────────────────────────────────────────────────────────────

/// Per-connection rate-limit tracking.
struct RateLimitInfo {
    /// Number of messages received in the current window.
    message_count: u32,
    /// When the current window started.
    window_start: Instant,
}

impl RateLimitInfo {
    fn new() -> Self {
        Self {
            message_count: 0,
            window_start: Instant::now(),
        }
    }
}

// ── WebSocket state ───────────────────────────────────────────────────────────

/// WebSocket connection state shared across all handlers.
pub struct WsState {
    /// Map of connection ID to per-connection message sender.
    pub connections: DashMap<Uuid, tokio::sync::mpsc::Sender<WsMessage>>,
    /// Map of connection ID to subscribed channels.
    pub subscriptions: DashMap<Uuid, HashSet<String>>,
    /// Per-connection message rate limiter state
    message_rate_limits: DashMap<Uuid, MessageRateLimit>,
    /// Number of active or upgrading WebSocket connections
    active_connections: AtomicUsize,
    ///Broadcast channel for sending messages to all connections
    /// Broadcast channel for sending messages to all connections.
    pub tx: broadcast::Sender<WsMessage>,
    /// Per-connection rate-limit state keyed by connection ID string.
    rate_limits: DashMap<String, RateLimitInfo>,
}

impl Default for WsState {
    fn default() -> Self {
        Self::new()
    }
}

impl WsState {
    #[must_use]
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            connections: DashMap::new(),
            subscriptions: DashMap::new(),
            message_rate_limits: DashMap::new(),
            active_connections: AtomicUsize::new(0),
            tx,
            rate_limits: DashMap::new(),
        }
    }

    /// Check whether `client_id` is within its rate limit.
    ///
    /// Returns `true` if the message should be processed, `false` if the limit
    /// has been exceeded for the current window.
    pub fn check_rate_limit(&self, client_id: &str) -> bool {
        let mut entry = self
            .rate_limits
            .entry(client_id.to_string())
            .or_insert_with(RateLimitInfo::new);

        let now = Instant::now();

        // Reset the window if it has expired.
        if now.duration_since(entry.window_start) > RATE_LIMIT_WINDOW {
            entry.message_count = 0;
            entry.window_start = now;
        }

        if entry.message_count >= MAX_MESSAGES_PER_WINDOW {
            return false;
        }

        entry.message_count += 1;
        true
    }

    /// Remove rate-limit tracking for a connection that has disconnected.
    fn cleanup_rate_limit(&self, client_id: &str) {
        self.rate_limits.remove(client_id);
    }

    /// Broadcast a message to all connected clients.
    pub fn broadcast(&self, message: WsMessage) {
        if let Err(e) = self.tx.send(message) {
            warn!("Failed to broadcast message: {}", e);
        }
    }

    /// Broadcast a message to clients subscribed to a specific channel.
    pub async fn broadcast_to_channel(&self, channel: &str, message: WsMessage) {
        let mut target_connections = Vec::new();

        // Find connections subscribed to this channel
        for entry in self.subscriptions.iter() {
        for entry in &self.subscriptions {
            let (connection_id, channels) = entry.pair();
            if channels.contains(channel) {
                target_connections.push(*connection_id);
            }
        }

        for connection_id in target_connections {
            if let Some(sender) = self.connections.get(&connection_id) {
                if let Err(e) = sender.send(message.clone()).await {
                    warn!(
                        "Failed to send message to connection {}: {}",
                        connection_id, e
                    );
                }
            }
        }
    }

    /// Subscribe a connection to channels.
    pub fn subscribe_connection(&self, connection_id: Uuid, channels: Vec<String>) {
        let mut subscription_set = self
            .subscriptions
            .entry(connection_id)
            .or_insert_with(HashSet::new);
        let mut subscription_set = self.subscriptions.entry(connection_id).or_default();

        for channel in channels {
            subscription_set.insert(channel.clone());
            info!(
                "Connection {} subscribed to channel: {}",
                connection_id, channel
            );
        }
    }

    /// Unsubscribe a connection from channels.
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

    /// Get the number of active connections.
    #[must_use]
    pub fn connection_count(&self) -> usize {
        self.active_connections.load(Ordering::Acquire)
    }

    /// Get the number of connections subscribed to a given channel.
    #[must_use]
    pub fn channel_subscription_count(&self, channel: &str) -> usize {
        self.subscriptions
            .iter()
            .filter(|entry| entry.value().contains(channel))
            .count()
    }

    /// Remove all state associated with a disconnected connection.
    pub fn cleanup_connection(&self, connection_id: Uuid) {
        self.connections.remove(&connection_id);
        self.subscriptions.remove(&connection_id);
        self.message_rate_limits.remove(&connection_id);
    }

    fn try_acquire_connection_permit(self: &Arc<Self>) -> Option<ConnectionPermit> {
        let mut current_connections = self.active_connections.load(Ordering::Acquire);

        loop {
            if current_connections >= MAX_CONCURRENT_CONNECTIONS {
                return None;
            }

            match self.active_connections.compare_exchange(
                current_connections,
                current_connections + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Some(ConnectionPermit {
                        state: Arc::clone(self),
                    });
                }
                Err(actual_connections) => current_connections = actual_connections,
            }
        }
    }

    fn check_message_rate_limit(&self, connection_id: Uuid) -> bool {
        self.check_message_rate_limit_at(connection_id, Instant::now())
    }

    fn check_message_rate_limit_at(&self, connection_id: Uuid, now: Instant) -> bool {
        let mut rate_limit =
            self.message_rate_limits
                .entry(connection_id)
                .or_insert(MessageRateLimit {
                    window_started_at: now,
                    message_count: 0,
                });

        if now.duration_since(rate_limit.window_started_at) >= MESSAGE_RATE_LIMIT_WINDOW {
            rate_limit.window_started_at = now;
            rate_limit.message_count = 0;
        }

        if rate_limit.message_count >= MAX_MESSAGES_PER_WINDOW {
            return false;
        }

        rate_limit.message_count += 1;
        true
        self.cleanup_rate_limit(&connection_id.to_string());
    }

    /// Close all WebSocket connections gracefully.
    pub async fn close_all_connections(&self) {
        let connection_ids: Vec<Uuid> = self.connections.iter().map(|entry| *entry.key()).collect();

        for connection_id in connection_ids {
            self.cleanup_connection(connection_id);
        }

        info!("All WebSocket connections have been closed");
    }
}

// ── Message types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// New snapshot available
    SnapshotUpdate {
        snapshot_id: String,
        epoch: i64,
        timestamp: String,
        hash: String,
    },
    /// Corridor metrics updated
    CorridorUpdate {
        corridor_key: String,
        #[serde(rename = "asset_a_code")]
        source_asset_code: String,
        #[serde(rename = "asset_a_issuer")]
        source_asset_issuer: String,
        #[serde(rename = "asset_b_code")]
        destination_asset_code: String,
        #[serde(rename = "asset_b_issuer")]
        destination_asset_issuer: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        success_rate: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        health_score: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        last_updated: Option<String>,
    },
    /// Anchor metrics updated
    AnchorUpdate {
        anchor_id: String,
        name: String,
        reliability_score: f64,
        status: String,
    },
    /// New payment event
    NewPayment {
        corridor_id: String,
        amount: f64,
        successful: bool,
        timestamp: String,
    },
    /// Health alert for corridor
    HealthAlert {
        corridor_id: String,
        severity: String,
        message: String,
        timestamp: String,
    },
    /// Subscription management
    Subscribe {
        channels: Vec<String>,
    },
    Unsubscribe {
        channels: Vec<String>,
    },
    /// Subscription confirmation
    SubscriptionConfirm {
        channels: Vec<String>,
        status: String,
    },
    /// Heartbeat/Ping message
    Ping {
        timestamp: i64,
    },
    /// Pong response
    Pong {
        timestamp: i64,
    },
    /// Connection established
    Connected {
        connection_id: String,
    },
    /// Connection status update
    ConnectionStatus {
        status: String,
    },
    /// Error message
    Error {
        message: String,
    },
    /// Server is shutting down
    ServerShutdown {
        message: String,
    },
}

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WsQueryParams {
    /// Optional authentication token
    pub token: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// WebSocket upgrade handler.
///
/// Rejects the connection with `503 Service Unavailable` when the server has
/// reached `MAX_CONNECTIONS`, and with `401 Unauthorized` when an invalid
/// token is supplied.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    State(state): State<Arc<WsState>>,
) -> Response {
    // Connection limit check — must happen before the upgrade so we can still
    // return an HTTP error response.
    if state.connection_count() >= MAX_CONNECTIONS {
        warn!(
            "Connection limit reached ({}/{}), rejecting new WebSocket connection",
            state.connection_count(),
            MAX_CONNECTIONS
        );
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Server at capacity. Please try again later."
            })),
        )
            .into_response();
    }

    // Validate authentication token if provided.
    if let Some(token) = params.token {
        if !validate_token(&token) {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response();
        }
    }

    let Some(connection_permit) = state.try_acquire_connection_permit() else {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": format!(
                    "WebSocket connection limit exceeded. Maximum {} concurrent connections allowed.",
                    MAX_CONCURRENT_CONNECTIONS
                )
            })),
        )
            .into_response();
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, connection_permit))
}

/// Validate authentication token.
fn validate_token(token: &str) -> bool {
    if let Ok(expected_token) = std::env::var("WS_AUTH_TOKEN") {
        token == expected_token
    } else {
        warn!("WS_AUTH_TOKEN not configured, allowing all WebSocket connections");
        true
    }
}

/// Handle individual WebSocket connection
async fn handle_socket(
    socket: WebSocket,
    state: Arc<WsState>,
    connection_permit: ConnectionPermit,
) {
/// Handle an individual WebSocket connection.
async fn handle_socket(socket: WebSocket, state: Arc<WsState>) {
    let connection_id = Uuid::new_v4();
    let client_id = connection_id.to_string();
    info!("New WebSocket connection: {}", connection_id);

    let (sender, receiver) = socket.split();
    let sender = Arc::new(tokio::sync::Mutex::new(sender));

    // Per-connection message channel.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(32);

    // Register the connection.
    state.connections.insert(connection_id, tx);
    crate::observability::metrics::set_active_connections(state.connection_count() as i64);

    // Subscribe to the broadcast channel.
    let mut broadcast_rx = state.tx.subscribe();

    // Send connection confirmation.
    let connected_msg = WsMessage::Connected {
        connection_id: client_id.clone(),
    };
    let _ = send_ws_message(&sender, &connected_msg).await;

    let send_sender = Arc::clone(&sender);
    let recv_sender = Arc::clone(&sender);
    let state_clone = Arc::clone(&state);

    // Task for receiving messages from client
    let mut recv_task = {
        let connection_id = connection_id;
    // ── Receive task ───────────────────────────────────────────────────────────
    let recv_task = {
        let client_id = client_id.clone();
        tokio::spawn(async move {
            let mut receiver = receiver;
            while let Some(Ok(msg)) = receiver.next().await {
                if should_rate_limit_message(&msg)
                    && !state_clone.check_message_rate_limit(connection_id)
                {
                    warn!("WebSocket rate limit exceeded for {}", connection_id);
                    let error_message = WsMessage::Error {
                        message: format!(
                            "Rate limit exceeded. Maximum {} messages per {} seconds.",
                            MAX_MESSAGES_PER_WINDOW,
                            MESSAGE_RATE_LIMIT_WINDOW.as_secs()
                        ),
                    };
                    let _ = send_ws_message(&recv_sender, &error_message).await;
                    let mut sender_guard = recv_sender.lock().await;
                    let _ = sender_guard.send(Message::Close(None)).await;
                    break;
                }

                match msg {
                    Message::Text(text) => {
                        // Rate limit check — applies to every text message.
                        if !state_clone.check_rate_limit(&client_id) {
                            warn!("Rate limit exceeded for connection {}", connection_id);
                            let error_msg = WsMessage::Error {
                                message: "Rate limit exceeded. Please slow down.".to_string(),
                            };
                            if let Ok(json) = serde_json::to_string(&error_msg) {
                                let mut sender_guard = recv_sender.lock().await;
                                let _ = sender_guard.send(Message::Text(json)).await;
                            }
                            // Drop the message but keep the connection open.
                            continue;
                        }

                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match ws_msg {
                                WsMessage::Ping { timestamp } => {
                                    info!("Received ping from {}", connection_id);
                                    let pong = WsMessage::Pong { timestamp };
                                    let _ = send_ws_message(&recv_sender, &pong).await;
                                }
                                WsMessage::Subscribe { channels } => {
                                    info!(
                                        "Connection {} subscribing to channels: {:?}",
                                        connection_id, channels
                                    );
                                    state_clone
                                        .subscribe_connection(connection_id, channels.clone());
                                    let confirm = WsMessage::SubscriptionConfirm {
                                        channels: channels.clone(),
                                        status: "subscribed".to_string(),
                                    };
                                    let _ = send_ws_message(&recv_sender, &confirm).await;
                                }
                                WsMessage::Unsubscribe { channels } => {
                                    info!(
                                        "Connection {} unsubscribing from channels: {:?}",
                                        connection_id, channels
                                    );
                                    state_clone
                                        .unsubscribe_connection(connection_id, channels.clone());
                                    let confirm = WsMessage::SubscriptionConfirm {
                                        channels: channels.clone(),
                                        status: "unsubscribed".to_string(),
                                    };
                                    let _ = send_ws_message(&recv_sender, &confirm).await;
                                }
                                _ => {
                                    warn!("Unexpected message type from client: {:?}", ws_msg);
                                }
                            }
                        } else {
                            warn!("Failed to parse WebSocket message: {}", text);
                        }
                    }
                    Message::Ping(data) => {
                        info!("Received WebSocket ping from {}", connection_id);
                        let mut sender_guard = recv_sender.lock().await;
                        let _ = sender_guard.send(Message::Pong(data)).await;
                    }
                    Message::Close(_) => {
                        info!("Client {} requested close", connection_id);
                        break;
                    }
                    _ => {}
                }
            }
        })
    };

    // Task for sending messages to client
    let mut send_task = {
        let connection_id = connection_id;
    // ── Send task ──────────────────────────────────────────────────────────────
    let send_task = {
        tokio::spawn(async move {
            let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = ping_interval.tick() => {
                        let ping = WsMessage::Ping {
                            timestamp: chrono::Utc::now().timestamp(),
                        };
                        if let Ok(json) = serde_json::to_string(&ping) {
                            let mut sender_guard = send_sender.lock().await;
                            if sender_guard.send(Message::Text(json)).await.is_err() {
                                error!("Failed to send ping to {}", connection_id);
                                break;
                            }
                        }
                    }
                    Ok(msg) = broadcast_rx.recv() => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            let mut sender_guard = send_sender.lock().await;
                            if sender_guard.send(Message::Text(json)).await.is_err() {
                                error!(
                                    "Failed to send broadcast message to {}",
                                    connection_id
                                );
                                break;
                            }
                        }
                    }
                    Some(msg) = rx.recv() => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            let mut sender_guard = send_sender.lock().await;
                            if sender_guard.send(Message::Text(json)).await.is_err() {
                                error!("Failed to send message to {}", connection_id);
                                break;
                            }
                        }
                    }
                }
            }
        })
    };

    tokio::select! {
        _ = &mut recv_task => {
            info!("Receive task finished for {}", connection_id);
        }
        _ = &mut send_task => {
            info!("Send task finished for {}", connection_id);
        }
    }

    recv_task.abort();
    send_task.abort();

    // Clean up connection
    state.cleanup_connection(connection_id);
    drop(connection_permit);
    state.cleanup_connection(connection_id);
    crate::observability::metrics::set_active_connections(state.connection_count() as i64);
    info!(
        "WebSocket connection {} closed. Active connections: {}",
        connection_id,
        state.connection_count()
    );
}

fn should_rate_limit_message(message: &Message) -> bool {
    !matches!(message, Message::Close(_) | Message::Pong(_))
}

async fn send_ws_message(sender: &SharedWebSocketSender, message: &WsMessage) -> Result<(), ()> {
    let json = serde_json::to_string(message).map_err(|error| {
        warn!("Failed to serialize WebSocket message: {}", error);
    })?;

    let mut sender_guard = sender.lock().await;
    sender_guard
        .send(Message::Text(json))
        .await
        .map_err(|error| {
            warn!("Failed to send WebSocket message: {}", error);
        })
}
// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_ws_state_creation() {
        let state = WsState::new();
        assert_eq!(state.connection_count(), 0);
    }

    #[test]
    fn test_validate_token_no_env() {
        // Without WS_AUTH_TOKEN env var, should accept any token.
        assert!(validate_token("any_token"));
    }

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::SnapshotUpdate {
            snapshot_id: "test-id".to_string(),
            epoch: 1,
            timestamp: "2024-01-01".to_string(),
            hash: "abc123".to_string(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize WsMessage in test");
        assert!(json.contains("snapshot_update"));
        assert!(json.contains("test-id"));
    }

    #[test]
    fn test_connection_limit_enforced() {
        let state = Arc::new(WsState::new());
        let mut permits = Vec::with_capacity(MAX_CONCURRENT_CONNECTIONS);

        for _ in 0..MAX_CONCURRENT_CONNECTIONS {
            permits.push(state.try_acquire_connection_permit().unwrap());
        }

        assert_eq!(state.connection_count(), MAX_CONCURRENT_CONNECTIONS);
        assert!(state.try_acquire_connection_permit().is_none());

        drop(permits.pop());

        assert_eq!(state.connection_count(), MAX_CONCURRENT_CONNECTIONS - 1);
        assert!(state.try_acquire_connection_permit().is_some());
    }

    #[test]
    fn test_message_rate_limit_enforced_and_reset() {
        let state = WsState::new();
        let connection_id = Uuid::new_v4();
        let window_start = Instant::now();

        for _ in 0..MAX_MESSAGES_PER_WINDOW {
            assert!(state.check_message_rate_limit_at(connection_id, window_start));
        }

        assert!(!state.check_message_rate_limit_at(connection_id, window_start));
        assert!(state.check_message_rate_limit_at(
            connection_id,
            window_start + MESSAGE_RATE_LIMIT_WINDOW + Duration::from_millis(1),
        ));
    }

    #[test]
    fn test_cleanup_connection_removes_rate_limit_state() {
        let state = Arc::new(WsState::new());
        let connection_id = Uuid::new_v4();
        let permit = state.try_acquire_connection_permit().unwrap();

        state.check_message_rate_limit(connection_id);
        state.subscribe_connection(connection_id, vec!["corridor:test".to_string()]);

        assert!(state.message_rate_limits.contains_key(&connection_id));
        assert_eq!(state.channel_subscription_count("corridor:test"), 1);

        state.cleanup_connection(connection_id);
        drop(permit);

        assert!(!state.message_rate_limits.contains_key(&connection_id));
        assert_eq!(state.channel_subscription_count("corridor:test"), 0);
    }

    #[test]
    fn test_rate_limit_error_message_is_clear() {
        let error = WsMessage::Error {
            message: format!(
                "Rate limit exceeded. Maximum {} messages per {} seconds.",
                MAX_MESSAGES_PER_WINDOW,
                MESSAGE_RATE_LIMIT_WINDOW.as_secs()
            ),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Rate limit exceeded"));
        assert!(json.contains(&MAX_MESSAGES_PER_WINDOW.to_string()));
        assert!(json.contains(&MESSAGE_RATE_LIMIT_WINDOW.as_secs().to_string()));
    fn test_websocket_rate_limit_allows_within_window() {
        let state = WsState::new();
        let client_id = "test-client-1";

        // First MAX_MESSAGES_PER_WINDOW messages must be allowed.
        for _ in 0..MAX_MESSAGES_PER_WINDOW {
            assert!(
                state.check_rate_limit(client_id),
                "message within limit should be allowed"
            );
        }
    }

    #[test]
    fn test_websocket_rate_limit_blocks_when_exceeded() {
        let state = WsState::new();
        let client_id = "test-client-2";

        // Exhaust the window.
        for _ in 0..MAX_MESSAGES_PER_WINDOW {
            state.check_rate_limit(client_id);
        }

        // The next message must be rejected.
        assert!(
            !state.check_rate_limit(client_id),
            "message beyond limit should be blocked"
        );
    }

    #[test]
    fn test_websocket_rate_limit_independent_per_client() {
        let state = WsState::new();
        let client_a = "client-a";
        let client_b = "client-b";

        // Exhaust client A's window.
        for _ in 0..MAX_MESSAGES_PER_WINDOW {
            state.check_rate_limit(client_a);
        }
        assert!(!state.check_rate_limit(client_a));

        // Client B should still be fully allowed.
        assert!(
            state.check_rate_limit(client_b),
            "independent client should not be affected by another client's limit"
        );
    }

    #[test]
    fn test_websocket_connection_limit() {
        let state = WsState::new();

        // Simulate connections up to the limit by inserting dummy senders.
        for _ in 0..MAX_CONNECTIONS {
            let (tx, _rx) = tokio::sync::mpsc::channel::<WsMessage>(1);
            state.connections.insert(Uuid::new_v4(), tx);
        }

        assert_eq!(state.connection_count(), MAX_CONNECTIONS);
        // The handler should reject at this point — verified by checking the count.
        assert!(state.connection_count() >= MAX_CONNECTIONS);
    }

    #[test]
    fn test_cleanup_removes_rate_limit_entry() {
        let state = WsState::new();
        let connection_id = Uuid::new_v4();
        let client_id = connection_id.to_string();

        // Create a rate limit entry.
        state.check_rate_limit(&client_id);
        assert!(state.rate_limits.contains_key(&client_id));

        // Insert a dummy connection so cleanup_connection can remove it.
        let (tx, _rx) = tokio::sync::mpsc::channel::<WsMessage>(1);
        state.connections.insert(connection_id, tx);

        state.cleanup_connection(connection_id);
        assert!(
            !state.rate_limits.contains_key(&client_id),
            "rate limit entry should be removed on cleanup"
        );
    }
}
