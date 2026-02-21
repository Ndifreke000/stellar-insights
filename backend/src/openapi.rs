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
        crate::api::anchors_cached::get_anchors,
        crate::api::corridors_cached::list_corridors,
        crate::api::corridors_cached::get_corridor_detail,
        crate::api::price_feed::get_price,
        crate::api::price_feed::get_prices,
        crate::api::price_feed::convert_to_usd,
        crate::api::price_feed::get_cache_stats,
        crate::api::cost_calculator::estimate_costs,
    ),
    components(
        schemas(
            crate::api::anchors_cached::AnchorsResponse,
            crate::api::anchors_cached::AnchorMetricsResponse,
            crate::api::corridors_cached::CorridorResponse,
            crate::api::corridors_cached::CorridorDetailResponse,
            crate::api::corridors_cached::SuccessRateDataPoint,
            crate::api::corridors_cached::LatencyDataPoint,
            crate::api::corridors_cached::LiquidityDataPoint,
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
        (name = "Anchors", description = "Anchor management and metrics endpoints"),
        (name = "Corridors", description = "Payment corridor analytics endpoints"),
        (name = "Prices", description = "Real-time asset price feed endpoints"),
        (name = "Cost Calculator", description = "Cross-border payment cost estimation and route comparison"),
        (name = "RPC", description = "Stellar RPC integration endpoints"),
        (name = "Fee Bumps", description = "Fee bump transaction tracking"),
        (name = "Cache", description = "Cache management and statistics"),
        (name = "Metrics", description = "System metrics and monitoring")
    )
)]
pub struct ApiDoc;
