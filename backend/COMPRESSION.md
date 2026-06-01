# Response Compression

The API now supports automatic response compression using gzip and Brotli algorithms to reduce payload sizes and improve response times.

## Features

- **Automatic Compression**: Responses are automatically compressed based on the client's `Accept-Encoding` header
- **Multiple Algorithms**: Supports both gzip and Brotli compression
- **Configurable Threshold**: Only compresses responses larger than a configurable size (default: 1KB)
- **Smart Selection**: Automatically selects the best compression algorithm based on client support

## Configuration

Set the minimum response size for compression in your `.env` file:

```env
# Minimum response size in bytes to trigger compression (default: 1024)
COMPRESSION_MIN_SIZE=1024
```

## How It Works

1. Client sends request with `Accept-Encoding` header (e.g., `Accept-Encoding: gzip, br`)
2. Server processes the request and generates response
3. If response size > `COMPRESSION_MIN_SIZE`, compression is applied
4. Server selects best compression algorithm based on client support:
   - Brotli (br) - preferred for better compression ratios
   - Gzip - widely supported fallback
5. Response is sent with appropriate `Content-Encoding` header

## Client Usage

Most HTTP clients automatically handle compression. Examples:

### cURL
```bash
# Compression is automatic
curl -H "Accept-Encoding: gzip, br" https://api.example.com/api/corridors
```

### JavaScript (fetch)
```javascript
// Compression is automatic
fetch('https://api.example.com/api/corridors')
  .then(response => response.json())
  .then(data => console.log(data));
```

### Rust (reqwest)
```rust
// Compression is automatic
let response = reqwest::get("https://api.example.com/api/corridors")
    .await?
    .json::<Vec<Corridor>>()
    .await?;
```

## Performance Impact

- **Large responses** (>10KB): 60-80% size reduction, faster transfer times
- **Medium responses** (1-10KB): 40-60% size reduction
- **Small responses** (<1KB): Not compressed to avoid overhead

## Testing

Test compression with cURL:

```bash
# Request with gzip
curl -H "Accept-Encoding: gzip" -i https://api.example.com/api/corridors

# Request with Brotli
curl -H "Accept-Encoding: br" -i https://api.example.com/api/corridors

# Request both (server picks best)
curl -H "Accept-Encoding: gzip, br" -i https://api.example.com/api/corridors
```

Look for the `Content-Encoding` header in the response to confirm compression is active.

## Endpoints Affected

All API endpoints support compression:
- `/api/anchors` - Anchor listings
- `/api/corridors` - Corridor listings and details
- `/api/metrics` - Metrics data
- `/api/rpc/*` - RPC endpoints
- `/api/liquidity-pools` - Liquidity pool data
- `/api/trustlines` - Trustline data
- All other endpoints

## Troubleshooting

If compression isn't working:

1. Check client sends `Accept-Encoding` header
2. Verify response size exceeds `COMPRESSION_MIN_SIZE`
3. Check server logs for compression initialization message
4. Ensure `tower-http` compression features are enabled in `Cargo.toml`
