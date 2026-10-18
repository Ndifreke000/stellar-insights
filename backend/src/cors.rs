//! CORS configuration.
//!
//! Origins come from the `CORS_ALLOWED_ORIGINS` environment variable, a
//! comma-separated allow-list such as `https://app.example.com,http://localhost:3000`.
//! Methods and headers are restricted to what the API actually uses.
//!
//! A `*` entry mirrors whatever origin the browser sends, which defeats the
//! purpose of CORS, so it is only accepted when the caller explicitly opts in
//! (the server does so in mock/dev mode only) and, in that case, credentials
//! are never enabled.

use crate::request_id::{CORRELATION_ID_HEADER, REQUEST_ID_HEADER};
use anyhow::{bail, Result};
use axum::http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, Method,
};
use std::time::Duration;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Origin used when `CORS_ALLOWED_ORIGINS` is not set (local frontend dev server).
pub const DEFAULT_ALLOWED_ORIGINS: &str = "http://localhost:3000";

/// How long browsers may cache a preflight response.
const PREFLIGHT_MAX_AGE: Duration = Duration::from_secs(3600);

/// Parsed form of the `CORS_ALLOWED_ORIGINS` value.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ParsedOrigins {
    /// Valid, normalised origins (`scheme://host[:port]`), without duplicates.
    pub origins: Vec<String>,
    /// Entries that were not valid origins and were dropped.
    pub rejected: Vec<String>,
    /// Whether a bare `*` entry was present.
    pub wildcard: bool,
}

/// Splits and validates a `CORS_ALLOWED_ORIGINS` value.
#[must_use]
pub fn parse_allowed_origins(raw: &str) -> ParsedOrigins {
    let mut parsed = ParsedOrigins::default();
    for entry in raw.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        if entry == "*" {
            parsed.wildcard = true;
        } else if let Some(origin) = normalize_origin(entry) {
            if !parsed.origins.contains(&origin) {
                parsed.origins.push(origin);
            }
        } else {
            parsed.rejected.push(entry.to_string());
        }
    }
    parsed
}

/// Returns the canonical serialisation a browser would send in `Origin`, or
/// `None` if `entry` is not a bare http(s) origin. Paths, query strings and
/// credentials never appear in an `Origin` header, so an entry containing them
/// could never match and is treated as a misconfiguration.
fn normalize_origin(entry: &str) -> Option<String> {
    let url = url::Url::parse(entry).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.host_str()?;
    if url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    Some(url.origin().ascii_serialization())
}

/// Builds the CORS layer for `raw` (a `CORS_ALLOWED_ORIGINS` value).
///
/// Fails if `raw` contains `*` while `allow_wildcard` is false, so a production
/// deployment cannot start with an open CORS policy.
pub fn build_cors_layer(raw: &str, allow_wildcard: bool) -> Result<CorsLayer> {
    let parsed = parse_allowed_origins(raw);

    if parsed.wildcard && !allow_wildcard {
        bail!(
            "CORS: wildcard origin ('*') is not permitted in production. \
             Set CORS_ALLOWED_ORIGINS to a comma-separated list of the actual frontend origins, \
             e.g. https://payraider.com,https://app.payraider.com"
        );
    }

    for entry in &parsed.rejected {
        tracing::warn!(
            "CORS: ignoring invalid origin '{}' in CORS_ALLOWED_ORIGINS \
             (expected scheme://host[:port] with no path)",
            entry
        );
    }

    let layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            CONTENT_TYPE,
            AUTHORIZATION,
            REQUEST_ID_HEADER.clone(),
            CORRELATION_ID_HEADER.clone(),
        ])
        // Let browser clients read the IDs so they can be quoted in bug reports.
        .expose_headers([REQUEST_ID_HEADER.clone(), CORRELATION_ID_HEADER.clone()])
        .max_age(PREFLIGHT_MAX_AGE);

    if parsed.wildcard {
        tracing::warn!(
            "CORS: wildcard origin enabled (dev/mock mode only); \
             credentials are disabled and the request origin is mirrored"
        );
        return Ok(layer.allow_origin(AllowOrigin::mirror_request()));
    }

    let origins = parsed
        .origins
        .iter()
        .map(|origin| HeaderValue::from_str(origin))
        .collect::<Result<Vec<_>, _>>()?;

    if origins.is_empty() {
        tracing::warn!(
            "CORS: no valid origins in CORS_ALLOWED_ORIGINS='{}'; \
             all cross-origin requests will be rejected",
            raw
        );
    }
    for origin in &parsed.origins {
        tracing::info!("CORS: allowing origin '{}'", origin);
    }

    Ok(layer
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true))
}
