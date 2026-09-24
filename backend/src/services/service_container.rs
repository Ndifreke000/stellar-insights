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
        fee_bump_tracker::FeeBumpTrackerService,
        liquidity_pool_analyzer::LiquidityPoolAnalyzer,
        price_feed::{default_asset_mapping, PriceFeedClient, PriceFeedConfig},
        webhook_dispatcher::WebhookDispatcher,
    },
};

/// Holds all constructed service instances.
pub struct ServiceContainer {
    pub fee_bump_tracker: Arc<FeeBumpTrackerService>,
    pub account_merge_detector: Arc<AccountMergeDetector>,
    pub lp_analyzer: Arc<LiquidityPoolAnalyzer>,
    pub price_feed: Arc<PriceFeedClient>,
    pub webhook_dispatcher: Arc<WebhookDispatcher>,
}

impl ServiceContainer {
    /// Build all services from shared infrastructure dependencies.
    pub fn build(pool: SqlitePool, rpc_client: Arc<StellarRpcClient>) -> Self {
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
        }
    }
}
