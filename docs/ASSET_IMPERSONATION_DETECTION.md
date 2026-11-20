# Asset Impersonation Detection

This document describes the asset impersonation detection system in PayRaider.

## Overview

PayRaider uses multiple techniques to detect asset impersonation and flag potentially malicious assets:

1. **Canonical Issuer Allowlist** - Maintains a list of known legitimate issuers for popular asset codes
2. **Lookalike Domain Detection** - Identifies domains that are visually similar (homoglyphs, punycode)
3. **Same-Code Different-Issuer Detection** - Flags assets that use popular codes with different issuers
4. **Trust Score Integration** - Incorporates impersonation risk into the overall verification score

## Detection Methods

### Canonical Issuer Allowlist

The system maintains a list of canonical issuers for popular asset codes:

| Asset Code | Canonical Issuer(s) | Source |
|------------|---------------------|--------|
| USDC | GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN | Circle (stellar.expert) |
| EURC | GDQO4YMT4JRVNQS4KGEc6ZbB5qPfB3Q5X7V7V7V7V7V7 | Circle (stellar.expert) |
| XLM | native | Stellar Foundation |

Assets with the same code but different issuers are flagged as potential impersonations.

### Lookalike Domain Detection

The system detects:

- **Homoglyph domains**: Using visually similar characters (e.g., `u5dc.com` vs `usdc.com`)
- **Punycode domains**: Internationalized domains that may appear legitimate
- **Edit distance**: Domains with small character differences

Example detections:
- `usdc.com` vs `u5dc.com` → Homoglyph similarity (100%)
- `circle.com` vs `circie.com` → Edit distance 93%
- `st3llar.foundation` vs `stellar.foundation` → Homoglyph detection

### Same-Code Different-Issuer Detection

When an asset uses a popular code (USDC, EURC, BTC, ETH) but is issued by an unknown account, it's flagged as high-risk impersonation.

## API Response

### Asset Verification with Impersonation Check

```json
{
  "asset_code": "USDC",
  "asset_issuer": "GUNKNOWN",
  "verified": true,
  "trust_indicators": {
    "stellar_expert_verified": true,
    "stellar_toml_verified": true,
    "anchor_registry_verified": false,
    "has_impersonation_risk": true,
    "risk_level": "high"
  },
  "impersonation_reports": [
    {
      "asset_code": "USDC",
      "asset_issuer": "GUNKNOWN",
      "report_type": "same_code_different_issuer",
      "risk_level": "high",
      "details": "Issuer GUNKNOWN is not a canonical issuer for USDC",
      "evidence": ["Canonical issuers for USDC: [GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN]"],
      "canonical_issuers": ["GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"]
    }
  ],
  "metrics": {
    "trustline_count": 15000,
    "transaction_count": 50000,
    "total_volume_usd": 2500000.0
  }
}
```

### Risk Levels

| Level | Description | Action Required |
|-------|-------------|-----------------|
| `low` | No impersonation detected | Proceed with standard checks |
| `medium` | Minor similarity detected | Review with caution |
| `high` | Significant impersonation risk | Verify before proceeding |
| `critical` | Confirmed impersonation attempt | Block or require admin override |

## Admin Workflow

### Admin Override

Admins can override impersonation flags with an audit trail:

```bash
POST /api/assets/canonical-issuers
Content-Type: application/json

{
  "asset_code": "USDC",
  "asset_issuer": "GUNKNOWN",
  "source": "manual_override",
  "verified": true
}
```

### Audit Log

All admin actions are logged:

| Field | Description |
|-------|-------------|
| `action` | `add_canonical_issuer`, `remove_canonical_issuer`, `override_risk` |
| `asset_code` | The affected asset code |
| `asset_issuer` | The affected issuer |
| `actor_account` | Admin account that made the change |
| `details` | Additional context about the action |
| `created_at` | Timestamp of the action |

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `IMPERSONATION_SIMILARITY_THRESHOLD` | 70 | Edit distance similarity threshold (0-100) |
| `IMPERSONATION_HOMOGLYPH_ENABLED` | true | Enable homoglyph detection |
| `IMPERSONATION_CANONICAL_ISSUERS_PATH` | (none) | Path to custom canonical issuer YAML file |

### Custom Canonical Issuers

Add custom canonical issuers via admin API or YAML config:

```yaml
# canonical_issuers.yaml
USDC:
  - GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN
  - GXYZ... # Custom issuer
EURC:
  - GDQO4YMT4JRVNQS4KGEc6ZbB5qPfB3Q5X7V7V7V7V7V7
```

## Testing

### Test Fixtures

The system includes test fixtures for homoglyph domains:

```rust
// Test cases for homoglyph detection
assert!(detector.is_homoglyph_similar("u5dc", "usdc"));  // 5=s
assert!(detector.is_homoglyph_similar("o", "0"));        // o=0
assert!(detector.is_homoglyph_similar("i", "l"));        // i=l
```

### Edge Cases Handled

- Empty strings and whitespace
- Unicode normalization
- Case-insensitive comparison
- Multi-hop paths with different issuers

## False Positive Mitigation

The system includes several safeguards against false positives:

1. **Multi-factor verification** - Impersonation risk is one of many factors
2. **Admin override capability** - False positives can be corrected
3. **Audit trail** - All overrides are logged for review
4. **Progressive risk levels** - Allows for nuanced decision making
5. **Configurable thresholds** - Can be adjusted based on deployment needs

## Integration with Corridor Data

Impersonation risk is incorporated into corridor selection:

1. Corridors using impersonated assets are flagged
2. Alternative corridors (without impersonation) are ranked higher
3. Admin overrides allow legitimate assets to bypass the flag
4. Users are warned about impersonation risk in corridor selection

## Security Considerations

- All impersonation reports are stored with audit trail
- Admin overrides require authentication
- Audit logs are immutable once created
- Risk levels are recalculated periodically
- Canonical issuer allowlist can be updated without system restart

## Future Enhancements

- [ ] Integration with Stellar Expert's asset reputation system
- [ ] Integration with Anchor directories
- [ ] Machine learning-based homoglyph detection
- [ ] Historical impersonation tracking
- [ ] User-reported impersonation verification workflow
- [ ] Automated suspension of confirmed scam assets