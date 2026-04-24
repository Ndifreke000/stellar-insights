use axum::{
    body::Body,
    extract::{Request, State},
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use std::{collections::HashMap, sync::Arc};

/// Configuration for a deprecated endpoint.
#[derive(Clone)]
pub struct DeprecationConfig {
    /// RFC 7231 date string for when the endpoint was deprecated (e.g. "Sat, 01 Jan 2026 00:00:00 GMT")
    pub deprecation_date: &'static str,
    /// RFC 7231 date string for when the endpoint will be removed
    pub sunset_date: &'static str,
    /// Optional link to migration docs
    pub link: Option<&'static str>,
}

/// Map of path prefix → deprecation config.
pub type DeprecationMap = Arc<HashMap<&'static str, DeprecationConfig>>;

/// Build the default deprecation registry.
/// Add entries here as endpoints are deprecated.
pub fn default_deprecation_map() -> DeprecationMap {
    let mut map = HashMap::new();

    // Example (uncomment and adjust when real endpoints are deprecated):
    // map.insert(
    //     "/api/v1/old-endpoint",
    //     DeprecationConfig {
    //         deprecation_date: "Sat, 01 Jan 2026 00:00:00 GMT",
    //         sunset_date:      "Sat, 01 Jul 2026 00:00:00 GMT",
    //         link: Some("https://docs.example.com/migration"),
    //     },
    // );

    Arc::new(map)
}

/// Middleware that injects `Deprecation` and `Sunset` headers for deprecated paths
/// and logs usage so operators can track adoption of the migration.
pub async fn deprecation_middleware(
    State(map): State<DeprecationMap>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    // Find the first matching prefix (exact or prefix match).
    let config = map
        .iter()
        .find(|(prefix, _)| path == **prefix || path.starts_with(*prefix))
        .map(|(_, cfg)| cfg.clone());

    let mut response = next.run(req).await;

    if let Some(cfg) = config {
        let headers = response.headers_mut();

        if let Ok(v) = HeaderValue::from_str(cfg.deprecation_date) {
            headers.insert("Deprecation", v);
        }
        if let Ok(v) = HeaderValue::from_str(cfg.sunset_date) {
            headers.insert("Sunset", v);
        }
        if let Some(link) = cfg.link {
            if let Ok(v) = HeaderValue::from_str(&format!("<{link}>; rel=\"deprecation\"")) {
                headers.insert("Link", v);
            }
        }

        tracing::warn!(
            path = %path,
            sunset = cfg.sunset_date,
            "Deprecated endpoint accessed"
        );
    }

    response
}
