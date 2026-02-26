use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Domain-specific errors for business logic and validation rules.
#[derive(Debug, thiserror::Error, Clone)]
pub enum DomainError {
    #[error("Corridor not found: {0}")]
    CorridorNotFound(String),

    #[error("Anchor not found: {0}")]
    AnchorNotFound(String),

    #[error("Invalid asset format: {0}")]
    InvalidAsset(String),

    #[error("Invalid time range: {start} to {end}")]
    InvalidTimeRange { start: String, end: String },

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Unsupported currency or asset: {0}")]
    UnsupportedCurrency(String),

    #[error("Calculation error: {0}")]
    CalculationError(String),
}

/// Structured error response format
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
}

/// Main API error type with structured error codes
#[derive(Debug)]
pub enum ApiError {
    NotFound {
        code: String,
        message: String,
        details: Option<HashMap<String, serde_json::Value>>,
    },
    BadRequest {
        code: String,
        message: String,
        details: Option<HashMap<String, serde_json::Value>>,
    },
    InternalError {
        code: String,
        message: String,
        details: Option<HashMap<String, serde_json::Value>>,
        source: Option<String>,
    },
    Unauthorized {
        code: String,
        message: String,
        details: Option<HashMap<String, serde_json::Value>>,
    },
}

impl ApiError {
    /// Create a NotFound error with a specific code
    pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::NotFound {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a NotFound error with details
    pub fn not_found_with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self::NotFound {
            code: code.into(),
            message: message.into(),
            details: Some(details),
        }
    }

    /// Create a BadRequest error
    pub fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::BadRequest {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a BadRequest error with details
    pub fn bad_request_with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self::BadRequest {
            code: code.into(),
            message: message.into(),
            details: Some(details),
        }
    }

    /// Create an InternalError
    pub fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InternalError {
            code: code.into(),
            message: message.into(),
            details: None,
            source: None,
        }
    }

    /// Create an Unauthorized error
    pub fn unauthorized(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Unauthorized {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Add details to any error variant
    pub fn with_details(mut self, details: HashMap<String, serde_json::Value>) -> Self {
        match &mut self {
            Self::NotFound { details: d, .. }
            | Self::BadRequest { details: d, .. }
            | Self::InternalError { details: d, .. }
            | Self::Unauthorized { details: d, .. } => {
                *d = Some(details);
            }
        }
        self
    }

    /// Get the HTTP status code for this error
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
        }
    }

    /// Convert to ErrorResponse with optional request ID
    pub fn to_error_response(&self, request_id: Option<String>) -> ErrorResponse {
        let include_stack_trace = cfg!(debug_assertions);

        let (code, message, details, source) = match self {
            Self::NotFound {
                code,
                message,
                details,
            } => (code.clone(), message.clone(), details.clone(), None),
            Self::BadRequest {
                code,
                message,
                details,
            } => (code.clone(), message.clone(), details.clone(), None),
            Self::InternalError {
                code,
                message,
                details,
                source,
            } => (
                code.clone(),
                message.clone(),
                details.clone(),
                source.clone(),
            ),
            Self::Unauthorized {
                code,
                message,
                details,
            } => (code.clone(), message.clone(), details.clone(), None),
        };

        ErrorResponse {
            error: ErrorDetail {
                code,
                message,
                details,
                request_id,
                stack_trace: if include_stack_trace { source } else { None },
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = self.to_error_response(None);
        (status, Json(error_response)).into_response()
    }
}

/// Extract request ID from request extensions and create error response
pub fn error_response_with_request_id(error: ApiError, req: &Request) -> Response {
    let status = error.status_code();
    let request_id = req
        .extensions()
        .get::<crate::request_id::RequestId>()
        .map(|id| id.0.clone());
    let error_response = error.to_error_response(request_id);
    (status, Json(error_response)).into_response()
}

/// Convert from anyhow::Error
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::InternalError {
            code: "INTERNAL_ERROR".to_string(),
            message: "An internal error occurred".to_string(),
            details: None,
            source: Some(err.to_string()),
        }
    }
}

/// Convert from sqlx::Error
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        let (code, message) = match &err {
            sqlx::Error::RowNotFound => (
                "NOT_FOUND".to_string(),
                "The requested resource was not found".to_string(),
            ),
            sqlx::Error::Database(db_err) => (
                "DATABASE_ERROR".to_string(),
                format!("Database error: {}", db_err.message()),
            ),
            _ => (
                "INTERNAL_ERROR".to_string(),
                "An internal error occurred".to_string(),
            ),
        };

        Self::InternalError {
            code,
            message,
            details: None,
            source: Some(err.to_string()),
        }
    }
}

/// Convert domain errors into API errors with stable error codes.
impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::CorridorNotFound(id) => Self::NotFound {
                code: "CORRIDOR_NOT_FOUND".to_string(),
                message: format!("Corridor not found: {id}"),
                details: None,
            },
            DomainError::AnchorNotFound(id) => Self::NotFound {
                code: "ANCHOR_NOT_FOUND".to_string(),
                message: format!("Anchor not found: {id}"),
                details: None,
            },
            DomainError::InvalidAsset(msg)
            | DomainError::InvalidConfiguration(msg)
            | DomainError::UnsupportedCurrency(msg)
            | DomainError::CalculationError(msg) => Self::BadRequest {
                code: "DOMAIN_VALIDATION_ERROR".to_string(),
                message: msg,
                details: None,
            },
            DomainError::InvalidTimeRange { start, end } => Self::BadRequest {
                code: "INVALID_TIME_RANGE".to_string(),
                message: format!("Invalid time range: {start} to {end}"),
                details: None,
            },
        }
    }
}

/// Convert RPC errors into API errors so handlers can use `?` consistently.
impl From<crate::rpc::error::RpcError> for ApiError {
    fn from(err: crate::rpc::error::RpcError) -> Self {
        Self::InternalError {
            code: "RPC_ERROR".to_string(),
            message: "External service error".to_string(),
            details: None,
            source: Some(err.to_string()),
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let error = ApiError::not_found("CORRIDOR_NOT_FOUND", "Corridor not found");
        assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_bad_request_error() {
        let error = ApiError::bad_request("INVALID_INPUT", "Invalid input provided");
        assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_internal_error() {
        let error = ApiError::internal("INTERNAL_ERROR", "Something went wrong");
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_unauthorized_error() {
        let error = ApiError::unauthorized("INVALID_TOKEN", "Invalid authentication token");
        assert_eq!(error.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_error_with_details() {
        let mut details = HashMap::new();
        details.insert(
            "corridor_id".to_string(),
            serde_json::Value::String("USDC-XLM".to_string()),
        );

        let error = ApiError::not_found("CORRIDOR_NOT_FOUND", "Corridor not found")
            .with_details(details.clone());

        match error {
            ApiError::NotFound {
                details: Some(d), ..
            } => {
                assert_eq!(d.get("corridor_id"), details.get("corridor_id"));
            }
            _ => panic!("Expected NotFound error with details"),
        }
    }

    #[test]
    fn test_error_response_serialization() {
        let mut details = HashMap::new();
        details.insert(
            "corridor_id".to_string(),
            serde_json::Value::String("USDC-XLM".to_string()),
        );

        let error = ApiError::not_found_with_details(
            "CORRIDOR_NOT_FOUND",
            "Corridor with ID 'USDC-XLM' not found",
            details,
        );

        let response =
            error.to_error_response(Some("550e8400-e29b-41d4-a716-446655440000".to_string()));

        assert_eq!(response.error.code, "CORRIDOR_NOT_FOUND");
        assert_eq!(
            response.error.message,
            "Corridor with ID 'USDC-XLM' not found"
        );
        assert_eq!(
            response.error.request_id,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
        assert!(response.error.details.is_some());
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("Test error");
        let api_error: ApiError = anyhow_err.into();

        match api_error {
            ApiError::InternalError { code, .. } => {
                assert_eq!(code, "INTERNAL_ERROR");
            }
            _ => panic!("Expected InternalError"),
        }
    }
}
