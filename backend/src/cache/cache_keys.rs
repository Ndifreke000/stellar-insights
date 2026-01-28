/// Cache key generation for different data types
/// Follows pattern: {entity}:{identifier}:{variant}
pub struct CacheKey;

impl CacheKey {
    /// Corridor metrics cache key
    /// TTL: 5 minutes
    pub fn corridor_metrics(corridor_key: &str) -> String {
        format!("corridor:metrics:{}", corridor_key)
    }

    /// Corridor list cache key with filters
    /// TTL: 5 minutes
    pub fn corridor_list(limit: i64, offset: i64, filters_hash: &str) -> String {
        format!("corridor:list:{}:{}:{}", limit, offset, filters_hash)
    }

    /// Anchor data cache key
    /// TTL: 10 minutes
    pub fn anchor_data(anchor_id: &str) -> String {
        format!("anchor:data:{}", anchor_id)
    }

    /// Anchor list cache key
    /// TTL: 10 minutes
    pub fn anchor_list(limit: i64, offset: i64) -> String {
        format!("anchor:list:{}:{}", limit, offset)
    }

    /// Dashboard stats cache key
    /// TTL: 1 minute
    pub fn dashboard_stats() -> String {
        "dashboard:stats".to_string()
    }

    /// Dashboard overview cache key
    /// TTL: 1 minute
    pub fn dashboard_overview() -> String {
        "dashboard:overview".to_string()
    }

    /// Anchor detail with assets cache key
    /// TTL: 10 minutes
    pub fn anchor_detail(anchor_id: &str) -> String {
        format!("anchor:detail:{}", anchor_id)
    }

    /// Anchor assets cache key
    /// TTL: 10 minutes
    pub fn anchor_assets(anchor_id: &str) -> String {
        format!("anchor:assets:{}", anchor_id)
    }

    /// Corridor detail cache key
    /// TTL: 5 minutes
    pub fn corridor_detail(corridor_key: &str) -> String {
        format!("corridor:detail:{}", corridor_key)
    }

    /// Cache invalidation pattern for corridors
    pub fn corridor_pattern() -> String {
        "corridor:*".to_string()
    }

    /// Cache invalidation pattern for anchors
    pub fn anchor_pattern() -> String {
        "anchor:*".to_string()
    }

    /// Cache invalidation pattern for dashboard
    pub fn dashboard_pattern() -> String {
        "dashboard:*".to_string()
    }
}

/// Generate hash for filter parameters to use in cache keys
pub fn hash_filters(filters: &str) -> String {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(filters.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}
