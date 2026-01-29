use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::aio::MultiplexedConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Rate limit configuration for an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub whitelist_ips: Vec<String>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
            whitelist_ips: vec![],
        }
    }
}

/// Rate limiter state
pub struct RateLimiter {
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    endpoint_configs: Arc<RwLock<HashMap<String, RateLimitConfig>>>,
    fallback_memory_store: Arc<RwLock<HashMap<String, (u32, i64)>>>,
}

impl RateLimiter {
    pub async fn new() -> anyhow::Result<Self> {
        let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        
        let connection = if let Ok(client) = redis::Client::open(redis_url.as_str()) {
            match client.get_multiplexed_tokio_connection().await {
                Ok(conn) => {
                    tracing::info!("Connected to Redis for rate limiting");
                    Some(conn)
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to Redis ({}), using memory-only rate limiting", e);
                    None
                }
            }
        } else {
            tracing::warn!("Invalid Redis URL, using memory-only rate limiting");
            None
        };

        Ok(Self {
            redis_connection: Arc::new(RwLock::new(connection)),
            endpoint_configs: Arc::new(RwLock::new(HashMap::new())),
            fallback_memory_store: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Register a rate limit config for an endpoint
    pub async fn register_endpoint(&self, path: String, config: RateLimitConfig) {
        self.endpoint_configs.write().await.insert(path, config);
    }

    /// Check if IP is in whitelist for an endpoint
    fn is_whitelisted(&self, ip: &str, config: &RateLimitConfig) -> bool {
        config.whitelist_ips.iter().any(|whitelisted_ip| {
            whitelisted_ip == ip || whitelisted_ip == "*"
        })
    }

    /// Check rate limit for an IP/endpoint combination
    pub async fn check_rate_limit(
        &self,
        ip: &str,
        endpoint: &str,
    ) -> (bool, RateLimitInfo) {
        // Get endpoint config
        let configs = self.endpoint_configs.read().await;
        let config = configs
            .get(endpoint)
            .cloned()
            .unwrap_or_default();

        // Check whitelist
        if self.is_whitelisted(ip, &config) {
            return (true, RateLimitInfo {
                limit: config.requests_per_minute,
                remaining: config.requests_per_minute,
                reset_after: 60,
                is_whitelisted: true,
            });
        }

        let key = format!("ratelimit:{}:{}", endpoint, ip);
        let limit = config.requests_per_minute;

        // Try Redis first
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match self.check_redis_limit(&mut conn, &key, limit).await {
                Ok((allowed, remaining, reset)) => {
                    return (allowed, RateLimitInfo {
                        limit,
                        remaining,
                        reset_after: reset,
                        is_whitelisted: false,
                    });
                }
                Err(_) => {
                }
            }
        }

        // Fall back to memory store
        let (allowed, remaining, reset) = self.check_memory_limit(&key, limit).await;
        (
            allowed,
            RateLimitInfo {
                limit,
                remaining,
                reset_after: reset,
                is_whitelisted: false,
            },
        )
    }

    /// Check rate limit in Redis
    async fn check_redis_limit(
        &self,
        conn: &mut MultiplexedConnection,
        key: &str,
        limit: u32,
    ) -> anyhow::Result<(bool, u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        use redis::AsyncCommands;

        let current: u32 = conn.get(key).await.unwrap_or(0);
        let ttl: i64 = conn.ttl(key).await.unwrap_or(-1);

        if current >= limit {
            return Ok((false, 0, if ttl > 0 { ttl as u32 } else { 60 }));
        }

        let new_count = current + 1;
        conn.incr::<_, _, ()>(key, 1).await?;

        if current == 0 {
            conn.expire::<_, ()>(key, 60).await?;
        }

        let remaining = if new_count >= limit { 0 } else { limit - new_count };
        Ok((new_count < limit, remaining, 60))
    }

    /// Check rate limit in memory (fallback)
    async fn check_memory_limit(
        &self,
        key: &str,
        limit: u32,
    ) -> (bool, u32, u32) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut store = self.fallback_memory_store.write().await;

        let (count, expiry) = store.get(key).copied().unwrap_or((0, now + 60));

        if now > expiry {
            // Reset counter
            store.insert(key.to_string(), (1, now + 60));
            (true, limit - 1, 60)
        } else if count >= limit {
            (false, 0, (expiry - now) as u32)
        } else {
            let new_count = count + 1;
            store.insert(key.to_string(), (new_count, expiry));
            let remaining = if new_count >= limit { 0 } else { limit - new_count };
            (new_count < limit, remaining, (expiry - now) as u32)
        }
    }
}

/// Rate limit information in response
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    pub reset_after: u32,
    pub is_whitelisted: bool,
}

/// Rate limit error response
#[derive(Debug)]
pub struct RateLimitError {
    pub info: RateLimitInfo,
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "Rate limit exceeded",
            "limit": self.info.limit,
            "reset_after": self.info.reset_after,
        });

        (
            StatusCode::TOO_MANY_REQUESTS,
            [(
                "RateLimit-Limit",
                self.info.limit.to_string(),
            ), (
                "RateLimit-Remaining",
                self.info.remaining.to_string(),
            ), (
                "RateLimit-Reset",
                self.info.reset_after.to_string(),
            )],
            axum::Json(body),
        )
            .into_response()
    }
}

/// Middleware for rate limiting
pub async fn rate_limit_middleware(
    State(limiter): State<Arc<RateLimiter>>,
    addr: ConnectInfo<std::net::SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let ip = addr.0.ip().to_string();
    let path = req.uri().path().to_string();

    let (allowed, info) = limiter.check_rate_limit(&ip, &path).await;

    if !allowed {
        return RateLimitError { info }.into_response();
    }

    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "RateLimit-Limit",
        info.limit.to_string().parse().unwrap(),
    );
    response.headers_mut().insert(
        "RateLimit-Remaining",
        info.remaining.to_string().parse().unwrap(),
    );
    response.headers_mut().insert(
        "RateLimit-Reset",
        info.reset_after.to_string().parse().unwrap(),
    );

    response
}
