/// Service container providing dependency injection for all application services.
///
/// Centralises construction so `main` stays thin and each service can be
/// built and tested in isolation.
use std::sync::Arc;

use sqlx::SqlitePool;

use crate::{
    rpc::StellarRpcClient,
    services::{
        account_merge_detector::AccountMergeDetector,
        broadcaster_port::BroadcasterPort,
        data_port::DataPort,
        fee_bump_tracker::FeeBumpTrackerService,
        liquidity_pool_analyzer::LiquidityPoolAnalyzer,
        price_feed::{default_asset_mapping, PriceFeedClient, PriceFeedConfig},
        realtime_broadcaster::RealtimeBroadcaster,
        webhook_dispatcher::WebhookDispatcher,
        webhook_event_service::WebhookEventService,
    },
    websocket::WsState,
};

/// Holds all constructed service instances.
pub struct ServiceContainer {
    pub fee_bump_tracker: Arc<FeeBumpTrackerService>,
    pub account_merge_detector: Arc<AccountMergeDetector>,
    pub lp_analyzer: Arc<LiquidityPoolAnalyzer>,
    pub price_feed: Arc<PriceFeedClient>,
    pub webhook_dispatcher: Arc<WebhookDispatcher>,
    pub webhook_event_service: Arc<WebhookEventService>,
    pub realtime_broadcaster: Arc<Box<dyn BroadcasterPort>>,
}

impl ServiceContainer {
    /// Build all services from shared infrastructure dependencies.
    pub fn build(
        pool: SqlitePool,
        rpc_client: Arc<StellarRpcClient>,
        ws_state: Arc<WsState>,
        db: Arc<crate::Database>,
    ) -> Self {
        let webhook_event_service = Arc::new(WebhookEventService::new(pool.clone()));
        
        // RealtimeBroadcaster needs: ws_state, data_port (Database), webhook_events
        let realtime_broadcaster = RealtimeBroadcaster::new(
            ws_state,
            Arc::clone(&db) as Arc<dyn DataPort>,
            webhook_event_service.clone(),
        );
        
        Self {
            fee_bump_tracker: Arc::new(FeeBumpTrackerService::new(pool.clone())),
            account_merge_detector: Arc::new(AccountMergeDetector::new(
                pool.clone(),
                rpc_client.clone(),
            )),
            lp_analyzer: Arc::new(LiquidityPoolAnalyzer::new(pool.clone(), rpc_client.clone())),
            price_feed: Arc::new(PriceFeedClient::new(
                PriceFeedConfig::default(),
                default_asset_mapping(),
            )),
            webhook_dispatcher: Arc::new(WebhookDispatcher::new(pool)),
            webhook_event_service,
            realtime_broadcaster: Arc::new(Box::new(realtime_broadcaster)),
        }
    }
}
