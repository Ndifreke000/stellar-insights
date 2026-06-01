# Structured Error Responses

This document describes the structured error response system implemented for the Stellar Insights backend API.

## Overview

The API now returns consistent, structured error responses that include error codes, helpful messages, optional details, and request IDs for tracing.

## Error Response Format

All error responses follow this structure:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {
      "field_name": "additional context"
    },
    "request_id": "550e8400-e29b-41d4-a716-446655440000",
    "stack_trace": "Only included in development mode"
  }
}
```

## Error Codes

### Not Found Errors (404)
- `ANCHOR_NOT_FOUND` - Anchor with specified ID not found
- `CORRIDOR_NOT_FOUND` - Corridor with specified ID not found
- `NOT_IMPLEMENTED` - Feature not yet implemented

### Bad Request Errors (400)
- `INVALID_INPUT` - Invalid input provided
- `INVALID_CORRIDOR_FORMAT` - Corridor key format is invalid

### Unauthorized Errors (401)
- `INVALID_CREDENTIALS` - Invalid username or password
- `INVALID_TOKEN` - Invalid or expired authentication token

### Internal Server Errors (500)
- `INTERNAL_ERROR` - Generic internal server error
- `DATABASE_ERROR` - Database operation failed

## Usage Examples

### Creating Errors

```rust
use crate::error::ApiError;
use std::collections::HashMap;

// Simple not found error
let error = ApiError::not_found("CORRIDOR_NOT_FOUND", "Corridor not found");

// Not found error with details
let mut details = HashMap::new();
details.insert("corridor_id".to_string(), serde_json::json!("USDC-XLM"));
let error = ApiError::not_found_with_details(
    "CORRIDOR_NOT_FOUND",
    "Corridor with ID 'USDC-XLM' not found",
    details,
);

// Bad request error
let error = ApiError::bad_request("INVALID_INPUT", "Name cannot be empty");

// Unauthorized error
let error = ApiError::unauthorized("INVALID_TOKEN", "Invalid authentication token");

// Internal error
let error = ApiError::internal("DATABASE_ERROR", "Failed to fetch data");
```

### Adding Details to Errors

```rust
let mut details = HashMap::new();
details.insert("field".to_string(), serde_json::json!("value"));

let error = ApiError::not_found("RESOURCE_NOT_FOUND", "Resource not found")
    .with_details(details);
```

### Automatic Conversions

The error system automatically converts from common error types:

```rust
// From anyhow::Error
let result: Result<T, ApiError> = some_operation()
    .await?; // Automatically converts anyhow::Error to ApiError

// From sqlx::Error
let result: Result<T, ApiError> = db.query()
    .await?; // Automatically converts sqlx::Error to ApiError
```

## Request ID Integration

Request IDs are automatically included in error responses when the request_id middleware is active. The request ID helps trace errors across logs and responses.

## Development vs Production

- **Development Mode** (`cfg!(debug_assertions)`): Stack traces are included in error responses
- **Production Mode**: Stack traces are omitted for security

## Migration Guide

### Before
```rust
return Err(ApiError::NotFound(format!("Anchor with id {} not found", id)));
```

### After
```rust
let mut details = HashMap::new();
details.insert("anchor_id".to_string(), serde_json::json!(id.to_string()));
return Err(ApiError::not_found_with_details(
    "ANCHOR_NOT_FOUND",
    format!("Anchor with id {} not found", id),
    details,
));
```

## Testing

Comprehensive tests are available in `tests/error_response_test.rs` covering:
- Error creation
- Error with details
- Error response serialization
- JSON format validation
- Automatic conversions
- Stack trace behavior

Run tests with:
```bash
cargo test error_response
```

## Benefits

1. **Consistency**: All errors follow the same structure
2. **Debuggability**: Request IDs and error codes make debugging easier
3. **Client-Friendly**: Structured errors are easier for clients to parse and handle
4. **Traceability**: Request IDs link errors to specific requests in logs
5. **Context**: Details field provides additional context about errors
6. **Security**: Stack traces only in development mode
