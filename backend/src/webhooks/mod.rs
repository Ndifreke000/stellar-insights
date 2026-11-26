/// Webhooks module for Zapier integration
/// Manages webhook registrations, event definitions, and dispatching
pub mod channel;
pub mod events;
pub mod queue;

use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub use channel::{WebhookChannel, WebhookEndpoint};

/// Webhook signature - for verifying webhook requests with versioning and rotatable secrets
pub struct WebhookSignature;

impl WebhookSignature {
    /// Current event version
    pub const EVENT_VERSION: &'static str = "v1";

    /// Generate HMAC-SHA256 signature for webhook payload
    #[must_use]
    pub fn sign(payload: &str, secret: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        Mac::update(&mut mac, payload.as_bytes());
        format!("sha256={}", hex::encode(Mac::finalize(mac).into_bytes()))
    }

    /// Generate HMAC-SHA256 signature for webhook payload with version prefix
    #[must_use]
    pub fn sign_with_version(payload: &str, secret: &str, version: &str) -> String {
        let signed_payload = format!("{}.{}", version, payload);
        Self::sign(&signed_payload, secret)
    }

    /// Signature header for a delivery while secret rotation may be active.
    ///
    /// Always the standard `sha256=<hex>` signature over the primary secret.
    /// This previously switched to `primary=sha256=<hex>` as soon as a
    /// secondary secret existed (and computed, then discarded, the
    /// secondary signature), so every receiver's verification -- which
    /// compares against `sha256=<hex>` -- started failing the moment
    /// rotation kicked in. The secondary secret is never disclosed to the
    /// webhook owner, so no receiver could verify against it anyway.
    #[must_use]
    pub fn sign_with_rotation(
        payload: &str,
        primary_secret: &str,
        _secondary_secret: Option<&str>,
    ) -> String {
        Self::sign(payload, primary_secret)
    }

    /// Verify webhook signature using primary secret
    #[must_use]
    pub fn verify(payload: &str, secret: &str, signature: &str) -> bool {
        let expected = Self::sign(payload, secret);
        crate::crypto::constant_time_eq(signature.as_bytes(), expected.as_bytes())
    }

    /// Verify webhook signature using primary and secondary secrets (for rotation overlap period)
    #[must_use]
    pub fn verify_with_rotation(
        payload: &str,
        primary_secret: &str,
        secondary_secret: Option<&str>,
        signature: &str,
    ) -> bool {
        // Try primary secret first
        if Self::verify(payload, primary_secret, signature) {
            return true;
        }

        // Try secondary secret during rotation period
        if let Some(sec) = secondary_secret {
            if Self::verify(payload, sec, signature) {
                return true;
            }
        }

        false
    }

    /// Verify webhook signature with version prefix
    #[must_use]
    pub fn verify_with_version(
        payload: &str,
        secret: &str,
        version: &str,
        signature: &str,
    ) -> bool {
        let signed_payload = format!("{}.{}", version, payload);
        Self::verify(&signed_payload, secret, signature)
    }
}

/// Maximum number of dispatch attempts before an event is marked failed.
pub const MAX_DISPATCH_RETRIES: u32 = 3;

/// Base delay (in milliseconds) used for exponential backoff between retries.
pub const DISPATCH_RETRY_BASE_DELAY_MS: u64 = 500;

/// Compute the exponential backoff delay for a given retry attempt (0-indexed).
#[must_use]
pub const fn dispatch_retry_delay_ms(attempt: u32) -> u64 {
    let shift = if attempt > 10 { 10 } else { attempt };
    DISPATCH_RETRY_BASE_DELAY_MS.saturating_mul(1u64 << shift)
}

/// Shared, observable state for the background webhook dispatcher.
///
/// The dispatcher task is spawned unsupervised, so this handle lets the rest
/// of the application observe whether it is alive and how many dispatch
/// attempts have failed, instead of failing silently.
#[derive(Debug, Default)]
pub struct WebhookDispatcherHealth {
    running: AtomicBool,
    consecutive_failures: AtomicU64,
    total_failures: AtomicU64,
}

impl WebhookDispatcherHealth {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the dispatcher as running (called when the task starts).
    pub fn mark_running(&self) {
        self.running.store(true, Ordering::SeqCst);
    }

    /// Mark the dispatcher as stopped (called when the task exits).
    pub fn mark_stopped(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Record a successful dispatch, resetting the consecutive failure count.
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
    }

    /// Record a failed dispatch attempt.
    pub fn record_failure(&self) {
        self.consecutive_failures.fetch_add(1, Ordering::SeqCst);
        self.total_failures.fetch_add(1, Ordering::SeqCst);
    }

    /// Whether the dispatcher task is currently running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Number of consecutive failed dispatch attempts.
    #[must_use]
    pub fn consecutive_failures(&self) -> u64 {
        self.consecutive_failures.load(Ordering::SeqCst)
    }

    /// Total number of failed dispatch attempts since startup.
    #[must_use]
    pub fn total_failures(&self) -> u64 {
        self.total_failures.load(Ordering::SeqCst)
    }

    /// Health check: the dispatcher is healthy when it is running and has not
    /// accumulated an excessive number of consecutive failures.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.is_running() && self.consecutive_failures() < u64::from(MAX_DISPATCH_RETRIES)
    }
}

/// Spawn the webhook dispatcher as a supervised background task.
///
/// The task is wrapped so that panics are caught and logged instead of
/// silently killing the dispatcher, and its liveness/failure state is exposed
/// through the returned [`WebhookDispatcherHealth`] handle for health checks.
pub fn spawn_webhook_dispatcher<F, Fut>(
    health: Arc<WebhookDispatcherHealth>,
    mut dispatch: F,
) -> tokio::task::JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = anyhow::Result<()>> + Send,
{
    health.mark_running();
    let task_health = Arc::clone(&health);

    tokio::spawn(async move {
        let result = std::panic::AssertUnwindSafe(async {
            loop {
                match dispatch().await {
                    Ok(()) => task_health.record_success(),
                    Err(err) => {
                        task_health.record_failure();
                        tracing::error!(
                            error = %err,
                            consecutive_failures = task_health.consecutive_failures(),
                            total_failures = task_health.total_failures(),
                            "webhook dispatcher iteration failed"
                        );
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        if let Err(panic) = futures::FutureExt::catch_unwind(result).await {
            let msg = panic
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            tracing::error!(panic = %msg, "webhook dispatcher task panicked; stopping");
        }

        task_health.mark_stopped();
    })
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
    // Rotation fields
    pub secret_rotation_enabled: bool,
    pub secondary_secret: Option<String>,
    pub secret_rotation_period_days: i32,
    pub next_rotation_date: Option<String>,
    // Auto-disable fields
    pub disabled_reason: Option<String>,
    pub disabled_at: Option<String>,
    pub enabled_at: Option<String>,
    pub last_success_at: Option<String>,
    pub consecutive_failures: i32,
    pub max_consecutive_failures: i32,
}

/// Webhook creation request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub event_types: Vec<String>,
    pub filters: Option<serde_json::Value>,
    pub event_version: Option<String>,
    pub secret_rotation_enabled: Option<bool>,
    pub secret_rotation_period_days: Option<i32>,
}

/// Webhook creation response
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WebhookResponse {
    pub id: String,
    pub url: String,
    pub event_types: Vec<String>,
    pub filters: Option<serde_json::Value>,
    pub is_active: bool,
    pub created_at: String,
}

/// Webhook event envelope with versioning
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookEventEnvelope {
    pub id: String, // Delivery ID for idempotency
    pub event: String,
    pub timestamp: i64,
    pub event_version: String,
    pub data: serde_json::Value,
}

impl WebhookEventEnvelope {
    /// Current event version
    pub const EVENT_VERSION: &'static str = "v1";
}

/// Delivery log entry - records each webhook delivery attempt
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeliveryLog {
    pub id: String,
    pub webhook_id: String,
    pub event_id: Option<String>,
    pub delivery_id: String,
    pub status: String,
    pub http_status: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub signature: Option<String>,
    pub event_version: String,
    pub timestamp: String,
    pub created_at: String,
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
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::CorridorHealthDegraded => "corridor.health_degraded",
            Self::AnchorStatusChanged => "anchor.status_changed",
            Self::PaymentCreated => "payment.created",
            Self::CorridorLiquidityDropped => "corridor.liquidity_dropped",
        }
    }

    #[must_use]
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
    pub db: SqlitePool,
    encryption_key: String,
}

impl WebhookService {
    #[must_use]
    pub fn new(db: SqlitePool) -> Self {
        let encryption_key = std::env::var("ENCRYPTION_KEY").unwrap_or_else(|_| {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        });
        Self { db, encryption_key }
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
        let filters_str = request
            .filters
            .as_ref()
            .map(std::string::ToString::to_string);
        let now = chrono::Utc::now().to_rfc3339();

        let encrypted_secret = crate::crypto::encrypt_data(&secret, &self.encryption_key)
            .unwrap_or_else(|_| secret.clone());

        let event_version = request.event_version.unwrap_or_else(|| "v1".to_string());
        let secret_rotation_enabled = request.secret_rotation_enabled.unwrap_or(false);
        let secret_rotation_period_days = request.secret_rotation_period_days.unwrap_or(30);
        let next_rotation_date = if secret_rotation_enabled {
            Some(
                chrono::Utc::now()
                    .checked_add_days(chrono::Days::new(secret_rotation_period_days as u64))
                    .unwrap_or(chrono::Utc::now())
                    .to_rfc3339(),
            )
        } else {
            None
        };

        sqlx::query(
            r"
            INSERT INTO webhooks (id, user_id, url, event_types, filters, secret, is_active, created_at, event_version, secret_rotation_enabled, secret_rotation_period_days, next_rotation_date)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&id)
        .bind(user_id)
        .bind(&request.url)
        .bind(&event_types_str)
        .bind(filters_str.as_deref())
        .bind(&encrypted_secret)
        .bind(true)
        .bind(&now)
        .bind(&event_version)
        .bind(secret_rotation_enabled as i32)
        .bind(secret_rotation_period_days)
        .bind(next_rotation_date.as_deref())
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
        let mut webhook = sqlx::query_as::<_, Webhook>(
            "SELECT id, user_id, url, event_types, filters, secret, is_active, created_at, last_fired_at FROM webhooks WHERE id = ?"
        )
        .bind(webhook_id)
        .fetch_optional(&self.db)
        .await?;

        if let Some(ref mut w) = webhook {
            w.secret = crate::crypto::decrypt_data(&w.secret, &self.encryption_key)
                .unwrap_or_else(|_| w.secret.clone());
        }

        Ok(webhook)
    }

    /// List webhooks for a user
    pub async fn list_webhooks(&self, user_id: &str) -> anyhow::Result<Vec<Webhook>> {
        let mut webhooks = sqlx::query_as::<_, Webhook>(
            "SELECT id, user_id, url, event_types, filters, secret, is_active, created_at, last_fired_at FROM webhooks WHERE user_id = ? AND is_active = 1 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        for w in &mut webhooks {
            w.secret = crate::crypto::decrypt_data(&w.secret, &self.encryption_key)
                .unwrap_or_else(|_| w.secret.clone());
        }

        Ok(webhooks)
    }

    /// Delete/deactivate webhook
    pub async fn delete_webhook(&self, webhook_id: &str, user_id: &str) -> anyhow::Result<bool> {
        let result = sqlx::query("UPDATE webhooks SET is_active = 0 WHERE id = ? AND user_id = ?")
            .bind(webhook_id)
            .bind(user_id)
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
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO webhook_events (id, webhook_id, event_type, payload, status, retries, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(id.clone())
        .bind(webhook_id)
        .bind(event_type)
        .bind(payload_str)
        .bind("pending")
        .bind(0)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok(id)
    }

    /// Get pending webhook events
    pub async fn get_pending_events(
        &self,
        limit: usize,
    ) -> anyhow::Result<Vec<(String, String, String, String)>> {
        let query_limit = limit as i64;

        let rows = sqlx::query(
            "SELECT we.id, we.webhook_id, we.event_type, we.payload
             FROM webhook_events we
             WHERE we.status = 'pending' AND we.retries < 3
             ORDER BY we.created_at ASC
             LIMIT ?",
        )
        .bind(query_limit)
        .fetch_all(&self.db)
        .await?;

        use sqlx::Row;
        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    row.get::<String, _>(0),
                    row.get::<String, _>(1),
                    row.get::<String, _>(2),
                    row.get::<String, _>(3),
                )
            })
            .collect())
    }

    /// Mark a webhook event as delivered
    pub async fn mark_event_delivered(&self, event_id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE webhook_events SET status = 'delivered' WHERE id = ?")
            .bind(event_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Set an event's delivery status, last error and retry count.
    pub async fn update_event_status(
        &self,
        event_id: &str,
        status: &str,
        last_error: Option<&str>,
        retries: i32,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE webhook_events SET status = ?, last_error = ?, retries = ? WHERE id = ?",
        )
        .bind(status)
        .bind(last_error)
        .bind(retries)
        .bind(event_id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// Record the time a webhook last delivered successfully.
    pub async fn update_last_fired(&self, webhook_id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE webhooks SET last_fired_at = ? WHERE id = ?")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(webhook_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Record a failed delivery attempt, incrementing the retry counter.
    ///
    /// Returns the updated retry count so callers can decide whether to keep
    /// retrying or give up. Events that exceed [`MAX_DISPATCH_RETRIES`] are
    /// marked as `failed` so they are no longer picked up.
    pub async fn record_event_failure(&self, event_id: &str) -> anyhow::Result<u32> {
        sqlx::query("UPDATE webhook_events SET retries = retries + 1 WHERE id = ?")
            .bind(event_id)
            .execute(&self.db)
            .await?;

        use sqlx::Row;
        let retries: i64 = sqlx::query("SELECT retries FROM webhook_events WHERE id = ?")
            .bind(event_id)
            .fetch_one(&self.db)
            .await?
            .get(0);

        let retries = u32::try_from(retries).unwrap_or(MAX_DISPATCH_RETRIES);
        if retries >= MAX_DISPATCH_RETRIES {
            sqlx::query("UPDATE webhook_events SET status = 'failed' WHERE id = ?")
                .bind(event_id)
                .execute(&self.db)
                .await?;
        }

        Ok(retries)
    }

    /// Create a delivery log entry
    pub async fn create_delivery_log(
        &self,
        webhook_id: &str,
        event_id: Option<&str>,
        delivery_id: &str,
        status: &str,
        http_status: Option<i32>,
        response_body: Option<&str>,
        error_message: Option<&str>,
        signature: Option<&str>,
        event_version: &str,
    ) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO delivery_logs (id, webhook_id, event_id, delivery_id, status, http_status, response_body, error_message, signature, event_version, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(webhook_id)
        .bind(event_id)
        .bind(delivery_id)
        .bind(status)
        .bind(http_status)
        .bind(response_body)
        .bind(error_message)
        .bind(signature)
        .bind(event_version)
        .bind(now.clone())
        .execute(&self.db)
        .await?;

        Ok(id)
    }

    /// Get delivery logs for a webhook
    pub async fn get_delivery_logs(
        &self,
        webhook_id: &str,
        limit: usize,
        offset: usize,
    ) -> anyhow::Result<Vec<DeliveryLog>> {
        let limit_val = limit as i64;
        let offset_val = offset as i64;

        let rows = sqlx::query_as::<_, DeliveryLog>(
            "SELECT id, webhook_id, event_id, delivery_id, status, http_status, response_body, error_message, signature, event_version, timestamp, created_at
             FROM delivery_logs
             WHERE webhook_id = ?
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(webhook_id)
        .bind(limit_val)
        .bind(offset_val)
        .fetch_all(&self.db)
        .await?;

        Ok(rows)
    }

    /// Enable a webhook that was auto-disabled
    pub async fn enable_webhook(&self, webhook_id: &str, user_id: &str) -> anyhow::Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE webhooks 
             SET is_active = 1, 
                 disabled_reason = NULL, 
                 disabled_at = NULL, 
                 enabled_at = ?,
                 consecutive_failures = 0
             WHERE id = ? AND user_id = ?",
        )
        .bind(&now)
        .bind(webhook_id)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Check if webhook should be auto-disabled based on consecutive failures
    pub async fn check_and_disable_webhook(
        &self,
        webhook_id: &str,
        reason: &str,
    ) -> anyhow::Result<bool> {
        // First, get current webhook state
        let webhook = self.get_webhook(webhook_id).await?;

        if let Some(ref w) = webhook {
            let current_failures = w.consecutive_failures;
            let max_failures = w.max_consecutive_failures;

            if current_failures >= max_failures {
                let now = chrono::Utc::now().to_rfc3339();
                let result = sqlx::query(
                    "UPDATE webhooks 
                     SET is_active = 0, 
                         disabled_reason = ?, 
                         disabled_at = ?
                     WHERE id = ?",
                )
                .bind(reason)
                .bind(&now)
                .bind(webhook_id)
                .execute(&self.db)
                .await?;

                Ok(result.rows_affected() > 0)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Increment consecutive failures for a webhook
    pub async fn increment_consecutive_failures(&self, webhook_id: &str) -> anyhow::Result<i32> {
        let result = sqlx::query(
            "UPDATE webhooks SET consecutive_failures = consecutive_failures + 1 WHERE id = ?",
        )
        .bind(webhook_id)
        .execute(&self.db)
        .await?;

        use sqlx::Row;
        let failures: i64 = sqlx::query("SELECT consecutive_failures FROM webhooks WHERE id = ?")
            .bind(webhook_id)
            .fetch_one(&self.db)
            .await?
            .get(0);

        Ok(failures as i32)
    }

    /// Reset consecutive failures on successful delivery
    pub async fn reset_consecutive_failures(&self, webhook_id: &str) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE webhooks 
             SET consecutive_failures = 0,
                 last_success_at = ?
             WHERE id = ?",
        )
        .bind(&now)
        .bind(webhook_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Check if secret rotation is needed and rotate if enabled
    pub async fn rotate_secrets_if_needed(&self, webhook_id: &str) -> anyhow::Result<bool> {
        // Get webhook with rotation settings
        let webhook = self.get_webhook(webhook_id).await?;

        if let Some(ref w) = webhook {
            if w.secret_rotation_enabled {
                if let Some(ref next_rotation) = w.next_rotation_date {
                    let now = chrono::Utc::now();
                    let next_rot = chrono::DateTime::parse_from_rfc3339(next_rotation)?;

                    if now > next_rot {
                        // Generate new secondary secret
                        let new_secret = Uuid::new_v4().to_string();
                        let new_next_rotation = now
                            .checked_add_days(chrono::Days::new(
                                w.secret_rotation_period_days as u64,
                            ))
                            .unwrap_or(now)
                            .to_rfc3339();

                        sqlx::query(
                            "UPDATE webhooks 
                             SET secondary_secret = ?, 
                                 secret_rotation_period_days = ?,
                                 next_rotation_date = ?
                             WHERE id = ?",
                        )
                        .bind(&new_secret)
                        .bind(w.secret_rotation_period_days)
                        .bind(&new_next_rotation)
                        .bind(webhook_id)
                        .execute(&self.db)
                        .await?;

                        Ok(true)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod signature_tests {
    use super::WebhookSignature;

    #[test]
    fn rotation_signature_stays_verifiable_with_primary_secret() {
        let body = r#"{"event":"test"}"#;
        let sig = WebhookSignature::sign_with_rotation(body, "primary", Some("secondary"));
        assert!(sig.starts_with("sha256="));
        assert!(WebhookSignature::verify(body, "primary", &sig));
        assert!(!WebhookSignature::verify(body, "other", &sig));
    }
}
