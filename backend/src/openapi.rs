use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Stellar Insights API",
        version = "1.0.0",
        description = "API for Stellar network analytics, anchor monitoring, and payment corridor insights",
        contact(
            name = "Stellar Insights Team",
            email = "support@stellarinsights.io"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "https://api.stellarinsights.io", description = "Production server")
    ),
    paths(
        // Anchors
        crate::api::anchors::get_anchor,
        crate::api::anchors::get_anchor_by_account,
        crate::api::anchors::get_anchors,
        crate::api::anchors::get_muxed_analytics,
        // Corridors
        crate::api::corridors::list_corridors,
        crate::api::corridors::get_corridor_detail,
        // Price Feed
        crate::api::price_feed::get_price,
        crate::api::price_feed::get_prices,
        crate::api::price_feed::convert_to_usd,
        crate::api::price_feed::get_cache_stats,
        // Cost Calculator
        crate::api::cost_calculator::estimate_costs,
        // Alerts
        crate::api::alerts::list_rules,
        crate::api::alerts::create_rule,
        crate::api::alerts::update_rule,
        crate::api::alerts::delete_rule,
        crate::api::alerts::list_history,
        crate::api::alerts::mark_history_read,
        crate::api::alerts::dismiss_history,
        crate::api::alerts::snooze_rule_from_history,
        // API Analytics
        crate::api::api_analytics::get_analytics_overview,
        // API Keys
        crate::api::api_keys::create_api_key,
        crate::api::api_keys::list_api_keys,
        crate::api::api_keys::get_api_key,
        crate::api::api_keys::rotate_api_key,
        crate::api::api_keys::revoke_api_key,
        // Contract Events
        crate::api::contract_events::get_verification_summary,
        crate::api::contract_events::list_contract_events,
        crate::api::contract_events::get_contract_event,
        crate::api::contract_events::get_events_for_epoch,
        crate::api::contract_events::get_event_stats,
        // Fee Bumps
        crate::api::fee_bump::get_fee_bump_stats,
        crate::api::fee_bump::get_recent_fee_bumps,
        // Liquidity Pools
        crate::api::liquidity_pools::list_pools,
        crate::api::liquidity_pools::get_pool_stats,
        crate::api::liquidity_pools::get_pool_rankings,
        crate::api::liquidity_pools::get_pool_detail,
        crate::api::liquidity_pools::get_pool_snapshots,
        // Metrics
        crate::api::metrics::metrics_overview,
        // ML
        crate::api::ml::predict_payment_success,
        crate::api::ml::get_model_status,
        crate::api::ml::retrain_model,
        // Network
        crate::api::network::get_network_info,
        crate::api::network::get_available_networks,
        crate::api::network::switch_network,
        // Prediction
        crate::api::prediction::predict_success,
        // RPC
        crate::api::rpc::rpc_health_check,
        crate::api::rpc::get_latest_ledger,
        crate::api::rpc::get_payments,
        crate::api::rpc::get_account_payments,
        crate::api::rpc::get_trades,
        crate::api::rpc::get_order_book,
        // SEP-31
        crate::api::sep31_proxy::get_info,
        crate::api::sep31_proxy::post_quote,
        crate::api::sep31_proxy::post_transaction,
        crate::api::sep31_proxy::get_transactions,
        crate::api::sep31_proxy::get_transaction,
        crate::api::sep31_proxy::get_customer,
        crate::api::sep31_proxy::put_customer,
        crate::api::sep31_proxy::list_anchors,
        // Transactions
        crate::api::transactions::create_transaction,
        crate::api::transactions::get_transaction,
        crate::api::transactions::add_signature,
        crate::api::transactions::submit_transaction,
        // Trustlines
        crate::api::trustlines::get_trustline_metrics,
        crate::api::trustlines::get_trustline_rankings,
        crate::api::trustlines::get_trustline_history,
        // Webhooks
        crate::api::webhooks::register_webhook,
        crate::api::webhooks::list_webhooks,
        crate::api::webhooks::get_webhook,
        crate::api::webhooks::delete_webhook,
        crate::api::webhooks::test_webhook,
        // Account Merges
        crate::api::account_merges::get_account_merge_stats,
        crate::api::account_merges::get_recent_account_merges,
        crate::api::account_merges::get_destination_patterns,
        // Achievements
        crate::api::achievements::get_quests,
        crate::api::achievements::get_achievements,
        // Asset Verification
        crate::api::asset_verification::verify_asset,
        crate::api::asset_verification::get_verification,
        crate::api::asset_verification::list_verified_assets,
        crate::api::asset_verification::report_suspicious_asset,
        // Auth
        crate::api::auth::login,
        crate::api::auth::refresh,
        crate::api::auth::logout,
        // Cache
        crate::api::cache_stats::get_cache_stats,
        crate::api::cache_stats::reset_cache_stats,
        // Governance
        crate::api::governance::create_proposal,
        crate::api::governance::activate_proposal,
        crate::api::governance::list_proposals,
        crate::api::governance::get_proposal,
        crate::api::governance::cast_vote,
        crate::api::governance::get_votes,
        crate::api::governance::has_voted,
        crate::api::governance::add_comment,
        crate::api::governance::get_comments,
        // OAuth
        crate::api::oauth::authorize,
        crate::api::oauth::token,
        crate::api::oauth::revoke,
        crate::api::oauth::list_apps,
        // SEP-10
        crate::api::sep10::get_info,
        crate::api::sep10::request_challenge,
        crate::api::sep10::verify_challenge,
        crate::api::sep10::logout,
        // SEP-24
        crate::api::sep24_proxy::get_info,
        crate::api::sep24_proxy::post_deposit_interactive,
        crate::api::sep24_proxy::post_withdraw_interactive,
        crate::api::sep24_proxy::get_transactions,
        crate::api::sep24_proxy::get_transaction,
        crate::api::sep24_proxy::list_anchors,
        // Verification Rewards
        crate::api::verification_rewards::verify_snapshot,
        crate::api::verification_rewards::get_user_stats,
        crate::api::verification_rewards::get_leaderboard,
        crate::api::verification_rewards::get_user_verifications,
        crate::api::verification_rewards::get_public_user_stats,
    ),
    components(
        schemas(
            crate::api::anchors::AnchorsResponse,
            crate::api::anchors::AnchorMetricsResponse,
            crate::api::corridors::CorridorResponse,
            crate::api::corridors::CorridorDetailResponse,
            crate::api::corridors::SuccessRateDataPoint,
            crate::api::corridors::LatencyDataPoint,
            crate::api::corridors::LiquidityDataPoint,
            crate::api::price_feed::PriceResponse,
            crate::api::price_feed::PricesResponse,
            crate::api::price_feed::ConvertResponse,
            crate::api::price_feed::CacheStatsResponse,
            crate::api::cost_calculator::PaymentRoute,
            crate::api::cost_calculator::CostCalculationRequest,
            crate::api::cost_calculator::RouteCostBreakdown,
            crate::api::cost_calculator::RouteEstimate,
            crate::api::cost_calculator::CostCalculationResponse,
            crate::api::cost_calculator::ErrorResponse,
        )
    ),
    tags(
        (name = "Alerts", description = "Alert management and notification endpoints"),
        (name = "Analytics", description = "API analytics endpoints"),
        (name = "Anchors", description = "Anchor management and metrics endpoints"),
        (name = "API Keys", description = "API key management endpoints"),
        (name = "Contract Events", description = "Smart contract event tracking"),
        (name = "Corridors", description = "Payment corridor analytics endpoints"),
        (name = "Fee Bumps", description = "Fee bump transaction tracking"),
        (name = "Liquidity Pools", description = "Liquidity pool analytics"),
        (name = "Metrics", description = "System metrics and monitoring"),
        (name = "ML", description = "Machine learning prediction endpoints"),
        (name = "Network", description = "Stellar network configuration"),
        (name = "Prediction", description = "Payment prediction endpoints"),
        (name = "Prices", description = "Real-time asset price feed endpoints"),
        (name = "Cost Calculator", description = "Cross-border payment cost estimation and route comparison"),
        (name = "RPC", description = "Stellar RPC integration endpoints"),
        (name = "SEP-31", description = "SEP-31 cross-border payment endpoints"),
        (name = "Transactions", description = "Transaction management endpoints"),
        (name = "Trustlines", description = "Trustline analytics endpoints"),
        (name = "Webhooks", description = "Webhook management endpoints"),
        (name = "Account Merges", description = "Account merge tracking endpoints"),
        (name = "Achievements", description = "Quest and achievement definitions"),
        (name = "Asset Verification", description = "Asset verification and reporting"),
        (name = "Auth", description = "Authentication endpoints"),
        (name = "Cache", description = "Cache management endpoints"),
        (name = "Governance", description = "Governance proposal and voting endpoints"),
        (name = "OAuth", description = "OAuth integration endpoints"),
        (name = "SEP-10", description = "SEP-10 authentication endpoints"),
        (name = "SEP-24", description = "SEP-24 hosted deposit/withdrawal endpoints"),
        (name = "Verification Rewards", description = "Snapshot verification reward endpoints"),
    )
)]
pub struct ApiDoc;
