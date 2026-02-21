/// Comprehensive test suite for validation middleware and handlers
#[cfg(test)]
mod tests {
    use actix_web::{test, web, App, http::StatusCode};
    use api_validation::validators;
    use api_validation::errors::ValidationError;

    #[test]
    fn test_sanitize_string_basic() {
        let input = "  hello world  ";
        let result = validators::sanitize_string(input, 100);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
    }

    #[test]
    fn test_sanitize_string_exceeds_max_length() {
        let input = "this is a very long string";
        let result = validators::sanitize_string(input, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_string_null_bytes() {
        let input = "hello\0world";
        let result = validators::sanitize_string(input, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_string_sql_injection() {
        let input = "SELECT * FROM users WHERE id=1";
        let result = validators::sanitize_string(input, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_string_xss_attempt() {
        let input = "<script>alert('xss')</script>";
        let result = validators::sanitize_string(input, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_positive_int_valid() {
        let result = validators::validate_positive_int(50, 1, 100, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_positive_int_below_min() {
        let result = validators::validate_positive_int(0, 1, 100, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_positive_int_above_max() {
        let result = validators::validate_positive_int(101, 1, 100, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_string_length_valid() {
        let input = "valid";
        let result = validators::validate_string_length(input, 1, 100, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_string_length_too_short() {
        let input = "";
        let result = validators::validate_string_length(input, 1, 100, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_string_length_too_long() {
        let input = "a".repeat(101);
        let result = validators::validate_string_length(&input, 1, 100, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_float_range_valid() {
        let result = validators::validate_float_range(50.5, 0.0, 100.0, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_float_range_below_min() {
        let result = validators::validate_float_range(-0.1, 0.0, 100.0, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_float_range_above_max() {
        let result = validators::validate_float_range(100.1, 0.0, 100.0, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_uuid_valid() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let result = validators::validate_uuid(uuid);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_uuid_invalid() {
        let uuid = "not-a-uuid";
        let result = validators::validate_uuid(uuid);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_uuid_invalid_format() {
        let uuid = "550e8400e29b41d4a716446655440000";
        let result = validators::validate_uuid(uuid);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_email_valid() {
        let email = "test@example.com";
        let result = validators::validate_email(email);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        let email = "invalid.email";
        let result = validators::validate_email(email);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_email_missing_domain() {
        let email = "test@";
        let result = validators::validate_email(email);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_alphanumeric_extended_valid() {
        assert!(validators::validate_alphanumeric_extended("test_value-123").is_ok());
        assert!(validators::validate_alphanumeric_extended("TEST").is_ok());
        assert!(validators::validate_alphanumeric_extended("123-456_abc").is_ok());
    }

    #[test]
    fn test_validate_alphanumeric_extended_invalid() {
        assert!(validators::validate_alphanumeric_extended("test value").is_err());
        assert!(validators::validate_alphanumeric_extended("test@value").is_err());
        assert!(validators::validate_alphanumeric_extended("test.value").is_err());
    }

    #[test]
    fn test_error_response_invalid_limit() {
        let error = ValidationError::InvalidLimit("test".to_string());
        assert!(error.to_string().contains("limit"));
    }

    #[test]
    fn test_error_response_injection_attempt() {
        let error = ValidationError::InjectionAttempt("test".to_string());
        assert!(error.to_string().contains("Suspicious"));
    }

    #[test]
    fn multiple_injection_patterns() {
        let patterns = vec![
            "'; DROP TABLE users; --",
            "1' OR '1'='1",
            "<img src=x onerror=alert('xss')>",
            "javascript:alert('xss')",
        ];

        for pattern in patterns {
            let result = validators::sanitize_string(pattern, 500);
            assert!(
                result.is_err(),
                "Pattern '{}' should be detected as injection attempt",
                pattern
            );
        }
    }
}
