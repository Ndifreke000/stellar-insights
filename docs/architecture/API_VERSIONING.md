# API Versioning Strategy

## Overview

PayRaider follows semantic versioning for API endpoints to ensure backward compatibility while enabling evolution of the API. This document outlines the versioning strategy, deprecation policy, and migration guidelines.

## Current API Versions

| Version | Status | Release Date | Sunset Date | Support Level |
|---------|--------|--------------|-------------|---------------|
| v1      | Active | 2024-01-01   | 2025-01-01  | Full Support  |
| v2      | Reserved | TBD        | TBD         | N/A           |

## Versioning Strategy

### URL-Based Versioning

All API endpoints are versioned via URL path:

```
/api/v1/anchors
/api/v2/anchors
```

### Content Negotiation (Accept Header)

Clients can also specify API version via the `Accept` header:

```bash
curl -H "Accept: application/vnd.payraider.v2+json" \
  http://localhost:8080/api/anchors
```

### Backward Compatibility

- Unversioned endpoints (`/api/anchors`) default to the latest stable version
- All v1 endpoints remain available during the support period
- Breaking changes are only introduced in new major versions

## Deprecation Policy

### Timeline

1. **Announcement Phase** (Month 1)
   - Breaking changes announced in release notes
   - Deprecation warnings added to API responses
   - Migration guide published

2. **Deprecation Phase** (Months 2-6)
   - Deprecated endpoints return `Deprecation` header
   - Clients have 6 months to migrate
   - Support team assists with migration

3. **Sunset Phase** (Month 7+)
   - Deprecated endpoints removed
   - Clients still using old version receive 410 Gone

### Deprecation Headers

Deprecated endpoints include:

```
Deprecation: true
Sunset: Wed, 01 Jan 2025 00:00:00 GMT
Link: </api/v2/anchors>; rel="successor-version"
```

## Migration Guide: v1 → v2

### Breaking Changes in v2

#### 1. Response Format Changes

**v1 Response:**
```json
{
  "id": "anchor-123",
  "name": "Anchor Name",
  "metrics": {
    "success_rate": 0.95
  }
}
```

**v2 Response:**
```json
{
  "id": "anchor-123",
  "name": "Anchor Name",
  "metrics": {
    "success_rate": 0.95,
    "p95_latency_ms": 250
  },
  "api_version": "v2"
}
```

#### 2. Endpoint Changes

| v1 Endpoint | v2 Endpoint | Change |
|-------------|-------------|--------|
| `GET /anchors` | `GET /anchors` | Added pagination required |
| `GET /corridors/:id` | `GET /corridors/:id` | Response schema updated |
| `POST /anchors` | `POST /anchors` | New required fields |

#### 3. Query Parameter Changes

**v1:**
```
GET /anchors?limit=10&offset=0
```

**v2:**
```
GET /anchors?page=1&page_size=10
```

### Migration Steps

1. **Update Client Code**
   ```typescript
   // Before (v1)
   const response = await fetch('/api/v1/anchors?limit=10&offset=0');
   
   // After (v2)
   const response = await fetch('/api/v2/anchors?page=1&page_size=10');
   ```

2. **Update Accept Headers**
   ```typescript
   const response = await fetch('/api/anchors', {
     headers: {
       'Accept': 'application/vnd.payraider.v2+json'
     }
   });
   ```

3. **Handle New Response Fields**
   ```typescript
   const data = await response.json();
   console.log(data.api_version); // "v2"
   console.log(data.metrics.p95_latency_ms); // New field
   ```

4. **Test Thoroughly**
   - Test with both v1 and v2 endpoints
   - Verify error handling
   - Check pagination behavior

## Version Negotiation

### Priority Order

1. **URL Path** (highest priority)
   ```
   /api/v2/anchors  → v2
   ```

2. **Accept Header**
   ```
   Accept: application/vnd.payraider.v2+json  → v2
   ```

3. **Default** (lowest priority)
   ```
   /api/anchors  → v1 (current stable)
   ```

## Error Handling

### Version Not Supported

```json
{
  "error": "unsupported_api_version",
  "message": "API version v3 is not supported",
  "supported_versions": ["v1", "v2"],
  "status": 400
}
```

### Deprecated Endpoint

```json
{
  "error": "deprecated_endpoint",
  "message": "This endpoint is deprecated. Use /api/v2/anchors instead",
  "sunset_date": "2025-01-01T00:00:00Z",
  "status": 200
}
```

## Tracking Breaking Changes

All breaking changes are documented in `CHANGELOG.md`:

```markdown
## [2.0.0] - 2025-01-01

### Breaking Changes
- Changed pagination from `limit/offset` to `page/page_size`
- Added required `api_version` field to all responses
- Removed deprecated `legacy_metrics` field

### Migration
See docs/API_VERSIONING.md for migration guide
```

## SLO Targets by Version

| Version | p50 Latency | p95 Latency | p99 Latency | Availability |
|---------|------------|------------|------------|--------------|
| v1      | 50ms       | 500ms      | 1000ms     | 99.9%        |
| v2      | 50ms       | 300ms      | 800ms      | 99.95%       |

## Testing Version Negotiation

### Test v1 Endpoint
```bash
curl -v http://localhost:8080/api/v1/anchors
# Should return v1 response format
```

### Test v2 Endpoint
```bash
curl -v http://localhost:8080/api/v2/anchors
# Should return v2 response format
```

### Test Accept Header
```bash
curl -H "Accept: application/vnd.payraider.v2+json" \
  http://localhost:8080/api/anchors
# Should return v2 response format
```

### Test Deprecation Warnings
```bash
curl -v http://localhost:8080/api/v1/anchors
# Should include Deprecation header
```

## Support and Questions

For API versioning questions:
- Check this documentation
- Review CHANGELOG.md for breaking changes
- Contact support@payraider.com
- Open an issue on GitHub

## References

- [Semantic Versioning](https://semver.org/)
- [API Versioning Best Practices](https://restfulapi.net/versioning/)
- [RFC 7231 - HTTP Semantics](https://tools.ietf.org/html/rfc7231)
