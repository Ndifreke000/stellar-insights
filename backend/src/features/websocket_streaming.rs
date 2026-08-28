use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::error;

/// WebSocket streaming configuration
#[derive(Debug, Clone)]
pub struct WebSocketStreamingConfig {
    pub max_subscribers: usize,
    pub buffer_size: usize,
    pub heartbeat_interval_secs: u64,
}

impl Default for WebSocketStreamingConfig {
    fn default() -> Self {
        Self {
            max_subscribers: 10_000,
            buffer_size: 128,
            heartbeat_interval_secs: 30,
        }
    }
}

/// Message types for WebSocket streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamMessage {
    #[serde(rename = "data")]
    Data {
        channel: String,
        payload: serde_json::Value,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat,
    #[serde(rename = "error")]
    Error { message: String },
}

/// WebSocket streaming service
pub struct WebSocketStreaming {
    config: WebSocketStreamingConfig,
    tx: broadcast::Sender<StreamMessage>,
}

impl WebSocketStreaming {
    /// Create a new WebSocket streaming service
    pub fn new(config: WebSocketStreamingConfig) -> Self {
        let (tx, _) = broadcast::channel(config.buffer_size);
        Self { config, tx }
    }

    /// Get a broadcast sender for publishing messages
    pub fn get_sender(&self) -> broadcast::Sender<StreamMessage> {
        self.tx.clone()
    }

    /// Subscribe to stream messages
    pub fn subscribe(&self) -> Result<broadcast::Receiver<StreamMessage>, ApiError> {
        if self.tx.receiver_count() >= self.config.max_subscribers {
            error!("Max subscribers reached: {}", self.config.max_subscribers);
            return Err(ApiError::service_unavailable(
                "WEBSOCKET_CAPACITY_REACHED",
                "WebSocket streaming at capacity",
            ));
        }
        Ok(self.tx.subscribe())
    }

    /// Publish a message to all subscribers
    pub async fn publish(&self, message: StreamMessage) -> Result<(), ApiError> {
        self.tx.send(message).map_err(|e| {
            error!("Failed to publish message: {}", e);
            ApiError::internal("WEBSOCKET_PUBLISH_FAILED", "Failed to publish message")
        })?;
        Ok(())
    }

    /// Publish data to a specific channel
    pub async fn publish_data(
        &self,
        channel: String,
        payload: serde_json::Value,
    ) -> Result<(), ApiError> {
        self.publish(StreamMessage::Data { channel, payload }).await
    }

    /// Get current subscriber count
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Get configuration
    pub fn config(&self) -> &WebSocketStreamingConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_websocket_streaming_creation() {
        let config = WebSocketStreamingConfig::default();
        let streaming = WebSocketStreaming::new(config);
        assert_eq!(streaming.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_websocket_subscribe() {
        let config = WebSocketStreamingConfig::default();
        let streaming = WebSocketStreaming::new(config);
        let _rx = streaming.subscribe().expect("Should subscribe");
        assert_eq!(streaming.subscriber_count(), 1);
    }

    #[tokio::test]
    async fn test_websocket_publish_data() {
        let config = WebSocketStreamingConfig::default();
        let streaming = WebSocketStreaming::new(config);
        let _rx = streaming.subscribe().expect("Should subscribe");

        let payload = serde_json::json!({"test": "data"});
        streaming
            .publish_data("test_channel".to_string(), payload)
            .await
            .expect("Should publish");
    }

    #[tokio::test]
    async fn test_websocket_max_subscribers() {
        let mut config = WebSocketStreamingConfig::default();
        config.max_subscribers = 2;
        let streaming = Arc::new(WebSocketStreaming::new(config));

        let _rx1 = streaming
            .subscribe()
            .expect("First subscription should work");
        let _rx2 = streaming
            .subscribe()
            .expect("Second subscription should work");
        let result = streaming.subscribe();
        assert!(result.is_err(), "Third subscription should fail");
    }

    #[tokio::test]
    async fn test_websocket_heartbeat_message() {
        let config = WebSocketStreamingConfig::default();
        let streaming = WebSocketStreaming::new(config);
        let mut rx = streaming.subscribe().expect("Should subscribe");

        streaming
            .publish(StreamMessage::Heartbeat)
            .await
            .expect("Should publish heartbeat");

        let msg = rx.recv().await.expect("Should receive message");
        match msg {
            StreamMessage::Heartbeat => (),
            _ => panic!("Expected heartbeat message"),
        }
    }
}
