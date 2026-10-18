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

/// Data freshness: how far behind real-time the ingested data is.
///
/// Liveness (can we reach the DB/RPC) hides the most common real failure
/// mode, which is ingestion silently stalling while every other component
/// still reports healthy. This reports the age of the most recently
/// ingested ledger so clients/ops can detect that directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFreshnessStatus {
    pub healthy: bool,
    pub lag_seconds: Option<i64>,
    pub max_lag_seconds: i64,
    pub last_ledger_close_time: Option<DateTime<Utc>>,
    pub message: Option<String>,
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
    /// Whether the database accepted a real write, not just a `SELECT 1`.
    pub db_writable: DetailedComponentHealth,
    /// How stale ingested data is; see [`DataFreshnessStatus`].
    pub data_freshness: DataFreshnessStatus,
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

/// Default maximum age (in seconds) a ledger can be before ingestion is
/// considered unhealthy. Configurable via `INGESTION_MAX_LAG_SECONDS`.
pub const DEFAULT_MAX_INGESTION_LAG_SECONDS: i64 = 120;

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

    /// Check that the database is actually writable, not just reachable.
    /// A `SELECT 1` can succeed against a read-only replica, a disk that is
    /// full, or a database stuck mid-recovery — none of which show up until
    /// something tries to write. This performs a real upsert against a
    /// dedicated probe table (migration
    /// 050_create_health_check_probe.sql) and reads it back.
    pub async fn check_database_writable_enhanced(db: &Arc<Database>) -> DetailedComponentHealth {
        let start = Instant::now();

        let probe = async {
            sqlx::query(
                r"
                INSERT INTO health_check_probe (id, checked_at)
                VALUES (1, $1)
                ON CONFLICT (id) DO UPDATE SET checked_at = excluded.checked_at
                ",
            )
            .bind(Utc::now())
            .execute(db.pool())
            .await?;

            sqlx::query_as::<_, (DateTime<Utc>,)>(
                "SELECT checked_at FROM health_check_probe WHERE id = 1",
            )
            .fetch_one(db.pool())
            .await
        };

        match tokio::time::timeout(std::time::Duration::from_secs(5), probe).await {
            Ok(Ok(_)) => DetailedComponentHealth::healthy(start.elapsed().as_millis() as u64),
            Ok(Err(e)) => DetailedComponentHealth::unhealthy(
                start.elapsed().as_millis() as u64,
                format!("Database is not writable: {}", e),
            ),
            Err(_) => DetailedComponentHealth::unhealthy(
                start.elapsed().as_millis() as u64,
                "Database writability check timed out".to_string(),
            ),
        }
    }

    /// Check ingestion lag: how far behind real-time is the most recently
    /// ingested ledger for `network`. Unhealthy once the lag exceeds
    /// `max_lag_seconds`.
    pub async fn check_ingestion_lag_enhanced(
        db: &Arc<Database>,
        network: StellarNetwork,
        max_lag_seconds: i64,
    ) -> DataFreshnessStatus {
        let row = sqlx::query_as::<_, (DateTime<Utc>,)>(
            "SELECT close_time FROM ledgers WHERE network = $1 ORDER BY sequence DESC LIMIT 1",
        )
        .bind(network.to_string())
        .fetch_optional(db.pool())
        .await;

        match row {
            Ok(Some((close_time,))) => {
                let lag_seconds = (Utc::now() - close_time).num_seconds();
                let healthy = lag_seconds <= max_lag_seconds;
                DataFreshnessStatus {
                    healthy,
                    lag_seconds: Some(lag_seconds),
                    max_lag_seconds,
                    last_ledger_close_time: Some(close_time),
                    message: if healthy {
                        None
                    } else {
                        Some(format!(
                            "Ingestion is {lag_seconds}s behind (max allowed {max_lag_seconds}s)"
                        ))
                    },
                }
            }
            Ok(None) => DataFreshnessStatus {
                healthy: false,
                lag_seconds: None,
                max_lag_seconds,
                last_ledger_close_time: None,
                message: Some(format!("No ledgers ingested yet for network '{network}'")),
            },
            Err(e) => DataFreshnessStatus {
                healthy: false,
                lag_seconds: None,
                max_lag_seconds,
                last_ledger_close_time: None,
                message: Some(format!("Failed to read ingestion lag: {e}")),
            },
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

    /// Generate recommendations that also account for writability and
    /// ingestion freshness.
    pub fn generate_recommendations_full(
        db_health: &DetailedComponentHealth,
        cache_health: &DetailedComponentHealth,
        rpc_health: &DetailedComponentHealth,
        db_writable: &DetailedComponentHealth,
        freshness: &DataFreshnessStatus,
    ) -> Vec<String> {
        let mut recommendations =
            Self::generate_recommendations(db_health, cache_health, rpc_health);

        if !db_writable.healthy {
            recommendations.push(
                "Database rejected a write. It may be read-only, full, or recovering."
                    .to_string(),
            );
        }

        if !freshness.healthy {
            match freshness.lag_seconds {
                Some(lag) => recommendations.push(format!(
                    "Ingestion is {lag}s behind (threshold {}s). Data may be stale.",
                    freshness.max_lag_seconds
                )),
                None => recommendations
                    .push("No ingested data found; ingestion may not have started.".to_string()),
            }
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

    /// Determine overall status including DB writability and ingestion
    /// freshness. Unhealthy when the DB can't be written to or ingestion
    /// lag exceeds its threshold — liveness alone (`determine_status`) is
    /// not enough to catch either failure mode.
    pub fn determine_status_full(
        db_health: &DetailedComponentHealth,
        cache_health: &DetailedComponentHealth,
        rpc_health: &DetailedComponentHealth,
        db_writable: &DetailedComponentHealth,
        freshness: &DataFreshnessStatus,
    ) -> String {
        if !db_writable.healthy || !freshness.healthy {
            return "unhealthy".to_string();
        }

        Self::determine_status(db_health, cache_health, rpc_health)
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

    fn fresh_status(lag_seconds: i64, max_lag_seconds: i64) -> DataFreshnessStatus {
        DataFreshnessStatus {
            healthy: lag_seconds <= max_lag_seconds,
            lag_seconds: Some(lag_seconds),
            max_lag_seconds,
            last_ledger_close_time: Some(Utc::now()),
            message: None,
        }
    }

    #[test]
    fn test_determine_status_full_unhealthy_when_lag_exceeds_threshold() {
        let db_health = DetailedComponentHealth::healthy(10);
        let cache_health = DetailedComponentHealth::healthy(10);
        let rpc_health = DetailedComponentHealth::healthy(10);
        let db_writable = DetailedComponentHealth::healthy(10);

        // Within threshold: everything else healthy -> healthy.
        let ok_freshness = fresh_status(30, DEFAULT_MAX_INGESTION_LAG_SECONDS);
        assert_eq!(
            EnhancedHealthChecker::determine_status_full(
                &db_health,
                &cache_health,
                &rpc_health,
                &db_writable,
                &ok_freshness
            ),
            "healthy"
        );

        // Lag exceeds threshold -> unhealthy, even though every other
        // component reports healthy (this is the exact scenario liveness
        // checks miss).
        let stale_freshness = fresh_status(600, DEFAULT_MAX_INGESTION_LAG_SECONDS);
        assert_eq!(
            EnhancedHealthChecker::determine_status_full(
                &db_health,
                &cache_health,
                &rpc_health,
                &db_writable,
                &stale_freshness
            ),
            "unhealthy"
        );
    }

    #[test]
    fn test_determine_status_full_unhealthy_when_db_not_writable() {
        let db_health = DetailedComponentHealth::healthy(10);
        let cache_health = DetailedComponentHealth::healthy(10);
        let rpc_health = DetailedComponentHealth::healthy(10);
        let not_writable =
            DetailedComponentHealth::unhealthy(10, "disk full".to_string());
        let freshness = fresh_status(10, DEFAULT_MAX_INGESTION_LAG_SECONDS);

        assert_eq!(
            EnhancedHealthChecker::determine_status_full(
                &db_health,
                &cache_health,
                &rpc_health,
                &not_writable,
                &freshness
            ),
            "unhealthy"
        );
    }

    #[tokio::test]
    async fn test_check_database_writable_enhanced() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let db = Arc::new(Database::new(pool));

        let health = EnhancedHealthChecker::check_database_writable_enhanced(&db).await;
        assert!(health.healthy);
    }

    #[tokio::test]
    async fn test_check_ingestion_lag_enhanced_no_data() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let db = Arc::new(Database::new(pool));

        let freshness = EnhancedHealthChecker::check_ingestion_lag_enhanced(
            &db,
            StellarNetwork::Mainnet,
            DEFAULT_MAX_INGESTION_LAG_SECONDS,
        )
        .await;
        assert!(!freshness.healthy);
        assert!(freshness.lag_seconds.is_none());
    }

    #[tokio::test]
    async fn test_check_ingestion_lag_enhanced_fresh_ledger() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let db = Arc::new(Database::new(pool));

        sqlx::query(
            "INSERT INTO ledgers (sequence, hash, close_time, network) VALUES (1, 'h', $1, 'mainnet')",
        )
        .bind(Utc::now())
        .execute(db.pool())
        .await
        .unwrap();

        let freshness = EnhancedHealthChecker::check_ingestion_lag_enhanced(
            &db,
            StellarNetwork::Mainnet,
            DEFAULT_MAX_INGESTION_LAG_SECONDS,
        )
        .await;
        assert!(freshness.healthy);
        assert_eq!(freshness.lag_seconds.unwrap(), 0);

        // A ledger on a different network must not count toward this
        // network's freshness.
        let testnet_freshness = EnhancedHealthChecker::check_ingestion_lag_enhanced(
            &db,
            StellarNetwork::Testnet,
            DEFAULT_MAX_INGESTION_LAG_SECONDS,
        )
        .await;
        assert!(!testnet_freshness.healthy);
    }
}
