//! handlers.rs — re-exports from split modules.
//!
//! Logic has been moved to:
//!   - crate::api::health      (health_check, HealthStatus, HealthChecks, ComponentHealth)
//!   - crate::api::metrics     (get_pool_metrics, pool_metrics)
//!   - crate::api::ingestion   (ingestion_status)
//!   - crate::api::anchors     (get_anchor, get_anchor_by_account, get_anchor_assets,
//!                               create_anchor, create_anchor_asset, update_anchor_metrics,
//!                               get_muxed_analytics, get_anchors)
//!   - crate::api::corridors   (list_corridors, get_corridor_detail, create_corridor,
//!                               update_corridor_metrics_from_transactions)

pub use crate::api::health::{health_check, ComponentHealth, HealthChecks, HealthStatus};
pub use crate::api::ingestion::ingestion_status;
pub use crate::api::metrics::{get_pool_metrics, pool_metrics};

pub use crate::api::anchors::{
    create_anchor, create_anchor_asset, get_anchor, get_anchor_assets, get_anchor_by_account,
    get_anchors, get_muxed_analytics, update_anchor_metrics,
};
pub use crate::api::corridors::{
    create_corridor, get_corridor_detail, list_corridors,
    update_corridor_metrics_from_transactions,
};
