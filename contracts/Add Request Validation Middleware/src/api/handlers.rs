use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use crate::errors::{ValidationError, FieldError};
use crate::validators;
use log::{debug, info};
use validator::Validate;

/// Query parameters for listing corridors
#[derive(Debug, Deserialize, Validate)]
pub struct ListCorridorsQuery {
    #[validate(custom = "validate_limit")]
    pub limit: Option<i64>,

    #[validate(custom = "validate_offset")]
    pub offset: Option<i64>,

    #[validate(length(min = 1, max = 50))]
    pub asset_code: Option<String>,

    #[validate(custom = "validate_success_rate")]
    pub success_rate_min: Option<f64>,

    #[validate(custom = "validate_success_rate")]
    pub success_rate_max: Option<f64>,

    #[validate(length(min = 1, max = 100))]
    pub filter: Option<String>,
}

/// Request body for creating/updating a corridor
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CorridorRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    #[validate(length(min = 0, max = 500))]
    pub description: Option<String>,

    #[validate(length(min = 1, max = 20))]
    pub asset_code: String,

    #[validate(custom = "validate_success_rate")]
    pub success_rate_threshold: Option<f64>,

    #[validate(custom = "validate_status")]
    pub status: Option<String>,
}

/// Response model for corridor
#[derive(Debug, Serialize)]
pub struct CorridorResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub asset_code: String,
    pub success_rate_threshold: Option<f64>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Custom validator for limit parameter
fn validate_limit(limit: i64) -> Result<(), validator::ValidationError> {
    if limit < 1 || limit > 1000 {
        return Err(validator::ValidationError::new("range"));
    }
    Ok(())
}

/// Custom validator for offset parameter
fn validate_offset(offset: i64) -> Result<(), validator::ValidationError> {
    if offset < 0 {
        return Err(validator::ValidationError::new("range"));
    }
    Ok(())
}

/// Custom validator for success rate
fn validate_success_rate(rate: f64) -> Result<(), validator::ValidationError> {
    if rate < 0.0 || rate > 100.0 {
        return Err(validator::ValidationError::new("range"));
    }
    Ok(())
}

/// Custom validator for status
fn validate_status(status: &str) -> Result<(), validator::ValidationError> {
    if !matches!(status, "active" | "inactive" | "pending") {
        return Err(validator::ValidationError::new("invalid_status"));
    }
    Ok(())
}

/// List corridors with validation
pub async fn list_corridors(
    query: web::Query<ListCorridorsQuery>,
) -> Result<HttpResponse> {
    debug!("list_corridors called with query params");

    // Validate query parameters
    let limit = query.limit.unwrap_or(10);
    let offset = query.offset.unwrap_or(0);

    validators::validate_positive_int(limit, 1, 1000, "limit")
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    validators::validate_positive_int(offset, 0, i64::MAX, "offset")
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    // Validate asset_code if provided
    if let Some(asset_code) = &query.asset_code {
        validators::sanitize_string(asset_code, 50)
            .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

        validators::validate_alphanumeric_extended(asset_code)
            .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
    }

    // Validate success rate if provided
    if let Some(rate) = query.success_rate_min {
        validators::validate_float_range(rate, 0.0, 100.0, "success_rate_min")
            .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
    }

    if let Some(rate) = query.success_rate_max {
        validators::validate_float_range(rate, 0.0, 100.0, "success_rate_max")
            .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;
    }

    info!("Returning list of corridors with limit: {}, offset: {}", limit, offset);

    // Mock response
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "data": [],
        "total": 0,
        "limit": limit,
        "offset": offset
    })))
}

/// Get a specific corridor by ID
pub async fn get_corridor(
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id = path.into_inner();

    debug!("get_corridor called with id: {}", id);

    // Validate UUID format
    validators::validate_uuid(&id)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    info!("Fetching corridor with id: {}", id);

    // Mock response
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "id": id,
        "name": "Example Corridor",
        "description": "A sample corridor",
        "asset_code": "ASSET123",
        "success_rate_threshold": 95.5,
        "status": "active",
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    })))
}

/// Create a new corridor
pub async fn create_corridor(
    body: web::Json<CorridorRequest>,
) -> Result<HttpResponse> {
    debug!("create_corridor called");

    // Validate the entire request body
    body.validate()
        .map_err(|e| {
            let errors: Vec<FieldError> = e
                .field_errors()
                .iter()
                .map(|(field, errs)| FieldError {
                    field: field.to_string(),
                    code: errs.first().map(|e| e.code.to_string()).unwrap_or_default(),
                    message: format!("Field '{}' validation failed", field),
                })
                .collect();

            actix_web::error::ErrorBadRequest(
                serde_json::to_string(&serde_json::json!({
                    "error": "VALIDATION_FAILED",
                    "message": "Request validation failed",
                    "details": errors
                }))
                .unwrap_or_default(),
            )
        })?;

    // Additional sanitization
    let name = validators::sanitize_string(&body.name, 100)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    let asset_code = validators::sanitize_string(&body.asset_code, 20)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    // Validate asset code format
    validators::validate_alphanumeric_extended(&asset_code)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    info!("Creating new corridor: {}", name);

    // Mock response
    Ok(HttpResponse::Created().json(serde_json::json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "name": name,
        "description": body.description.as_ref().map(|d| validators::sanitize_string(d, 500).unwrap_or_default()),
        "asset_code": asset_code,
        "success_rate_threshold": body.success_rate_threshold,
        "status": body.status.as_deref().unwrap_or("pending"),
        "created_at": chrono::Utc::now().to_rfc3339(),
        "updated_at": chrono::Utc::now().to_rfc3339()
    })))
}

/// Update an existing corridor
pub async fn update_corridor(
    path: web::Path<String>,
    body: web::Json<CorridorRequest>,
) -> Result<HttpResponse> {
    let id = path.into_inner();

    debug!("update_corridor called with id: {}", id);

    // Validate UUID format
    validators::validate_uuid(&id)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    // Validate the request body
    body.validate()
        .map_err(|e| {
            let errors: Vec<FieldError> = e
                .field_errors()
                .iter()
                .map(|(field, errs)| FieldError {
                    field: field.to_string(),
                    code: errs.first().map(|e| e.code.to_string()).unwrap_or_default(),
                    message: format!("Field '{}' validation failed", field),
                })
                .collect();

            actix_web::error::ErrorBadRequest(
                serde_json::to_string(&serde_json::json!({
                    "error": "VALIDATION_FAILED",
                    "message": "Request validation failed",
                    "details": errors
                }))
                .unwrap_or_default(),
            )
        })?;

    let name = validators::sanitize_string(&body.name, 100)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    info!("Updating corridor: {} with name: {}", id, name);

    // Mock response
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "id": id,
        "name": name,
        "description": body.description.as_ref().map(|d| validators::sanitize_string(d, 500).unwrap_or_default()),
        "asset_code": validators::sanitize_string(&body.asset_code, 20).unwrap_or_default(),
        "success_rate_threshold": body.success_rate_threshold,
        "status": body.status.as_deref().unwrap_or("pending"),
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": chrono::Utc::now().to_rfc3339()
    })))
}

/// Delete a corridor
pub async fn delete_corridor(
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let id = path.into_inner();

    debug!("delete_corridor called with id: {}", id);

    // Validate UUID format
    validators::validate_uuid(&id)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    info!("Deleting corridor: {}", id);

    // Mock response
    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corridor_request_validation() {
        let request = CorridorRequest {
            name: "Test Corridor".to_string(),
            description: Some("Test Description".to_string()),
            asset_code: "TEST123".to_string(),
            success_rate_threshold: Some(95.5),
            status: Some("active".to_string()),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_corridor_request_validation_empty_name() {
        let request = CorridorRequest {
            name: "".to_string(),
            description: None,
            asset_code: "TEST123".to_string(),
            success_rate_threshold: None,
            status: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_validate_status_valid() {
        assert!(validate_status("active").is_ok());
        assert!(validate_status("inactive").is_ok());
        assert!(validate_status("pending").is_ok());
    }

    #[test]
    fn test_validate_status_invalid() {
        assert!(validate_status("unknown").is_err());
    }
}
