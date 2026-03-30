use async_trait::async_trait;
use crate::models::corridor::CorridorMetrics;

/// Defines the contract for fetching data needed by the broadcaster.
/// Decouples RealtimeBroadcaster from the concrete Database type.
#[async_trait]
pub trait DataPort: Send + Sync {
    async fn fetch_corridor_updates(
        &self,
    ) -> Result<Vec<CorridorMetrics>, Box<dyn std::error::Error + Send + Sync>>;
}
