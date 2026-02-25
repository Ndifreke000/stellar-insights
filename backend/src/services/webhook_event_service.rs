/// Webhook Event Service
/// Triggers webhook events for corridor events, anchor status changes, and alerts
use anyhow::Result;
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::webhooks::{
    WebhookEventType, WebhookService, CorridorHealthDegradedEvent, 
    AnchorStatusChangedEvent, PaymentCreatedEvent, CorridorLiquidityDroppedEvent,
    CorridorMetrics
};

/// Webhook Event Service - triggers events for registered webhooks
pub struct WebhookEventService {
    webhook_service: Arc<WebhookService>,
}

impl WebhookEventService {
    pub fn new(db: SqlitePool) -> Self {
        Self {
            webhook_service: Arc::new(WebhookService::new(db)),
        }
    }

    /// Trigger corridor health degradation event
    pub async fn trigger_corridor_health_degraded(
        &self,
        corridor_key: &str,
        old_metrics: &CorridorMetrics,
        new_metrics: &CorridorMetrics,
        severity: &str,
        changes: Vec<String>,
    ) -> Result<()> {
        let event = CorridorHealthDegradedEvent {
            corridor_key: corridor_key.to_string(),
            old_metrics: old_metrics.clone(),
            new_metrics: new_metrics.clone(),
            severity: severity.to_string(),
            changes,
        };

        let payload = json!(event);
        self.trigger_event(WebhookEventType::CorridorHealthDegraded, payload).await
    }

    /// Trigger anchor status change event
    pub async fn trigger_anchor_status_changed(
        &self,
        anchor_id: &str,
        name: &str,
        old_status: &str,
        new_status: &str,
        reliability_score: f64,
        failed_txn_count: i64,
    ) -> Result<()> {
        let event = AnchorStatusChangedEvent {
            anchor_id: anchor_id.to_string(),
            name: name.to_string(),
            old_status: old_status.to_string(),
            new_status: new_status.to_string(),
            reliability_score,
            failed_txn_count,
        };

        let payload = json!(event);
        self.trigger_event(WebhookEventType::AnchorStatusChanged, payload).await
    }

    /// Trigger payment created event
    pub async fn trigger_payment_created(
        &self,
        payment_id: &str,
        source: &str,
        destination: &str,
        asset_code: &str,
        asset_issuer: &str,
        amount: f64,
        timestamp: &str,
    ) -> Result<()> {
        let event = PaymentCreatedEvent {
            payment_id: payment_id.to_string(),
            source: source.to_string(),
            destination: destination.to_string(),
            asset_code: asset_code.to_string(),
            asset_issuer: asset_issuer.to_string(),
            amount,
            timestamp: timestamp.to_string(),
        };

        let payload = json!(event);
        self.trigger_event(WebhookEventType::PaymentCreated, payload).await
    }

    /// Trigger corridor liquidity dropped event
    pub async fn trigger_corridor_liquidity_dropped(
        &self,
        corridor_key: &str,
        liquidity_depth_usd: f64,
        threshold: f64,
        liquidity_trend: &str,
        severity: &str,
    ) -> Result<()> {
        let event = CorridorLiquidityDroppedEvent {
            corridor_key: corridor_key.to_string(),
            liquidity_depth_usd,
            threshold,
            liquidity_trend: liquidity_trend.to_string(),
            severity: severity.to_string(),
        };

        let payload = json!(event);
        self.trigger_event(WebhookEventType::CorridorLiquidityDropped, payload).await
    }

    /// Generic method to trigger an event for all matching webhooks
    async fn trigger_event(&self, event_type: WebhookEventType, payload: serde_json::Value) -> Result<()> {
        let event_type_str = event_type.as_str();
        
        // Get all active webhooks that subscribe to this event type
        let webhooks = self.get_webhooks_for_event(&event_type_str).await?;
        
        for webhook in webhooks {
            // Apply filters if any
            if let Some(filters) = &webhook.filters {
                if let Ok(filter_obj) = serde_json::from_str::<serde_json::Value>(filters) {
                    if !self.apply_filters(&payload, &filter_obj) {
                        continue; // Skip this webhook as it doesn't match filters
                    }
                }
            }

            // Create webhook event for delivery
            let _ = self.webhook_service
                .create_webhook_event(&webhook.id, event_type_str, payload.clone())
                .await;

            tracing::info!(
                "Webhook event triggered: webhook_id={}, event_type={}",
                webhook.id,
                event_type_str
            );
        }

        Ok(())
    }

    /// Get all active webhooks that subscribe to a specific event type
    async fn get_webhooks_for_event(&self, event_type: &str) -> Result<Vec<crate::webhooks::Webhook>> {
        // For now, we'll get all active webhooks and filter in memory
        // In a production system, you might want to optimize this with a better query
        let all_webhooks = sqlx::query_as::<_, crate::webhooks::Webhook>(
            "SELECT id, user_id, url, event_types, filters, secret, is_active, created_at, last_fired_at 
             FROM webhooks WHERE is_active = 1"
        )
        .fetch_all(&self.webhook_service.db)
        .await?;

        let matching_webhooks: Vec<crate::webhooks::Webhook> = all_webhooks
            .into_iter()
            .filter(|w| {
                w.event_types.split(',').any(|et| et.trim() == event_type)
            })
            .collect();

        Ok(matching_webhooks)
    }

    /// Apply filters to determine if webhook should be triggered
    fn apply_filters(&self, payload: &serde_json::Value, filters: &serde_json::Value) -> bool {
        // Simple filter implementation - can be extended
        if let Some(filter_obj) = filters.as_object() {
            for (key, expected_value) in filter_obj {
                if let Some(payload_value) = payload.get(key) {
                    // Simple equality check for now
                    if payload_value != expected_value {
                        return false;
                    }
                } else {
                    // Filter key not found in payload
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_filter_application() {
        let service = WebhookEventService::new(sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap());
        
        let payload = json!({
            "corridor_key": "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN->XLM:native",
            "severity": "warning"
        });

        let filters = json!({
            "severity": "warning"
        });

        assert!(service.apply_filters(&payload, &filters));

        let mismatched_filters = json!({
            "severity": "critical"
        });

        assert!(!service.apply_filters(&payload, &mismatched_filters));
    }
}
