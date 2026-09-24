use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ETagConfig {
    /// Default TTL in seconds for Cache-Control max-age
    pub default_ttl_seconds: usize,
    /// Whether to include network name in the cache key
    pub network_scoped: bool,
}

impl Default for ETagConfig {
    fn default() -> Self {
        Self {
            default_ttl_seconds: 60,
            network_scoped: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETagResponse {
    pub etag: String,
    pub cache_control: String,
    pub not_modified: bool,
}
