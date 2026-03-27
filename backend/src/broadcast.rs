use crate::models::corridor::Corridor;
use crate::models::Anchor;
use crate::websocket::{WsMessage, WsState};
use async_trait::async_trait;
use std::future::Future;
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
        source_asset_code: corridor.source_asset_code.clone(),
        source_asset_issuer: corridor.source_asset_issuer.clone(),
        destination_asset_code: corridor.destination_asset_code.clone(),
        destination_asset_issuer: corridor.destination_asset_issuer.clone(),
        success_rate: None,
        health_score: None,
        last_updated: None,
    };
    ws_state.broadcast(message);
}

/// Unified notification payload for external channels.
#[derive(Debug, Clone)]
pub struct Message {
    pub subject: String,
    pub body: String,
    pub metadata: Option<serde_json::Value>,
}

impl Message {
    #[must_use]
    pub const fn new(subject: String, body: String, metadata: Option<serde_json::Value>) -> Self {
        Self {
            subject,
            body,
            metadata,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelDeliveryError {
    pub channel: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct NotificationBatchError {
    pub failures: Vec<ChannelDeliveryError>,
}

impl std::fmt::Display for NotificationBatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "notification delivery failed for {} channel(s)",
            self.failures.len()
        )
    }
}

impl std::error::Error for NotificationBatchError {}

/// Unified interface for notification channels.
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    fn name(&self) -> &'static str;
    async fn send(&self, message: Message) -> anyhow::Result<()>;
}

pub struct NotificationService {
    channels: Vec<Arc<dyn NotificationChannel>>,
}

impl NotificationService {
    #[must_use]
    pub const fn new(channels: Vec<Arc<dyn NotificationChannel>>) -> Self {
        Self { channels }
    }

    pub fn register_channel(&mut self, channel: Arc<dyn NotificationChannel>) {
        self.channels.push(channel);
    }

    pub async fn notify_all(&self, message: Message) -> anyhow::Result<()> {
        let mut failures = Vec::new();

        for channel in &self.channels {
            if let Err(err) = channel.send(message.clone()).await {
                failures.push(ChannelDeliveryError {
                    channel: channel.name().to_string(),
                    reason: err.to_string(),
                });
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(anyhow::Error::new(NotificationBatchError { failures }))
        }
    }

    pub async fn notify_with<F, Fut>(&self, mut make_message: F) -> anyhow::Result<()>
    where
        F: FnMut(&dyn NotificationChannel) -> Fut,
        Fut: Future<Output = anyhow::Result<Message>>,
    {
        let mut failures = Vec::new();

        for channel in &self.channels {
            match make_message(channel.as_ref()).await {
                Ok(message) => {
                    if let Err(err) = channel.send(message).await {
                        failures.push(ChannelDeliveryError {
                            channel: channel.name().to_string(),
                            reason: err.to_string(),
                        });
                    }
                }
                Err(err) => failures.push(ChannelDeliveryError {
                    channel: channel.name().to_string(),
                    reason: err.to_string(),
                }),
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(anyhow::Error::new(NotificationBatchError { failures }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Mutex;

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

    struct MockChannel {
        name: &'static str,
        sent: Arc<AtomicUsize>,
        fail: bool,
        last_subject: Arc<Mutex<Option<String>>>,
    }

    #[async_trait]
    impl NotificationChannel for MockChannel {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn send(&self, message: Message) -> anyhow::Result<()> {
            self.sent.fetch_add(1, Ordering::SeqCst);
            *self.last_subject.lock().await = Some(message.subject);
            if self.fail {
                anyhow::bail!("forced failure");
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn notification_service_sends_to_all_channels() {
        let sent1 = Arc::new(AtomicUsize::new(0));
        let sent2 = Arc::new(AtomicUsize::new(0));
        let subject1 = Arc::new(Mutex::new(None));
        let subject2 = Arc::new(Mutex::new(None));
        let c1 = Arc::new(MockChannel {
            name: "telegram",
            sent: Arc::clone(&sent1),
            fail: false,
            last_subject: Arc::clone(&subject1),
        });
        let c2 = Arc::new(MockChannel {
            name: "email",
            sent: Arc::clone(&sent2),
            fail: false,
            last_subject: Arc::clone(&subject2),
        });

        let service = NotificationService::new(vec![c1, c2]);
        let result = service
            .notify_all(Message::new(
                "Health Alert".to_string(),
                "Corridor degraded".to_string(),
                None,
            ))
            .await;

        assert!(result.is_ok());
        assert_eq!(sent1.load(Ordering::SeqCst), 1);
        assert_eq!(sent2.load(Ordering::SeqCst), 1);
        assert_eq!(subject1.lock().await.as_deref(), Some("Health Alert"));
        assert_eq!(subject2.lock().await.as_deref(), Some("Health Alert"));
    }

    #[tokio::test]
    async fn notification_service_returns_batch_error() {
        let ok = Arc::new(MockChannel {
            name: "ok_channel",
            sent: Arc::new(AtomicUsize::new(0)),
            fail: false,
            last_subject: Arc::new(Mutex::new(None)),
        });
        let failing = Arc::new(MockChannel {
            name: "failing_channel",
            sent: Arc::new(AtomicUsize::new(0)),
            fail: true,
            last_subject: Arc::new(Mutex::new(None)),
        });

        let service = NotificationService::new(vec![ok, failing]);
        let err = service
            .notify_all(Message::new(
                "Subject".to_string(),
                "Body".to_string(),
                Some(serde_json::json!({"severity":"high"})),
            ))
            .await
            .unwrap_err();

        assert!(err.to_string().contains("notification delivery failed"));
    }
}
