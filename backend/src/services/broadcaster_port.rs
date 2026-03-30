use async_trait::async_trait;
use crate::models::corridor::CorridorMetrics;
use crate::models::AnchorMetrics;

/// Defines the contract for broadcasting real-time events.
/// Any struct implementing this can be used in place of RealtimeBroadcaster.
#[async_trait]
pub trait BroadcasterPort: Send + Sync {
    async fn broadcast_corridor_update(&self, corridor: CorridorMetrics);
    async fn broadcast_anchor_status(
        &self,
        anchor_id: String,
        anchor_name: String,
        anchor: AnchorMetrics,
        old_status: String,
    );
    async fn broadcast_payment(
        &self,
        corridor_key: String,
        amount: f64,
        successful: bool,
        timestamp: String,
    );
    async fn broadcast_health_alert(
        &self,
        corridor_id: String,
        severity: String,
        message: String,
    );
    fn connection_count(&self) -> usize;
    fn channel_subscription_count(&self, channel: &str) -> usize;
}
