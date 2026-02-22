use regex::Regex;
use crate::errors::ValidationError;

/// Sanitize string inputs to prevent injection attacks
pub fn sanitize_string(input: &str, max_length: usize) -> Result<String, ValidationError> {
    // Check length
    if input.len() > max_length {
        return Err(ValidationError::LengthError(format!(
            "Input exceeds maximum length of {}",
            max_length
        )));
    }

    // Check for null bytes
    if input.contains('\0') {
        return Err(ValidationError::InjectionAttempt(
            "Null bytes detected in input".to_string(),
        ));
    }

    // Check for common injection patterns
    if has_injection_patterns(input) {
        return Err(ValidationError::InjectionAttempt(
            "Suspicious patterns detected in input".to_string(),
        ));
    }

    // Trim whitespace
    Ok(input.trim().to_string())
}

/// Check for common injection patterns
fn has_injection_patterns(input: &str) -> bool {
    let suspicious_patterns = vec![
        r"(?i)(union|select|insert|update|delete|drop|create|alter).*(?i)(table|database|from)",
        r"(?i)<script[^>]*>",
        r"(?i)javascript:",
        r"(?i)onerror\s*=",
        r"(?i)onload\s*=",
        r#"(\r\n|\n|\r)"#,
        r"(?i)cmd\.exe|powershell|bash|sh\s+",
    ];

    for pattern in suspicious_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(input) {
                return true;
            }
        }
    }

    false
}

/// Validate a positive integer within bounds
pub fn validate_positive_int(
    value: i64,
    min: i64,
    max: i64,
    field_name: &str,
) -> Result<(), ValidationError> {
    if value < min || value > max {
        return Err(ValidationError::RangeError(format!(
            "{} must be between {} and {}, got {}",
            field_name, min, max, value
        )));
    }
    Ok(())
}

/// Validate a string field with length constraints
pub fn validate_string_length(
    value: &str,
    min: usize,
    max: usize,
    field_name: &str,
) -> Result<(), ValidationError> {
    if value.len() < min || value.len() > max {
        return Err(ValidationError::LengthError(format!(
            "{} length must be between {} and {}, got {}",
            field_name,
            min,
            max,
            value.len()
        )));
    }
    Ok(())
}

/// Validate a float within bounds
pub fn validate_float_range(
    value: f64,
    min: f64,
    max: f64,
    field_name: &str,
) -> Result<(), ValidationError> {
    if value < min || value > max {
        return Err(ValidationError::RangeError(format!(
            "{} must be between {} and {}, got {}",
            field_name, min, max, value
        )));
    }
    Ok(())
}

/// Validate UUID format
pub fn validate_uuid(id: &str) -> Result<(), ValidationError> {
    match uuid::Uuid::parse_str(id) {
        Ok(_) => Ok(()),
        Err(_) => Err(ValidationError::InvalidId(format!(
            "'{}' is not a valid UUID",
            id
        ))),
    }
}

/// Validate alphanumeric string (allowing hyphens and underscores)
pub fn validate_alphanumeric_extended(input: &str) -> Result<(), ValidationError> {
    let re = Regex::new(r"^[a-zA-Z0-9_-]+$")
        .map_err(|_| ValidationError::InvalidFormat("Invalid regex pattern".to_string()))?;

    if !re.is_match(input) {
        return Err(ValidationError::InvalidFormat(
            "Input must contain only alphanumeric characters, hyphens, and underscores".to_string(),
        ));
    }

    Ok(())
}

/// Validate email format
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    let re = Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
    )
    .map_err(|_| ValidationError::InvalidFormat("Invalid regex pattern".to_string()))?;

    if !re.is_match(email) {
        return Err(ValidationError::InvalidFormat(
            "'{}' is not a valid email format".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_string() {
        assert!(sanitize_string("valid_input", 100).is_ok());
        assert!(sanitize_string("input_with_spaces  ", 100).is_ok());
    }

    #[test]
    fn test_sanitize_string_too_long() {
        let result = sanitize_string("very long string", 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_injection_detection() {
        assert!(has_injection_patterns("SELECT * FROM users"));
        assert!(has_injection_patterns("<script>alert('xss')</script>"));
        assert!(!has_injection_patterns("normal_safe_input"));
    }

    #[test]
    fn test_validate_positive_int() {
        assert!(validate_positive_int(50, 1, 100, "test").is_ok());
        assert!(validate_positive_int(0, 1, 100, "test").is_err());
        assert!(validate_positive_int(101, 1, 100, "test").is_err());
    }

    #[test]
    fn test_validate_uuid() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(validate_uuid(valid_uuid).is_ok());
        assert!(validate_uuid("not-a-uuid").is_err());
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("invalid.email").is_err());
    }

    #[test]
    fn test_alphanumeric_extended() {
        assert!(validate_alphanumeric_extended("test_value-123").is_ok());
        assert!(validate_alphanumeric_extended("test value").is_err());
    }
}
