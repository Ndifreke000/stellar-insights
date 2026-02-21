/// Webhook Dispatcher Service
/// Processes webhook events and sends them to registered webhooks with retry logic

use anyhow::Result;
use reqwest::Client;
use sqlx::SqlitePool;
use std::time::Duration;
use uuid::Uuid;

use crate::webhooks::{WebhookService, WebhookSignature, WebhookEventEnvelope};

/// Webhook dispatcher - sends events to webhooks asynchronously
pub struct WebhookDispatcher {
    db: SqlitePool,
    http_client: Client,
}

impl WebhookDispatcher {
    /// Create new webhook dispatcher
    pub fn new(db: SqlitePool) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { db, http_client }
    }

    /// Run dispatcher loop - processes pending webhook events
    pub async fn run(&self) -> Result<()> {
        tracing::info!("Starting webhook dispatcher");

        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            if let Err(e) = self.process_pending_events().await {
                tracing::error!("Error processing webhook events: {}", e);
            }
        }
    }

    /// Process all pending webhook events
    async fn process_pending_events(&self) -> Result<()> {
        let service = WebhookService::new(self.db.clone());

        // Fetch pending events (max 10 per run)
        let events = service.get_pending_events(10).await?;

        for (event_id, webhook_id, event_type, payload_str) in events {
            // Get webhook details
            let webhook = match service.get_webhook(&webhook_id).await? {
                Some(w) => w,
                None => {
                    // Webhook was deleted, mark event as failed
                    let _ = service
                        .update_event_status(&event_id, "failed", Some("webhook_deleted"), 0)
                        .await;
                    continue;
                }
            };

            if !webhook.is_active {
                let _ = service
                    .update_event_status(&event_id, "failed", Some("webhook_inactive"), 0)
                    .await;
                continue;
            }

            // Attempt delivery
            match self
                .deliver_webhook(&webhook.url, &payload_str, &webhook.secret, &event_type)
                .await
            {
                Ok(_) => {
                    // Success
                    let _ = service
                        .update_event_status(&event_id, "delivered", None, 0)
                        .await;

                    // Update webhook's last_fired_at
                    let _ = service.update_last_fired(&webhook_id).await;

                    tracing::info!(
                        "Webhook delivered successfully: webhook_id={}, event={}",
                        webhook_id,
                        event_type
                    );
                }
                Err(e) => {
                    // Determine retry count from event
                    let current_retries = self
                        .get_event_retries(&event_id)
                        .await
                        .unwrap_or(0);

                    if current_retries < 3 {
                        // Retry later
                        let _ = service
                            .update_event_status(&event_id, "pending", Some(&e.to_string()), current_retries + 1)
                            .await;

                        tracing::warn!(
                            "Webhook delivery failed (will retry): webhook_id={}, error={}, retries={}",
                            webhook_id,
                            e,
                            current_retries + 1
                        );
                    } else {
                        // Max retries exceeded
                        let _ = service
                            .update_event_status(&event_id, "failed", Some(&e.to_string()), 3)
                            .await;

                        tracing::error!(
                            "Webhook delivery failed (max retries): webhook_id={}, error={}",
                            webhook_id,
                            e
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Deliver webhook to URL
    async fn deliver_webhook(
        &self,
        url: &str,
        payload: &str,
        secret: &str,
        event_type: &str,
    ) -> Result<()> {
        let delivery_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp();

        // Create envelope
        let envelope = WebhookEventEnvelope {
            id: delivery_id.clone(),
            event: event_type.to_string(),
            timestamp,
            data: serde_json::from_str(payload)?,
        };

        let body = serde_json::to_string(&envelope)?;
        let signature = WebhookSignature::sign(&body, secret);

        tracing::debug!(
            "Sending webhook to {}: delivery_id={}, signature={}...",
            url,
            delivery_id,
            &signature[..20]
        );

        let response = self
            .http_client
            .post(url)
            .header("X-Zapier-Event", event_type)
            .header("X-Zapier-Signature", signature)
            .header("X-Zapier-Timestamp", timestamp.to_string())
            .header("X-Zapier-Delivery-ID", delivery_id)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!(
                "Webhook failed with status {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
        }
    }

    /// Get current retry count for an event
    async fn get_event_retries(&self, event_id: &str) -> Result<i32> {
        let record = sqlx::query!("SELECT retries FROM webhook_events WHERE id = ?", event_id)
            .fetch_optional(&self.db)
            .await?;

        Ok(record.map(|r| r.retries as i32).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_dispatcher_creation() {
        // This is a smoke test for basic creation
        // Full tests would require mocking the database and HTTP client
    }
}
