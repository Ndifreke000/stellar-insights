use crate::models::message_queue_system::{
    Message, MessagePriority, MessageStatus, QueueConfig, QueueMetrics, QueueStats,
};
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

type MessageStore = Arc<DashMap<String, Message>>;
type QueueStore = Arc<DashMap<String, Vec<String>>>;

#[derive(Clone)]
pub struct MessageQueueSystem {
    config: QueueConfig,
    messages: MessageStore,
    queues: QueueStore,
    metrics: Arc<RwLock<QueueMetrics>>,
}

impl MessageQueueSystem {
    pub fn new(config: QueueConfig) -> Self {
        let metrics = Arc::new(RwLock::new(QueueMetrics {
            total_messages: 0,
            pending_messages: 0,
            processing_messages: 0,
            completed_messages: 0,
            failed_messages: 0,
            average_processing_time_ms: 0,
        }));

        info!("Message Queue System initialized with config: {:?}", config);

        Self {
            config,
            messages: Arc::new(DashMap::new()),
            queues: Arc::new(DashMap::new()),
            metrics,
        }
    }

    pub async fn execute(&self) -> Result<serde_json::Value> {
        debug!("Executing Message Queue System health check");
        self.health_check().await
    }

    pub async fn health_check(&self) -> Result<serde_json::Value> {
        let metrics = self.metrics.read().await;
        let response = serde_json::json!({
            "status": "healthy",
            "queue_name": self.config.name,
            "metrics": {
                "total_messages": metrics.total_messages,
                "pending_messages": metrics.pending_messages,
                "processing_messages": metrics.processing_messages,
                "completed_messages": metrics.completed_messages,
                "failed_messages": metrics.failed_messages,
            }
        });
        info!("Message Queue System health check passed");
        Ok(response)
    }

    pub async fn enqueue(
        &self,
        payload: serde_json::Value,
        priority: MessagePriority,
    ) -> Result<String> {
        let message_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let message = Message {
            id: message_id.clone(),
            queue_name: self.config.name.clone(),
            payload,
            priority,
            status: MessageStatus::Pending,
            created_at: now,
            updated_at: now,
            retry_count: 0,
            max_retries: 3,
            metadata: Default::default(),
        };

        self.messages.insert(message_id.clone(), message);

        let mut queue = self
            .queues
            .entry(self.config.name.clone())
            .or_insert_with(Vec::new);
        queue.push(message_id.clone());

        let mut metrics = self.metrics.write().await;
        metrics.total_messages += 1;
        metrics.pending_messages += 1;

        debug!("Message enqueued: {}", message_id);
        Ok(message_id)
    }

    pub async fn dequeue(&self) -> Result<Option<Message>> {
        let mut queue = match self.queues.get_mut(&self.config.name) {
            Some(q) => q,
            None => return Ok(None),
        };

        if queue.is_empty() {
            return Ok(None);
        }

        let message_id = queue.remove(0);
        drop(queue);

        if let Some((_, mut message)) = self.messages.remove(&message_id) {
            message.status = MessageStatus::Processing;
            message.updated_at = chrono::Utc::now();

            let mut metrics = self.metrics.write().await;
            metrics.pending_messages = metrics.pending_messages.saturating_sub(1);
            metrics.processing_messages += 1;

            self.messages.insert(message_id.clone(), message.clone());
            debug!("Message dequeued: {}", message_id);
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    pub async fn mark_completed(&self, message_id: &str, processing_time_ms: u64) -> Result<()> {
        if let Some((_, mut message)) = self.messages.remove(message_id) {
            message.status = MessageStatus::Completed;
            message.updated_at = chrono::Utc::now();

            let mut metrics = self.metrics.write().await;
            metrics.processing_messages = metrics.processing_messages.saturating_sub(1);
            metrics.completed_messages += 1;

            if metrics.average_processing_time_ms == 0 {
                metrics.average_processing_time_ms = processing_time_ms;
            } else {
                metrics.average_processing_time_ms =
                    (metrics.average_processing_time_ms + processing_time_ms) / 2;
            }

            self.messages.insert(message_id.to_string(), message);
            debug!("Message marked as completed: {}", message_id);
            Ok(())
        } else {
            error!("Message not found: {}", message_id);
            Err(anyhow::anyhow!("Message not found: {}", message_id))
        }
    }

    pub async fn mark_failed(&self, message_id: &str, error: &str) -> Result<()> {
        if let Some((_, mut message)) = self.messages.remove(message_id) {
            message.retry_count += 1;

            if message.retry_count >= message.max_retries {
                message.status = MessageStatus::Failed;
                let mut metrics = self.metrics.write().await;
                metrics.processing_messages = metrics.processing_messages.saturating_sub(1);
                metrics.failed_messages += 1;

                if self.config.dead_letter_queue_enabled {
                    let dlq_name = format!("{}-dlq", self.config.name);
                    let mut dlq = self.queues.entry(dlq_name).or_insert_with(Vec::new);
                    dlq.push(message_id.to_string());
                    info!("Message moved to DLQ: {}", message_id);
                }
            } else {
                message.status = MessageStatus::Retrying;
                let mut queue = self
                    .queues
                    .entry(self.config.name.clone())
                    .or_insert_with(Vec::new);
                queue.push(message_id.to_string());

                let mut metrics = self.metrics.write().await;
                metrics.processing_messages = metrics.processing_messages.saturating_sub(1);
                metrics.pending_messages += 1;
            }

            message.updated_at = chrono::Utc::now();
            message
                .metadata
                .insert("last_error".to_string(), error.to_string());

            let retry_count = message.retry_count;
            self.messages.insert(message_id.to_string(), message);
            debug!(
                "Message marked as failed: {} (retry: {})",
                message_id, retry_count
            );
            Ok(())
        } else {
            error!("Message not found: {}", message_id);
            Err(anyhow::anyhow!("Message not found: {}", message_id))
        }
    }

    pub async fn get_message(&self, message_id: &str) -> Result<Option<Message>> {
        Ok(self.messages.get(message_id).map(|m| m.clone()))
    }

    pub async fn get_queue_stats(&self) -> Result<QueueStats> {
        let metrics = self.metrics.read().await.clone();

        let oldest_message_age_secs = self
            .messages
            .iter()
            .filter(|m| m.value().queue_name == self.config.name)
            .map(|m| {
                let age = chrono::Utc::now()
                    .signed_duration_since(m.value().created_at)
                    .num_seconds();
                age as u64
            })
            .max();

        Ok(QueueStats {
            queue_name: self.config.name.clone(),
            metrics,
            oldest_message_age_secs,
        })
    }

    pub async fn purge_queue(&self) -> Result<u64> {
        let queue_name = self.config.name.clone();
        let count = if let Some((_, queue)) = self.queues.remove(&queue_name) {
            queue.len() as u64
        } else {
            0
        };

        let mut metrics = self.metrics.write().await;
        metrics.pending_messages = 0;

        info!("Queue purged: {} messages removed", count);
        Ok(count)
    }

    pub async fn get_metrics(&self) -> QueueMetrics {
        self.metrics.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_config_default() {
        let config = QueueConfig::default();
        assert_eq!(config.name, "default");
        assert_eq!(config.max_workers, 4);
        assert_eq!(config.batch_size, 10);
    }

    #[tokio::test]
    async fn test_message_queue_creation() {
        let config = QueueConfig::default();
        let mq = MessageQueueSystem::new(config);
        let metrics = mq.get_metrics().await;
        assert_eq!(metrics.total_messages, 0);
        assert_eq!(metrics.pending_messages, 0);
    }

    #[tokio::test]
    async fn test_enqueue_message() {
        let config = QueueConfig::default();
        let mq = MessageQueueSystem::new(config);

        let payload = serde_json::json!({"test": "data"});
        let result = mq.enqueue(payload, MessagePriority::Normal).await;

        assert!(result.is_ok());
        let metrics = mq.get_metrics().await;
        assert_eq!(metrics.total_messages, 1);
        assert_eq!(metrics.pending_messages, 1);
    }

    #[tokio::test]
    async fn test_dequeue_message() {
        let config = QueueConfig::default();
        let mq = MessageQueueSystem::new(config);

        let payload = serde_json::json!({"test": "data"});
        let msg_id = mq.enqueue(payload, MessagePriority::Normal).await.unwrap();

        let dequeued = mq.dequeue().await.unwrap();
        assert!(dequeued.is_some());
        let msg = dequeued.unwrap();
        assert_eq!(msg.id, msg_id);
        assert_eq!(msg.status, MessageStatus::Processing);
    }

    #[tokio::test]
    async fn test_mark_completed() {
        let config = QueueConfig::default();
        let mq = MessageQueueSystem::new(config);

        let payload = serde_json::json!({"test": "data"});
        let msg_id = mq.enqueue(payload, MessagePriority::Normal).await.unwrap();
        let _ = mq.dequeue().await;

        let result = mq.mark_completed(&msg_id, 100).await;
        assert!(result.is_ok());

        let metrics = mq.get_metrics().await;
        assert_eq!(metrics.completed_messages, 1);
        assert_eq!(metrics.processing_messages, 0);
    }

    #[tokio::test]
    async fn test_purge_queue() {
        let config = QueueConfig::default();
        let mq = MessageQueueSystem::new(config);

        let payload = serde_json::json!({"test": "data"});
        let _ = mq.enqueue(payload.clone(), MessagePriority::Normal).await;
        let _ = mq.enqueue(payload, MessagePriority::Normal).await;

        let purged = mq.purge_queue().await.unwrap();
        assert_eq!(purged, 2);

        let metrics = mq.get_metrics().await;
        assert_eq!(metrics.pending_messages, 0);
    }
}
