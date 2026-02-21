/// Webhooks module for Zapier integration
/// Manages webhook registrations, event definitions, and dispatching
pub mod events;

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::SqlitePool;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Webhook signature - for verifying webhook requests
pub struct WebhookSignature;

impl WebhookSignature {
    /// Generate HMAC-SHA256 signature for webhook payload
    pub fn sign(payload: &str, secret: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());

        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    }

    /// Verify webhook signature
    pub fn verify(payload: &str, secret: &str, signature: &str) -> bool {
        let expected = Self::sign(payload, secret);
        signature == expected
    }
}

/// Webhook Configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Webhook {
    pub id: String,
    pub user_id: String,
    pub url: String,
    pub event_types: String,     // comma-separated
    pub filters: Option<String>, // JSON
    pub secret: String,
    pub is_active: bool,
    pub created_at: String,
    pub last_fired_at: Option<String>,
}

/// Webhook creation request
#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub event_types: Vec<String>, // e.g., ["corridor.health_degraded", "anchor.status_changed"]
    pub filters: Option<serde_json::Value>, // Optional filters
}

/// Webhook creation response
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub id: String,
    pub url: String,
    pub event_types: Vec<String>,
    pub filters: Option<serde_json::Value>,
    pub is_active: bool,
    pub created_at: String,
}

/// Webhook event envelope (what gets sent to webhook URL)
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookEventEnvelope {
    pub id: String, // Delivery ID for idempotency
    pub event: String,
    pub timestamp: i64,
    pub data: serde_json::Value,
}

/// Event types that can trigger webhooks
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WebhookEventType {
    CorridorHealthDegraded,
    AnchorStatusChanged,
    PaymentCreated,
    CorridorLiquidityDropped,
}

impl WebhookEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CorridorHealthDegraded => "corridor.health_degraded",
            Self::AnchorStatusChanged => "anchor.status_changed",
            Self::PaymentCreated => "payment.created",
            Self::CorridorLiquidityDropped => "corridor.liquidity_dropped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "corridor.health_degraded" => Some(Self::CorridorHealthDegraded),
            "anchor.status_changed" => Some(Self::AnchorStatusChanged),
            "payment.created" => Some(Self::PaymentCreated),
            "corridor.liquidity_dropped" => Some(Self::CorridorLiquidityDropped),
            _ => None,
        }
    }
}

/// Webhook service - manages webhook operations
pub struct WebhookService {
    db: SqlitePool,
}

impl WebhookService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Register a new webhook
    pub async fn register_webhook(
        &self,
        user_id: &str,
        request: CreateWebhookRequest,
    ) -> anyhow::Result<WebhookResponse> {
        let id = Uuid::new_v4().to_string();
        let secret = Uuid::new_v4().to_string();
        let event_types_str = request.event_types.join(",");
        let filters_str = request.filters.as_ref().map(|f| f.to_string());
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query!(
            r#"
            INSERT INTO webhooks (id, user_id, url, event_types, filters, secret, is_active, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            user_id,
            request.url,
            event_types_str,
            filters_str,
            secret,
            true,
            now
        )
        .execute(&self.db)
        .await?;

        Ok(WebhookResponse {
            id,
            url: request.url,
            event_types: request.event_types,
            filters: request.filters,
            is_active: true,
            created_at: now,
        })
    }

    /// Get webhook by ID
    pub async fn get_webhook(&self, webhook_id: &str) -> anyhow::Result<Option<Webhook>> {
        let webhook = sqlx::query_as::<_, Webhook>(
            "SELECT id, user_id, url, event_types, filters, secret, is_active, created_at, last_fired_at FROM webhooks WHERE id = ?"
        )
        .bind(webhook_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(webhook)
    }

    /// List webhooks for a user
    pub async fn list_webhooks(&self, user_id: &str) -> anyhow::Result<Vec<Webhook>> {
        let webhooks = sqlx::query_as::<_, Webhook>(
            "SELECT id, user_id, url, event_types, filters, secret, is_active, created_at, last_fired_at FROM webhooks WHERE user_id = ? AND is_active = 1 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(webhooks)
    }

    /// Delete/deactivate webhook
    pub async fn delete_webhook(&self, webhook_id: &str, user_id: &str) -> anyhow::Result<bool> {
        let result = sqlx::query!(
            "UPDATE webhooks SET is_active = 0 WHERE id = ? AND user_id = ?",
            webhook_id,
            user_id
        )
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Record webhook event for delivery
    pub async fn create_webhook_event(
        &self,
        webhook_id: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        let payload_str = payload.to_string();

        sqlx::query!(
            r#"
            INSERT INTO webhook_events (id, webhook_id, event_type, payload, status, retries, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            webhook_id,
            event_type,
            payload_str,
            "pending",
            0,
            chrono::Utc::now().to_rfc3339()
        )
        .execute(&self.db)
        .await?;

        Ok(id)
    }

    /// Get pending webhook events
    pub async fn get_pending_events(
        &self,
        limit: usize,
    ) -> anyhow::Result<Vec<(String, String, String, String)>> {
        let events = sqlx::query!(
            r#"
            SELECT we.id, we.webhook_id, we.event_type, we.payload
            FROM webhook_events we
            WHERE we.status = 'pending' AND we.retries < 3
            ORDER BY we.created_at ASC
            LIMIT ?
            "#,
            limit as i64
        )
        .fetch_all(&self.db)
        .await?;

        Ok(events
            .into_iter()
            .map(|e| (e.id, e.webhook_id, e.event_type, e.payload))
            .collect())
    }

    /// Update webhook event status
    pub async fn update_event_status(
        &self,
        event_id: &str,
        status: &str,
        error: Option<&str>,
        retries: i32,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            "UPDATE webhook_events SET status = ?, last_error = ?, retries = ? WHERE id = ?",
            status,
            error,
            retries,
            event_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Update webhook's last_fired_at timestamp
    pub async fn update_last_fired(&self, webhook_id: &str) -> anyhow::Result<()> {
        sqlx::query!(
            "UPDATE webhooks SET last_fired_at = ? WHERE id = ?",
            chrono::Utc::now().to_rfc3339(),
            webhook_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_signature() {
        let payload = r#"{"event":"test"}"#;
        let secret = "my-secret";

        let signature = WebhookSignature::sign(payload, secret);
        assert!(WebhookSignature::verify(payload, secret, &signature));
    }

    #[test]
    fn test_event_type_conversion() {
        let event = WebhookEventType::CorridorHealthDegraded;
        assert_eq!(event.as_str(), "corridor.health_degraded");
        assert_eq!(
            WebhookEventType::from_str("corridor.health_degraded"),
            Some(WebhookEventType::CorridorHealthDegraded)
        );
    }
}
