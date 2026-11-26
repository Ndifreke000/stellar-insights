use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, Query, State,
    },
    response::{IntoResponse, Response},
    routing::MethodRouter,
    Json,
};
use dashmap::DashMap;
use futures::{sink::SinkExt, stream::SplitSink, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Default WebSocket limits — all overridable via environment variables.
/// See `.env.example` for documentation on each variable.
const DEFAULT_MAX_CONCURRENT_CONNECTIONS: usize = 10_000;
const DEFAULT_MAX_CONNECTIONS_PER_IP: usize = 10;
const DEFAULT_MAX_CONNECT_ATTEMPTS_PER_IP: u32 = 20;
const DEFAULT_IP_RATE_LIMIT_WINDOW_SECS: u64 = 60;
const DEFAULT_MAX_PENDING_OUTGOING_MESSAGES: usize = 32;
const DEFAULT_MAX_TEXT_MESSAGE_SIZE: usize = 64 * 1024;
const DEFAULT_MAX_BINARY_MESSAGE_SIZE: usize = 64 * 1024;
const DEFAULT_WS_IDLE_TIMEOUT_SECS: u64 = 300;
const DEFAULT_MAX_MESSAGES_PER_WINDOW: u32 = 100;
const DEFAULT_MESSAGE_RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// Maximum number of messages retained per channel in the replay buffer (#2791).
/// Clients that reconnect and supply `last_seq` receive all messages they missed,
/// up to this window.  Defaults to 200; override with `WS_REPLAY_BUFFER_SIZE`.
const DEFAULT_REPLAY_BUFFER_SIZE: usize = 200;

// Keep the old names as runtime values loaded from env (used by WsState).
fn ws_max_concurrent_connections() -> usize {
    std::env::var("WS_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_CONCURRENT_CONNECTIONS)
}
fn ws_max_connections_per_ip() -> usize {
    std::env::var("WS_MAX_CONNECTIONS_PER_IP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_CONNECTIONS_PER_IP)
}
fn ws_max_connect_attempts_per_ip() -> u32 {
    std::env::var("WS_MAX_CONNECT_ATTEMPTS_PER_IP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_CONNECT_ATTEMPTS_PER_IP)
}
fn ws_ip_rate_limit_window() -> Duration {
    Duration::from_secs(
        std::env::var("WS_IP_RATE_LIMIT_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_IP_RATE_LIMIT_WINDOW_SECS),
    )
}
fn ws_max_pending_outgoing_messages() -> usize {
    std::env::var("WS_MAX_PENDING_OUTGOING_MESSAGES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_PENDING_OUTGOING_MESSAGES)
}
fn ws_max_text_message_size() -> usize {
    std::env::var("WS_MAX_TEXT_MESSAGE_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_TEXT_MESSAGE_SIZE)
}
fn ws_max_binary_message_size() -> usize {
    std::env::var("WS_MAX_BINARY_MESSAGE_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_BINARY_MESSAGE_SIZE)
}
fn ws_idle_timeout() -> Duration {
    Duration::from_secs(
        std::env::var("WS_IDLE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_WS_IDLE_TIMEOUT_SECS),
    )
}
fn ws_max_messages_per_window() -> u32 {
    std::env::var("WS_MAX_MESSAGES_PER_WINDOW")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_MESSAGES_PER_WINDOW)
}
fn ws_message_rate_limit_window() -> Duration {
    Duration::from_secs(
        std::env::var("WS_MESSAGE_RATE_LIMIT_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MESSAGE_RATE_LIMIT_WINDOW_SECS),
    )
}

fn ws_replay_buffer_size() -> usize {
    std::env::var("WS_REPLAY_BUFFER_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_REPLAY_BUFFER_SIZE)
}

// Legacy constant aliases kept for any internal code that still references them.
const MAX_CONCURRENT_CONNECTIONS: usize = DEFAULT_MAX_CONCURRENT_CONNECTIONS;
const MAX_CONNECTIONS_PER_IP: usize = DEFAULT_MAX_CONNECTIONS_PER_IP;
const MAX_CONNECT_ATTEMPTS_PER_IP: u32 = DEFAULT_MAX_CONNECT_ATTEMPTS_PER_IP;
const IP_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(DEFAULT_IP_RATE_LIMIT_WINDOW_SECS);
const MAX_PENDING_OUTGOING_MESSAGES: usize = DEFAULT_MAX_PENDING_OUTGOING_MESSAGES;
const MAX_TEXT_MESSAGE_SIZE: usize = DEFAULT_MAX_TEXT_MESSAGE_SIZE;
const MAX_BINARY_MESSAGE_SIZE: usize = DEFAULT_MAX_BINARY_MESSAGE_SIZE;
const WS_IDLE_TIMEOUT: Duration = Duration::from_secs(DEFAULT_WS_IDLE_TIMEOUT_SECS);
const MAX_MESSAGES_PER_WINDOW: u32 = DEFAULT_MAX_MESSAGES_PER_WINDOW;
const MESSAGE_RATE_LIMIT_WINDOW: Duration =
    Duration::from_secs(DEFAULT_MESSAGE_RATE_LIMIT_WINDOW_SECS);

type SharedWebSocketSender = Arc<tokio::sync::Mutex<SplitSink<WebSocket, Message>>>;

// ── Replay buffer (#2791) ──────────────────────────────────────────────────

/// A single entry in a channel's replay buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEntry {
    /// Monotonically increasing per-channel sequence number (1-based).
    pub seq: u64,
    pub message: WsMessage,
}

/// Per-channel replay buffer: retains the last `capacity` messages.
struct ReplayBuffer {
    entries: VecDeque<ReplayEntry>,
    capacity: usize,
    /// Next sequence number to assign.
    next_seq: u64,
}

impl ReplayBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
            next_seq: 1,
        }
    }

    /// Push a message, assigning it the next sequence number.
    fn push(&mut self, message: WsMessage) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(ReplayEntry { seq, message });
        seq
    }

    /// Return all entries with `seq > after_seq`, in order.
    fn since(&self, after_seq: u64) -> Vec<ReplayEntry> {
        self.entries
            .iter()
            .filter(|e| e.seq > after_seq)
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
struct MessageRateLimit {
    window_started_at: Instant,
    message_count: u32,
}

struct ConnectionPermit {
    state: Arc<WsState>,
    ip: IpAddr,
}

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        self.state.active_connections.fetch_sub(1, Ordering::AcqRel);
        self.state.release_ip_connection(self.ip);
    }
}

// ── Per-IP rate limiting ─────────────────────────────────────────────────────

struct IpRateLimit {
    /// Number of active connections from this IP.
    active_connections: usize,
    /// Number of connection attempts in the current window.
    connect_attempts: u32,
    /// Start of the current rate-limit window.
    window_start: Instant,
}

impl IpRateLimit {
    fn new() -> Self {
        Self {
            active_connections: 0,
            connect_attempts: 0,
            window_start: Instant::now(),
        }
    }
}

// ── Rate limiting ─────────────────────────────────────────────────────────────

/// Per-connection rate-limit tracking (string-keyed, for text message limiting).
struct RateLimitInfo {
    message_count: u32,
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

pub struct WsState {
    pub connections: DashMap<Uuid, tokio::sync::mpsc::Sender<WsMessage>>,
    pub subscriptions: DashMap<Uuid, HashSet<String>>,
    message_rate_limits: DashMap<Uuid, MessageRateLimit>,
    active_connections: AtomicUsize,
    pub tx: broadcast::Sender<WsMessage>,
    rate_limits: DashMap<String, RateLimitInfo>,
    ip_rate_limits: DashMap<IpAddr, IpRateLimit>,
    max_concurrent_connections: usize,
    max_connections_per_ip: usize,
    idle_timeout: Duration,
    /// Redis client for cross-instance pub/sub. `None` when `REDIS_URL` is unset.
    redis_client: Option<redis::Client>,
    /// Shared, auto-reconnecting connection used to publish (created lazily).
    redis_publisher: tokio::sync::OnceCell<redis::aio::ConnectionManager>,
    /// True while this instance is subscribed to the fan-out channel. We only
    /// publish through Redis when we'll receive our own message back;
    /// otherwise messages are delivered locally so they are never lost.
    redis_subscribed: AtomicBool,
    /// Per-channel replay buffers, keyed by channel name (#2791).
    /// `None` channel key holds the global (non-channel) broadcast buffer.
    replay_buffers: DashMap<String, tokio::sync::Mutex<ReplayBuffer>>,
    /// Per-channel sequence counters, also used as a fast path to read the
    /// current head sequence without locking the buffer.
    channel_seq: DashMap<String, AtomicU64>,
    /// Authenticated Stellar account for each connection, populated when the
    /// client supplies a valid SEP-10 token at upgrade time (#2793).
    pub authenticated_accounts: DashMap<Uuid, String>,
}

/// Redis channel used for cross-instance WebSocket message fan-out.
const REDIS_WS_CHANNEL: &str = "ws:broadcast";

/// Envelope published on [`REDIS_WS_CHANNEL`].
#[derive(Debug, Serialize, Deserialize)]
struct FanoutEnvelope {
    /// Instance that published the message (for debugging).
    origin: String,
    /// `Some` for channel-targeted messages, `None` for broadcasts to everyone.
    channel: Option<String>,
    message: WsMessage,
}

impl Default for WsState {
    fn default() -> Self {
        Self::new()
    }
}

impl WsState {
    #[must_use]
    pub fn new() -> Self {
        let max_concurrent_connections = ws_max_concurrent_connections();
        let max_connections_per_ip = ws_max_connections_per_ip();
        let idle_timeout = ws_idle_timeout();

        let (tx, _rx) = broadcast::channel(100);
        // Only fan out through Redis when explicitly configured; defaulting to
        // localhost made every broadcast vanish when no local Redis was running.
        let redis_client = std::env::var("REDIS_URL").ok().and_then(|url| {
            redis::Client::open(url.as_str())
                .map_err(|e| {
                    warn!(
                        "WsState: invalid REDIS_URL, cross-instance broadcast disabled: {}",
                        e
                    );
                })
                .ok()
        });
        Self {
            connections: DashMap::new(),
            subscriptions: DashMap::new(),
            message_rate_limits: DashMap::new(),
            active_connections: AtomicUsize::new(0),
            tx,
            rate_limits: DashMap::new(),
            ip_rate_limits: DashMap::new(),
            max_concurrent_connections,
            max_connections_per_ip,
            idle_timeout,
            redis_client,
            redis_publisher: tokio::sync::OnceCell::new(),
            redis_subscribed: AtomicBool::new(false),
            replay_buffers: DashMap::new(),
            channel_seq: DashMap::new(),
            authenticated_accounts: DashMap::new(),
        }
    }

    #[must_use]
    pub fn with_limits(
        max_concurrent_connections: usize,
        max_connections_per_ip: usize,
        idle_timeout: Duration,
    ) -> Self {
        let mut state = Self::new();
        state.max_concurrent_connections = max_concurrent_connections;
        state.max_connections_per_ip = max_connections_per_ip;
        state.idle_timeout = idle_timeout;
        state
    }

    #[must_use]
    pub fn max_connections(&self) -> usize {
        self.max_concurrent_connections
    }

    #[must_use]
    pub fn max_connections_per_ip(&self) -> usize {
        self.max_connections_per_ip
    }

    #[must_use]
    pub fn idle_timeout(&self) -> Duration {
        self.idle_timeout
    }

    /// Spawn the Redis subscriber task that relays cross-instance broadcasts to
    /// local connections. Call once after creating `Arc<WsState>`.
    pub fn spawn_redis_subscriber(self: &Arc<Self>) {
        let Some(client) = self.redis_client.clone() else {
            info!("WsState: Redis not configured, cross-instance broadcast disabled");
            return;
        };
        let state = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                match client.get_async_pubsub().await {
                    Ok(mut pubsub) => {
                        if let Err(e) = pubsub.subscribe(REDIS_WS_CHANNEL).await {
                            warn!("WsState Redis subscribe error: {}", e);
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                        info!(
                            "WsState: subscribed to Redis channel '{}'",
                            REDIS_WS_CHANNEL
                        );
                        state.redis_subscribed.store(true, Ordering::Release);
                        let mut stream = pubsub.on_message();
                        loop {
                            match stream.next().await {
                                Some(msg) => {
                                    let payload: String = match msg.get_payload() {
                                        Ok(p) => p,
                                        Err(e) => {
                                            warn!("WsState Redis payload error: {}", e);
                                            continue;
                                        }
                                    };
                                    state.deliver_fanout_payload(&payload);
                                }
                                None => {
                                    warn!("WsState Redis pub/sub stream ended, reconnecting");
                                    break;
                                }
                            }
                        }
                        state.redis_subscribed.store(false, Ordering::Release);
                    }
                    Err(e) => {
                        warn!("WsState Redis connection error: {}, retrying in 5s", e);
                    }
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    /// Check whether `client_id` is within its rate limit.
    /// Returns `true` if the message should be processed.
    pub fn check_rate_limit(&self, client_id: &str) -> bool {
        let max_messages_per_window = ws_max_messages_per_window();
        let message_rate_limit_window = ws_message_rate_limit_window();

        let mut entry = self
            .rate_limits
            .entry(client_id.to_string())
            .or_insert_with(RateLimitInfo::new);

        let now = Instant::now();
        if now.duration_since(entry.window_start) > message_rate_limit_window {
            entry.message_count = 0;
            entry.window_start = now;
        }

        if entry.message_count >= max_messages_per_window {
            return false;
        }

        entry.message_count += 1;
        true
    }

    fn cleanup_rate_limit(&self, client_id: &str) {
        self.rate_limits.remove(client_id);
    }

    /// Check per-IP connection limit and connection-attempt rate limit.
    /// Returns `Ok(())` if the connection should be allowed, `Err` with a message otherwise.
    pub fn check_ip_limits(&self, ip: IpAddr) -> Result<(), &'static str> {
        let max_connect_attempts = ws_max_connect_attempts_per_ip();
        let ip_rate_limit_window = ws_ip_rate_limit_window();

        let mut entry = self
            .ip_rate_limits
            .entry(ip)
            .or_insert_with(IpRateLimit::new);

        // Reset attempt window if expired.
        let now = Instant::now();
        if now.duration_since(entry.window_start) > ip_rate_limit_window {
            entry.connect_attempts = 0;
            entry.window_start = now;
        }

        if entry.connect_attempts >= max_connect_attempts {
            return Err("Too many connection attempts from this IP");
        }
        entry.connect_attempts += 1;

        if entry.active_connections >= self.max_connections_per_ip {
            return Err("Too many connections from this IP");
        }
        entry.active_connections += 1;

        Ok(())
    }

    fn release_ip_connection(&self, ip: IpAddr) {
        if let Some(mut entry) = self.ip_rate_limits.get_mut(&ip) {
            entry.active_connections = entry.active_connections.saturating_sub(1);
        }
    }

    /// Broadcast to every client on every instance.
    pub fn broadcast(self: &Arc<Self>, message: WsMessage) {
        self.fanout(None, message);
    }

    /// Send to clients subscribed to `channel`, on every instance.
    pub fn broadcast_to_channel(self: &Arc<Self>, channel: &str, message: WsMessage) {
        self.fanout(Some(channel.to_string()), message);
    }

    /// Broadcast only to clients connected to *this* instance (e.g. to announce
    /// that this instance is shutting down).
    pub fn broadcast_local(&self, message: WsMessage) {
        if let Err(e) = self.tx.send(message) {
            warn!("Failed to broadcast message: {}", e);
        }
    }

    /// Publish through Redis so every instance (including this one, via its
    /// subscriber) delivers to its local connections. Falls back to local
    /// delivery whenever Redis isn't available.
    fn fanout(self: &Arc<Self>, channel: Option<String>, message: WsMessage) {
        if self.redis_client.is_none() || !self.redis_subscribed.load(Ordering::Acquire) {
            self.deliver_local(channel.as_deref(), message);
            return;
        }

        let envelope = FanoutEnvelope {
            origin: crate::distributed_lock::instance_id().to_string(),
            channel,
            message,
        };
        let payload = match serde_json::to_string(&envelope) {
            Ok(p) => p,
            Err(e) => {
                warn!("WsState: failed to serialise broadcast: {}", e);
                self.deliver_local(envelope.channel.as_deref(), envelope.message);
                return;
            }
        };

        let state = Arc::clone(self);
        tokio::spawn(async move {
            let published = match state.publisher().await {
                Some(mut conn) => redis::cmd("PUBLISH")
                    .arg(REDIS_WS_CHANNEL)
                    .arg(&payload)
                    .query_async::<i64>(&mut conn)
                    .await
                    .map_err(|e| warn!("WsState broadcast: Redis publish failed: {}", e))
                    .is_ok(),
                None => false,
            };
            if !published {
                // Don't lose the message for this instance's clients.
                state.deliver_local(envelope.channel.as_deref(), envelope.message);
            }
        });
    }

    async fn publisher(&self) -> Option<redis::aio::ConnectionManager> {
        let client = self.redis_client.clone()?;
        self.redis_publisher
            .get_or_try_init(|| redis::aio::ConnectionManager::new(client))
            .await
            .map_err(|e| warn!("WsState: Redis publisher connect failed: {}", e))
            .ok()
            .cloned()
    }

    fn deliver_local(&self, channel: Option<&str>, message: WsMessage) {
        let network_changed = matches!(message, WsMessage::NetworkChanged { .. });
        match channel {
            Some(channel) => self.deliver_to_channel_local(channel, message),
            None => self.broadcast_local(message),
        }
        // Subscriptions are per-instance, so every instance drains its own.
        if network_changed {
            self.drain_all_subscriptions();
        }
    }

    /// Handle a payload received from the Redis fan-out channel.
    fn deliver_fanout_payload(&self, payload: &str) {
        if let Ok(envelope) = serde_json::from_str::<FanoutEnvelope>(payload) {
            self.deliver_local(envelope.channel.as_deref(), envelope.message);
            return;
        }
        // Instances from before the envelope format publish bare messages;
        // keep accepting them so rolling deploys don't drop updates.
        match serde_json::from_str::<WsMessage>(payload) {
            Ok(message) => self.broadcast_local(message),
            Err(e) => warn!("WsState Redis deserialize error: {}", e),
        }
    }

    fn deliver_to_channel_local(&self, channel: &str, message: WsMessage) {
        let mut target_connections = Vec::new();
        for entry in self.subscriptions.iter() {
            let (connection_id, channels) = entry.pair();
            if channels.contains(channel) {
                target_connections.push(*connection_id);
            }
        }

        // Record in replay buffer and stamp with a sequence number (#2791).
        // We use a synchronous path here: assign the next sequence number via
        // the fast-path atomic counter and push to the buffer in a spawned task
        // so we don't block the caller.
        let seq = self
            .channel_seq
            .entry(channel.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::AcqRel)
            + 1;

        let sequenced = WsMessage::Sequenced {
            channel: channel.to_string(),
            seq,
            inner: Box::new(message.clone()),
        };

        // Record in the replay buffer (async, fire-and-forget).
        {
            let capacity = ws_replay_buffer_size();
            let buf_entry = self
                .replay_buffers
                .entry(channel.to_string())
                .or_insert_with(|| tokio::sync::Mutex::new(ReplayBuffer::new(capacity)));
            // Try a non-blocking lock; if the buffer is busy we still deliver
            // the message — we just skip recording this one entry.
            if let Ok(mut buf) = buf_entry.try_lock() {
                buf.push(message);
            };
        }

        // Deliver to each subscriber. `try_send` never blocks the broadcast
        // loop. When the per-connection queue is full we disconnect the slow
        // consumer rather than dropping messages silently: the connection task
        // will see the channel closed and clean up (#2792).
        let mut slow_consumers = Vec::new();
        for connection_id in target_connections {
            if let Some(sender) = self.connections.get(&connection_id) {
                match sender.try_send(sequenced.clone()) {
                    Ok(()) => {}
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        warn!(
                            "Slow consumer: outgoing queue full for connection {}, disconnecting",
                            connection_id
                        );
                        slow_consumers.push(connection_id);
                    }
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                        // Connection is already gone; clean up below.
                        slow_consumers.push(connection_id);
                    }
                }
            }
        }
        // Drop slow/closed connections outside the iteration so we don't hold
        // any DashMap read references while mutating the same map.
        for connection_id in slow_consumers {
            self.cleanup_connection(connection_id);
        }
    }

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

    #[must_use]
    pub fn connection_count(&self) -> usize {
        self.active_connections.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn channel_subscription_count(&self, channel: &str) -> usize {
        self.subscriptions
            .iter()
            .filter(|entry| entry.value().contains(channel))
            .count()
    }

    pub fn cleanup_connection(&self, connection_id: Uuid) {
        self.connections.remove(&connection_id);
        self.subscriptions.remove(&connection_id);
        self.message_rate_limits.remove(&connection_id);
        self.cleanup_rate_limit(&connection_id.to_string());
        self.authenticated_accounts.remove(&connection_id);
    }

    fn try_acquire_connection_permit(self: &Arc<Self>, ip: IpAddr) -> Option<ConnectionPermit> {
        let max = self.max_concurrent_connections;
        let mut current = self.active_connections.load(Ordering::Acquire);
        loop {
            if current >= max {
                return None;
            }
            match self.active_connections.compare_exchange(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Some(ConnectionPermit {
                        state: Arc::clone(self),
                        ip,
                    })
                }
                Err(actual) => current = actual,
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
    }

    /// Remove all subscriptions for every connected client.
    /// Useful when the network configuration changes — stale subscriptions from the
    /// previous network must be drained so no incorrect data is pushed.
    pub fn drain_all_subscriptions(&self) {
        let connection_ids: Vec<Uuid> = self.subscriptions.iter().map(|e| *e.key()).collect();
        for connection_id in connection_ids {
            self.subscriptions.remove(&connection_id);
        }
        info!("All WebSocket subscriptions have been drained");
    }

    /// Broadcast a `NetworkChanged` message to every connected client and drain
    /// all stale subscriptions. Clients that receive `NetworkChanged` should
    /// re-subscribe to channels relevant to the new network.
    ///
    /// Fans out to every instance; each one drains its own subscriptions when
    /// it delivers the message (see `deliver_local`).
    pub fn broadcast_network_change(self: &Arc<Self>, network: &str) {
        info!("Broadcasting network change to: {}", network);
        self.broadcast(WsMessage::NetworkChanged {
            network: network.to_string(),
        });
    }

    pub fn close_all_connections(&self) {
        let connection_ids: Vec<Uuid> = self.connections.iter().map(|e| *e.key()).collect();
        for connection_id in connection_ids {
            self.cleanup_connection(connection_id);
        }
        info!("All WebSocket connections have been closed");
    }

    // ── Replay buffer (#2791) ──────────────────────────────────────────────

    /// Record `message` in the replay buffer for `channel` and return its
    /// assigned sequence number. Call this whenever a message is delivered to
    /// channel subscribers so reconnecting clients can catch up.
    ///
    /// Uses a channel key of `""` for global (non-channel) broadcasts.
    pub async fn record_for_replay(&self, channel: &str, message: WsMessage) -> u64 {
        let capacity = ws_replay_buffer_size();
        let buf_entry = self
            .replay_buffers
            .entry(channel.to_string())
            .or_insert_with(|| tokio::sync::Mutex::new(ReplayBuffer::new(capacity)));
        let mut buf = buf_entry.lock().await;
        let seq = buf.push(message);
        // Update the fast-path counter.
        self.channel_seq
            .entry(channel.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .store(seq, Ordering::Release);
        seq
    }

    /// Return all buffered messages for `channel` that were assigned a
    /// sequence number strictly greater than `after_seq`.
    ///
    /// Returns an empty `Vec` when the channel has no buffer or when the
    /// client is already up to date. Clients that reconnect pass the last
    /// sequence number they received; this returns only the delta.
    pub async fn replay_since(&self, channel: &str, after_seq: u64) -> Vec<ReplayEntry> {
        if let Some(buf_entry) = self.replay_buffers.get(channel) {
            buf_entry.lock().await.since(after_seq)
        } else {
            vec![]
        }
    }

    /// Current head sequence number for `channel` (0 if no messages yet).
    #[must_use]
    pub fn channel_head_seq(&self, channel: &str) -> u64 {
        self.channel_seq
            .get(channel)
            .map(|a| a.load(Ordering::Acquire))
            .unwrap_or(0)
    }
}

fn is_oversized_message(message: &Message) -> bool {
    match message {
        Message::Text(text) => text.len() > ws_max_text_message_size(),
        Message::Binary(data) => data.len() > ws_max_binary_message_size(),
        _ => false,
    }
}

// ── Message types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    SnapshotUpdate {
        snapshot_id: String,
        epoch: i64,
        timestamp: String,
        hash: String,
    },
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
    AnchorUpdate {
        anchor_id: String,
        name: String,
        reliability_score: f64,
        status: String,
    },
    NewPayment {
        corridor_id: String,
        amount: f64,
        successful: bool,
        timestamp: String,
    },
    HealthAlert {
        corridor_id: String,
        severity: String,
        message: String,
        timestamp: String,
    },
    Subscribe {
        channels: Vec<String>,
        /// Last sequence number the client received on each channel.
        /// On reconnect, pass the highest `seq` seen per channel so the server
        /// can replay any messages missed during the disconnect (#2791).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_seq: Option<std::collections::HashMap<String, u64>>,
    },
    Unsubscribe {
        channels: Vec<String>,
    },
    SubscriptionConfirm {
        channels: Vec<String>,
        status: String,
    },
    Ping {
        timestamp: i64,
    },
    Pong {
        timestamp: i64,
    },
    Connected {
        connection_id: String,
    },
    ConnectionStatus {
        status: String,
    },
    Error {
        message: String,
    },
    ServerShutdown {
        message: String,
    },
    /// Sent to a slow consumer before the server disconnects it. Clients that
    /// reconnect and supply `last_seq` per channel will catch up via replay
    /// (#2792).
    SlowConsumerDisconnected {
        message: String,
    },
    /// Broadcast when the server switches to a different Stellar network.
    /// Receiving clients should discard stale subscriptions and re-subscribe.
    NetworkChanged {
        network: String,
    },
    /// Wraps any channel message with its per-channel sequence number (#2791).
    /// Clients store the highest `seq` they receive per channel and pass it
    /// back in `Subscribe.last_seq` on reconnect to trigger replay.
    Sequenced {
        channel: String,
        seq: u64,
        inner: Box<WsMessage>,
    },
    /// Sent to a reconnecting client for each message it missed (#2791).
    Replayed {
        channel: String,
        seq: u64,
        inner: Box<WsMessage>,
    },
}

// ── Private channel prefix ────────────────────────────────────────────────────

/// Channels whose names begin with this prefix are private and require a
/// validated SEP-10 session token before the subscription is accepted (#2793).
/// Example private channel names: `private:alerts:GABC…`, `private:account:GABC…`
pub const PRIVATE_CHANNEL_PREFIX: &str = "private:";

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WsQueryParams {
    /// Optional SEP-10 session token. Required when subscribing to any channel
    /// whose name starts with `private:` (#2793).
    pub token: Option<String>,
}

/// `MethodRouter` for `GET /ws` (built in the library crate so Axum types stay consistent).
#[must_use]
pub fn ws_route() -> MethodRouter<Arc<WsState>> {
    axum::routing::get(ws_handler)
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// WebSocket upgrade handler.
///
/// Rejects with `503 Service Unavailable` when the server has reached
/// `MAX_CONCURRENT_CONNECTIONS`, and with `401 Unauthorized` for invalid tokens.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    State(state): State<Arc<WsState>>,
) -> Response {
    // Behind the ingress the TCP peer is the proxy; use the forwarded client IP
    // so per-IP limits apply per client rather than per ingress pod.
    let client_ip = crate::client_ip::client_ip(&headers, Some(addr)).unwrap_or_else(|| addr.ip());

    // Per-IP rate limit check — connection attempts and concurrent connections per IP.
    if let Err(reason) = state.check_ip_limits(client_ip) {
        warn!("Per-IP limit exceeded for {}: {}", client_ip, reason);
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": reason })),
        )
            .into_response();
    }

    // Global connection limit check — must happen before upgrade so we can return HTTP error.
    let Some(connection_permit) = state.try_acquire_connection_permit(client_ip) else {
        // Release the IP slot we just incremented since we won't proceed.
        state.release_ip_connection(client_ip);
        warn!(
            "Connection limit reached ({}/{}), rejecting new WebSocket connection",
            state.connection_count(),
            state.max_connections()
        );
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": format!(
                    "Server at capacity. Maximum {} concurrent connections allowed.",
                    state.max_connections()
                )
            })),
        )
            .into_response();
    };

    // Validate authentication token if provided.
    // A token is *required* to subscribe to private channels later; we validate
    // it here at upgrade time so unauthenticated clients get a clean 401 (#2793).
    let authenticated_account: Option<String> = if let Some(ref token) = params.token {
        // Prefer SEP-10 session validation when WS_SEP10_REDIS_URL / REDIS_URL
        // is configured; fall back to the legacy static WS_AUTH_TOKEN env var.
        match validate_sep10_token(token).await {
            Ok(account) => Some(account),
            Err(_) => {
                // Legacy static-token path: reject if WS_AUTH_TOKEN is set and
                // doesn't match; allow if neither mechanism is configured.
                if !validate_token(token) {
                    return (
                        axum::http::StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({"error": "Unauthorized"})),
                    )
                        .into_response();
                }
                None
            }
        }
    } else {
        None
    };

    ws.on_upgrade(move |socket| {
        handle_socket(socket, state, connection_permit, authenticated_account)
    })
}

fn validate_token(token: &str) -> bool {
    if let Ok(expected) = std::env::var("WS_AUTH_TOKEN") {
        crate::crypto::constant_time_eq(token.as_bytes(), expected.as_bytes())
    } else {
        warn!("WS_AUTH_TOKEN not configured, allowing all WebSocket connections");
        true
    }
}

/// Validate a SEP-10 session token and return the authenticated Stellar account.
///
/// Uses a lightweight Redis GET to check the session created by `Sep10Service`.
/// Returns `Err` when Redis is unreachable or the token is invalid/expired (#2793).
async fn validate_sep10_token(token: &str) -> Result<String, ()> {
    use redis::AsyncCommands;
    let redis_url = std::env::var("REDIS_URL").map_err(|_| ())?;
    let client = redis::Client::open(redis_url.as_str()).map_err(|_| ())?;
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| ())?;
    let key = format!("sep10:session:{token}");
    let session_json: Option<String> = conn.get(&key).await.map_err(|_| ())?;
    let json = session_json.ok_or(())?;
    let session: serde_json::Value = serde_json::from_str(&json).map_err(|_| ())?;
    let expires_at = session["expires_at"].as_i64().ok_or(())?;
    if chrono::Utc::now().timestamp() > expires_at {
        return Err(());
    }
    let account = session["account"].as_str().ok_or(())?.to_string();
    Ok(account)
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<WsState>,
    connection_permit: ConnectionPermit,
    authenticated_account: Option<String>,
) {
    let connection_id = Uuid::new_v4();
    let client_id = connection_id.to_string();
    info!("New WebSocket connection: {}", connection_id);

    let (sender, receiver) = socket.split();
    let sender = Arc::new(tokio::sync::Mutex::new(sender));
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(ws_max_pending_outgoing_messages());

    state.connections.insert(connection_id, tx);
    // Record authenticated account so private-channel guards can check it (#2793).
    if let Some(ref account) = authenticated_account {
        state
            .authenticated_accounts
            .insert(connection_id, account.clone());
    }
    crate::observability::metrics::set_active_connections(state.connection_count() as i64);

    let mut broadcast_rx = state.tx.subscribe();

    let _ = send_ws_message(
        &sender,
        &WsMessage::Connected {
            connection_id: client_id.clone(),
        },
    )
    .await;

    let send_sender = Arc::clone(&sender);
    let recv_sender = Arc::clone(&sender);
    let state_clone = Arc::clone(&state);

    // ── Receive task ───────────────────────────────────────────────────────────
    let idle_timeout = state.idle_timeout();
    let recv_task = {
        let client_id = client_id.clone();
        tokio::spawn(async move {
            let mut receiver = receiver;
            loop {
                let next_message = tokio::time::timeout(idle_timeout, receiver.next()).await;
                let msg = match next_message {
                    Ok(Some(Ok(msg))) => msg,
                    Ok(Some(Err(err))) => {
                        error!("WebSocket receive error for {}: {}", connection_id, err);
                        break;
                    }
                    Ok(None) => break,
                    Err(_) => {
                        warn!("WebSocket idle timeout for {}", connection_id);
                        let mut guard = recv_sender.lock().await;
                        let _ = guard.send(Message::Close(None)).await;
                        break;
                    }
                };

                if is_oversized_message(&msg) {
                    warn!(
                        "Oversized WebSocket message received from {}",
                        connection_id
                    );
                    let _ = send_ws_message(
                        &recv_sender,
                        &WsMessage::Error {
                            message: "Message size exceeds allowed limit.".to_string(),
                        },
                    )
                    .await;
                    let mut guard = recv_sender.lock().await;
                    let _ = guard.send(Message::Close(None)).await;
                    break;
                }

                if should_rate_limit_message(&msg)
                    && !state_clone.check_message_rate_limit(connection_id)
                {
                    warn!("WebSocket rate limit exceeded for {}", connection_id);
                    let _ = send_ws_message(
                        &recv_sender,
                        &WsMessage::Error {
                            message: format!(
                                "Rate limit exceeded. Maximum {} messages per {} seconds.",
                                MAX_MESSAGES_PER_WINDOW,
                                MESSAGE_RATE_LIMIT_WINDOW.as_secs()
                            ),
                        },
                    )
                    .await;
                    let mut guard = recv_sender.lock().await;
                    let _ = guard.send(Message::Close(None)).await;
                    break;
                }

                match msg {
                    Message::Text(text) => {
                        if !state_clone.check_rate_limit(&client_id) {
                            warn!("Rate limit exceeded for connection {}", connection_id);
                            if let Ok(json) = serde_json::to_string(&WsMessage::Error {
                                message: "Rate limit exceeded. Please slow down.".to_string(),
                            }) {
                                let mut guard = recv_sender.lock().await;
                                let _ = guard.send(Message::Text(json.into())).await;
                            }
                            continue;
                        }

                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match ws_msg {
                                WsMessage::Ping { timestamp } => {
                                    info!("Received ping from {}", connection_id);
                                    let _ = send_ws_message(
                                        &recv_sender,
                                        &WsMessage::Pong { timestamp },
                                    )
                                    .await;
                                }
                                WsMessage::Subscribe { channels, last_seq } => {
                                    info!(
                                        "Connection {} subscribing to: {:?}",
                                        connection_id, channels
                                    );

                                    // Private channels require a valid SEP-10 session (#2793).
                                    let is_authenticated = state_clone
                                        .authenticated_accounts
                                        .contains_key(&connection_id);
                                    let private_denied: Vec<&String> = channels
                                        .iter()
                                        .filter(|ch| {
                                            ch.starts_with(PRIVATE_CHANNEL_PREFIX)
                                                && !is_authenticated
                                        })
                                        .collect();
                                    if !private_denied.is_empty() {
                                        warn!(
                                            "Connection {} denied subscription to private channels {:?}: unauthenticated",
                                            connection_id, private_denied
                                        );
                                        let _ = send_ws_message(
                                            &recv_sender,
                                            &WsMessage::Error {
                                                message: "Authentication required for private channels. Supply a valid SEP-10 token as the `token` query parameter.".to_string(),
                                            },
                                        )
                                        .await;
                                        continue;
                                    }

                                    state_clone
                                        .subscribe_connection(connection_id, channels.clone());

                                    // Replay missed messages (#2791): for each
                                    // channel the client supplies a last_seq for,
                                    // send all buffered entries it hasn't seen.
                                    if let Some(ref seqs) = last_seq {
                                        for ch in &channels {
                                            let after = seqs.get(ch).copied().unwrap_or(0);
                                            if after > 0 {
                                                let missed = state_clone
                                                    .replay_buffers
                                                    .get(ch)
                                                    .map(|b| {
                                                        b.try_lock()
                                                            .ok()
                                                            .map(|buf| buf.since(after))
                                                            .unwrap_or_default()
                                                    })
                                                    .unwrap_or_default();
                                                for entry in missed {
                                                    let replay = WsMessage::Replayed {
                                                        channel: ch.clone(),
                                                        seq: entry.seq,
                                                        inner: Box::new(entry.message),
                                                    };
                                                    let _ = send_ws_message(&recv_sender, &replay)
                                                        .await;
                                                }
                                            }
                                        }
                                    }

                                    let _ = send_ws_message(
                                        &recv_sender,
                                        &WsMessage::SubscriptionConfirm {
                                            channels,
                                            status: "subscribed".to_string(),
                                        },
                                    )
                                    .await;
                                }
                                WsMessage::Unsubscribe { channels } => {
                                    info!(
                                        "Connection {} unsubscribing from: {:?}",
                                        connection_id, channels
                                    );
                                    state_clone
                                        .unsubscribe_connection(connection_id, channels.clone());
                                    let _ = send_ws_message(
                                        &recv_sender,
                                        &WsMessage::SubscriptionConfirm {
                                            channels,
                                            status: "unsubscribed".to_string(),
                                        },
                                    )
                                    .await;
                                }
                                _ => {
                                    warn!("Unexpected message type from client: {:?}", ws_msg);
                                }
                            }
                        } else {
                            warn!(
                                "Failed to parse WebSocket message from {}: {}",
                                connection_id, text
                            );
                            let _ = send_ws_message(
                                &recv_sender,
                                &WsMessage::Error {
                                    message:
                                        "Invalid message format. Expected JSON with a 'type' field."
                                            .to_string(),
                                },
                            )
                            .await;
                        }
                    }
                    Message::Ping(data) => {
                        let mut guard = recv_sender.lock().await;
                        let _ = guard.send(Message::Pong(data)).await;
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

    // ── Send task ──────────────────────────────────────────────────────────────
    let send_task = {
        tokio::spawn(async move {
            let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = ping_interval.tick() => {
                        let ping = WsMessage::Ping { timestamp: chrono::Utc::now().timestamp() };
                        if let Ok(json) = serde_json::to_string(&ping) {
                            let mut guard = send_sender.lock().await;
                            if guard.send(Message::Text(json.into())).await.is_err() {
                                error!("Failed to send ping to {}", connection_id);
                                break;
                            }
                        }
                    }
                    Ok(msg) = broadcast_rx.recv() => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            let mut guard = send_sender.lock().await;
                            if guard.send(Message::Text(json.into())).await.is_err() {
                                error!("Failed to send broadcast to {}", connection_id);
                                break;
                            }
                        }
                    }
                    Some(msg) = rx.recv() => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            let mut guard = send_sender.lock().await;
                            if guard.send(Message::Text(json.into())).await.is_err() {
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
        _ = recv_task => { info!("Receive task finished for {}", connection_id); }
        _ = send_task => { info!("Send task finished for {}", connection_id); }
    }

    state.cleanup_connection(connection_id);
    drop(connection_permit);
    crate::observability::metrics::set_active_connections(state.connection_count() as i64);
    info!(
        "WebSocket connection {} closed. Active: {}",
        connection_id,
        state.connection_count()
    );
}

fn should_rate_limit_message(message: &Message) -> bool {
    !matches!(message, Message::Close(_) | Message::Pong(_))
}

async fn send_ws_message(sender: &SharedWebSocketSender, message: &WsMessage) -> Result<(), ()> {
    let json = serde_json::to_string(message).map_err(|e| {
        warn!("Failed to serialize WebSocket message: {}", e);
    })?;
    let mut guard = sender.lock().await;
    guard.send(Message::Text(json.into())).await.map_err(|e| {
        warn!("Failed to send WebSocket message: {}", e);
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::collection_is_never_read,
    clippy::expect_used,
    clippy::unwrap_used
)]
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
    fn test_ip_connection_limit_enforced() {
        let state = Arc::new(WsState::new());
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        for _ in 0..MAX_CONNECTIONS_PER_IP {
            assert!(state.check_ip_limits(ip).is_ok());
        }
        // Next attempt should be blocked by active connection limit.
        assert!(state.check_ip_limits(ip).is_err());
    }

    #[test]
    fn test_ip_connection_limit_releases_on_drop() {
        let state = Arc::new(WsState::new());
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        for _ in 0..MAX_CONNECTIONS_PER_IP {
            state.check_ip_limits(ip).unwrap();
        }
        assert!(state.check_ip_limits(ip).is_err());

        state.release_ip_connection(ip);
        assert!(state.check_ip_limits(ip).is_ok());
    }

    #[test]
    fn test_ip_attempt_rate_limit_enforced() {
        let state = Arc::new(WsState::new());
        // Use a different IP so active_connections limit doesn't interfere.
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Exhaust attempt budget (active connections will also cap at MAX_CONNECTIONS_PER_IP,
        // so release after each to keep active count low).
        for i in 0..MAX_CONNECT_ATTEMPTS_PER_IP {
            if i < MAX_CONNECTIONS_PER_IP as u32 {
                assert!(
                    state.check_ip_limits(ip).is_ok(),
                    "attempt {} should pass",
                    i
                );
            } else {
                // Release one active slot so active_connections isn't the blocker.
                state.release_ip_connection(ip);
                assert!(
                    state.check_ip_limits(ip).is_ok(),
                    "attempt {} should pass",
                    i
                );
            }
        }
        // Now attempt budget is exhausted.
        state.release_ip_connection(ip);
        assert!(
            state.check_ip_limits(ip).is_err(),
            "should be blocked by attempt rate limit"
        );
    }

    #[test]
    fn test_ip_limits_independent_per_ip() {
        let state = Arc::new(WsState::new());
        let ip_a: IpAddr = "1.2.3.4".parse().unwrap();
        let ip_b: IpAddr = "5.6.7.8".parse().unwrap();

        for _ in 0..MAX_CONNECTIONS_PER_IP {
            state.check_ip_limits(ip_a).unwrap();
        }
        assert!(state.check_ip_limits(ip_a).is_err());
        assert!(
            state.check_ip_limits(ip_b).is_ok(),
            "ip_b should be unaffected"
        );
    }

    #[test]
    fn test_default_limits_match_spec() {
        let state = WsState::new();
        assert_eq!(state.max_connections(), 10_000);
        assert_eq!(state.max_connections_per_ip(), 10);
        assert_eq!(state.idle_timeout(), Duration::from_secs(300));
    }

    #[test]
    fn test_connection_limit_enforced() {
        let state = Arc::new(WsState::with_limits(10, 10, Duration::from_secs(300)));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let mut permits = Vec::with_capacity(10);

        for _ in 0..10 {
            permits.push(state.try_acquire_connection_permit(ip).unwrap());
        }

        assert_eq!(state.connection_count(), 10);
        assert!(state.try_acquire_connection_permit(ip).is_none());

        drop(permits.pop());
        assert_eq!(state.connection_count(), 9);
        assert!(state.try_acquire_connection_permit(ip).is_some());
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
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let permit = state.try_acquire_connection_permit(ip).unwrap();

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
    }

    #[test]
    fn test_websocket_rate_limit_allows_within_window() {
        let state = WsState::new();
        let client_id = "test-client-1";
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
        for _ in 0..MAX_MESSAGES_PER_WINDOW {
            state.check_rate_limit(client_id);
        }
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
        for _ in 0..MAX_MESSAGES_PER_WINDOW {
            state.check_rate_limit(client_a);
        }
        assert!(!state.check_rate_limit(client_a));
        assert!(
            state.check_rate_limit(client_b),
            "independent client should not be affected"
        );
    }

    #[test]
    fn test_websocket_connection_limit() {
        let state = Arc::new(WsState::with_limits(5, 5, Duration::from_secs(300)));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let mut permits = Vec::new();
        for _ in 0..5 {
            permits.push(state.try_acquire_connection_permit(ip).unwrap());
        }
        assert_eq!(state.connection_count(), 5);
        assert!(state.try_acquire_connection_permit(ip).is_none());
    }

    #[test]
    fn test_cleanup_removes_rate_limit_entry() {
        let state = WsState::new();
        let connection_id = Uuid::new_v4();
        let client_id = connection_id.to_string();

        state.check_rate_limit(&client_id);
        assert!(state.rate_limits.contains_key(&client_id));

        let (tx, _rx) = tokio::sync::mpsc::channel::<WsMessage>(1);
        state.connections.insert(connection_id, tx);

        state.cleanup_connection(connection_id);
        assert!(
            !state.rate_limits.contains_key(&client_id),
            "rate limit entry should be removed on cleanup"
        );
    }
}
