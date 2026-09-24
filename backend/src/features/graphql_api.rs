use crate::models::graphql_api::{GraphQLErrorDetail, GraphQLHealthStatus, GraphQLRequest, GraphQLResponse};
use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema, SimpleObject};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Error, Debug)]
pub enum GraphQLAPIError {
    #[error("GraphQL API is disabled")]
    Disabled,
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("Query execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Query too deep (max depth: {0})")]
    QueryTooDeep(u32),
}

#[derive(Debug, Clone)]
pub struct GraphQLAPIConfig {
    pub enabled: bool,
    pub path: String,
    pub max_query_depth: u32,
    pub version: String,
}

impl Default for GraphQLAPIConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: "/graphql".to_string(),
            max_query_depth: 10,
            version: "1.0.0".to_string(),
        }
    }
}

#[derive(SimpleObject)]
struct HealthType {
    status: String,
    version: String,
}

#[derive(SimpleObject)]
struct AnchorSummaryType {
    count: i32,
}

struct QueryRoot {
    version: String,
    anchor_count: Arc<AtomicU64>,
}

#[Object]
impl QueryRoot {
    async fn health(&self) -> HealthType {
        HealthType {
            status: "ok".to_string(),
            version: self.version.clone(),
        }
    }

    async fn anchor_count(&self) -> AnchorSummaryType {
        AnchorSummaryType {
            count: self.anchor_count.load(Ordering::Relaxed) as i32,
        }
    }
}

type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub struct GraphQLAPI {
    config: GraphQLAPIConfig,
    schema: AppSchema,
    request_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
}

impl GraphQLAPI {
    pub fn new(config: GraphQLAPIConfig, anchor_count: u64) -> Self {
        let counter = Arc::new(AtomicU64::new(anchor_count));
        let schema = Schema::build(
            QueryRoot {
                version: config.version.clone(),
                anchor_count: Arc::clone(&counter),
            },
            EmptyMutation,
            EmptySubscription,
        )
        .finish();

        info!(
            "GraphQL API initialized at {} (enabled={})",
            config.path, config.enabled
        );

        Self {
            config,
            schema,
            request_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn config(&self) -> &GraphQLAPIConfig {
        &self.config
    }

    pub fn health_status(&self) -> GraphQLHealthStatus {
        GraphQLHealthStatus {
            enabled: self.config.enabled,
            endpoint: self.config.path.clone(),
            version: self.config.version.clone(),
        }
    }

    pub fn metrics(&self) -> (u64, u64) {
        (
            self.request_count.load(Ordering::Relaxed),
            self.error_count.load(Ordering::Relaxed),
        )
    }

    fn validate_query_depth(&self, query: &str) -> Result<(), GraphQLAPIError> {
        let depth = query.matches('{').count() as u32;
        if depth > self.config.max_query_depth {
            warn!("GraphQL query depth {} exceeds max {}", depth, self.config.max_query_depth);
            return Err(GraphQLAPIError::QueryTooDeep(self.config.max_query_depth));
        }
        Ok(())
    }

    pub async fn execute(&self, request: GraphQLRequest) -> Result<GraphQLResponse, GraphQLAPIError> {
        if !self.config.enabled {
            return Err(GraphQLAPIError::Disabled);
        }

        if request.query.trim().is_empty() {
            return Err(GraphQLAPIError::InvalidQuery("Query must not be empty".to_string()));
        }

        self.validate_query_depth(&request.query)?;
        self.request_count.fetch_add(1, Ordering::Relaxed);

        let start = Instant::now();
        debug!("Executing GraphQL query: {}", request.query);

        let result = self
            .schema
            .execute(async_graphql::Request::new(request.query))
            .await;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        if !result.errors.is_empty() {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            let errors: Vec<GraphQLErrorDetail> = result
                .errors
                .iter()
                .map(|e| GraphQLErrorDetail {
                    message: e.message.clone(),
                    path: None, // Simplified for now
                })
                .collect();

            error!("GraphQL query returned {} error(s)", errors.len());

            return Ok(GraphQLResponse {
                success: false,
                data: result.data.into_json().ok(),
                errors: Some(errors),
                execution_time_ms,
            });
        }

        info!("GraphQL query executed in {}ms", execution_time_ms);

        Ok(GraphQLResponse {
            success: true,
            data: result.data.into_json().ok(),
            errors: None,
            execution_time_ms,
        })
    }
}

pub async fn graphql_handler(
    axum::Extension(api): axum::Extension<Arc<GraphQLAPI>>,
    axum::Json(request): axum::Json<GraphQLRequest>,
) -> axum::Json<GraphQLResponse> {
    match api.execute(request).await {
        Ok(response) => axum::Json(response),
        Err(err) => axum::Json(GraphQLResponse {
            success: false,
            data: None,
            errors: Some(vec![GraphQLErrorDetail {
                message: err.to_string(),
                path: None,
            }]),
            execution_time_ms: 0,
        }),
    }
}

pub async fn graphql_health_handler(
    axum::Extension(api): axum::Extension<Arc<GraphQLAPI>>,
) -> axum::Json<GraphQLHealthStatus> {
    axum::Json(api.health_status())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_api() -> GraphQLAPI {
        GraphQLAPI::new(GraphQLAPIConfig::default(), 42)
    }

    #[test]
    fn test_graphql_api_config_default() {
        let config = GraphQLAPIConfig::default();
        assert!(config.enabled);
        assert_eq!(config.path, "/graphql");
        assert_eq!(config.max_query_depth, 10);
    }

    #[test]
    fn test_health_status() {
        let api = test_api();
        let health = api.health_status();
        assert!(health.enabled);
        assert_eq!(health.endpoint, "/graphql");
    }

    #[tokio::test]
    async fn test_execute_health_query() {
        let api = test_api();
        let response = api
            .execute(GraphQLRequest {
                query: "{ health { status version } }".to_string(),
                variables: None,
                operation_name: None,
            })
            .await
            .unwrap();

        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.errors.is_none());
        assert!(response.execution_time_ms < 5000);
    }

    #[tokio::test]
    async fn test_execute_anchor_count_query() {
        let api = test_api();
        let response = api
            .execute(GraphQLRequest {
                query: "{ anchorCount { count } }".to_string(),
                variables: None,
                operation_name: None,
            })
            .await
            .unwrap();

        assert!(response.success);
        let data = response.data.unwrap();
        assert_eq!(data["anchorCount"]["count"], 42);
    }

    #[tokio::test]
    async fn test_execute_empty_query_fails() {
        let api = test_api();
        let result = api
            .execute(GraphQLRequest {
                query: "   ".to_string(),
                variables: None,
                operation_name: None,
            })
            .await;

        assert!(matches!(result, Err(GraphQLAPIError::InvalidQuery(_))));
    }

    #[tokio::test]
    async fn test_execute_disabled_api() {
        let api = GraphQLAPI::new(
            GraphQLAPIConfig {
                enabled: false,
                ..GraphQLAPIConfig::default()
            },
            0,
        );

        let result = api
            .execute(GraphQLRequest {
                query: "{ health { status } }".to_string(),
                variables: None,
                operation_name: None,
            })
            .await;

        assert!(matches!(result, Err(GraphQLAPIError::Disabled)));
    }

    #[tokio::test]
    async fn test_query_depth_validation() {
        let api = GraphQLAPI::new(
            GraphQLAPIConfig {
                max_query_depth: 2,
                ..GraphQLAPIConfig::default()
            },
            0,
        );

        let result = api
            .execute(GraphQLRequest {
                query: "{ health { status version } anchorCount { count } }".to_string(),
                variables: None,
                operation_name: None,
            })
            .await;

        assert!(matches!(result, Err(GraphQLAPIError::QueryTooDeep(_))));
    }

    #[tokio::test]
    async fn test_metrics_increment() {
        let api = test_api();
        let (requests_before, _) = api.metrics();

        api.execute(GraphQLRequest {
            query: "{ health { status } }".to_string(),
            variables: None,
            operation_name: None,
        })
        .await
        .unwrap();

        let (requests_after, _) = api.metrics();
        assert_eq!(requests_after, requests_before + 1);
    }
}
