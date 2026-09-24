use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Error, Debug)]
pub enum ServiceMeshError {
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    #[error("Communication error: {0}")]
    CommunicationError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub health_check_path: String,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshConfig {
    pub services: Vec<ServiceConfig>,
    pub circuit_breaker_threshold: u32,
    pub timeout_ms: u64,
}

impl Default for ServiceMeshConfig {
    fn default() -> Self {
        Self {
            services: vec![],
            circuit_breaker_threshold: 5,
            timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceMesh {
    config: ServiceMeshConfig,
    service_registry: Arc<std::sync::Mutex<HashMap<String, ServiceConfig>>>,
    circuit_breakers: Arc<std::sync::Mutex<HashMap<String, CircuitBreaker>>>,
}

#[derive(Debug, Clone)]
struct CircuitBreaker {
    failure_count: u32,
    is_open: bool,
    last_failure_time: Option<std::time::SystemTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
    pub service: Option<String>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            success: true,
            message: "Service mesh operation completed".to_string(),
            service: None,
        }
    }
}

impl ServiceMesh {
    pub fn new(config: ServiceMeshConfig) -> Self {
        let mut registry = HashMap::new();
        let mut breakers = HashMap::new();

        for service in &config.services {
            registry.insert(service.name.clone(), service.clone());
            breakers.insert(
                service.name.clone(),
                CircuitBreaker {
                    failure_count: 0,
                    is_open: false,
                    last_failure_time: None,
                },
            );
        }

        Self {
            config,
            service_registry: Arc::new(std::sync::Mutex::new(registry)),
            circuit_breakers: Arc::new(std::sync::Mutex::new(breakers)),
        }
    }

    pub async fn execute(&self, service_name: &str) -> Result<Response, ServiceMeshError> {
        debug!("Executing service mesh operation for: {}", service_name);

        let registry = self.service_registry.lock().map_err(|e| {
            error!("Failed to acquire service registry lock: {}", e);
            ServiceMeshError::ConfigError("Registry lock error".to_string())
        })?;

        let service = registry
            .get(service_name)
            .ok_or_else(|| {
                error!("Service not found: {}", service_name);
                ServiceMeshError::ServiceNotFound(service_name.to_string())
            })?
            .clone();

        drop(registry);

        // Check circuit breaker
        let breakers = self.circuit_breakers.lock().map_err(|e| {
            error!("Failed to acquire circuit breaker lock: {}", e);
            ServiceMeshError::ConfigError("Circuit breaker lock error".to_string())
        })?;

        if let Some(breaker) = breakers.get(service_name) {
            if breaker.is_open {
                error!("Circuit breaker open for service: {}", service_name);
                return Err(ServiceMeshError::CommunicationError(
                    "Service circuit breaker is open".to_string(),
                ));
            }
        }

        drop(breakers);

        info!(
            "Service mesh routing to: {} ({}:{})",
            service.name, service.host, service.port
        );

        Ok(Response {
            success: true,
            message: format!("Connected to service: {}", service.name),
            service: Some(service.name),
        })
    }

    pub fn register_service(&self, service: ServiceConfig) -> Result<(), ServiceMeshError> {
        let mut registry = self.service_registry.lock().map_err(|e| {
            error!("Failed to acquire service registry lock: {}", e);
            ServiceMeshError::ConfigError("Registry lock error".to_string())
        })?;

        let mut breakers = self.circuit_breakers.lock().map_err(|e| {
            error!("Failed to acquire circuit breaker lock: {}", e);
            ServiceMeshError::ConfigError("Circuit breaker lock error".to_string())
        })?;

        registry.insert(service.name.clone(), service.clone());
        breakers.insert(
            service.name.clone(),
            CircuitBreaker {
                failure_count: 0,
                is_open: false,
                last_failure_time: None,
            },
        );

        info!("Service registered: {}", service.name);
        Ok(())
    }

    pub fn get_service(&self, service_name: &str) -> Result<ServiceConfig, ServiceMeshError> {
        let registry = self.service_registry.lock().map_err(|e| {
            error!("Failed to acquire service registry lock: {}", e);
            ServiceMeshError::ConfigError("Registry lock error".to_string())
        })?;

        registry.get(service_name).cloned().ok_or_else(|| {
            error!("Service not found: {}", service_name);
            ServiceMeshError::ServiceNotFound(service_name.to_string())
        })
    }

    pub fn list_services(&self) -> Result<Vec<ServiceConfig>, ServiceMeshError> {
        let registry = self.service_registry.lock().map_err(|e| {
            error!("Failed to acquire service registry lock: {}", e);
            ServiceMeshError::ConfigError("Registry lock error".to_string())
        })?;

        Ok(registry.values().cloned().collect())
    }

    pub fn record_failure(&self, service_name: &str) -> Result<(), ServiceMeshError> {
        let mut breakers = self.circuit_breakers.lock().map_err(|e| {
            error!("Failed to acquire circuit breaker lock: {}", e);
            ServiceMeshError::ConfigError("Circuit breaker lock error".to_string())
        })?;

        if let Some(breaker) = breakers.get_mut(service_name) {
            breaker.failure_count += 1;
            breaker.last_failure_time = Some(std::time::SystemTime::now());

            if breaker.failure_count >= self.config.circuit_breaker_threshold {
                breaker.is_open = true;
                error!(
                    "Circuit breaker opened for service: {} (failures: {})",
                    service_name, breaker.failure_count
                );
            }
        }

        Ok(())
    }

    pub fn reset_service(&self, service_name: &str) -> Result<(), ServiceMeshError> {
        let mut breakers = self.circuit_breakers.lock().map_err(|e| {
            error!("Failed to acquire circuit breaker lock: {}", e);
            ServiceMeshError::ConfigError("Circuit breaker lock error".to_string())
        })?;

        if let Some(breaker) = breakers.get_mut(service_name) {
            breaker.failure_count = 0;
            breaker.is_open = false;
            breaker.last_failure_time = None;
            info!("Service reset: {}", service_name);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_mesh_config_default() {
        let config = ServiceMeshConfig::default();
        assert_eq!(config.circuit_breaker_threshold, 5);
        assert_eq!(config.timeout_ms, 30000);
        assert!(config.services.is_empty());
    }

    #[test]
    fn test_response_default() {
        let response = Response::default();
        assert!(response.success);
        assert_eq!(response.message, "Service mesh operation completed");
    }

    #[test]
    fn test_service_mesh_creation() {
        let config = ServiceMeshConfig::default();
        let mesh = ServiceMesh::new(config);
        assert!(mesh.list_services().unwrap().is_empty());
    }

    #[test]
    fn test_service_registration() {
        let config = ServiceMeshConfig::default();
        let mesh = ServiceMesh::new(config);

        let service = ServiceConfig {
            name: "test-service".to_string(),
            host: "localhost".to_string(),
            port: 8080,
            health_check_path: "/health".to_string(),
            retry_count: 3,
        };

        assert!(mesh.register_service(service).is_ok());
        assert_eq!(mesh.list_services().unwrap().len(), 1);
    }
}
