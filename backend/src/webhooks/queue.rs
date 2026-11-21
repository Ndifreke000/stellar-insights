/// Per-endpoint FIFO queue for webhook delivery with parallelism across endpoints
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::webhooks::{DeliveryLog, Webhook, WebhookService};

/// A single webhook delivery job
#[derive(Debug, Clone)]
pub struct WebhookJob {
    pub job_id: String,
    pub webhook_id: String,
    pub webhook: Webhook,
    pub event_id: String,
    pub event_type: String,
    pub payload: String,
    pub event_version: String,
    pub created_at: i64,
}

/// Queue manager for webhook jobs with per-endpoint ordering
pub struct WebhookQueueManager {
    // Per-endpoint queues
    endpoint_queues: Arc<Mutex<HashMap<String, Vec<WebhookJob>>>>,
    // Service for database operations
    service: Arc<WebhookService>,
    // Number of parallel dispatchers
    parallelism: usize,
    // Dispatcher running flag
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl WebhookQueueManager {
    /// Create a new queue manager
    pub fn new(service: WebhookService, parallelism: usize) -> Self {
        Self {
            endpoint_queues: Arc::new(Mutex::new(HashMap::new())),
            service: Arc::new(service),
            parallelism,
            running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Add a job to the per-endpoint queue
    pub async fn add_job(&self, job: WebhookJob) {
        let mut queues = self.endpoint_queues.lock().await;
        let queue = queues.entry(job.webhook_id.clone()).or_default();
        queue.push(job);
    }

    /// Get the next job for a specific endpoint (FIFO order)
    pub async fn get_next_job(&self, webhook_id: &str) -> Option<WebhookJob> {
        let mut queues = self.endpoint_queues.lock().await;
        queues
            .get_mut(webhook_id)
            .and_then(|queue| queue.pop())
    }

    /// Check if there are jobs for a specific endpoint
    pub async fn has_jobs_for_endpoint(&self, webhook_id: &str) -> bool {
        let queues = self.endpoint_queues.lock().await;
        if let Some(queue) = queues.get(webhook_id) {
            !queue.is_empty()
        } else {
            false
        }
    }

    /// Get all endpoints that have pending jobs
    pub async fn get_endpoints_with_pending_jobs(&self) -> Vec<String> {
        let queues = self.endpoint_queues.lock().await;
        queues
            .iter()
            .filter(|(_, queue)| !queue.is_empty())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get job count for a specific endpoint
    pub async fn get_job_count(&self, webhook_id: &str) -> usize {
        let queues = self.endpoint_queues.lock().await;
        queues
            .get(webhook_id)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    /// Start the queue manager - spawns parallel dispatcher tasks
    pub async fn start(&mut self) {
        if self
            .running
            .compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst)
            .is_err()
        {
            // Already running
            return;
        }

        let service = self.service.clone();
        let queues = self.endpoint_queues.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                // Get endpoints with pending jobs
                let endpoints = queues.lock().await;
                let endpoints_with_jobs: Vec<String> = endpoints
                    .iter()
                    .filter(|(_, queue)| !queue.is_empty())
                    .map(|(id, _)| id.clone())
                    .collect();

                // Process each endpoint with a pending job
                for webhook_id in endpoints_with_jobs {
                    let mut queues_mut = queues.lock().await;
                    if let Some(queue) = queues_mut.get_mut(&webhook_id) {
                        if let Some(job) = queue.pop() {
                            drop(queues_mut); // Release lock before blocking

                            // Process the job
                            if let Err(e) = process_job(&service, &job).await {
                                tracing::error!(
                                    "Failed to process webhook job: webhook_id={}, error={}",
                                    webhook_id,
                                    e
                                );
                            }
                        }
                    }
                }

                drop(endpoints);

                // Small sleep to prevent tight loop
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });
    }

    /// Stop the queue manager
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Process a single webhook job
async fn process_job(service: &Arc<WebhookService>, job: &WebhookJob) -> anyhow::Result<()> {
    // Get webhook details
    let webhook = if let Some(w) = service.get_webhook(&job.webhook_id).await? {
        w
    } else {
        // Webhook was deleted, mark event as failed
        service
            .update_event_status(&job.event_id, "failed", Some("webhook_deleted"), 0)
            .await?;
        return Ok(());
    };

    if !webhook.is_active {
        service
            .update_event_status(&job.event_id, "failed", Some("webhook_inactive"), 0)
            .await?;
        return Ok(());
    }

    // Attempt delivery
    match deliver_webhook(service, job).await {
        Ok(()) => {
            // Success
            service
                .update_event_status(&job.event_id, "delivered", None, 0)
                .await?;

            // Reset consecutive failures
            service.reset_consecutive_failures(&job.webhook_id).await?;

            // Update last_fired_at
            service.update_last_fired(&job.webhook_id).await?;

            tracing::info!(
                "Webhook delivered successfully: webhook_id={}, event={}",
                job.webhook_id,
                job.event_type
            );

            Ok(())
        }
        Err(e) => {
            // Increment consecutive failures
            let failures = service.increment_consecutive_failures(&job.webhook_id).await?;

            // Check if webhook should be auto-disabled
            if failures >= webhook.max_consecutive_failures {
                service
                    .check_and_disable_webhook(
                        &job.webhook_id,
                        &format!("max_failures_exceeded: {}", failures),
                    )
                    .await?;
            }

            // Determine retry count from event
            let current_retries = get_event_retries(service, &job.event_id).await.unwrap_or(0);

            if current_retries < 3 {
                // Retry later
                service
                    .update_event_status(
                        &job.event_id,
                        "pending",
                        Some(&e.to_string()),
                        current_retries + 1,
                    )
                    .await?;

                tracing::warn!(
                    "Webhook delivery failed (will retry): webhook_id={}, error={}, retries={}",
                    job.webhook_id,
                    e,
                    current_retries + 1
                );
            } else {
                // Max retries exceeded
                service
                    .update_event_status(&job.event_id, "failed", Some(&e.to_string()), 3)
                    .await?;

                tracing::error!(
                    "Webhook delivery failed (max retries): webhook_id={}, error={}",
                    job.webhook_id,
                    e
                );
            }

            Ok(())
        }
    }
}

/// Deliver a webhook job
async fn deliver_webhook(service: &WebhookService, job: &WebhookJob) -> anyhow::Result<()> {
    let webhook = &job.webhook;
    let delivery_id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp();

    // Create envelope with version
    let envelope = crate::webhooks::WebhookEventEnvelope {
        id: delivery_id.clone(),
        event: job.event_type.clone(),
        timestamp,
        event_version: job.event_version.clone(),
        data: serde_json::from_str(&job.payload)?,
    };

    let body = serde_json::to_string(&envelope)?;

    // Sign with rotation support
    let signature = crate::webhooks::WebhookSignature::sign_with_rotation(
        &body,
        &webhook.secret,
        webhook.secondary_secret.as_deref(),
    );

    // Log delivery attempt
    service
        .create_delivery_log(
            &job.webhook_id,
            Some(&job.event_id),
            &delivery_id,
            "pending",
            None,
            None,
            None,
            Some(signature.as_str()),
            &job.event_version,
        )
        .await;

    // Build HTTP request
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

    let request = client
        .post(&webhook.url)
        .header("X-Zapier-Event", &job.event_type)
        .header("X-Zapier-Signature", &signature)
        .header("X-Zapier-Timestamp", timestamp.to_string())
        .header("X-Zapier-Delivery-ID", &delivery_id)
        .header("X-Zapier-Event-Version", &job.event_version)
        .header("Content-Type", "application/json")
        .body(body.clone());

    let response = request.send().await?;

    // Log delivery result
    let resp_status = response.status();
    let status = if resp_status.is_success() {
        "delivered"
    } else {
        "failed"
    };

    let http_status = resp_status.as_u16().try_into().ok();
    let response_body = response.text().await.ok();

    service
        .create_delivery_log(
            &job.webhook_id,
            Some(&job.event_id),
            &delivery_id,
            status,
            http_status,
            response_body.as_deref(),
            None,
            Some(signature.as_str()),
            &job.event_version,
        )
        .await;

    if resp_status.is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Webhook failed with status {}: {}",
            resp_status,
            response_body.unwrap_or_default()
        ))
    }
}

/// Get current retry count for an event
async fn get_event_retries(service: &WebhookService, event_id: &str) -> anyhow::Result<i32> {
    let retries: Option<i64> = sqlx::query_scalar(
        "SELECT retries FROM webhook_events WHERE id = ?",
    )
    .bind(event_id)
    .fetch_optional(&service.db)
    .await?;

    Ok(retries.unwrap_or(0) as i32)
}