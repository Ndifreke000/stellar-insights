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
        )
    ),
    tags(
        (name = "Anchors", description = "Anchor management and metrics endpoints"),
        (name = "Corridors", description = "Payment corridor analytics endpoints"),
        (name = "RPC", description = "Stellar RPC integration endpoints"),
        (name = "Fee Bumps", description = "Fee bump transaction tracking"),
        (name = "Cache", description = "Cache management and statistics"),
        (name = "Metrics", description = "System metrics and monitoring")
    )
)]
pub struct ApiDoc;
