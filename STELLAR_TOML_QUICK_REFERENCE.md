# Stellar.toml Quick Reference

Quick reference for working with stellar.toml (SEP-1) implementation.

## Key Concepts

- **stellar.toml**: Metadata file hosted at `https://domain/.well-known/stellar.toml`
- **SEP-1**: Stellar Ecosystem Proposal defining stellar.toml format
- **Caching**: 24-hour cache for successes, 1-hour for failures
- **SSRF Protection**: Strict domain validation prevents malicious requests

## API Usage

### Fetch Metadata

```rust
use stellar_insights_backend::services::stellar_toml::StellarTomlClient;

let client = StellarTomlClient::new(
    redis_connection,
    Some("Public Global Stellar Network ; September 2015".to_string()),
)?;

let toml = client.fetch_toml("stellar.org").await?;
```

### Background Refresh

```rust
tokio::spawn(async move {
    client.background_refresh("stellar.org").await
});
```

### Invalidate Cache

```rust
client.invalidate_cache("stellar.org").await?;
```

## Cache Keys

```
stellar_toml:{domain}
```

Examples:
- `stellar_toml:stellar.org`
- `stellar_toml:anchor.example.com`

## Cache TTLs

- Success: 24 hours (86,400 seconds)
- Failure: 1 hour (3,600 seconds)

## URL Format

```
https://{domain}/.well-known/stellar.toml
```

Fallback:
```
http://{domain}/.well-known/stellar.toml
```

## Metadata Fields

### Organization

```toml
ORGANIZATION_NAME = "Example Anchor"
ORGANIZATION_DBA = "Example DBA"
ORGANIZATION_URL = "https://example.com"
ORGANIZATION_LOGO = "https://example.com/logo.png"
ORGANIZATION_DESCRIPTION = "Description"
ORGANIZATION_SUPPORT_EMAIL = "support@example.com"
```

### Currencies

```toml
[[CURRENCIES]]
code = "USD"
issuer = "GXXXXXX..."
display_decimals = 2
name = "US Dollar"
desc = "USD stablecoin"
is_asset_anchored = true
anchor_asset_type = "fiat"
anchor_asset = "USD"
```

### Principals

```toml
[[PRINCIPALS]]
name = "Jane Doe"
email = "jane@example.com"
keybase = "janedoe"
twitter = "janedoe"
```

## Security

### Valid Domains

✅ `stellar.org`
✅ `anchor.example.com`
✅ `test-anchor.stellar.org`

### Invalid Domains

❌ `127.0.0.1` (IP address)
❌ `localhost` (localhost)
❌ `192.168.1.1` (private network)
❌ `10.0.0.1` (private network)
❌ `example..com` (path traversal)

## Error Handling

```rust
match client.fetch_toml(domain).await {
    Ok(toml) => {
        // Use metadata
    }
    Err(e) => {
        // Log and continue without metadata
        tracing::warn!("Failed to fetch: {}", e);
    }
}
```

## Frontend Usage

```tsx
import { AnchorCard } from '@/components/anchors/AnchorCard';

<AnchorCard anchor={anchor} />
```

## Testing

```bash
# Run tests
cargo test stellar_toml

# Test specific function
cargo test test_domain_validation

# With output
cargo test stellar_toml -- --nocapture
```

## Redis Commands

```bash
# Check cache
redis-cli GET "stellar_toml:stellar.org"

# Delete cache
redis-cli DEL "stellar_toml:stellar.org"

# List all stellar.toml keys
redis-cli KEYS "stellar_toml:*"

# Check TTL
redis-cli TTL "stellar_toml:stellar.org"
```

## Manual Testing

```bash
# Fetch stellar.toml
curl https://stellar.org/.well-known/stellar.toml

# Check if file exists
curl -I https://example.com/.well-known/stellar.toml

# Test with timeout
curl --max-time 10 https://example.com/.well-known/stellar.toml
```

## Common Issues

### No Metadata Showing

1. Check anchor has `home_domain`
2. Verify stellar.toml exists
3. Check Redis is running
4. Look for errors in logs

### Slow Responses

1. Enable Redis caching
2. Check network connectivity
3. Use background refresh

### SSRF Errors

1. Use valid domain format
2. Avoid IP addresses
3. Avoid localhost/private networks

## Performance

- Cache Hit: < 5ms
- Cache Miss (HTTPS): 100-500ms
- Cache Miss (HTTP fallback): 200-1000ms
- Parse Time: < 10ms

## Limits

- Request Timeout: 10 seconds
- Max Response Size: 1MB
- Max Redirects: 3
- Max Domain Length: 253 characters

## Logging

```rust
// Info level
tracing::info!("Fetched stellar.toml for {}", domain);

// Warning level
tracing::warn!("Failed to fetch for {}: {}", domain, error);

// Debug level
tracing::debug!("Cache hit for domain: {}", domain);
```

## Environment Variables

```bash
# Redis (required for caching)
REDIS_URL=redis://localhost:6379

# Network passphrase (optional)
STELLAR_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
```

## File Locations

### Backend
- Service: `backend/src/services/stellar_toml.rs`
- Models: `backend/src/models.rs`
- API: `backend/src/api/anchors.rs`
- Tests: `backend/tests/stellar_toml_test.rs`

### Frontend
- Types: `frontend/src/types/anchor.ts`
- Component: `frontend/src/components/anchors/AnchorCard.tsx`

## Resources

- [SEP-1 Specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0001.md)
- [Full Documentation](backend/STELLAR_TOML_IMPLEMENTATION.md)
- [Stellar Developer Docs](https://developers.stellar.org)
