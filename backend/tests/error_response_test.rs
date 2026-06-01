use std::collections::HashMap;
use stellar_insights_backend::error::{ApiError, ErrorResponse};

#[test]
fn test_not_found_error_creation() {
    let error = ApiError::not_found("CORRIDOR_NOT_FOUND", "Corridor not found");

    match error {
        ApiError::NotFound { code, message, .. } => {
            assert_eq!(code, "CORRIDOR_NOT_FOUND");
            assert_eq!(message, "Corridor not found");
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[test]
fn test_not_found_error_with_details() {
    let mut details = HashMap::new();
    details.insert("corridor_id".to_string(), serde_json::json!("USDC-XLM"));

    let error = ApiError::not_found_with_details(
        "CORRIDOR_NOT_FOUND",
        "Corridor with ID 'USDC-XLM' not found",
        details.clone(),
    );

    match error {
        ApiError::NotFound {
            code,
            message,
            details: d,
        } => {
            assert_eq!(code, "CORRIDOR_NOT_FOUND");
            assert_eq!(message, "Corridor with ID 'USDC-XLM' not found");
            assert!(d.is_some());
            let d = d.unwrap();
            assert_eq!(d.get("corridor_id"), details.get("corridor_id"));
        }
        _ => panic!("Expected NotFound error with details"),
    }
}

#[test]
fn test_bad_request_error() {
    let error = ApiError::bad_request("INVALID_INPUT", "Invalid input provided");

    match error {
        ApiError::BadRequest { code, message, .. } => {
            assert_eq!(code, "INVALID_INPUT");
            assert_eq!(message, "Invalid input provided");
        }
        _ => panic!("Expected BadRequest error"),
    }
}

#[test]
fn test_unauthorized_error() {
    let error = ApiError::unauthorized("INVALID_TOKEN", "Invalid authentication token");

    match error {
        ApiError::Unauthorized { code, message, .. } => {
            assert_eq!(code, "INVALID_TOKEN");
            assert_eq!(message, "Invalid authentication token");
        }
        _ => panic!("Expected Unauthorized error"),
    }
}

#[test]
fn test_internal_error() {
    let error = ApiError::internal("INTERNAL_ERROR", "Something went wrong");

    match error {
        ApiError::InternalError { code, message, .. } => {
            assert_eq!(code, "INTERNAL_ERROR");
            assert_eq!(message, "Something went wrong");
        }
        _ => panic!("Expected InternalError"),
    }
}

#[test]
fn test_error_with_details_method() {
    let mut details = HashMap::new();
    details.insert("field".to_string(), serde_json::json!("value"));

    let error = ApiError::not_found("TEST_ERROR", "Test message").with_details(details.clone());

    match error {
        ApiError::NotFound { details: d, .. } => {
            assert!(d.is_some());
            let d = d.unwrap();
            assert_eq!(d.get("field"), details.get("field"));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[test]
fn test_error_response_serialization() {
    let mut details = HashMap::new();
    details.insert("corridor_id".to_string(), serde_json::json!("USDC-XLM"));

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

    // Test JSON serialization
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("CORRIDOR_NOT_FOUND"));
    assert!(json.contains("corridor_id"));
    assert!(json.contains("USDC-XLM"));
    assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));
}

#[test]
fn test_error_response_format() {
    let mut details = HashMap::new();
    details.insert("corridor_id".to_string(), serde_json::json!("USDC-XLM"));

    let error = ApiError::not_found_with_details(
        "CORRIDOR_NOT_FOUND",
        "Corridor with ID 'USDC-XLM' not found",
        details,
    );

    let response =
        error.to_error_response(Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
    let json_value = serde_json::to_value(&response).unwrap();

    // Verify the structure matches the expected format
    assert!(json_value.get("error").is_some());
    let error_obj = json_value.get("error").unwrap();
    assert!(error_obj.get("code").is_some());
    assert!(error_obj.get("message").is_some());
    assert!(error_obj.get("details").is_some());
    assert!(error_obj.get("request_id").is_some());
}

#[test]
fn test_from_anyhow_error() {
    let anyhow_err = anyhow::anyhow!("Test error");
    let api_error: ApiError = anyhow_err.into();

    match api_error {
        ApiError::InternalError { code, message, .. } => {
            assert_eq!(code, "INTERNAL_ERROR");
            assert_eq!(message, "An internal error occurred");
        }
        _ => panic!("Expected InternalError"),
    }
}

#[test]
fn test_stack_trace_in_debug_mode() {
    let error = ApiError::InternalError {
        code: "TEST_ERROR".to_string(),
        message: "Test message".to_string(),
        details: None,
        source: Some("Stack trace here".to_string()),
    };

    let response = error.to_error_response(None);

    // In debug mode (cfg!(debug_assertions)), stack_trace should be included
    if cfg!(debug_assertions) {
        assert!(response.error.stack_trace.is_some());
    } else {
        assert!(response.error.stack_trace.is_none());
    }
}
