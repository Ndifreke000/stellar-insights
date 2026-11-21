use serde::{Deserialize, Serialize};

/// Supported API version identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiVersionId {
    V1,
    V2,
}

impl ApiVersionId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "v1",
            Self::V2 => "v2",
        }
    }

    #[must_use]
    pub const fn number(self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
        }
    }

    #[must_use]
    pub const fn from_number(n: u32) -> Option<Self> {
        match n {
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            _ => None,
        }
    }
}

/// Lifecycle state of an API version or specific route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionLifecycle {
    /// Active, supported API route (no deprecation headers).
    Current,
    /// Preview / successor API version (no deprecation headers).
    Preview,
    /// Deprecated route or version (emits RFC 9745 Deprecation + RFC 8594 Sunset headers).
    Deprecated,
    /// Retired route past sunset (returns 410 Gone when enforced).
    Sunset,
    /// Infrastructure or metadata route exempt from API version deprecation headers
    /// (e.g. `/health`, `/livez`, `/readyz`, `/api/version`, `/api/docs`).
    Unversioned,
}

impl VersionLifecycle {
    #[must_use]
    pub const fn as_header_value(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Preview => "preview",
            Self::Deprecated => "deprecated",
            Self::Sunset => "sunset",
            Self::Unversioned => "unversioned",
        }
    }
}

/// RFC 9745 (`Deprecation`) and RFC 8594 (`Sunset`) metadata for a deprecated route or version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeprecationNotice {
    /// Unix epoch timestamp (seconds) when deprecation took effect.
    /// Emitted in RFC 9745 Structured Field Date format: `@<unix_seconds>`.
    pub deprecated_at_unix: i64,
    /// RFC 8594 HTTP-date (IMF-fixdate) when the route/version will be removed,
    /// e.g. `"Thu, 31 Dec 2026 00:00:00 GMT"`.
    pub sunset_http_date: String,
    /// Optional Unix epoch timestamp corresponding to `sunset_http_date`,
    /// used when `enforce_sunset` is enabled to return `410 Gone`.
    pub sunset_at_unix: Option<i64>,
    /// Optional successor URI-reference emitted in `Link: <uri>; rel="successor-version"`.
    pub successor_uri: Option<String>,
    /// Optional documentation URI-reference emitted in `Link: <uri>; rel="deprecation"`.
    pub docs_uri: Option<String>,
    /// Optional human-readable warning text.
    pub warning_message: Option<String>,
    /// When `true` and the current time is at or past `sunset_at_unix`,
    /// requests are rejected with `410 Gone`.
    pub enforce_sunset: bool,
}

impl DeprecationNotice {
    /// Format the RFC 9745 `Deprecation` header value (`@<unix_timestamp>`).
    #[must_use]
    pub fn rfc9745_header_value(&self) -> String {
        format!("@{}", self.deprecated_at_unix)
    }
}

/// Route-table entry describing version & deprecation policy for a URL prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteVersionMetadata {
    /// Longest-prefix matched path prefix (e.g. `"/api/v1"`, `"/api/v2"`, `"/health"`).
    pub path_prefix: String,
    /// Resolved API version for requests matching this prefix, or `None` for unversioned routes.
    pub version: Option<ApiVersionId>,
    /// Lifecycle status for this route.
    pub lifecycle: VersionLifecycle,
    /// Whether `Accept: application/vnd.payraider.v<N>+json` negotiation can override the version.
    pub allow_accept_negotiation: bool,
    /// Deprecation metadata emitted when `lifecycle` is `Deprecated` or `Sunset`.
    pub deprecation: Option<DeprecationNotice>,
}

/// Top-level versioning policy configuration.
#[derive(Clone, Debug)]
pub struct VersioningConfig {
    pub current_version: u32,
    pub min_supported_version: u32,
    pub supported_versions: Vec<ApiVersionId>,
    pub routes: Vec<RouteVersionMetadata>,
}

/// `2026-06-30T00:00:00Z` in Unix seconds (v1 deprecation notice date).
pub const V1_DEPRECATION_UNIX: i64 = 1_782_777_600;
/// `2026-12-31T00:00:00Z` in Unix seconds (v1 sunset date).
pub const V1_SUNSET_UNIX: i64 = 1_798_675_200;
/// RFC 8594 IMF-fixdate string for v1 sunset.
pub const V1_SUNSET_HTTP_DATE: &str = "Thu, 31 Dec 2026 00:00:00 GMT";
/// RFC 3339 ISO-8601 timestamp for v1 sunset (for JSON metadata responses).
pub const V1_SUNSET_ISO8601: &str = "2026-12-31T00:00:00Z";
/// RFC 3339 ISO-8601 timestamp for v1 deprecation notice.
pub const V1_DEPRECATION_ISO8601: &str = "2026-06-30T00:00:00Z";

impl Default for VersioningConfig {
    fn default() -> Self {
        let v1_notice = DeprecationNotice {
            deprecated_at_unix: V1_DEPRECATION_UNIX,
            sunset_http_date: V1_SUNSET_HTTP_DATE.to_string(),
            sunset_at_unix: Some(V1_SUNSET_UNIX),
            successor_uri: Some("/api/v2/".to_string()),
            docs_uri: Some("/api/docs".to_string()),
            warning_message: Some(
                "API v1 is deprecated. Please migrate to v2. See docs/API_VERSIONING.md"
                    .to_string(),
            ),
            // Keep serving v1 until v2 reaches full feature parity; operators
            // flip this per-route or globally when retiring endpoints.
            enforce_sunset: false,
        };

        Self {
            current_version: 1,
            min_supported_version: 1,
            supported_versions: vec![ApiVersionId::V1, ApiVersionId::V2],
            routes: vec![
                // Operational, probe, documentation, and discovery routes are
                // unversioned and must never emit spurious deprecation headers.
                RouteVersionMetadata {
                    path_prefix: "/health".to_string(),
                    version: None,
                    lifecycle: VersionLifecycle::Unversioned,
                    allow_accept_negotiation: false,
                    deprecation: None,
                },
                RouteVersionMetadata {
                    path_prefix: "/livez".to_string(),
                    version: None,
                    lifecycle: VersionLifecycle::Unversioned,
                    allow_accept_negotiation: false,
                    deprecation: None,
                },
                RouteVersionMetadata {
                    path_prefix: "/readyz".to_string(),
                    version: None,
                    lifecycle: VersionLifecycle::Unversioned,
                    allow_accept_negotiation: false,
                    deprecation: None,
                },
                RouteVersionMetadata {
                    path_prefix: "/api/version".to_string(),
                    version: None,
                    lifecycle: VersionLifecycle::Unversioned,
                    allow_accept_negotiation: false,
                    deprecation: None,
                },
                RouteVersionMetadata {
                    path_prefix: "/api/docs".to_string(),
                    version: None,
                    lifecycle: VersionLifecycle::Unversioned,
                    allow_accept_negotiation: false,
                    deprecation: None,
                },
                RouteVersionMetadata {
                    path_prefix: "/swagger-ui".to_string(),
                    version: None,
                    lifecycle: VersionLifecycle::Unversioned,
                    allow_accept_negotiation: false,
                    deprecation: None,
                },
                RouteVersionMetadata {
                    path_prefix: "/ws".to_string(),
                    version: None,
                    lifecycle: VersionLifecycle::Unversioned,
                    allow_accept_negotiation: false,
                    deprecation: None,
                },
                RouteVersionMetadata {
                    path_prefix: "/graphql".to_string(),
                    version: None,
                    lifecycle: VersionLifecycle::Unversioned,
                    allow_accept_negotiation: false,
                    deprecation: None,
                },
                // Explicit v2 routes (active preview, no deprecation headers).
                RouteVersionMetadata {
                    path_prefix: "/api/v2".to_string(),
                    version: Some(ApiVersionId::V2),
                    lifecycle: VersionLifecycle::Preview,
                    allow_accept_negotiation: false,
                    deprecation: None,
                },
                // Explicit v1 routes (deprecated, RFC 9745 + RFC 8594 headers).
                RouteVersionMetadata {
                    path_prefix: "/api/v1".to_string(),
                    version: Some(ApiVersionId::V1),
                    lifecycle: VersionLifecycle::Deprecated,
                    allow_accept_negotiation: false,
                    deprecation: Some(v1_notice.clone()),
                },
                // Unversioned `/api/*` routes default to v1 but honor
                // `Accept: application/vnd.payraider.v<N>+json` negotiation.
                RouteVersionMetadata {
                    path_prefix: "/api".to_string(),
                    version: Some(ApiVersionId::V1),
                    lifecycle: VersionLifecycle::Deprecated,
                    allow_accept_negotiation: true,
                    deprecation: Some(v1_notice),
                },
            ],
        }
    }
}
