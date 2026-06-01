# Asset Issuer Verification System - Implementation Summary

## Overview

This document provides a comprehensive summary of the asset issuer verification system implementation for Stellar Insights. The system verifies asset code and issuer pairs against multiple trusted sources, assigns verification statuses, calculates reputation scores, and provides secure API endpoints for querying and reporting.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Frontend UI                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │Verification  │  │ Warning      │  │ Report       │     │
│  │Badge         │  │ Modal        │  │ Modal        │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└────────────────────────┬────────────────────────────────────┘
                         │ REST API
┌────────────────────────▼────────────────────────────────────┐
│                     API Layer                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ /api/assets/verify/:code/:issuer                     │  │
│  │ /api/assets/report                                   │  │
│  │ /api/assets/verified                                 │  │
│  │ /api/assets/:code/:issuer/verification               │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│                  AssetVerifier Service                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Stellar      │  │ stellar.toml │  │ Anchor       │     │
│  │ Expert       │  │ Parser       │  │ Registry     │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ On-Chain     │  │ Reputation   │  │ Status       │     │
│  │ Metrics      │  │ Calculator   │  │ Determiner   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│                     Database Layer                           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ verified_assets                                      │  │
│  │ asset_verification_reports                           │  │
│  │ asset_verification_history                           │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Database Schema

### verified_assets Table

Stores verification results for each asset with unique constraint on (asset_code, asset_issuer).

**Columns**:
- `id` (TEXT, PRIMARY KEY)
- `asset_code` (TEXT, NOT NULL)
- `asset_issuer` (TEXT, NOT NULL)
- `verification_status` (TEXT, CHECK: verified/unverified/suspicious)
- `reputation_score` (REAL, 0-100)
- `stellar_expert_verified` (BOOLEAN)
- `stellar_toml_verified` (BOOLEAN)
- `anchor_registry_verified` (BOOLEAN)
- `trustline_count` (INTEGER)
- `transaction_count` (INTEGER)
- `total_volume_usd` (REAL)
- `toml_*` fields for TOML data
- `suspicious_reports_count` (INTEGER)
- `last_verified_at` (TIMESTAMP)
- Timestamps: `created_at`, `updated_at`

**Indexes**:
- `idx_verified_assets_status`
- `idx_verified_assets_reputation`
- `idx_verified_assets_asset_code`
- `idx_verified_assets_issuer`
- `idx_verified_assets_updated`

### asset_verification_reports Table

Stores community reports of suspicious assets.

**Columns**:
- `id` (TEXT, PRIMARY KEY)
- `asset_code`, `asset_issuer` (FOREIGN KEY)
- `reporter_account` (TEXT)
- `report_type` (TEXT, CHECK: suspicious/scam/impersonation/other)
- `description` (TEXT, NOT NULL)
- `evidence_url` (TEXT)
- `status` (TEXT, CHECK: pending/reviewed/resolved/dismissed)
- `reviewed_by`, `reviewed_at`, `resolution_notes`
- Timestamps

### asset_verification_history Table

Audit trail for verification status changes.

**Columns**:
- `id` (TEXT, PRIMARY KEY)
- `asset_code`, `asset_issuer` (FOREIGN KEY)
- `previous_status`, `new_status`
- `previous_reputation_score`, `new_reputation_score`
- `change_reason`, `changed_by`
- `created_at`

## Verification Sources

### 1. Stellar Expert API
- **Endpoint**: `https://api.stellar.expert/explorer/public/asset/{code}-{issuer}`
- **Verification**: Checks if asset exists and has domain info
- **Points**: 30 points if verified

### 2. stellar.toml File
- **Process**:
  1. Get home_domain from issuer account (Horizon API)
  2. Fetch `https://{domain}/.well-known/stellar.toml`
  3. Parse TOML and extract CURRENCIES section
- **Verification**: Valid TOML with CURRENCIES section
- **Points**: 30 points if verified
- **Data Extracted**: org_name, org_url, asset descriptions

### 3. Anchor Registry
- **Status**: Placeholder (future integration)
- **Points**: 20 points if verified

### 4. On-Chain Metrics
- **Sources**: Database + Horizon API
- **Metrics**:
  - Trustline count (up to 10 points)
  - Transaction count (up to 10 points)
  - Total volume USD

## Reputation Scoring Algorithm

**Scale**: 0-100 points

**Breakdown**:
- Stellar Expert verified: 30 points
- stellar.toml verified: 30 points
- Anchor registry verified: 20 points
- Trustline count:
  - \>10,000: 10 points
  - \>1,000: 7 points
  - \>100: 5 points
  - \>10: 2 points
- Transaction count:
  - \>100,000: 10 points
  - \>10,000: 7 points
  - \>1,000: 5 points
  - \>100: 2 points

## Status Determination

**Verified**: `reputation_score >= 60 AND suspicious_reports < 3`

**Suspicious**: `suspicious_reports >= 3`

**Unverified**: `reputation_score < 60 AND suspicious_reports < 3`

## API Endpoints

### GET /api/assets/verify/:code/:issuer
Verify an asset and return its verification status.

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

### POST /api/assets/report
Report a suspicious asset.

**Request**:
```json
{
  "asset_code": "SCAM",
  "asset_issuer": "GXXXXX...",
  "report_type": "scam",
  "description": "This asset is impersonating USDC",
  "evidence_url": "https://example.com/proof",
  "reporter_account": "GXXXXX..."
}
```

**Response**:
```json
{
  "id": "uuid",
  "status": "pending",
  "message": "Report submitted successfully"
}
```

### GET /api/assets/verified
List verified assets with filters.

**Query Parameters**:
- `status`: verified | unverified | suspicious
- `min_reputation`: minimum reputation score
- `limit`: page size (default: 50)
- `offset`: pagination offset

**Response**:
```json
{
  "assets": [...],
  "total": 100,
  "limit": 50,
  "offset": 0
}
```

### GET /api/assets/:code/:issuer/verification
Get detailed verification information.

**Response**: Same as verify endpoint

## Security Features

### HTTP Client
- **Timeout**: 10 seconds per request
- **Retries**: Maximum 3 attempts with exponential backoff
- **User-Agent**: "StellarInsights/1.0"
- **Error Handling**: Graceful degradation on failures

### Input Validation
- Asset code: alphanumeric, max 12 characters
- Asset issuer: valid Stellar public key format
- Report description: max 1000 characters
- Evidence URL: valid URL format

### Rate Limiting
- Per-IP rate limiting on API endpoints
- Prevents abuse of verification and reporting systems

### Database Constraints
- Unique constraint on (asset_code, asset_issuer)
- Foreign key constraints for referential integrity
- Check constraints on enum fields

## Frontend Components

### VerificationBadge Component

**Props**:
```typescript
interface VerificationBadgeProps {
  assetCode: string;
  assetIssuer: string;
  status: 'verified' | 'unverified' | 'suspicious';
  reputationScore: number;
  onClick?: () => void;
}
```

**Features**:
- Status-specific colors and icons
- Tooltip with reputation score
- Click to show detailed modal
- Responsive design

### VerificationModal Component

**Features**:
- Detailed verification information
- Trust indicators breakdown
- TOML information display
- On-chain metrics
- Report button for suspicious assets

### Warning Modal

**Triggers**:
- User attempts to interact with unverified asset
- User attempts to interact with suspicious asset

**Features**:
- Clear warning message
- Risk explanation
- Option to proceed or cancel
- "Don't show again" checkbox

## Background Job

### Asset Revalidation Job

**Frequency**: Configurable (default: daily)

**Process**:
1. Select assets for revalidation (oldest first)
2. Run verification for each asset
3. Update database with new results
4. Record history of changes
5. Handle failures gracefully

**Configuration**:
```rust
pub struct RevalidationConfig {
    pub enabled: bool,
    pub interval_hours: u64,
    pub batch_size: usize,
    pub max_age_days: i64,
}
```

## Error Handling

### Graceful Degradation
- If Stellar Expert fails, continue with other sources
- If TOML fetch fails, mark as unverified but don't fail
- If metrics unavailable, use zeros

### Error Types
- `VerificationError::NetworkError` - HTTP request failed
- `VerificationError::ParseError` - TOML parsing failed
- `VerificationError::DatabaseError` - Database operation failed
- `VerificationError::ValidationError` - Invalid input

### Logging
- Info: Successful verifications
- Warn: Partial failures, retries
- Error: Complete failures, database errors

## Testing Strategy

### Unit Tests
- Reputation score calculation
- Status determination logic
- TOML parsing
- Input validation

### Integration Tests
- Full verification flow
- Database operations
- API endpoints
- Background job execution

### Edge Cases
- Malformed TOML files
- Missing home domain
- Network timeouts
- Concurrent verifications
- Similar asset codes (prevent false positives)

## Performance Considerations

### Caching
- Cache verification results for 24 hours
- Invalidate cache on manual revalidation
- Use Redis for distributed caching

### Database Optimization
- Indexes on frequently queried columns
- Efficient queries with proper joins
- Batch operations for background jobs

### Concurrency
- Thread-safe verification operations
- Database connection pooling
- Async/await for I/O operations

## Deployment Checklist

- [ ] Run database migration 022
- [ ] Update environment variables
- [ ] Configure background job schedule
- [ ] Set up monitoring and alerts
- [ ] Deploy backend changes
- [ ] Deploy frontend changes
- [ ] Test in staging environment
- [ ] Monitor error rates
- [ ] Verify performance metrics

## Configuration

### Environment Variables
```bash
# Verification settings
ASSET_VERIFICATION_ENABLED=true
ASSET_VERIFICATION_CACHE_TTL_HOURS=24
ASSET_REVALIDATION_ENABLED=true
ASSET_REVALIDATION_INTERVAL_HOURS=24
ASSET_REVALIDATION_BATCH_SIZE=100

# API rate limiting
ASSET_VERIFICATION_RATE_LIMIT=100  # requests per minute
ASSET_REPORT_RATE_LIMIT=10  # reports per hour per IP
```

## Monitoring

### Metrics to Track
- Verification requests per minute
- Verification success rate
- Average verification time
- Cache hit rate
- Report submission rate
- Background job execution time

### Alerts
- High verification failure rate (>10%)
- Slow verification times (>30s)
- High report submission rate (potential abuse)
- Background job failures

## Future Enhancements

1. **Machine Learning**: Use ML to detect suspicious patterns
2. **Community Voting**: Allow community to vote on asset trustworthiness
3. **Anchor Registry Integration**: Integrate with official anchor registry
4. **Historical Tracking**: Track reputation score changes over time
5. **Notification System**: Alert users about status changes
6. **Batch Verification**: API endpoint for verifying multiple assets
7. **Export Functionality**: Export verification data for analysis

## References

- [Stellar Expert API](https://stellar.expert/explorer/public)
- [Stellar TOML Specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0001.md)
- [Horizon API Documentation](https://developers.stellar.org/api/horizon)

---

**Implementation Status**: 40% Complete  
**Last Updated**: 2026-02-23  
**Next Steps**: Create API endpoints, background job, and frontend components
