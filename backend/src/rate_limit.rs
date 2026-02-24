use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::aio::MultiplexedConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::api_key::hash_api_key;

/// Rate limit configuration for an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub whitelist_ips: Vec<String>,
    /// Per-client rate limits (overrides default)
    pub client_limits: Option<ClientRateLimits>,
}

/// Client-specific rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRateLimits {
    /// Rate limit for authenticated users with API keys
    pub authenticated: u32,
    /// Rate limit for premium/paid tier clients
    pub premium: u32,
    /// Rate limit for anonymous/IP-based clients
    pub anonymous: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
            whitelist_ips: vec![],
            client_limits: Some(ClientRateLimits {
                authenticated: 200,
                premium: 1000,
                anonymous: 60,
            }),
        }
    }
}

/// Client identification for rate limiting
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientIdentifier {
    /// Authenticated client with API key
    ApiKey(String),
    /// Authenticated user via JWT
    User(String),
    /// Anonymous client identified by IP
    IpAddress(String),
}

impl ClientIdentifier {
    /// Get the tier for this client type
    pub fn tier(&self) -> ClientTier {
        match self {
            ClientIdentifier::ApiKey(_) => ClientTier::Authenticated,
            ClientIdentifier::User(_) => ClientTier::Authenticated,
            ClientIdentifier::IpAddress(_) => ClientTier::Anonymous,
        }
    }

    /// Get the identifier string for rate limit key
    pub fn as_key(&self) -> String {
        match self {
            ClientIdentifier::ApiKey(key) => format!("apikey:{}", key),
            ClientIdentifier::User(id) => format!("user:{}", id),
            ClientIdentifier::IpAddress(ip) => format!("ip:{}", ip),
        }
    }
}

/// Client tier for rate limiting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientTier {
    Anonymous,
    Authenticated,
    Premium,
}

/// Rate limiter state
pub struct RateLimiter {
    redis_connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    endpoint_configs: Arc<RwLock<HashMap<String, RateLimitConfig>>>,
    fallback_memory_store: Arc<RwLock<HashMap<String, (u32, i64)>>>,
    db_pool: Option<sqlx::SqlitePool>,
}

impl RateLimiter {
    pub async fn new() -> anyhow::Result<Self> {
        Self::new_with_db(None).await
    }

    pub async fn new_with_db(db_pool: Option<sqlx::SqlitePool>) -> anyhow::Result<Self> {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let connection = if let Ok(client) = redis::Client::open(redis_url.as_str()) {
            match client.get_multiplexed_tokio_connection().await {
                Ok(conn) => {
                    tracing::info!("Connected to Redis for rate limiting");
                    Some(conn)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to connect to Redis ({}), using memory-only rate limiting",
                        e
                    );
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
            db_pool,
        })
    }

    /// Register a rate limit config for an endpoint
    pub async fn register_endpoint(&self, path: String, config: RateLimitConfig) {
        self.endpoint_configs.write().await.insert(path, config);
    }

    /// Extract client identifier from request
    async fn extract_client_identifier(&self, req: &Request) -> ClientIdentifier {
        // Try to extract API key from Authorization header
        if let Some(auth_header) = req.headers().get(header::AUTHORIZATION) {
            if let Ok(auth_str) = auth_header.to_str() {
                // Check for API key format: "Bearer si_live_..."
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    if token.starts_with("si_live_") || token.starts_with("si_test_") {
                        // Validate API key against database if available
                        if let Some(pool) = &self.db_pool {
                            let key_hash = hash_api_key(token);
                            if let Ok(Some(api_key)) =
                                self.get_api_key_by_hash(pool, &key_hash).await
                            {
                                // Update last_used_at timestamp
                                let _ = self.update_api_key_last_used(pool, &api_key.id).await;
                                return ClientIdentifier::ApiKey(api_key.id);
                            }
                        }
                    }
                }
            }
        }

        // Try to extract authenticated user from extensions (set by auth middleware)
        if let Some(auth_user) = req.extensions().get::<crate::auth_middleware::AuthUser>() {
            return ClientIdentifier::User(auth_user.user_id.clone());
        }

        // Fall back to IP address
        if let Some(connect_info) = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>() {
            return ClientIdentifier::IpAddress(connect_info.0.ip().to_string());
        }

        // Default fallback
        ClientIdentifier::IpAddress("unknown".to_string())
    }

    /// Get API key from database by hash
    async fn get_api_key_by_hash(
        &self,
        pool: &sqlx::SqlitePool,
        key_hash: &str,
    ) -> Result<Option<crate::models::api_key::ApiKey>, sqlx::Error> {
        sqlx::query_as::<_, crate::models::api_key::ApiKey>(
            "SELECT * FROM api_keys WHERE key_hash = ? AND status = 'active' AND (expires_at IS NULL OR expires_at > datetime('now'))"
        )
        .bind(key_hash)
        .fetch_optional(pool)
        .await
    }

    /// Update API key last_used_at timestamp
    async fn update_api_key_last_used(
        &self,
        pool: &sqlx::SqlitePool,
        key_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE api_keys SET last_used_at = datetime('now') WHERE id = ?")
            .bind(key_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Get client tier (check for premium status)
    async fn get_client_tier(&self, client: &ClientIdentifier) -> ClientTier {
        // For now, use basic tier logic
        // TODO: Implement premium tier detection from database
        match client {
            ClientIdentifier::ApiKey(_) => ClientTier::Authenticated,
            ClientIdentifier::User(_) => ClientTier::Authenticated,
            ClientIdentifier::IpAddress(_) => ClientTier::Anonymous,
        }
    }

    /// Get rate limit for client based on tier
    fn get_limit_for_client(&self, config: &RateLimitConfig, tier: ClientTier) -> u32 {
        if let Some(client_limits) = &config.client_limits {
            match tier {
                ClientTier::Anonymous => client_limits.anonymous,
                ClientTier::Authenticated => client_limits.authenticated,
                ClientTier::Premium => client_limits.premium,
            }
        } else {
            config.requests_per_minute
        }
    }

    /// Check if IP is in whitelist for an endpoint
    fn is_whitelisted(&self, ip: &str, config: &RateLimitConfig) -> bool {
        config
            .whitelist_ips
            .iter()
            .any(|whitelisted_ip| whitelisted_ip == ip || whitelisted_ip == "*")
    }

    /// Check rate limit for a client/endpoint combination
    pub async fn check_rate_limit_for_client(
        &self,
        client: &ClientIdentifier,
        endpoint: &str,
        ip: &str,
    ) -> (bool, RateLimitInfo) {
        // Get endpoint config
        let configs = self.endpoint_configs.read().await;
        let config = configs.get(endpoint).cloned().unwrap_or_default();

        // Check IP whitelist (still applies for all clients)
        if self.is_whitelisted(ip, &config) {
            return (
                true,
                RateLimitInfo {
                    limit: config.requests_per_minute,
                    remaining: config.requests_per_minute,
                    reset_after: 60,
                    is_whitelisted: true,
                    client_id: Some(client.as_key()),
                },
            );
        }

        // Get client tier and corresponding limit
        let tier = self.get_client_tier(client).await;
        let limit = self.get_limit_for_client(&config, tier);

        let key = format!("ratelimit:{}:{}", endpoint, client.as_key());

        // Try Redis first
        if let Some(conn) = self.redis_connection.read().await.as_ref() {
            let mut conn = conn.clone();
            match self.check_redis_limit(&mut conn, &key, limit).await {
                Ok((allowed, remaining, reset)) => {
                    return (
                        allowed,
                        RateLimitInfo {
                            limit,
                            remaining,
                            reset_after: reset,
                            is_whitelisted: false,
                            client_id: Some(client.as_key()),
                        },
                    );
                }
                Err(_) => {}
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
                client_id: Some(client.as_key()),
            },
        )
    }

    /// Check rate limit for an IP/endpoint combination (legacy method)
    pub async fn check_rate_limit(&self, ip: &str, endpoint: &str) -> (bool, RateLimitInfo) {
        let client = ClientIdentifier::IpAddress(ip.to_string());
        self.check_rate_limit_for_client(&client, endpoint, ip)
            .await
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

        let remaining = if new_count >= limit {
            0
        } else {
            limit - new_count
        };
        Ok((new_count < limit, remaining, 60))
    }

    /// Check rate limit in memory (fallback)
    async fn check_memory_limit(&self, key: &str, limit: u32) -> (bool, u32, u32) {
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
            let remaining = if new_count >= limit {
                0
            } else {
                limit - new_count
            };
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
    pub client_id: Option<String>,
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
            [
                ("RateLimit-Limit", self.info.limit.to_string()),
                ("RateLimit-Remaining", self.info.remaining.to_string()),
                ("RateLimit-Reset", self.info.reset_after.to_string()),
            ],
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

    // Extract client identifier from request
    let client = limiter.extract_client_identifier(&req).await;

    let (allowed, info) = limiter
        .check_rate_limit_for_client(&client, &path, &ip)
        .await;

    if !allowed {
        return RateLimitError { info }.into_response();
    }

    let mut response = next.run(req).await;

    // Add rate limit headers
    response
        .headers_mut()
        .insert("RateLimit-Limit", info.limit.to_string().parse().unwrap());
    response.headers_mut().insert(
        "RateLimit-Remaining",
        info.remaining.to_string().parse().unwrap(),
    );
    response.headers_mut().insert(
        "RateLimit-Reset",
        info.reset_after.to_string().parse().unwrap(),
    );

    // Optionally add client identifier for debugging (sanitized)
    if let Some(client_id) = &info.client_id {
        if let Ok(header_value) = client_id.parse() {
            response
                .headers_mut()
                .insert("X-RateLimit-Client", header_value);
        }
    }

    response
}
