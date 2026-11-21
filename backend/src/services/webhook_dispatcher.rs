/// Webhook Dispatcher Service
/// Processes webhook events and sends them to registered webhooks with retry logic
use anyhow::Result;
use reqwest::Client;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::webhooks::queue::{WebhookJob, WebhookQueueManager};
use crate::webhooks::{WebhookEventEnvelope, WebhookService};
use crate::webhooks::{MAX_DISPATCH_RETRIES, WebhookSignature};

/// Webhook dispatcher - sends events to webhooks asynchronously
pub struct WebhookDispatcher {
    db: SqlitePool,
    http_client: Client,
    queue_manager: Option<WebhookQueueManager>,
}

impl WebhookDispatcher {
    /// Create new webhook dispatcher
    #[must_use]
    pub fn new(db: SqlitePool) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            db,
            http_client,
            queue_manager: None,
        }
    }

    /// Initialize the dispatcher with queue manager
    pub fn with_queue_manager(mut self, queue_manager: WebhookQueueManager) -> Self {
        self.queue_manager = Some(queue_manager);
        self
    }

    /// Start the webhook dispatcher
    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting webhook dispatcher");

        // If queue manager is configured, start it
        if let Some(ref mut qm) = self.queue_manager {
            qm.start().await;
        }

        let service = WebhookService::new(self.db.clone());

        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            // Process pending events from database (for events that need immediate dispatch)
            if let Err(e) = self.process_pending_events(&service).await {
                tracing::error!("Error processing webhook events: {}", e);
            }
        }
    }

    /// Process all pending webhook events
    async fn process_pending_events(&self, service: &WebhookService) -> Result<()> {
        // Fetch pending events (max 10 per run)
        let events = service.get_pending_events(10).await?;

        for (event_id, webhook_id, event_type, payload_str) in events {
            // Get webhook details
            let webhook = if let Some(w) = service.get_webhook(&webhook_id).await? {
                w
            } else {
                // Webhook was deleted, mark event as failed
                let _ = service
                    .update_event_status(&event_id, "failed", Some("webhook_deleted"), 0)
                    .await;
                continue;
            };

            if !webhook.is_active {
                let _ = service
                    .update_event_status(&event_id, "failed", Some("webhook_inactive"), 0)
                    .await;
                continue;
            }

            // Create job for the event
            let job = WebhookJob {
                job_id: Uuid::new_v4().to_string(),
                webhook_id: webhook_id.clone(),
                webhook,
                event_id: event_id.clone(),
                event_type,
                payload: payload_str,
                event_version: WebhookEventEnvelope::EVENT_VERSION.to_string(),
                created_at: chrono::Utc::now().timestamp(),
            };

            // Queue the job
            if let Some(ref qm) = self.queue_manager {
                qm.add_job(job).await;
            } else {
                // Fallback to direct delivery if queue manager not configured
                self.deliver_webhook_directly(service, &job).await?;
            }
        }

        Ok(())
    }

    /// Direct delivery fallback (when queue manager not configured)
    async fn deliver_webhook_directly(
        &self,
        service: &WebhookService,
        job: &WebhookJob,
    ) -> Result<()> {
        let webhook = &job.webhook;

        let delivery_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp();

        // Create envelope with version
        let envelope = WebhookEventEnvelope {
            id: delivery_id.clone(),
            event: job.event_type.clone(),
            timestamp,
            event_version: job.event_version.clone(),
            data: serde_json::from_str(&job.payload)?,
        };

        let body = serde_json::to_string(&envelope)?;

        // Sign with rotation support
        let signature = WebhookSignature::sign_with_rotation(
            &body,
            &webhook.secret,
            webhook.secondary_secret.as_deref(),
        );

        // Log delivery attempt
        let _ = service
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

        let request = self
            .http_client
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

        let _ = service
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

        // Update event status
        if resp_status.is_success() {
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
        } else {
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
            let current_retries = self.get_event_retries(&job.event_id).await.unwrap_or(0);

            if current_retries < 3 {
                // Retry later
                service
                    .update_event_status(
                        &job.event_id,
                        "pending",
                        Some(&format!(
                            "HTTP {}: {}",
                            resp_status,
                            response_body.unwrap_or_default()
                        )),
                        current_retries + 1,
                    )
                    .await?;
            } else {
                // Max retries exceeded
                service
                    .update_event_status(
                        &job.event_id,
                        "failed",
                        Some(&format!(
                            "HTTP {}: {}",
                            resp_status,
                            response_body.unwrap_or_default()
                        )),
                        3,
                    )
                    .await?;
            }
        }

        Ok(())
    }

    /// Get current retry count for an event
    async fn get_event_retries(&self, event_id: &str) -> Result<i32> {
        let retries: Option<i64> = sqlx::query_scalar("SELECT retries FROM webhook_events WHERE id = ?")
            .bind(event_id)
            .fetch_optional(&self.db)
            .await?;

        Ok(retries.unwrap_or(0) as i32)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_webhook_dispatcher_creation() {
        // This is a smoke test for basic creation
        // Full tests would require mocking the database and HTTP client
    }
}