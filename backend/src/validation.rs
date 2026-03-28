//! Request parameter validation to prevent invalid inputs (NaN, infinity, negative values, invalid ranges).

use crate::error::{ApiError, ApiResult};
use validator::Validate;

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
}
