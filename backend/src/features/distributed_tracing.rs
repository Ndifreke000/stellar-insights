use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum DistributedTracingError {
    #[error("Trace not found: {0}")]
    TraceNotFound(String),
    #[error("Invalid span: {0}")]
    InvalidSpan(String),
    #[error("Storage error: {0}")]
    StorageError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanConfig {
    pub operation_name: String,
    pub service_name: String,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTracingConfig {
    pub enabled: bool,
    pub sample_rate: f64,
    pub max_spans_per_trace: u32,
}

impl Default for DistributedTracingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sample_rate: 0.1,
            max_spans_per_trace: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub service_name: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub tags: HashMap<String, String>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct DistributedTracing {
    config: DistributedTracingConfig,
    spans: Arc<std::sync::Mutex<HashMap<String, Vec<Span>>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
    pub trace_id: Option<String>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            success: true,
            message: "Distributed tracing operation completed".to_string(),
            trace_id: None,
        }
    }
}

impl DistributedTracing {
    pub fn new(config: DistributedTracingConfig) -> Self {
        Self {
            config,
            spans: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    pub async fn execute(&self) -> Result<Response, DistributedTracingError> {
        debug!("Executing distributed tracing operation");

        if !self.config.enabled {
            return Ok(Response {
                success: true,
                message: "Distributed tracing is disabled".to_string(),
                trace_id: None,
            });
        }

        let trace_id = Uuid::new_v4().to_string();
        info!("Created new trace: {}", trace_id);

        Ok(Response {
            success: true,
            message: "Trace created successfully".to_string(),
            trace_id: Some(trace_id),
        })
    }

    pub fn start_span(
        &self,
        trace_id: &str,
        config: SpanConfig,
    ) -> Result<Span, DistributedTracingError> {
        debug!("Starting span for trace: {}", trace_id);

        let span_id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| DistributedTracingError::StorageError(e.to_string()))?
            .as_millis() as i64;

        let span = Span {
            span_id: span_id.clone(),
            trace_id: trace_id.to_string(),
            parent_span_id: None,
            operation_name: config.operation_name,
            service_name: config.service_name,
            start_time: now,
            end_time: None,
            tags: config.tags,
            status: "ACTIVE".to_string(),
        };

        let mut spans = self.spans.lock().map_err(|e| {
            error!("Failed to acquire spans lock: {}", e);
            DistributedTracingError::StorageError("Lock error".to_string())
        })?;

        let trace_spans = spans.entry(trace_id.to_string()).or_insert_with(Vec::new);

        if trace_spans.len() >= self.config.max_spans_per_trace as usize {
            error!("Max spans per trace exceeded for trace: {}", trace_id);
            return Err(DistributedTracingError::InvalidSpan(
                "Max spans exceeded".to_string(),
            ));
        }

        trace_spans.push(span.clone());
        info!("Span started: {} for trace: {}", span_id, trace_id);

        Ok(span)
    }

    pub fn end_span(&self, trace_id: &str, span_id: &str) -> Result<(), DistributedTracingError> {
        debug!("Ending span: {} for trace: {}", span_id, trace_id);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| DistributedTracingError::StorageError(e.to_string()))?
            .as_millis() as i64;

        let mut spans = self.spans.lock().map_err(|e| {
            error!("Failed to acquire spans lock: {}", e);
            DistributedTracingError::StorageError("Lock error".to_string())
        })?;

        let trace_spans = spans.get_mut(trace_id).ok_or_else(|| {
            error!("Trace not found: {}", trace_id);
            DistributedTracingError::TraceNotFound(trace_id.to_string())
        })?;

        let span = trace_spans
            .iter_mut()
            .find(|s| s.span_id == span_id)
            .ok_or_else(|| {
                error!("Span not found: {}", span_id);
                DistributedTracingError::InvalidSpan(span_id.to_string())
            })?;

        span.end_time = Some(now);
        span.status = "COMPLETED".to_string();

        info!("Span ended: {} for trace: {}", span_id, trace_id);
        Ok(())
    }

    pub fn get_trace(&self, trace_id: &str) -> Result<Vec<Span>, DistributedTracingError> {
        let spans = self.spans.lock().map_err(|e| {
            error!("Failed to acquire spans lock: {}", e);
            DistributedTracingError::StorageError("Lock error".to_string())
        })?;

        spans.get(trace_id).cloned().ok_or_else(|| {
            error!("Trace not found: {}", trace_id);
            DistributedTracingError::TraceNotFound(trace_id.to_string())
        })
    }

    pub fn list_traces(&self) -> Result<Vec<String>, DistributedTracingError> {
        let spans = self.spans.lock().map_err(|e| {
            error!("Failed to acquire spans lock: {}", e);
            DistributedTracingError::StorageError("Lock error".to_string())
        })?;

        Ok(spans.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distributed_tracing_config_default() {
        let config = DistributedTracingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.sample_rate, 0.1);
        assert_eq!(config.max_spans_per_trace, 1000);
    }

    #[test]
    fn test_response_default() {
        let response = Response::default();
        assert!(response.success);
        assert_eq!(response.message, "Distributed tracing operation completed");
    }

    #[test]
    fn test_distributed_tracing_creation() {
        let config = DistributedTracingConfig::default();
        let tracing = DistributedTracing::new(config);
        assert!(tracing.list_traces().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_trace_creation() {
        let config = DistributedTracingConfig::default();
        let tracing = DistributedTracing::new(config);

        let response = tracing.execute().await.unwrap();
        assert!(response.success);
        assert!(response.trace_id.is_some());
    }

    #[test]
    fn test_span_creation() {
        let config = DistributedTracingConfig::default();
        let tracing = DistributedTracing::new(config);

        let trace_id = "test-trace-123";
        let span_config = SpanConfig {
            operation_name: "test-operation".to_string(),
            service_name: "test-service".to_string(),
            tags: HashMap::new(),
        };

        let span = tracing.start_span(trace_id, span_config).unwrap();
        assert_eq!(span.trace_id, trace_id);
        assert_eq!(span.status, "ACTIVE");
    }
}
