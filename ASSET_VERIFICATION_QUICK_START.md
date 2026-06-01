# Asset Verification System - Quick Start Guide

## Overview

The Asset Verification System provides secure API endpoints to verify Stellar assets, check their reputation, and report suspicious assets.

## API Endpoints

### 1. Verify an Asset

Verifies an asset and returns its complete verification status.

**Endpoint**: `GET /api/assets/verify/:code/:issuer`

**Example**:
```bash
curl http://localhost:8080/api/assets/verify/USDC/GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN
```

**Response**:
```json
{
  "asset_code": "USDC",
  "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
  "verification_status": "verified",
  "reputation_score": 85.0,
  "trust_indicators": {
    "stellar_expert_verified": true,
    "stellar_toml_verified": true,
    "anchor_registry_verified": false,
    "has_suspicious_reports": false
  },
  "toml_info": {
    "home_domain": "centre.io",
    "name": "USD Coin",
    "org_name": "Centre Consortium",
    "org_url": "https://centre.io"
  },
  "metrics": {
    "trustline_count": 50000,
    "transaction_count": 1000000,
    "total_volume_usd": 50000000.0
  },
  "last_verified_at": "2026-02-23T10:00:00Z"
}
```

### 2. Get Verification Details

Retrieves existing verification details without re-verifying.

**Endpoint**: `GET /api/assets/:code/:issuer/verification`

**Example**:
```bash
curl http://localhost:8080/api/assets/USDC/GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN/verification
```

### 3. List Verified Assets

Lists verified assets with optional filters.

**Endpoint**: `GET /api/assets/verified`

**Query Parameters**:
- `status` - Filter by status (verified, unverified, suspicious)
- `min_reputation` - Minimum reputation score (0-100)
- `limit` - Results per page (default: 50, max: 100)
- `offset` - Pagination offset (default: 0)

**Examples**:
```bash
# List all verified assets
curl http://localhost:8080/api/assets/verified

# List only verified assets with high reputation
curl "http://localhost:8080/api/assets/verified?status=verified&min_reputation=80"

# Paginated results
curl "http://localhost:8080/api/assets/verified?limit=20&offset=40"
```

**Response**:
```json
{
  "assets": [...],
  "total": 100,
  "limit": 50,
  "offset": 0
}
```

### 4. Report Suspicious Asset

Reports a suspicious or fraudulent asset.

**Endpoint**: `POST /api/assets/report`

**Request Body**:
```json
{
  "asset_code": "SCAM",
  "asset_issuer": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
  "report_type": "scam",
  "description": "This asset is impersonating USDC",
  "evidence_url": "https://example.com/proof",
  "reporter_account": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
}
```

**Report Types**:
- `suspicious` - General suspicious activity
- `scam` - Confirmed scam
- `impersonation` - Impersonating another asset
- `other` - Other issues

**Example**:
```bash
curl -X POST http://localhost:8080/api/assets/report \
  -H "Content-Type: application/json" \
  -d '{
    "asset_code": "SCAM",
    "asset_issuer": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "report_type": "scam",
    "description": "Impersonating legitimate USDC asset",
    "evidence_url": "https://example.com/proof"
  }'
```

**Response**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "pending",
  "message": "Report submitted successfully"
}
```

## Verification Status

### Verified
- Reputation score >= 60
- Less than 3 suspicious reports
- Indicates trustworthy asset

### Unverified
- Reputation score < 60
- Less than 3 suspicious reports
- Proceed with caution

### Suspicious
- 3 or more suspicious reports
- High risk - avoid interaction

## Reputation Score

**Scale**: 0-100 points

**Components**:
- Stellar Expert verified: 30 points
- stellar.toml verified: 30 points
- Anchor registry verified: 20 points
- Trustline count: up to 10 points
- Transaction count: up to 10 points

**Interpretation**:
- 80-100: Highly trusted
- 60-79: Trusted
- 40-59: Moderate trust
- 0-39: Low trust

## Error Responses

### 400 Bad Request
```json
{
  "error": "Invalid asset code",
  "message": "Asset code must be 1-12 characters"
}
```

### 404 Not Found
```json
{
  "error": "Not found",
  "message": "Asset verification not found. Use /verify endpoint to verify this asset."
}
```

### 500 Internal Server Error
```json
{
  "error": "Internal server error",
  "message": "Failed to verify asset"
}
```

## Input Validation

### Asset Code
- 1-12 alphanumeric characters
- Case-sensitive

### Asset Issuer
- Valid Stellar public key
- 56 characters starting with 'G'

### Description (Reports)
- 1-1000 characters
- Required for reports

### Evidence URL (Reports)
- Valid HTTP/HTTPS URL
- Optional

## Rate Limiting

All endpoints are rate-limited to prevent abuse. If you exceed the limit, you'll receive a 429 Too Many Requests response.

## Background Revalidation

Assets are automatically revalidated periodically (default: every 7 days). This ensures verification data stays current.

## Best Practices

1. **Cache Results**: Cache verification results on your end to reduce API calls
2. **Check Before Transactions**: Always verify assets before large transactions
3. **Report Suspicious Assets**: Help the community by reporting scams
4. **Monitor Status Changes**: Periodically re-check assets you interact with
5. **Use Filters**: Use the list endpoint with filters for efficient queries

## Integration Example (JavaScript)

```javascript
async function verifyAsset(assetCode, assetIssuer) {
  const response = await fetch(
    `http://localhost:8080/api/assets/verify/${assetCode}/${assetIssuer}`
  );
  
  if (!response.ok) {
    throw new Error('Verification failed');
  }
  
  const data = await response.json();
  
  if (data.verification_status === 'suspicious') {
    console.warn('WARNING: This asset has been reported as suspicious!');
    return false;
  }
  
  if (data.reputation_score < 60) {
    console.warn('CAUTION: This asset has a low reputation score');
  }
  
  return data;
}

// Usage
const asset = await verifyAsset('USDC', 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN');
console.log(`Asset status: ${asset.verification_status}`);
console.log(`Reputation: ${asset.reputation_score}/100`);
```

## Support

For issues or questions:
- Check the full documentation: `ASSET_VERIFICATION_COMPLETE.md`
- Review API implementation: `backend/src/api/asset_verification.rs`
- Run tests: `cargo test asset_verification`

---

**Quick Start Guide** | Asset Verification System | Stellar Insights
