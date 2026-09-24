# API Versioning Strategy & Migration Guide

## 1. Overview

PayRaider uses semantic API versioning to evolve features while maintaining stability for existing clients, SDKs (TypeScript, Python), and third-party integrations. This document outlines the API versioning strategy, deprecation policy, version negotiation rules, and migration guidelines from v1 to v2.

---

## 2. API Version Lifecycle & Roadmap

| Version | Status | Release Date | Deprecation Notice | Sunset Date | Support Level |
|---------|--------|--------------|-------------------|-------------|---------------|
| **v1**  | Deprecated (Transition) | 2024-01-01 | 2026-06-30 | 2026-12-31 | Maintenance & Bugfixes |
| **v2**  | Active / Preview | 2026-07-01 | N/A | TBD | Full Support |

- **Current Stable Version:** `v1` (served as default for unversioned routes)
- **Successor Version:** `v2`
- **Deprecation Policy Period:** Minimum of **6 months** prior to sunsetting any major API version.

---

## 3. Version Negotiation Mechanisms

PayRaider supports two version negotiation methods, evaluated in strict order of precedence:

### 3.1 Precedence Rules

1. **URL Path Prefix (Highest Priority)**
   Directly specifies the API version in the URL path.
   - `/api/v1/corridors` -> Routes to API v1
   - `/api/v2/corridors` -> Routes to API v2

2. **Accept Header Content Negotiation (Secondary Priority)**
   Allows clients calling unversioned endpoints (e.g. `/api/corridors`) to negotiate the version using vendor-specific MIME types:
   - `Accept: application/vnd.payraider.v1+json` -> Negotiates API v1
   - `Accept: application/vnd.payraider.v2+json` -> Negotiates API v2

3. **Default Fallback (Lowest Priority)**
   Requests to `/api/*` without explicit path versioning or vendor Accept headers default to `v1`.

### 3.2 Response Headers

All responses include version metadata headers:
- `X-API-Version`: The resolved version (`v1` or `v2`)
- `X-API-Status`: Version lifecycle status (`supported`, `deprecated`, or `preview`)
- `Content-Type`: Matching negotiated media type (e.g. `application/vnd.payraider.v2+json` or `application/json`)

### 3.3 Unsupported Versions

Requests requesting unsupported versions return HTTP `400 Bad Request`:
```json
{
  "error": "unsupported_api_version",
  "message": "API version v3 is not supported",
  "supported_versions": ["v1", "v2"],
  "status": 400
}
```

---

## 4. Deprecation & Sunset Policy

In accordance with RFC 7231, RFC 8594 (Sunset header), and RFC 2145:

### 4.1 Notice Timeline
1. **Notice Phase (6 Months Prior to Sunset)**
   - Deprecation announced in `CHANGELOG.md` and release notes.
   - API endpoints emit standard deprecation headers.
2. **Deprecation Window**
   - Endpoints continue functioning without breaking schema changes.
   - Usage logged to identify unmigrated clients.
3. **Sunset Date**
   - Endpoints retired or return `410 Gone`.

### 4.2 Deprecation HTTP Headers on v1
All requests resolving to `v1` endpoints return:
```http
Deprecation: true
Sunset: Thu, 31 Dec 2026 00:00:00 GMT
Link: </api/v2/>; rel="successor-version"
Warning: 299 - "API v1 is deprecated. Please migrate to v2. See docs/API_VERSIONING.md"
```

---

## 5. Migration Guide: v1 → v2

### 5.1 Key Differences Between v1 and v2

1. **Envelope & Version Tagging:**
   v2 responses include `api_version: "v2"` and strict ISO-8601 UTC timestamps.

2. **Standardized Pagination:**
   - **v1:** `limit` and `offset` query parameters.
   - **v2:** Standardized `page` (1-indexed) and `page_size` parameters.

3. **Enhanced Latency & SLO Tracking:**
   v2 includes endpoint response latency metrics in debug headers (`Server-Timing`).

### 5.2 Code Examples

#### cURL (Version Negotiation)
```bash
# Requesting v2 representation on unversioned endpoint:
curl -H "Accept: application/vnd.payraider.v2+json" http://localhost:8080/api/corridors

# Calling explicit v1 endpoint (observe deprecation headers):
curl -i http://localhost:8080/api/v1/corridors
```

#### TypeScript / JavaScript
```typescript
// Migrate client initialization to v2
const response = await fetch("https://api.payraider.com/api/corridors", {
  headers: {
    "Accept": "application/vnd.payraider.v2+json",
    "Content-Type": "application/json"
  }
});
const data = await response.json();
```

#### Python
```python
import requests

# Using Accept header version negotiation
headers = {"Accept": "application/vnd.payraider.v2+json"}
response = requests.get("http://localhost:8080/api/corridors", headers=headers)
print(response.headers.get("X-API-Version"))  # 'v2'
```

---

## 6. Breaking Changes Tracking

All breaking changes between API versions are cataloged in `CHANGELOG.md` under dedicated `[Breaking Changes]` headings and tagged with affected endpoints and migration paths.

---

## 7. Verification

Verify the versioning behavior:
```bash
# 1. Verify docs present
cat docs/API_VERSIONING.md

# 2. Test content negotiation for v2
curl -i -H "Accept: application/vnd.payraider.v2+json" http://localhost:8080/api/corridors

# 3. Verify deprecation headers on v1
curl -i http://localhost:8080/api/v1/corridors
```
