use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Retrying,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub queue_name: String,
    pub payload: serde_json::Value,
    pub priority: MessagePriority,
    pub status: MessageStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub name: String,
    pub max_workers: usize,
    pub batch_size: usize,
    pub visibility_timeout_secs: u64,
    pub message_retention_secs: u64,
    pub dead_letter_queue_enabled: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            max_workers: 4,
            batch_size: 10,
            visibility_timeout_secs: 300,
            message_retention_secs: 86400,
            dead_letter_queue_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMetrics {
    pub total_messages: u64,
    pub pending_messages: u64,
    pub processing_messages: u64,
    pub completed_messages: u64,
    pub failed_messages: u64,
    pub average_processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageProcessingResult {
    pub message_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub queue_name: String,
    pub metrics: QueueMetrics,
    pub oldest_message_age_secs: Option<u64>,
}
