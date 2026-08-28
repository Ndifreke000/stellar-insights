# API Request Signing

## Overview

Request signing provides cryptographic protection against tampering and replay attacks for API calls. This document describes the request signing scheme used by Stellar Insights API.

## Signing Scheme

### Canonical Request Format

All requests are signed using an HMAC-SHA256 signature computed over a canonical request representation. The canonical form ensures that semantically equivalent requests produce the same signature, regardless of parameter order or representation differences.

**Canonical Request Format:**
```
METHOD
PATH
QUERY_PARAM_1=VALUE_1
QUERY_PARAM_2=VALUE_2
...
BODY_HASH
TIMESTAMP
NONCE
```

Where:
- **METHOD**: HTTP method in uppercase (GET, POST, PUT, DELETE, etc.)
- **PATH**: URL path component (e.g., `/api/v1/anchors`)
- **QUERY_PARAMS**: Query parameters sorted alphabetically by key, one per line in `key=value` format. Parameters are included only if present in the request.
- **BODY_HASH**: SHA-256 hash of the request body in hexadecimal format. For requests without a body (GET, DELETE), this is the hash of an empty byte string.
- **TIMESTAMP**: Unix timestamp (seconds since epoch) in decimal format
- **NONCE**: A unique identifier for this request (see Nonce section below)

### Signature Computation

1. Build the canonical request string as described above
2. Compute HMAC-SHA256 of the canonical request using the client's signing secret as the key
3. Encode the result as hexadecimal
4. Include the signature in the `X-Signature` header

### Example Canonical Request

For a POST to `/api/v1/data?format=json&limit=10` with body `{"key":"value"}` at timestamp 1692374400 with nonce `abc123`:

```
POST
/api/v1/data
format=json
limit=10
a63c9e789ea5de2e03c4326fa73f6d8c27bb67c2fa0a0d6d7ecfeeef9e9f5fbf
1692374400
abc123
```

## Request Headers

Signed requests must include three headers:

- **X-Signature**: The HMAC-SHA256 signature (hexadecimal encoded)
- **X-Timestamp**: The Unix timestamp used in the signature
- **X-Nonce**: A unique request identifier for replay protection

Example:
```
X-Signature: 8f5a3c9e8a2b1d7f6e4c9a8b7d6e5f4a3b2c1d0e9f8a7b6c5d4e3f2a1b0c9d
X-Timestamp: 1692374400
X-Nonce: 550e8400-e29b-41d4-a716-446655440000
```

## Clock Skew and Timestamp Validation

To prevent replay attacks while allowing for reasonable clock drift between client and server:

- Requests with timestamps outside a ±5 minute window from the server's current time are rejected
- A timestamp more than 5 minutes in the future or past is considered expired
- Clients should synchronize their clocks with NTP or similar services to ensure requests are within this window

## Nonce and Replay Protection

Each request must include a unique nonce in the `X-Nonce` header. The server tracks recently-used nonces and rejects any request attempting to reuse a nonce within the current clock-skew window.

**Nonce Requirements:**
- Must be unique per request
- Should not be predictable (use UUIDs or random identifiers)
- The server retains nonces for the duration of the clock-skew window (±5 minutes from when first used)
- Attempting to resubmit a request with the same nonce within this window results in rejection

**Example Nonce Generation:**
```rust
use uuid::Uuid;
let nonce = Uuid::new_v4().to_string();
```

## Secret Management

### Client Secret Storage

- Store signing secrets securely in your application's secret management system
- Never commit secrets to version control
- Rotate secrets regularly
- Support multiple concurrent secrets during rotation (new clients with old secret should still work)

### Server Configuration

The server expects signing secrets to be provided via environment variable or configuration. Each client is associated with exactly one active signing secret at any given time.

## Error Handling

All signature verification failures return a single, non-revealing error:
```
HTTP 401 Unauthorized
{
  "error": "Invalid request signature or headers"
}
```

This error message does not indicate which specific check failed (missing header, invalid signature, expired timestamp, or replayed nonce) to prevent information leakage to attackers.

## Implementation Guide

### For Client Libraries

1. **Build canonical request**: Format the request method, path, sorted query parameters, body hash, timestamp, and nonce
2. **Compute signature**: HMAC-SHA256 the canonical form with your signing secret
3. **Add headers**: Include X-Signature, X-Timestamp, and X-Nonce headers
4. **Send request**: Include the signed request headers with your HTTP request

### Example (Pseudocode)

```pseudocode
function sign_request(method, path, query_params, body, signing_secret):
    timestamp = current_unix_timestamp()
    nonce = generate_uuid()
    body_hash = sha256_hex(body)
    
    canonical = build_canonical_request(
        method, path, sorted(query_params), body_hash, timestamp, nonce
    )
    
    signature = hmac_sha256_hex(canonical, signing_secret)
    
    return {
        "X-Signature": signature,
        "X-Timestamp": timestamp,
        "X-Nonce": nonce
    }
```

## Security Considerations

### What This Protects

- **Tampering**: Any modification to method, path, query params, or body changes the signature and is detected
- **Replay Attacks**: The nonce + timestamp combination prevents exact replays within the clock-skew window
- **Man-in-the-Middle**: HTTPS (TLS) should still be used to prevent interception and signature stealing

### What This Does NOT Protect

- **Confidentiality**: The signing scheme does not provide encryption. Use HTTPS to protect request bodies in transit.
- **Long-term Replay**: Nonces are only tracked for the clock-skew window. Requests that arrive after this window may be accepted if replayed.

### Best Practices

1. Always use HTTPS/TLS in production
2. Synchronize client clocks with NTP for accurate timestamps
3. Use random, non-predictable nonces (UUIDs are recommended)
4. Rotate signing secrets periodically
5. Monitor for repeated signature validation failures, which may indicate an attack
6. Never log or expose signing secrets
