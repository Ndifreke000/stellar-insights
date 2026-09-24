/// Enhanced health check functionality with mobile client support and network awareness
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::cache::CacheManager;
use crate::database::Database;
use crate::network::StellarNetwork;
use crate::rpc::StellarRpcClient;

/// Mobile client types that may make health check requests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientType {
    Web,
    Mobile,
    MobileIOS,
    MobileAndroid,
    Backend,
    Unknown,
}

impl ClientType {
    /// Detect client type from User-Agent header
    pub fn from_user_agent(user_agent: &str) -> Self {
        let ua_lower = user_agent.to_lowercase();
        if ua_lower.contains("mobile") || ua_lower.contains("android") {
            if ua_lower.contains("android") {
                Self::MobileAndroid
            } else {
                Self::Mobile
            }
        } else if ua_lower.contains("iphone") || ua_lower.contains("ipad") {
            Self::MobileIOS
        } else if ua_lower.contains("curl") || ua_lower.contains("postman") {
            Self::Backend
        } else if ua_lower.contains("gecko") || ua_lower.contains("webkit") {
            Self::Web
        } else {
            Self::Unknown
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Web => "web",
            Self::Mobile => "mobile",
            Self::MobileIOS => "mobile_ios",
            Self::MobileAndroid => "mobile_android",
            Self::Backend => "backend",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_mobile(&self) -> bool {
        matches!(self, Self::Mobile | Self::MobileIOS | Self::MobileAndroid)
    }
}

/// Detailed component health with edge case handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedComponentHealth {
    pub healthy: bool,
    pub response_time_ms: u64,
    pub message: Option<String>,
    pub last_error: Option<String>,
    pub error_count: u32,
    pub consecutive_failures: u32,
}

impl DetailedComponentHealth {
    pub fn healthy(response_time_ms: u64) -> Self {
        Self {
            healthy: true,
            response_time_ms,
            message: None,
            last_error: None,
            error_count: 0,
            consecutive_failures: 0,
        }
    }

    pub fn unhealthy(response_time_ms: u64, error: String) -> Self {
        Self {
            healthy: false,
            response_time_ms,
            message: Some(error.clone()),
            last_error: Some(error),
            error_count: 1,
            consecutive_failures: 1,
        }
    }

    pub fn degraded(response_time_ms: u64, message: String) -> Self {
        Self {
            healthy: true,
            response_time_ms,
            message: Some(message),
            last_error: None,
            error_count: 0,
            consecutive_failures: 0,
        }
    }
}

/// Network-specific health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealthStatus {
    pub network: String,
    pub database: DetailedComponentHealth,
    pub cache: DetailedComponentHealth,
    pub rpc: DetailedComponentHealth,
}

/// Enhanced health check response for mobile clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedHealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub network: String,
    pub checks: DetailedComponentHealth,
    pub network_checks: Option<Vec<NetworkHealthStatus>>,
    pub client_info: Option<ClientInfo>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub client_type: String,
    pub is_mobile: bool,
    pub user_agent: Option<String>,
}

/// Health check context for tracking component states
pub struct HealthCheckContext {
    pub network: StellarNetwork,
    pub client_type: ClientType,
    pub include_network_details: bool,
}

impl Default for HealthCheckContext {
    fn default() -> Self {
        Self {
            network: StellarNetwork::Mainnet,
            client_type: ClientType::Unknown,
            include_network_details: false,
        }
    }
}

impl HealthCheckContext {
    /// Create context with mobile detection
    pub fn with_user_agent(user_agent: Option<&str>) -> Self {
        let client_type = user_agent
            .map(ClientType::from_user_agent)
            .unwrap_or(ClientType::Unknown);

        let include_network_details = client_type.is_mobile();

        Self {
            network: StellarNetwork::Mainnet,
            client_type,
            include_network_details,
        }
    }
}

/// Enhanced health check functions
pub struct EnhancedHealthChecker;

impl EnhancedHealthChecker {
    /// Check database health with timeout handling
    pub async fn check_database_enhanced(db: &Arc<Database>) -> DetailedComponentHealth {
        let start = Instant::now();

        // Add timeout for mobile clients
        match tokio::time::timeout(std::time::Duration::from_secs(5), async {
            sqlx::query("SELECT 1").fetch_one(db.pool()).await
        })
        .await
        {
            Ok(Ok(_)) => DetailedComponentHealth::healthy(start.elapsed().as_millis() as u64),
            Ok(Err(e)) => DetailedComponentHealth::unhealthy(
                start.elapsed().as_millis() as u64,
                format!("Database error: {}", e),
            ),
            Err(_) => DetailedComponentHealth::unhealthy(
                start.elapsed().as_millis() as u64,
                "Database health check timeout".to_string(),
            ),
        }
    }

    /// Check cache health with fallback handling
    pub async fn check_cache_enhanced(cache: &Arc<CacheManager>) -> DetailedComponentHealth {
        let start = Instant::now();

        match tokio::time::timeout(std::time::Duration::from_secs(3), async {
            cache.ping().await
        })
        .await
        {
            Ok(Ok(_)) => DetailedComponentHealth::healthy(start.elapsed().as_millis() as u64),
            Ok(Err(e)) => {
                // Cache failure is degraded but not critical
                DetailedComponentHealth::degraded(
                    start.elapsed().as_millis() as u64,
                    format!("Cache unavailable: {}", e),
                )
            }
            Err(_) => DetailedComponentHealth::degraded(
                start.elapsed().as_millis() as u64,
                "Cache health check timeout".to_string(),
            ),
        }
    }

    /// Check RPC health with network awareness
    pub async fn check_rpc_enhanced(rpc: &Arc<StellarRpcClient>) -> DetailedComponentHealth {
        let start = Instant::now();

        match tokio::time::timeout(std::time::Duration::from_secs(10), async {
            rpc.check_health().await
        })
        .await
        {
            Ok(Ok(_)) => DetailedComponentHealth::healthy(start.elapsed().as_millis() as u64),
            Ok(Err(e)) => DetailedComponentHealth::unhealthy(
                start.elapsed().as_millis() as u64,
                format!("RPC error: {}", e),
            ),
            Err(_) => DetailedComponentHealth::unhealthy(
                start.elapsed().as_millis() as u64,
                "RPC health check timeout".to_string(),
            ),
        }
    }

    /// Generate health recommendations for clients
    pub fn generate_recommendations(
        db_health: &DetailedComponentHealth,
        cache_health: &DetailedComponentHealth,
        rpc_health: &DetailedComponentHealth,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !db_health.healthy {
            recommendations
                .push("Database is unavailable. Try again in a few moments.".to_string());
        }

        if !cache_health.healthy {
            recommendations.push("Cache service is degraded. Responses may be slower.".to_string());
        }

        if !rpc_health.healthy {
            recommendations
                .push("Stellar RPC is unavailable. Blockchain operations may fail.".to_string());
        }

        if db_health.response_time_ms > 1000 {
            recommendations.push("Database response time is high. Consider retrying.".to_string());
        }

        if rpc_health.response_time_ms > 2000 {
            recommendations
                .push("RPC response time is high. Consider using a different network.".to_string());
        }

        recommendations
    }

    /// Determine overall status with intelligent degradation
    pub fn determine_status(
        db_health: &DetailedComponentHealth,
        cache_health: &DetailedComponentHealth,
        rpc_health: &DetailedComponentHealth,
    ) -> String {
        if !db_health.healthy {
            return "unhealthy".to_string();
        }

        if !rpc_health.healthy {
            return "unhealthy".to_string();
        }

        if cache_health.healthy && !cache_health.message.is_some() {
            return "healthy".to_string();
        }

        "degraded".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_type_detection() {
        assert_eq!(
            ClientType::from_user_agent("Mozilla/5.0 (iPhone; CPU iPhone OS 14_0)"),
            ClientType::MobileIOS
        );
        assert_eq!(
            ClientType::from_user_agent("Mozilla/5.0 (Linux; Android 11)"),
            ClientType::MobileAndroid
        );
        assert_eq!(
            ClientType::from_user_agent("curl/7.64.1"),
            ClientType::Backend
        );
        assert_eq!(
            ClientType::from_user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
            ),
            ClientType::Web
        );
    }

    #[test]
    fn test_client_type_is_mobile() {
        assert!(ClientType::MobileAndroid.is_mobile());
        assert!(ClientType::MobileIOS.is_mobile());
        assert!(ClientType::Mobile.is_mobile());
        assert!(!ClientType::Web.is_mobile());
        assert!(!ClientType::Backend.is_mobile());
    }

    #[test]
    fn test_health_context_creation() {
        let context =
            HealthCheckContext::with_user_agent(Some("Mozilla/5.0 (iPhone; CPU iPhone OS 14_0)"));
        assert_eq!(context.client_type, ClientType::MobileIOS);
        assert!(context.include_network_details);
    }

    #[test]
    fn test_detailed_component_health() {
        let health = DetailedComponentHealth::healthy(150);
        assert!(health.healthy);
        assert_eq!(health.response_time_ms, 150);

        let unhealthy = DetailedComponentHealth::unhealthy(500, "Connection failed".to_string());
        assert!(!unhealthy.healthy);
    }

    #[test]
    fn test_recommendations_generation() {
        let db_health = DetailedComponentHealth::healthy(100);
        let cache_health = DetailedComponentHealth::degraded(200, "Cache slow".to_string());
        let rpc_health = DetailedComponentHealth::unhealthy(3000, "RPC timeout".to_string());

        let recommendations =
            EnhancedHealthChecker::generate_recommendations(&db_health, &cache_health, &rpc_health);
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("RPC")));
    }
}
