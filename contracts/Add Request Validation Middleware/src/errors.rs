use actix_web::{
    error::{JsonPayloadError, ResponseError},
    http::StatusCode,
    HttpResponse,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use validator::ValidationError as ValidatorError;

/// Comprehensive error types for API validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<Vec<FieldError>>,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug)]
pub enum ValidationError {
    InvalidLimit(String),
    InvalidOffset(String),
    InvalidId(String),
    InvalidQuery(String),
    InvalidContentType(String),
    MissingRequired(String),
    InvalidFormat(String),
    RangeError(String),
    LengthError(String),
    InjectionAttempt(String),
    ValidationFailed(Vec<FieldError>),
    ParseError(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidLimit(msg) => write!(f, "Invalid limit parameter: {}", msg),
            ValidationError::InvalidOffset(msg) => write!(f, "Invalid offset parameter: {}", msg),
            ValidationError::InvalidId(msg) => write!(f, "Invalid ID parameter: {}", msg),
            ValidationError::InvalidQuery(msg) => write!(f, "Invalid query parameter: {}", msg),
            ValidationError::InvalidContentType(msg) => write!(f, "Invalid content-type: {}", msg),
            ValidationError::MissingRequired(msg) => write!(f, "Missing required field: {}", msg),
            ValidationError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            ValidationError::RangeError(msg) => write!(f, "Value out of range: {}", msg),
            ValidationError::LengthError(msg) => write!(f, "Invalid length: {}", msg),
            ValidationError::InjectionAttempt(msg) => write!(f, "Suspicious input detected: {}", msg),
            ValidationError::ValidationFailed(_) => write!(f, "Validation failed"),
            ValidationError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl ResponseError for ValidationError {
    fn error_response(&self) -> HttpResponse {
        let (status, error, message, details) = match self {
            ValidationError::InvalidLimit(msg) => (
                StatusCode::BAD_REQUEST,
                "INVALID_LIMIT".to_string(),
                format!("The limit parameter must be between 1 and 1000. {}", msg),
                None,
            ),
            ValidationError::InvalidOffset(msg) => (
                StatusCode::BAD_REQUEST,
                "INVALID_OFFSET".to_string(),
                format!("The offset parameter must be >= 0. {}", msg),
                None,
            ),
            ValidationError::InvalidId(msg) => (
                StatusCode::BAD_REQUEST,
                "INVALID_ID".to_string(),
                format!("The ID parameter is invalid. {}", msg),
                None,
            ),
            ValidationError::InvalidQuery(msg) => (
                StatusCode::BAD_REQUEST,
                "INVALID_QUERY".to_string(),
                msg.clone(),
                None,
            ),
            ValidationError::InvalidContentType(ct) => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "INVALID_CONTENT_TYPE".to_string(),
                format!("Unsupported content-type: {}. Expected application/json", ct),
                None,
            ),
            ValidationError::MissingRequired(field) => (
                StatusCode::BAD_REQUEST,
                "MISSING_REQUIRED".to_string(),
                format!("Required field '{}' is missing", field),
                None,
            ),
            ValidationError::InvalidFormat(msg) => (
                StatusCode::BAD_REQUEST,
                "INVALID_FORMAT".to_string(),
                msg.clone(),
                None,
            ),
            ValidationError::RangeError(msg) => (
                StatusCode::BAD_REQUEST,
                "RANGE_ERROR".to_string(),
                msg.clone(),
                None,
            ),
            ValidationError::LengthError(msg) => (
                StatusCode::BAD_REQUEST,
                "LENGTH_ERROR".to_string(),
                msg.clone(),
                None,
            ),
            ValidationError::InjectionAttempt(msg) => (
                StatusCode::BAD_REQUEST,
                "INJECTION_ATTEMPT".to_string(),
                format!("Suspicious input detected: {}", msg),
                None,
            ),
            ValidationError::ValidationFailed(errors) => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_FAILED".to_string(),
                "One or more validation errors occurred".to_string(),
                Some(errors.clone()),
            ),
            ValidationError::ParseError(msg) => (
                StatusCode::BAD_REQUEST,
                "PARSE_ERROR".to_string(),
                msg.clone(),
                None,
            ),
        };

        HttpResponse::build(status).json(ValidationErrorResponse {
            error,
            message,
            details,
            request_id: None,
        })
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ValidationError::InvalidContentType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

impl From<ValidatorError> for ValidationError {
    fn from(err: ValidatorError) -> Self {
        ValidationError::ParseError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::InvalidLimit("must be positive".to_string());
        assert!(err.to_string().contains("limit"));
    }

    #[test]
    fn test_response_error_status_content_type() {
        let err = ValidationError::InvalidContentType("text/html".to_string());
        assert_eq!(err.status_code(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
}
