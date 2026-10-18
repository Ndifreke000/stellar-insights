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

    #[error("Payment already used: {0}")]
    PaymentAlreadyUsed(String),
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
    Conflict {
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
    Forbidden {
        code: String,
        message: String,
        details: Option<HashMap<String, serde_json::Value>>,
    },
    ServiceUnavailable {
        code: String,
        message: String,
        details: Option<HashMap<String, serde_json::Value>>,
    },
}

pub type AppError = ApiError;

/// Error code returned when a transaction hash has already been consumed to
/// confirm an invoice. Prevents replaying one payment across many invoices.
pub const PAYMENT_ALREADY_USED: &str = "PAYMENT_ALREADY_USED";

impl ApiError {
    /// Create a `NotFound` error with a specific code
    pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::NotFound {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a `NotFound` error with details
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

    /// Create a `BadRequest` error
    pub fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::BadRequest {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a `BadRequest` error with details
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

    /// Create a `Conflict` error
    pub fn conflict(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Conflict {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a `Conflict` error with details
    pub fn conflict_with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self::Conflict {
            code: code.into(),
            message: message.into(),
            details: Some(details),
        }
    }

    /// Create the standard `PAYMENT_ALREADY_USED` conflict error.
    ///
    /// Returned when a transaction hash has already been consumed to confirm
    /// an invoice, so a single payment cannot be replayed across invoices.
    pub fn payment_already_used(tx_hash: impl Into<String>) -> Self {
        let tx_hash = tx_hash.into();
        let mut details = HashMap::new();
        details.insert(
            "tx_hash".to_string(),
            serde_json::Value::String(tx_hash.clone()),
        );
        Self::Conflict {
            code: PAYMENT_ALREADY_USED.to_string(),
            message: "This payment has already been used to confirm an invoice".to_string(),
            details: Some(details),
        }
    }

    /// Create an `InternalError`
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

    /// Create a forbidden error.
    pub fn forbidden(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Forbidden {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create a ServiceUnavailable error
    pub fn service_unavailable(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Add details to any error variant
    #[must_use]
    pub fn with_details(mut self, details: HashMap<String, serde_json::Value>) -> Self {
        match &mut self {
            Self::NotFound { details: d, .. }
            | Self::BadRequest { details: d, .. }
            | Self::Conflict { details: d, .. }
            | Self::InternalError { details: d, .. }
            | Self::Unauthorized { details: d, .. }
            | Self::Forbidden { details: d, .. }
            | Self::ServiceUnavailable { details: d, .. } => {
                *d = Some(details);
            }
        }
        self
    }

    /// Get the HTTP status code for this error
    const fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            Self::Forbidden { .. } => StatusCode::FORBIDDEN,
            Self::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Convert to `ErrorResponse` with optional request ID
    #[must_use]
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
            Self::Conflict {
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
            Self::Forbidden {
                code,
                message,
                details,
            } => (code.clone(), message.clone(), details.clone(), None),
            Self::ServiceUnavailable {
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

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { message, .. } => write!(f, "{}", message),
            Self::BadRequest { message, .. } => write!(f, "{}", message),
            Self::Conflict { message, .. } => write!(f, "{}", message),
            Self::InternalError { message, .. } => write!(f, "{}", message),
            Self::Unauthorized { message, .. } => write!(f, "{}", message),
            Self::Forbidden { message, .. } => write!(f, "{}", message),
            Self::ServiceUnavailable { message, .. } => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = self.to_error_response(None);
        (status, Json(body)).into_response()
    }
}

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::CorridorNotFound(id) => {
                Self::not_found("CORRIDOR_NOT_FOUND", format!("Corridor not found: {id}"))
            }
            DomainError::AnchorNotFound(id) => {
                Self::not_found("ANCHOR_NOT_FOUND", format!("Anchor not found: {id}"))
            }
            DomainError::InvalidAsset(asset) => {
                Self::bad_request("INVALID_ASSET", format!("Invalid asset format: {asset}"))
            }
            DomainError::InvalidTimeRange { start, end } => Self::bad_request(
                "INVALID_TIME_RANGE",
                format!("Invalid time range: {start} to {end}"),
            ),
            DomainError::InvalidConfiguration(msg) => {
                Self::bad_request("INVALID_CONFIGURATION", msg)
            }
            DomainError::UnsupportedCurrency(currency) => Self::bad_request(
                "UNSUPPORTED_CURRENCY",
                format!("Unsupported currency or asset: {currency}"),
            ),
            DomainError::CalculationError(msg) => {
                Self::internal("CALCULATION_ERROR", format!("Calculation error: {msg}"))
            }
            DomainError::PaymentAlreadyUsed(tx_hash) => Self::payment_already_used(tx_hash),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_already_used_maps_to_conflict() {
        let err = ApiError::payment_already_used("abc123");
        assert_eq!(err.status_code(), StatusCode::CONFLICT);

        let body = err.to_error_response(None);
        assert_eq!(body.error.code, PAYMENT_ALREADY_USED);
        let details = body.error.details.expect("details should be present");
        assert_eq!(
            details.get("tx_hash"),
            Some(&serde_json::Value::String("abc123".to_string()))
        );
    }

    #[test]
    fn domain_payment_already_used_converts_to_api_error() {
        let api: ApiError = DomainError::PaymentAlreadyUsed("deadbeef".to_string()).into();
        assert_eq!(api.status_code(), StatusCode::CONFLICT);
        assert_eq!(api.to_error_response(None).error.code, PAYMENT_ALREADY_USED);
    }
}
