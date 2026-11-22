# API Versioning & Deprecation Policy

## 1. Overview

PayRaider uses a **single, route-metadata-driven versioning and deprecation policy** implemented in `backend/src/middleware/api_versioning.rs` (`ApiVersionPolicy` + `api_versioning_middleware`). All routes—whether versioned (`/api/v1/*`, `/api/v2/*`), unversioned (`/api/*`), or operational (`/health`, `/livez`, `/readyz`, `/api/version`, `/api/docs`)—evaluate against one authoritative table, ensuring consistent headers across every response and keeping `GET /api/version` in lockstep with runtime behavior.

---

## 2. API Version Lifecycle

| Version | Status | Release Date | Deprecation Date (RFC 9745) | Sunset Date (RFC 8594) | Support Level |
|---------|--------|--------------|-----------------------------|------------------------|---------------|
| **v1**  | `deprecated` | 2024-01-01 | `2026-06-30T00:00:00Z` (`@1782777600`) | `Thu, 31 Dec 2026 00:00:00 GMT` | Maintenance & Bugfixes |
| **v2**  | `preview` | 2026-07-01 | N/A | TBD | Active Development |

- **Current Default Version:** `v1` (fallback for unversioned `/api/*` routes)
- **Successor Version:** `v2`
- **Minimum Deprecation Notice Window:** **6 months** prior to sunsetting any major API version or public endpoint.

---

## 3. Version Resolution Precedence

Every incoming request is resolved in strict precedence order:

1. **Explicit URL Path Version (Highest Priority)**
   - `/api/v2/*` → resolves to `v2` (`X-API-Status: preview`, no deprecation headers).
   - `/api/v1/*` → resolves to `v1` (`X-API-Status: deprecated`, RFC 9745/8594 headers).
   - `/api/v<N>/*` for any unsupported version `N` (e.g. `/api/v3/...`, `/api/v10/...`) → rejected with `400 Bad Request` (`UNSUPPORTED_API_VERSION`).
   - Explicit URL prefixes take priority over any `Accept` header (e.g. `GET /api/v1/corridors` with `Accept: application/vnd.payraider.v2+json` still resolves to `v1`).

2. **Route Metadata Table (Longest Boundary-Aware Prefix Match)**
   - Specific route entries override broader prefixes (e.g. `/api/v1/legacy/report` overrides `/api/v1`).
   - Operational and discovery paths (`/health`, `/livez`, `/readyz`, `/api/version`, `/api/docs`, `/swagger-ui`, `/ws`, `/graphql`) are registered as `unversioned` and never emit deprecation headers.

3. **`Accept` Header Content Negotiation (Unversioned `/api/*` Routes Only)**
   - `Accept: application/vnd.payraider.v2+json` → resolves to `v2` (`preview`) and adds `Vary: Accept`.
   - `Accept: application/vnd.payraider.v1+json` → resolves to `v1` (`deprecated`) and adds `Vary: Accept`.
   - Any unsupported vendor version (e.g. `application/vnd.payraider.v3+json`) → rejected with `400 Bad Request`.

4. **Default Fallback**
   - Unversioned `/api/*` requests without a vendor `Accept` header default to `v1` and emit `Vary: Accept`.

---

## 4. Standard Response Headers

### 4.1 Version Metadata Headers

Every versioned API response includes:

- `X-API-Version`: resolved version (`v1` or `v2`)
- `X-API-Status`: lifecycle state (`current`, `preview`, `deprecated`, or `sunset`)
- `Vary: Accept`: included on negotiable unversioned routes so caches separate representations by version

### 4.2 Deprecation & Sunset Headers (RFC 9745 & RFC 8594)

Deprecated routes and versions emit standards-compliant headers:

```http
X-API-Version: v1
X-API-Status: deprecated
Deprecation: @1782777600
Sunset: Thu, 31 Dec 2026 00:00:00 GMT
Link: </api/v2/>; rel="successor-version", </api/docs>; rel="deprecation"
Warning: 299 - "API v1 is deprecated. Please migrate to v2. See docs/API_VERSIONING.md"
```

- **`Deprecation` (RFC 9745):** Structured Field Date (`@<unix-epoch-seconds>`) indicating when deprecation took effect (`@1782777600` = `2026-06-30T00:00:00Z`).
- **`Sunset` (RFC 8594):** IMF-fixdate HTTP-date indicating scheduled retirement (`Thu, 31 Dec 2026 00:00:00 GMT`).
- **`Link` (RFC 8288):** `rel="successor-version"` points to the replacement route/version; `rel="deprecation"` points to deprecation documentation.
- **Current / Preview Routes (`/api/v2/*` or negotiated `v2`):** never emit `Deprecation`, `Sunset`, or deprecation `Warning` headers.

---

## 5. Error Responses

All versioning rejections use the standard PayRaider `ErrorResponse` envelope:

### Unsupported Version (`400 Bad Request`)

```json
{
  "error": {
    "code": "UNSUPPORTED_API_VERSION",
    "message": "API version v3 is not supported",
    "details": {
      "requested_version": "v3",
      "supported_versions": ["v1", "v2"]
    }
  }
}
```

### Retired Endpoint Past Sunset (`410 Gone`)

When a route metadata entry sets `enforce_sunset: true` and the current UTC timestamp is at or past `sunset_at_unix`, the middleware returns `410 Gone` with `Sunset` and `Link: <...>; rel="successor-version"` headers:

```json
{
  "error": {
    "code": "ENDPOINT_SUNSET",
    "message": "This API endpoint reached its sunset date (Thu, 31 Dec 2026 00:00:00 GMT) and is no longer available",
    "details": {
      "path": "/api/v1/legacy/report",
      "sunset": "Thu, 31 Dec 2026 00:00:00 GMT",
      "successor": "/api/v2/reports"
    }
  }
}
```

---

## 6. Registering a Route Deprecation

Add a `RouteVersionMetadata` entry in `backend/src/models/api_versioning.rs` (`VersioningConfig::default()`) or attach one via `ApiVersionPolicy::with_route(...)`:

```rust
RouteVersionMetadata {
    path_prefix: "/api/v1/old-endpoint".to_string(),
    version: Some(ApiVersionId::V1),
    lifecycle: VersionLifecycle::Deprecated,
    allow_accept_negotiation: false,
    deprecation: Some(DeprecationNotice {
        deprecated_at_unix: 1_782_777_600,
        sunset_http_date: "Thu, 31 Dec 2026 00:00:00 GMT".to_string(),
        sunset_at_unix: Some(1_798_675_200),
        successor_uri: Some("/api/v2/new-endpoint".to_string()),
        docs_uri: Some("/api/docs".to_string()),
        warning_message: Some("Use /api/v2/new-endpoint instead.".to_string()),
        enforce_sunset: false,
    }),
}
```

---

## 7. Discovery Endpoint (`GET /api/version`)

`GET /api/version` reads directly from `ApiVersionPolicy::default().version_summary()` and returns:

```json
{
  "current": "v1",
  "supported": ["v1", "v2"],
  "deprecated": ["v1"],
  "deprecation_dates": {
    "v1": "2026-06-30T00:00:00Z"
  },
  "sunset_dates": {
    "v1": "2026-12-31T00:00:00Z"
  }
}
```

---

## 8. Verification

```bash
# 1. Deprecated v1 route emits RFC 9745 Deprecation (@timestamp) + RFC 8594 Sunset
curl -i http://localhost:8080/api/v1/corridors

# 2. Content negotiation for v2 omits deprecation headers and sets Vary: Accept
curl -i -H "Accept: application/vnd.payraider.v2+json" http://localhost:8080/api/corridors

# 3. Operational routes omit deprecation headers
curl -i http://localhost:8080/health

# 4. Unsupported version returns 400 with standard error envelope
curl -i -H "Accept: application/vnd.payraider.v3+json" http://localhost:8080/api/corridors
```
