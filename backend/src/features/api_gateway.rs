use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Error, Debug)]
pub enum ApiGatewayError {
    #[error("Route not found: {0}")]
    RouteNotFound(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Service unavailable")]
    ServiceUnavailable,
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub path: String,
    pub method: String,
    pub target_service: String,
    pub timeout_ms: u64,
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiGatewayConfig {
    pub routes: Vec<RouteConfig>,
    pub default_timeout_ms: u64,
}

impl Default for ApiGatewayConfig {
    fn default() -> Self {
        Self {
            routes: vec![],
            default_timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct APIGateway {
    config: ApiGatewayConfig,
    route_cache: Arc<std::sync::Mutex<HashMap<String, RouteConfig>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
    pub route: Option<String>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            success: true,
            message: "Gateway request processed".to_string(),
            route: None,
        }
    }
}

impl APIGateway {
    pub fn new(config: ApiGatewayConfig) -> Self {
        let mut route_cache = HashMap::new();
        for route in &config.routes {
            let key = format!("{} {}", route.method, route.path);
            route_cache.insert(key, route.clone());
        }

        Self {
            config,
            route_cache: Arc::new(std::sync::Mutex::new(route_cache)),
        }
    }

    pub async fn execute(&self, method: &str, path: &str) -> Result<Response, ApiGatewayError> {
        debug!("Processing gateway request: {} {}", method, path);

        let key = format!("{} {}", method, path);
        let cache = self.route_cache.lock().map_err(|e| {
            error!("Failed to acquire route cache lock: {}", e);
            ApiGatewayError::ConfigError("Cache lock error".to_string())
        })?;

        let route = cache
            .get(&key)
            .ok_or_else(|| {
                error!("Route not found: {}", key);
                ApiGatewayError::RouteNotFound(key.clone())
            })?
            .clone();

        info!(
            "Route found: {} -> {} (timeout: {}ms)",
            key, route.target_service, route.timeout_ms
        );

        Ok(Response {
            success: true,
            message: format!("Routed to {}", route.target_service),
            route: Some(route.target_service),
        })
    }

    pub fn register_route(&self, route: RouteConfig) -> Result<(), ApiGatewayError> {
        let key = format!("{} {}", route.method, route.path);
        let mut cache = self.route_cache.lock().map_err(|e| {
            error!("Failed to acquire route cache lock: {}", e);
            ApiGatewayError::ConfigError("Cache lock error".to_string())
        })?;

        cache.insert(key.clone(), route.clone());
        info!("Route registered: {}", key);
        Ok(())
    }

    pub fn get_route(&self, method: &str, path: &str) -> Result<RouteConfig, ApiGatewayError> {
        let key = format!("{} {}", method, path);
        let cache = self.route_cache.lock().map_err(|e| {
            error!("Failed to acquire route cache lock: {}", e);
            ApiGatewayError::ConfigError("Cache lock error".to_string())
        })?;

        cache
            .get(&key)
            .cloned()
            .ok_or_else(|| {
                error!("Route not found: {}", key);
                ApiGatewayError::RouteNotFound(key)
            })
    }

    pub fn list_routes(&self) -> Result<Vec<RouteConfig>, ApiGatewayError> {
        let cache = self.route_cache.lock().map_err(|e| {
            error!("Failed to acquire route cache lock: {}", e);
            ApiGatewayError::ConfigError("Cache lock error".to_string())
        })?;

        Ok(cache.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_gateway_config_default() {
        let config = ApiGatewayConfig::default();
        assert_eq!(config.default_timeout_ms, 30000);
        assert!(config.routes.is_empty());
    }

    #[test]
    fn test_response_default() {
        let response = Response::default();
        assert!(response.success);
        assert_eq!(response.message, "Gateway request processed");
    }

    #[test]
    fn test_api_gateway_creation() {
        let config = ApiGatewayConfig::default();
        let gateway = APIGateway::new(config);
        assert!(gateway.list_routes().unwrap().is_empty());
    }

    #[test]
    fn test_route_registration() {
        let config = ApiGatewayConfig::default();
        let gateway = APIGateway::new(config);

        let route = RouteConfig {
            path: "/api/test".to_string(),
            method: "GET".to_string(),
            target_service: "test-service".to_string(),
            timeout_ms: 5000,
            rate_limit: Some(100),
        };

        assert!(gateway.register_route(route).is_ok());
        assert_eq!(gateway.list_routes().unwrap().len(), 1);
    }
}
