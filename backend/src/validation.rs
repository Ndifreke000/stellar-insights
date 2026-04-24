//! Request parameter validation to prevent invalid inputs (NaN, infinity, negative values, invalid ranges).

use crate::error::{ApiError, ApiResult};
use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Request},
    Json,
};
use validator::Validate;

// ── ValidatedJson extractor ───────────────────────────────────────────────────

/// Axum extractor that deserializes JSON and immediately runs `validator::Validate`.
/// Handlers use `ValidatedJson<T>` instead of `Json<T>` to get automatic validation.
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + Validate,
    S: Send + Sync,
    Json<T>: FromRequest<S>,
{
    type Rejection = axum::response::Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| e.into_response())?;
        validate_request(&value).map_err(|e| {
            use axum::response::IntoResponse;
            e.into_response()
        })?;
        Ok(ValidatedJson(value))
    }
}

impl<T> std::ops::Deref for ValidatedJson<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

/// Validates a single optional filter value: must be finite (no NaN/Infinity), and within [`min_allowed`, `max_allowed`].
#[inline]
fn validate_filter_f64(
    value: Option<f64>,
    min_allowed: f64,
    max_allowed: f64,
    param_name: &str,
) -> ApiResult<()> {
    let v = match value {
        None => return Ok(()),
        Some(x) => x,
    };
    if !v.is_finite() {
        return Err(ApiError::bad_request(
            "INVALID_PARAMETER",
            format!(
                "{} must be a finite number (got {}).",
                param_name,
                if v.is_nan() { "NaN" } else { "infinity" }
            ),
        ));
    }
    if v < min_allowed || v > max_allowed {
        return Err(ApiError::bad_request(
            "INVALID_PARAMETER",
            format!("{param_name} must be between {min_allowed} and {max_allowed} (got {v})."),
        ));
    }
    Ok(())
}

/// Validates corridor list query filter parameters.
/// - `success_rate_min/max`: finite, in [0, 100], and min <= max when both set.
/// - `volume_min/max`: finite, >= 0, and min <= max when both set.
pub fn validate_corridor_filters(
    success_rate_min: Option<f64>,
    success_rate_max: Option<f64>,
    volume_min: Option<f64>,
    volume_max: Option<f64>,
) -> ApiResult<()> {
    const SUCCESS_RATE_MIN: f64 = 0.0;
    const SUCCESS_RATE_MAX: f64 = 100.0;
    const VOLUME_MIN: f64 = 0.0;
    const VOLUME_MAX: f64 = 1e18;

    validate_filter_f64(
        success_rate_min,
        SUCCESS_RATE_MIN,
        SUCCESS_RATE_MAX,
        "success_rate_min",
    )?;
    validate_filter_f64(
        success_rate_max,
        SUCCESS_RATE_MIN,
        SUCCESS_RATE_MAX,
        "success_rate_max",
    )?;
    validate_filter_f64(volume_min, VOLUME_MIN, VOLUME_MAX, "volume_min")?;
    validate_filter_f64(volume_max, VOLUME_MIN, VOLUME_MAX, "volume_max")?;

    if let (Some(min), Some(max)) = (success_rate_min, success_rate_max) {
        if min > max {
            return Err(ApiError::bad_request(
                "INVALID_PARAMETER",
                "success_rate_min must be less than or equal to success_rate_max.",
            ));
        }
    }
    if let (Some(min), Some(max)) = (volume_min, volume_max) {
        if min > max {
            return Err(ApiError::bad_request(
                "INVALID_PARAMETER",
                "volume_min must be less than or equal to volume_max.",
            ));
        }
    }

    Ok(())
}

/// Validates any request struct that implements [`validator::Validate`].
/// Converts validation errors into a structured [`ApiError::BadRequest`].
pub fn validate_request<T: Validate>(req: &T) -> ApiResult<()> {
    req.validate().map_err(|e| {
        // Collect all field errors into a single readable message
        let messages: Vec<String> = e
            .field_errors()
            .into_iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |ve| {
                    let msg = ve
                        .message
                        .as_ref()
                        .map(|m| m.as_ref().to_string())
                        .unwrap_or_else(|| format!("invalid value for field '{field}'"));
                    format!("{field}: {msg}")
                })
            })
            .collect();
        ApiError::bad_request("VALIDATION_ERROR", messages.join("; "))
    })
}

/// Business-logic validation for `CreateCorridorRequest`: source and destination
/// asset pairs must not be identical.
pub fn validate_corridor_not_self_referential(
    source_code: &str,
    source_issuer: &str,
    dest_code: &str,
    dest_issuer: &str,
) -> ApiResult<()> {
    if source_code.eq_ignore_ascii_case(dest_code) && source_issuer == dest_issuer {
        return Err(ApiError::bad_request(
            "VALIDATION_ERROR",
            "Source and destination assets cannot be the same",
        ));
    }
    Ok(())
}

/// Business-logic validation for `CreateAnchorRequest`: stellar account must
/// start with 'G' (Ed25519 public key prefix on Stellar).
pub fn validate_stellar_account(account: &str) -> ApiResult<()> {
    if !account.starts_with('G') {
        return Err(ApiError::bad_request(
            "VALIDATION_ERROR",
            "Stellar account must be a valid public key starting with 'G'",
        ));
    }
    Ok(())
}

/// Validates Stellar address format (G followed by 55 base32 characters)
pub fn validate_stellar_address(address: &str) -> ApiResult<()> {
    if address.len() != 56 {
        return Err(ApiError::bad_request(
            "INVALID_ADDRESS",
            format!("Stellar address must be 56 characters (got {})", address.len()),
        ));
    }
    if !address.starts_with('G') {
        return Err(ApiError::bad_request(
            "INVALID_ADDRESS",
            "Stellar address must start with 'G'",
        ));
    }
    // Validate base32 characters (A-Z, 2-7)
    if !address[1..].chars().all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c)) {
        return Err(ApiError::bad_request(
            "INVALID_ADDRESS",
            "Stellar address contains invalid characters (must be A-Z, 2-7)",
        ));
    }
    Ok(())
}

/// Validates asset code format (1-12 alphanumeric characters)
pub fn validate_asset_code(code: &str) -> ApiResult<()> {
    if code.is_empty() || code.len() > 12 {
        return Err(ApiError::bad_request(
            "INVALID_ASSET_CODE",
            format!("Asset code must be 1-12 characters (got {})", code.len()),
        ));
    }
    if !code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ApiError::bad_request(
            "INVALID_ASSET_CODE",
            "Asset code must contain only alphanumeric characters",
        ));
    }
    Ok(())
}

/// Validates webhook URL (must be HTTPS and not internal/private IP)
pub fn validate_webhook_url(url: &str) -> ApiResult<()> {
    let parsed = url::Url::parse(url).map_err(|_| {
        ApiError::bad_request("INVALID_URL", "Invalid URL format")
    })?;
    
    // Must be HTTPS
    if parsed.scheme() != "https" {
        return Err(ApiError::bad_request(
            "INVALID_URL",
            "Webhook URL must use HTTPS scheme",
        ));
    }
    
    // Check for SSRF vulnerabilities - block private/internal IPs
    if let Some(host) = parsed.host_str() {
        // Block localhost
        if host == "localhost" || host == "127.0.0.1" || host == "::1" {
            return Err(ApiError::bad_request(
                "INVALID_URL",
                "Webhook URL cannot point to localhost",
            ));
        }
        
        // Block private IP ranges
        if host.starts_with("10.") 
            || host.starts_with("192.168.") 
            || host.starts_with("172.16.") 
            || host.starts_with("172.17.")
            || host.starts_with("172.18.")
            || host.starts_with("172.19.")
            || host.starts_with("172.20.")
            || host.starts_with("172.21.")
            || host.starts_with("172.22.")
            || host.starts_with("172.23.")
            || host.starts_with("172.24.")
            || host.starts_with("172.25.")
            || host.starts_with("172.26.")
            || host.starts_with("172.27.")
            || host.starts_with("172.28.")
            || host.starts_with("172.29.")
            || host.starts_with("172.30.")
            || host.starts_with("172.31.")
            || host.starts_with("169.254.") {
            return Err(ApiError::bad_request(
                "INVALID_URL",
                "Webhook URL cannot point to private IP ranges",
            ));
        }
    }
    
    Ok(())
}

/// Validates and sanitizes string input to prevent XSS and injection attacks
pub fn sanitize_string(input: &str, max_length: usize) -> ApiResult<String> {
    if input.len() > max_length {
        return Err(ApiError::bad_request(
            "INPUT_TOO_LONG",
            format!("Input exceeds maximum length of {} characters", max_length),
        ));
    }
    
    // Remove control characters and potential XSS vectors
    let sanitized: String = input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();
    
    // Check for SQL injection patterns
    let lower = sanitized.to_lowercase();
    let sql_patterns = ["drop table", "delete from", "insert into", "update ", "union select", "--", "/*", "*/", "xp_", "sp_"];
    for pattern in &sql_patterns {
        if lower.contains(pattern) {
            return Err(ApiError::bad_request(
                "INVALID_INPUT",
                "Input contains potentially malicious content",
            ));
        }
    }
    
    Ok(sanitized)
}

/// Validates JSON payload size
pub fn validate_payload_size(size: usize, max_size: usize) -> ApiResult<()> {
    if size > max_size {
        return Err(ApiError::bad_request(
            "PAYLOAD_TOO_LARGE",
            format!("Payload size {} exceeds maximum of {} bytes", size, max_size),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_corridor_filters_ok() {
        assert!(validate_corridor_filters(Some(95.0), Some(100.0), Some(1e5), Some(1e7)).is_ok());
        assert!(validate_corridor_filters(None, None, None, None).is_ok());
        assert!(validate_corridor_filters(Some(0.0), Some(100.0), Some(0.0), None).is_ok());
    }

    #[test]
    fn test_validate_corridor_filters_nan() {
        assert!(validate_corridor_filters(Some(f64::NAN), None, None, None).is_err());
        assert!(validate_corridor_filters(None, Some(f64::NAN), None, None).is_err());
        assert!(validate_corridor_filters(None, None, Some(f64::NAN), None).is_err());
        assert!(validate_corridor_filters(None, None, None, Some(f64::NAN)).is_err());
    }

    #[test]
    fn test_validate_corridor_filters_infinity() {
        assert!(validate_corridor_filters(Some(f64::INFINITY), None, None, None).is_err());
        assert!(validate_corridor_filters(None, None, Some(f64::NEG_INFINITY), None).is_err());
    }

    #[test]
    fn test_validate_corridor_filters_negative() {
        assert!(validate_corridor_filters(Some(-1.0), None, None, None).is_err());
        assert!(validate_corridor_filters(None, None, Some(-100.0), None).is_err());
    }

    #[test]
    fn test_validate_corridor_filters_success_rate_range() {
        assert!(validate_corridor_filters(Some(101.0), None, None, None).is_err());
        assert!(validate_corridor_filters(None, Some(150.0), None, None).is_err());
    }

    #[test]
    fn test_validate_corridor_filters_min_max_order() {
        assert!(validate_corridor_filters(Some(100.0), Some(95.0), None, None).is_err());
        assert!(validate_corridor_filters(None, None, Some(1e7), Some(1e5)).is_err());
    }

    #[test]
    fn test_validate_corridor_not_self_referential() {
        assert!(validate_corridor_not_self_referential(
            "USDC",
            "GISSUER123456789012345678901234567890123456789012345678",
            "XLM",
            "native"
        )
        .is_ok());
        assert!(
            validate_corridor_not_self_referential("USDC", "GISSUER1", "USDC", "GISSUER1").is_err()
        );
        // Case-insensitive code comparison
        assert!(
            validate_corridor_not_self_referential("usdc", "GISSUER1", "USDC", "GISSUER1").is_err()
        );
        // Different issuer = different asset, even if code matches
        assert!(
            validate_corridor_not_self_referential("USDC", "GISSUER1", "USDC", "GISSUER2").is_ok()
        );
    }

    #[test]
    fn test_validate_stellar_account() {
        assert!(validate_stellar_account(
            "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
        )
        .is_ok());
        assert!(validate_stellar_account(
            "ABCD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
        )
        .is_err());
        assert!(validate_stellar_account("").is_err());
    }

    #[test]
    fn test_validate_stellar_address() {
        assert!(validate_stellar_address(
            "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
        )
        .is_ok());
        assert!(validate_stellar_address("GBBD47IF6LWK7P7").is_err()); // Too short
        assert!(validate_stellar_address(
            "ABCD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
        )
        .is_err()); // Doesn't start with G
        assert!(validate_stellar_address(
            "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA!"
        )
        .is_err()); // Invalid character
    }

    #[test]
    fn test_validate_asset_code() {
        assert!(validate_asset_code("USDC").is_ok());
        assert!(validate_asset_code("XLM").is_ok());
        assert!(validate_asset_code("A").is_ok());
        assert!(validate_asset_code("ABCDEFGHIJKL").is_ok()); // 12 chars
        assert!(validate_asset_code("").is_err()); // Empty
        assert!(validate_asset_code("ABCDEFGHIJKLM").is_err()); // 13 chars
        assert!(validate_asset_code("USD$").is_err()); // Special char
    }

    #[test]
    fn test_validate_webhook_url() {
        assert!(validate_webhook_url("https://example.com/webhook").is_ok());
        assert!(validate_webhook_url("http://example.com/webhook").is_err()); // HTTP not allowed
        assert!(validate_webhook_url("https://localhost/webhook").is_err()); // Localhost blocked
        assert!(validate_webhook_url("https://127.0.0.1/webhook").is_err()); // Localhost IP
        assert!(validate_webhook_url("https://10.0.0.1/webhook").is_err()); // Private IP
        assert!(validate_webhook_url("https://192.168.1.1/webhook").is_err()); // Private IP
        assert!(validate_webhook_url("https://169.254.169.254/metadata").is_err()); // AWS metadata
    }

    #[test]
    fn test_sanitize_string() {
        assert_eq!(sanitize_string("Hello World", 100).unwrap(), "Hello World");
        assert_eq!(sanitize_string("Test\nLine", 100).unwrap(), "Test\nLine");
        assert!(sanitize_string("A".repeat(101).as_str(), 100).is_err()); // Too long
        assert!(sanitize_string("DROP TABLE users", 100).is_err()); // SQL injection
        assert!(sanitize_string("'; DELETE FROM anchors; --", 100).is_err()); // SQL injection
    }

    #[test]
    fn test_validate_payload_size() {
        assert!(validate_payload_size(1000, 10000).is_ok());
        assert!(validate_payload_size(10000, 10000).is_ok());
        assert!(validate_payload_size(10001, 10000).is_err());
    }
}
